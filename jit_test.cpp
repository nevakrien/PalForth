#include "llvm/ExecutionEngine/Orc/LLJIT.h"
#include "llvm/ExecutionEngine/Orc/ThreadSafeModule.h"
#include "llvm/IR/IRBuilder.h"
#include "llvm/IR/LLVMContext.h"
#include "llvm/IR/Module.h"
#include "llvm/Support/SourceMgr.h"
#include "llvm/AsmParser/Parser.h"
#include "llvm/Support/TargetSelect.h"
#include "llvm/Support/raw_ostream.h"
#include <cassert>
#include <cstring>
#include <iostream>

extern "C" {
#include "ctypes.h"
#include "stack.h"
}

using namespace llvm;
using namespace llvm::orc;

int main() {
    InitializeNativeTarget();
    InitializeNativeTargetAsmPrinter();

    auto jit = cantFail(LLJITBuilder().create());

    LLVMContext ctx;
    SMDiagnostic err;
    std::unique_ptr<Module> coreMod = parseAssemblyFile("vm.ll", err, ctx);
    if (!coreMod) {
        err.print("jit_test", errs());
        return 1;
    }

    cantFail(jit->addIRModule(ThreadSafeModule(std::move(coreMod), std::make_unique<LLVMContext>())));

    std::unique_ptr<Module> mod = std::make_unique<Module>("test_wrapper", ctx);
    IRBuilder<> builder(ctx);

    // Type definitions
    Type* voidptr = PointerType::getUnqual(Type::getInt8Ty(ctx));
    FunctionType* fnTy = FunctionType::get(voidptr, {voidptr, voidptr}, false);
    StructType* codeTy = StructType::create(ctx, "struct.Code");
    codeTy->setBody({voidptr, voidptr}, /*isPacked=*/false);

    Function* wrapperFn = Function::Create(fnTy, Function::ExternalLinkage, "jit_func", mod.get());
    auto args = wrapperFn->args().begin();
    Value* llvm_code_ptr = &*args++;
    Value* llvm_vm = &*args;

    BasicBlock* entry = BasicBlock::Create(ctx, "entry", wrapperFn);
    builder.SetInsertPoint(entry);

    // Allocate local dummy Code on stack
    Value* local_code = builder.CreateAlloca(codeTy, nullptr, "local_code");

    auto emitRealCall = [&](const char* name, int64_t imm) -> Value* {
        // Get function
        auto callee = mod->getOrInsertFunction(name, fnTy);

        // Fill struct fields
        Value* xt = ConstantPointerNull::get(cast<PointerType>(voidptr));
        Value* immVal = ConstantInt::get(Type::getInt64Ty(ctx), imm);
        Value* immPtr = builder.CreateIntToPtr(immVal, voidptr);

        Value* xtField = builder.CreateStructGEP(codeTy, local_code, 0);
        Value* constField = builder.CreateStructGEP(codeTy, local_code, 1);
        builder.CreateStore(xt, xtField);
        builder.CreateStore(immPtr, constField);

        return builder.CreateCall(callee, {local_code, llvm_vm});
    };

    llvm_code_ptr = emitRealCall("frame_alloc", 5);
    llvm_code_ptr = emitRealCall("push_local", 0);
    llvm_code_ptr = emitRealCall("pick", 1);
    llvm_code_ptr = emitRealCall("inject", 5 * sizeof(Word));
    llvm_code_ptr = emitRealCall("param_drop", 1);
    llvm_code_ptr = emitRealCall("pick", 1);
    llvm_code_ptr = emitRealCall("push_local", 0);
    llvm_code_ptr = emitRealCall("inject", 5 * sizeof(Word));
    llvm_code_ptr = emitRealCall("param_drop", 1);
    llvm_code_ptr = emitRealCall("frame_free", 5);

    builder.CreateRet(llvm_code_ptr);

    // Print LLVM IR
    mod->print(outs(), nullptr);

    cantFail(jit->addIRModule(ThreadSafeModule(std::move(mod), std::make_unique<LLVMContext>())));

    auto sym = cantFail(jit->lookup("jit_func"));
    auto compiled_fn = (Code*(*)(Code*, VM*))static_cast<uintptr_t>(sym.getValue());

    void* buffer[10];
    void* buffer2[10];
    VM vm = {};
    vm.data_stack = STACK_LIT(buffer, 10);
    vm.param_stack = STACK_LIT(buffer2, 10);

    Word src[5] = {(Word*)1, (Word*)3, (Word*)1, (Word*)1, (Word*)-1};
    Word tgt[5] = {0};

    *stack_alloc(&vm, &vm.param_stack, 1) = (Word)tgt;
    *stack_alloc(&vm, &vm.param_stack, 1) = (Word)src;

    Code dummy = {nullptr, nullptr};
    compiled_fn(&dummy, &vm);

    assert(memcmp(src, tgt, sizeof(src)) == 0);
    stack_free(&vm, &vm.param_stack, 2);

    printf("[LLVM JIT] frame injections: passed\n");
    return 0;
}
