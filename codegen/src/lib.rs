mod exprs;

use std::borrow::Cow;

use inkwell::{
    AddressSpace,
    builder::Builder,
    context::Context,
    module::{Linkage, Module},
    types::{
        AnyType, AnyTypeEnum, BasicMetadataTypeEnum, BasicType, BasicTypeEnum, FunctionType,
        StructType,
    },
    values::{AnyValue, AnyValueEnum, PointerValue},
};

use hir::{
    Hir, TyMap, VarId,
    exprs::{Expr, ExprId, LitExpr},
    items::AdtId,
    types::Ty,
};
use slotmap::SecondaryMap;

pub struct Codegen<'ctx, 'hir> {
    hir: &'hir Hir,
    ty_map: &'hir TyMap,
    ctx: &'ctx Context,
    builder: Builder<'ctx>,
    module: Module<'ctx>,
    structs: SecondaryMap<AdtId, StructType<'ctx>>,
    vars: SecondaryMap<VarId, PointerValue<'ctx>>,
}

impl<'ctx, 'hir> Codegen<'ctx, 'hir> {
    pub fn new(hir: &'hir Hir, ty_map: &'hir TyMap, ctx: &'ctx Context, module_name: &str) -> Self {
        let this = Self {
            hir,
            ty_map,
            ctx,
            builder: ctx.create_builder(),
            module: ctx.create_module(module_name),
            structs: Self::build_structs(hir, ctx),
            vars: SecondaryMap::new(),
        };
        this.populate_structs();
        this
    }

    fn report_warning(&self, msg: impl Into<Cow<'static, str>>) {
        todo!("warnings")
    }

    fn build_structs(hir: &Hir, ctx: &'ctx Context) -> SecondaryMap<AdtId, StructType<'ctx>> {
        hir.adts
            .iter()
            .map(|(id, ident)| (id, ctx.opaque_struct_type(&ident.to_string())))
            .collect()
    }

    fn populate_structs(&self) {
        for (id, ty) in &self.structs {
            let field_tys: Vec<_> = self
                .hir
                .adt_info(id)
                .fields
                .values()
                .map(|ty| {
                    if let Ty::Adt(inner_id) = ty
                        && *inner_id == id
                    {
                        self.ctx
                            .ptr_type(AddressSpace::default())
                            .as_basic_type_enum()
                    } else {
                        self.convert_ty(ty)
                    }
                })
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
            Ty::Array(_) => self
                .ctx
                .ptr_type(AddressSpace::default())
                .as_basic_type_enum(),
            Ty::Fn(params, _) => todo!(),
            Ty::Adt(id) => self.structs[*id].as_basic_type_enum(),
        }
    }

    fn codegen_function(&mut self, id: VarId, params: &[VarId], ret_ty: &Ty, body: ExprId) {
        let fn_name = self.hir.var_ident(id).ident.to_string();

        let param_tys: Vec<_> = params
            .iter()
            .map(|p| {
                if self.hir.var_info(*p).mutable {
                    self.ctx
                        .ptr_type(AddressSpace::default())
                        .as_basic_type_enum()
                } else {
                    self.convert_ty(self.ty_map.var_ty(*p))
                }
                .into()
            })
            .collect();
        let ret_ty = self.convert_ty(&ret_ty);
        let fn_ty = ret_ty.fn_type(&param_tys, false);

        let function = self.module.add_function(&fn_name, fn_ty, None);

        let entry_block = self.ctx.append_basic_block(function, "entry");
        self.builder.position_at_end(entry_block);
        let body = self.codegen_expr(body);
        self.builder.build_return(Some(&body)).unwrap();

        assert!(function.verify(true));
    }
}
