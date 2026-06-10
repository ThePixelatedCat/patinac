use cranelift::{
    codegen::{
        self, Context,
        ir::{AbiParam, InstBuilder as _, Signature, Type, condcodes::IntCC, types},
        isa::CallConv,
    },
    frontend::{FunctionBuilder, FunctionBuilderContext},
    module::{FuncId, FuncOrDataId, Linkage, Module},
};

use crate::Codegen;

impl<'ctx> Codegen<'_, '_> {
    fn runtime_func(&mut self, name: &str, params: Vec<Type>, ret: Option<Type>) -> FuncId {
        if let Some(FuncOrDataId::Func(func)) = self.module.declarations().get_name(name) {
            return func;
        }
        let sig = Signature {
            params: params.into_iter().map(AbiParam::new).collect(),
            returns: ret.into_iter().map(AbiParam::new).collect(),
            call_conv: self.module.isa().default_call_conv(),
        };
        self.module
            .declare_function(name, Linkage::Import, &sig)
            .unwrap()
    }

    pub(crate) fn printf(&mut self) -> FuncId {
        self.runtime_func("printf", vec![self.ptr_ty()], Some(types::I32))
    }

    pub(crate) fn malloc(
        &mut self,
        ctx: &mut Context,
        func_ctx: &mut FunctionBuilderContext,
    ) -> FuncId {
        if let Some(FuncOrDataId::Func(func)) = self.module.declarations().get_name("ptn_malloc") {
            return func;
        }

        let sig = Signature {
            params: vec![AbiParam::new(types::I64)],
            returns: vec![AbiParam::new(self.ptr_ty())],
            call_conv: CallConv::Fast,
        };
        let func = self
            .module
            .declare_function("ptn_malloc", Linkage::Local, &sig)
            .unwrap();

        let panic = self.panic(ctx, func_ctx);

        let mut builder = FunctionBuilder::new(&mut ctx.func, func_ctx);
        builder.func.signature = sig;

        // Create the function's blocks.
        let entry_block = builder.create_block();
        let panic_block = builder.create_block();
        let ret_block = builder.create_block();
        builder.switch_to_block(entry_block);
        builder.append_block_params_for_function_params(entry_block);
        builder.seal_block(entry_block);

        let c_malloc = self.c_malloc();
        let size = builder.block_params(entry_block)[0];
        let ptr = self.call(&mut builder, c_malloc, &[size])[0];
        let is_null = builder.ins().icmp_imm(IntCC::Equal, ptr, 0);
        builder.ins().brif(is_null, panic_block, [], ret_block, []);

        builder.seal_block(panic_block);

        builder.switch_to_block(panic_block);
        let panic_msg =
            self.emit_global_string(&mut builder, "alloc_panic_msg", b"allocation failed\0");
        self.call(&mut builder, panic, &[panic_msg]);
        builder.ins().jump(ret_block, []);

        builder.seal_block(ret_block);

        builder.switch_to_block(ret_block);
        builder.ins().return_(&[ptr]);

        codegen::verify_function(&builder.func, self.module.isa()).unwrap();
        builder.finalize();
        self.module.define_function(func, ctx).unwrap();
        ctx.clear();

        func
    }

    fn c_malloc(&mut self) -> FuncId {
        self.runtime_func("malloc", vec![types::I64], Some(self.ptr_ty()))
    }

    pub(crate) fn free(&mut self) -> FuncId {
        self.runtime_func("free", vec![self.ptr_ty()], None)
    }

    pub(crate) fn panic(
        &mut self,
        ctx: &mut Context,
        func_ctx: &mut FunctionBuilderContext,
    ) -> FuncId {
        if let Some(FuncOrDataId::Func(func)) = self.module.declarations().get_name("panic") {
            return func;
        }

        let sig = Signature {
            params: vec![AbiParam::new(self.ptr_ty())],
            returns: vec![],
            call_conv: CallConv::Fast,
        };
        let func = self
            .module
            .declare_function("panic", Linkage::Local, &sig)
            .unwrap();

        let mut builder = FunctionBuilder::new(&mut ctx.func, func_ctx);
        builder.func.signature = sig;

        // Create the function's entry block.
        let entry_block = builder.create_block();
        builder.switch_to_block(entry_block);
        builder.append_block_params_for_function_params(entry_block);
        builder.seal_block(entry_block);

        let printf = self.printf();
        let msg = builder.block_params(entry_block)[0];
        self.call(&mut builder, printf, &[msg]);

        let exit = self.exit();
        let exit_code = builder.ins().iconst(types::I32, 1);
        self.call(&mut builder, exit, &[exit_code]);

        builder.ins().return_(&[]);

        codegen::verify_function(&builder.func, self.module.isa()).unwrap();
        builder.finalize();
        self.module.define_function(func, ctx).unwrap();
        ctx.clear();

        func
    }

    fn exit(&mut self) -> FuncId {
        self.runtime_func("exit", vec![types::I32], None)
    }
}
