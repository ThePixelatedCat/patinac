mod exprs;
#[cfg(test)]
mod test;
mod witnesses;

use std::{borrow::Cow, fmt::Display, iter, path::PathBuf};

use clap::ValueEnum;
use inkwell::{
    AddressSpace,
    builder::Builder,
    context::Context,
    module::Module,
    passes::PassBuilderOptions,
    targets::{FileType, InitializationConfig, Target, TargetMachine, TargetMachineOptions},
    types::{BasicType, BasicTypeEnum, FunctionType, StructType},
    values::{FunctionValue, PointerValue},
};
use slotmap::SecondaryMap;

use hir::{
    Hir, TyMap, VarId,
    exprs::ExprId,
    items::{AdtId, ExecKind},
    types::{Param, Ty},
};

#[derive(PartialEq, Eq)]
pub enum CodegenMode {
    IRDump,
    Emit(PathBuf),
    Silent,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[repr(u8)]
pub enum OptLevel {
    #[default]
    #[value(name = "0")]
    O0 = 0,
    #[value(name = "1")]
    O1 = 1,
    #[value(name = "2")]
    O2 = 2,
    #[value(name = "3")]
    O3 = 3,
}

impl Display for OptLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value().unwrap().get_name().fmt(f)
    }
}

impl OptLevel {
    pub fn opt_string(self) -> String {
        match self {
            //Self::O3 => "mem2reg,instcombine,reassociate,gvn,sccp,dce,simplifycfg",
            Self::O0 | Self::O1 | Self::O2 | Self::O3 => {
                format!("default<O{}>", self as u8)
            }
        }
    }
}

pub struct Codegen<'ctx, 'hir> {
    hir: &'hir Hir,
    ty_map: &'hir TyMap,
    ctx: &'ctx Context,
    builder: Builder<'ctx>,
    module: Module<'ctx>,
    target: TargetMachine,
    structs: SecondaryMap<AdtId, StructType<'ctx>>,
    funcs: SecondaryMap<VarId, FunctionValue<'ctx>>,
    vars: SecondaryMap<VarId, PointerValue<'ctx>>,
    printf: FunctionValue<'ctx>,
}

pub fn create_ctx() -> Context {
    Context::create()
}

impl<'ctx, 'hir> Codegen<'ctx, 'hir> {
    pub fn new(hir: &'hir Hir, ty_map: &'hir TyMap, ctx: &'ctx Context, module_name: &str) -> Self {
        let module = ctx.create_module(module_name);

        Target::initialize_native(&InitializationConfig::default()).unwrap();
        let triple = TargetMachine::get_default_triple();
        let target = Target::from_triple(&triple).unwrap();
        let target_machine = target
            .create_target_machine_from_options(&triple, TargetMachineOptions::default())
            .unwrap();

        let this = Self {
            printf: Self::printf(ctx, &module),
            hir,
            ty_map,
            ctx,
            builder: ctx.create_builder(),
            module,
            target: target_machine,
            structs: Self::build_structs(hir, ctx),
            funcs: SecondaryMap::new(),
            vars: SecondaryMap::new(),
        };
        this.populate_structs();
        this
    }

