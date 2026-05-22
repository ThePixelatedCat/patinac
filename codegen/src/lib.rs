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
    types::{AnyType, AnyTypeEnum, BasicType, BasicTypeEnum, StructType},
    values::{FunctionValue, PointerValue},
};
use slotmap::SecondaryMap;

use hir::{
    Hir, TyMap, VarId,
    exprs::ExprId,
    items::{AdtId, ExecKind},
    types::Ty,
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
    structs: SecondaryMap<AdtId, StructType<'ctx>>,
    funcs: SecondaryMap<VarId, FunctionValue<'ctx>>,
    vars: SecondaryMap<VarId, AllocInfo<'ctx>>,
    printf: FunctionValue<'ctx>,
}

#[derive(Clone, Copy)]
struct AllocInfo<'ctx> {
    ptr: PointerValue<'ctx>,
    ty: AnyTypeEnum<'ctx>,
}

pub fn create_ctx() -> Context {
    Context::create()
}

impl<'ctx, 'hir> Codegen<'ctx, 'hir> {
    pub fn new(hir: &'hir Hir, ty_map: &'hir TyMap, ctx: &'ctx Context, module_name: &str) -> Self {
        let module = ctx.create_module(module_name);
        let this = Self {
            printf: Self::printf(ctx, &module),
            hir,
            ty_map,
            ctx,
            builder: ctx.create_builder(),
            module,
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
        for exec in self.hir.execs() {
            match &exec.kind {
                ExecKind::Const { .. } => todo!("Constants"),
                ExecKind::Fn { params, .. } => {
                    let Ty::Fn(_, ret_ty) = self.hir.var_ty(exec.id) else {
                        unreachable!("ICE")
                    };
                    let func = self.build_func(exec.id, params, ret_ty);
                    self.funcs.insert(exec.id, func);
                    self.vars.insert(
                        exec.id,
                        AllocInfo {
                            ptr: func.as_global_value().as_pointer_value(),
                            ty: func.get_type().as_any_type_enum(),
                        },
                    );
                }
            }
        }

        if let Some(main) = self.hir.main() {
            let fn_ty = self.ctx.i32_type().fn_type(&[], false);
            let func = self.module.add_function("main", fn_ty, None);
            self.funcs.insert(main.id, func);
            self.vars.insert(
                main.id,
                AllocInfo {
                    ptr: func.as_global_value().as_pointer_value(),
                    ty: func.get_type().as_any_type_enum(),
                },
            );

            let ExecKind::Fn { body, .. } = main.kind else {
                unreachable!("ICE")
            };

            let entry_block = self.ctx.append_basic_block(func, "entry");
            self.builder.position_at_end(entry_block);
            let _ = self.codegen_expr(body);
            self.builder
                .build_return(Some(&self.ctx.i32_type().const_zero()))
                .unwrap();

            assert!(func.verify(true));
        }

        for exec in self.hir.execs() {
            match &exec.kind {
                ExecKind::Const { .. } => todo!("Constants"),
                ExecKind::Fn { params, body } => {
                    let func = self.funcs[exec.id];
                    self.populate_func(func, params, *body);
                }
            }
        }

        self.module.verify().unwrap();

        Target::initialize_native(&InitializationConfig::default()).unwrap();
        let triple = TargetMachine::get_default_triple();
        let target = Target::from_triple(&triple).unwrap();
        let target_machine = target
            .create_target_machine_from_options(&triple, TargetMachineOptions::default())
            .unwrap();

        self.module
            .set_data_layout(&target_machine.get_target_data().get_data_layout());
        self.module.set_triple(&triple);

        self.module
            .run_passes(
                &opt_level.opt_string(),
                &target_machine,
                PassBuilderOptions::create(),
            )
            .unwrap();

        match mode {
            CodegenMode::IRDump => self.module.print_to_stderr(),
            CodegenMode::Emit(path) => {
                target_machine
                    .write_to_file(&self.module, FileType::Object, &path)
                    .unwrap();
            }
            CodegenMode::Silent => {}
        }
    }

    fn report_warning(&self, msg: impl Into<Cow<'static, str>>) {
        drop(msg);
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
                .map(|(_, ty)| self.convert_ty(ty))
                .collect();
            ty.set_body(&field_tys, false);
        }
    }

    fn convert_ty(&self, ty: &Ty) -> BasicTypeEnum<'ctx> {
        match ty {
            Ty::Int | Ty::UInt => self.ctx.i64_type().as_basic_type_enum(),
            Ty::Byte => self.ctx.i8_type().as_basic_type_enum(),
            Ty::Float => self.ctx.f64_type().as_basic_type_enum(),
            Ty::Char => todo!(),
            Ty::Bool => self.ctx.bool_type().as_basic_type_enum(),
            Ty::Tuple(inner_tys) => {
                let inner_tys: Vec<_> = inner_tys.iter().map(|ty| self.convert_ty(ty)).collect();
                self.ctx.struct_type(&inner_tys, false).as_basic_type_enum()
            }
            Ty::Array(_) => todo!(),
            Ty::Fn(_, _) => todo!(),
            Ty::Adt(id) => self.structs[*id].as_basic_type_enum(),
        }
    }

    fn build_func(&self, id: VarId, params: &[VarId], ret_ty: &Ty) -> FunctionValue<'ctx> {
        let param_tys: Vec<_> = params
            .iter()
            .map(|p| {
                if self.hir.var_info(*p).mutable {
                    self.ctx
                        .ptr_type(AddressSpace::default())
                        .as_basic_type_enum()
                } else {
                    self.convert_ty(self.hir.var_ty(*p))
                }
                .into()
            })
            .collect();
        let ret_ty = self.convert_ty(ret_ty);
        let fn_ty = ret_ty.fn_type(&param_tys, false);

        let fn_name = self.hir.var_info(id).ident.str();
        self.module.add_function(&fn_name, fn_ty, None)
    }

    fn populate_func(&mut self, function: FunctionValue<'ctx>, params: &[VarId], body: ExprId) {
        let entry_block = self.ctx.append_basic_block(function, "entry");
        self.builder.position_at_end(entry_block);

        for (param, arg) in iter::zip(params, function.get_param_iter()) {
            let info = if self.hir.var_info(*param).mutable {
                AllocInfo {
                    ptr: arg.into_pointer_value(),
                    ty: self.convert_ty(self.hir.var_ty(*param)).as_any_type_enum(),
                }
            } else {
                let ty = arg.get_type();
                let ptr = self
                    .builder
                    .build_alloca(ty, &self.hir.var_info(*param).ident.str())
                    .unwrap();
                self.builder.build_store(ptr, arg).unwrap();
                AllocInfo {
                    ptr,
                    ty: ty.as_any_type_enum(),
                }
            };

            self.vars.insert(*param, info);
        }

        let body = self.codegen_expr(body);
        self.builder.build_return(Some(&body)).unwrap();

        assert!(function.verify(true));
    }

    fn alloca(&self, ty: BasicTypeEnum<'ctx>, name: &str) -> AllocInfo<'ctx> {
        let curr_block = self.builder.get_insert_block().unwrap();
        let head_block = curr_block
            .get_parent()
            .unwrap()
            .get_first_basic_block()
            .unwrap();

        self.builder.position_at_end(head_block);
        let ptr = self.builder.build_alloca(ty, name).unwrap();
        self.builder.position_at_end(curr_block);
        AllocInfo {
            ptr,
            ty: ty.as_any_type_enum(),
        }
    }

    fn curr_function(&self) -> FunctionValue<'ctx> {
        self.builder
            .get_insert_block()
            .unwrap()
            .get_parent()
            .unwrap()
    }
}