    fn printf(ctx: &'ctx Context, module: &Module<'ctx>) -> FunctionValue<'ctx> {
        let ty = ctx
            .i32_type()
            .fn_type(&[ctx.ptr_type(AddressSpace::default()).into()], true);
        module.add_function("printf", ty, None)
    }

    pub fn codegen(&mut self, opt_level: OptLevel, mode: CodegenMode) {
        for (adt, _) in self.hir.adts() {
            self.build_constructor(adt);
        }

        for exec in self.hir.execs() {
            match &exec.kind {
                ExecKind::Const { .. } => todo!("Constants"),
                ExecKind::Fn { .. } => {
                    self.build_func(exec.id);
                }
            }
        }

        if let Some(main) = self.hir.main() {
            let fn_ty = self.ctx.i32_type().fn_type(&[], false);
            let func = self.module.add_function("main", fn_ty, None);
            self.funcs.insert(main.id, func);

            let ExecKind::Fn { body, .. } = main.kind else {
                unreachable!("ICE")
            };

            let entry_block = self.ctx.append_basic_block(func, "entry");
            self.builder.position_at_end(entry_block);
            let _ = self.emit_expr(body);
            self.builder
                .build_return(Some(&self.ctx.i32_type().const_zero()))
                .unwrap();

            assert!(func.verify(true));
        }

        for exec in self.hir.execs() {
            match &exec.kind {
                ExecKind::Const { .. } => todo!("Constants"),
                ExecKind::Fn { params, body } => {
                    let Ty::Fn(_, ret_ty) = self.hir.var_ty(exec.id) else {
                        unreachable!("ICE")
                    };
                    self.populate_func(self.funcs[exec.id], params, ret_ty, *body);
                }
            }
        }

        self.module.verify().unwrap();

        self.module
            .set_data_layout(&self.target.get_target_data().get_data_layout());
        self.module.set_triple(&self.target.get_triple());

        self.module
            .run_passes(
                &opt_level.opt_string(),
                &self.target,
                PassBuilderOptions::create(),
            )
            .unwrap();

        match mode {
            CodegenMode::IRDump => self.module.print_to_stderr(),
            CodegenMode::Emit(path) => {
                self.target
                    .write_to_file(&self.module, FileType::Object, &path)
                    .unwrap();
            }
            CodegenMode::Silent => {}
        }
    }

    fn report_warning(&self, msg: impl Into<Cow<'static, str>>) {
        todo!("warnings");
    }

    fn build_structs(hir: &Hir, ctx: &'ctx Context) -> SecondaryMap<AdtId, StructType<'ctx>> {
        hir.adts()
            .map(|(id, ident)| (id, ctx.opaque_struct_type(&ident.ident.str())))
            .collect()
    }

    fn populate_structs(&self) {
        for (id, ty) in &self.structs {
            let field_tys: Vec<_> = (&self.hir.adt_info(id).fields)
                .into_iter()
                .map(|(_, ty)| self.lower_ty(ty))
                .collect();
            ty.set_body(&field_tys, false);
        }
    }

    fn build_constructor(&mut self, adt: AdtId) {
        let info = self.hir.adt_info(adt);

        let func = self.build_func(info.constructor_id);
        let entry_block = self.ctx.append_basic_block(func, "entry");
        self.builder.position_at_end(entry_block);

        let ty = self.lower_ty(&Ty::Adt(adt));
        let out_ptr = func.get_first_param().unwrap().into_pointer_value();
        for (idx, (arg, field_ty)) in
            iter::zip(func.get_param_iter().skip(1), info.fields.tys()).enumerate()
        {
            let field_ptr = self
                .builder
                .build_struct_gep(
                    ty,
                    out_ptr,
                    u32::try_from(idx).unwrap(),
                    &format!("field{idx}"),
                )
                .unwrap();
            self.emit_copy(field_ty, arg, field_ptr);
        }

        self.builder.build_return(None).unwrap();

        assert!(func.verify(true));
    }

    fn build_func(&mut self, id: VarId) -> FunctionValue<'ctx> {
        let Ty::Fn(params, ret_ty) = self.hir.var_ty(id) else {
            unreachable!("ICE")
        };
        let func = self.module.add_function(
            &self.hir.var_info(id).ident.str(),
            self.build_func_ty(params, ret_ty),
            None,
        );
        self.funcs.insert(id, func);
        func
    }

    fn build_func_ty(&self, params: &[Param], ret_ty: &Ty) -> FunctionType<'ctx> {
        let mut param_tys: Vec<_> = params
            .iter()
            .map(|p| {
                if p.mutable || is_indirect(&p.ty) {
                    self.ptr_ty()
                } else {
                    self.lower_ty(&p.ty)
                }
                .into()
            })
            .collect();

        if is_indirect(ret_ty) {
            param_tys.insert(0, self.ptr_ty().into());
            self.ctx.void_type().fn_type(&param_tys, false)
        } else {
            self.lower_ty(ret_ty).fn_type(&param_tys, false)
        }
    }

    fn populate_func(
        &mut self,
        func: FunctionValue<'ctx>,
        params: &[VarId],
        ret_ty: &Ty,
        body: ExprId,
    ) {
        let entry_block = self.ctx.append_basic_block(func, "entry");
        self.builder.position_at_end(entry_block);

        // Skip the first argument if it's an out-pointer
        let offset = if is_indirect(ret_ty) { 1 } else { 0 };
        for (arg, param) in iter::zip(func.get_param_iter().skip(offset), params) {
            let ty = self.hir.var_ty(*param);
            if self.hir.var_info(*param).mutable || is_indirect(ty) {
                self.vars.insert(*param, arg.into_pointer_value());
            } else {
                let ptr = self.emit_alloca(arg.get_type(), &self.hir.var_info(*param).ident.str());
                self.builder.build_store(ptr, arg).unwrap();
                self.vars.insert(*param, ptr);
            }
        }

        let body = self.emit_expr(body);

        if is_indirect(ret_ty) {
            let out_ptr = func.get_first_param().unwrap().into_pointer_value();
            self.emit_move(ret_ty, body.into_pointer_value(), out_ptr);
            self.builder.build_return(None).unwrap();
        } else {
            self.builder.build_return(Some(&body)).unwrap();
        }

        assert!(func.verify(true));
    }

    fn lower_ty(&self, ty: &Ty) -> BasicTypeEnum<'ctx> {
        match ty {
            Ty::Int | Ty::UInt => self.ctx.i64_type().as_basic_type_enum(),
            Ty::Byte => self.ctx.i8_type().as_basic_type_enum(),
            Ty::Float => self.ctx.f64_type().as_basic_type_enum(),
            Ty::Char => todo!(),
            Ty::Bool => self.ctx.bool_type().as_basic_type_enum(),
            Ty::Tuple(inner_tys) => {
                let inner_tys: Vec<_> = inner_tys.iter().map(|ty| self.lower_ty(ty)).collect();
                self.ctx.struct_type(&inner_tys, false).as_basic_type_enum()
            }
            Ty::Array(_) => todo!(),
            Ty::Fn(_, _) => todo!(),
            Ty::Adt(id) => self.structs[*id].as_basic_type_enum(),
        }
    }

    fn ptr_ty(&self) -> BasicTypeEnum<'ctx> {
        self.ctx
            .ptr_type(AddressSpace::default())
            .as_basic_type_enum()
    }

    fn emit_alloca(&self, ty: BasicTypeEnum<'ctx>, name: &str) -> PointerValue<'ctx> {
        self.builder.build_alloca(ty, name).unwrap()
    }

    fn emit_alloca_entry(&self, ty: BasicTypeEnum<'ctx>, name: &str) -> PointerValue<'ctx> {
        let curr_block = self.builder.get_insert_block().unwrap();
        let head_block = curr_block
            .get_parent()
            .unwrap()
            .get_first_basic_block()
            .unwrap();

        self.builder.position_at_end(head_block);
        let ptr = self.emit_alloca(ty, name);
        self.builder.position_at_end(curr_block);
        ptr
    }

    fn emit_move(&self, ty: &Ty, from: PointerValue<'ctx>, to: PointerValue<'ctx>) {
        let ty = self.lower_ty(ty);
        let size = ty.size_of().unwrap();
        let align = self.target.get_target_data().get_abi_alignment(&ty);

        self.builder
            .build_memmove(to, align, from, align, size)
            .unwrap();
    }

    fn curr_function(&self) -> FunctionValue<'ctx> {
        self.builder
            .get_insert_block()
            .unwrap()
            .get_parent()
            .unwrap()
    }

    fn is_trivial(&self, ty: &Ty) -> bool {
        match ty {
            Ty::Int | Ty::UInt | Ty::Byte | Ty::Float | Ty::Char | Ty::Bool => true,
            Ty::Array(_) => false,
            Ty::Fn(_, _) => todo!(),
            Ty::Tuple(inner) => inner.iter().all(|ty| self.is_trivial(ty)),
            Ty::Adt(id) => (&self.hir.adt_info(*id).fields)
                .into_iter()
                .all(|(_, ty)| self.is_trivial(ty)),
        }
    }
}

const fn is_indirect(ty: &Ty) -> bool {
    match ty {
        Ty::Int | Ty::UInt | Ty::Byte | Ty::Float | Ty::Char | Ty::Bool => false,
        Ty::Array(_) | Ty::Fn(_, _) | Ty::Adt(_) => true,
        Ty::Tuple(inner) => inner.len() > 1,
    }
}
