#include "llvm/ExecutionEngine/Orc/LLJIT.h"
#include "llvm/ExecutionEngine/Orc/ThreadSafeModule.h"
#include "llvm/IR/IRBuilder.h"
#include "llvm/IR/LLVMContext.h"
#include "llvm/IR/Module.h"
#include "llvm/Support/SourceMgr.h"
#include "llvm/AsmParser/Parser.h"
#include "llvm/Support/TargetSelect.h"
#include "llvm/Support/raw_ostream.h"
#include "llvm/IR/PassManager.h"
#include "llvm/Passes/PassBuilder.h"

#include "llvm/Linker/Linker.h"


#include <cassert>
#include <cstring>
#include <iostream>

extern "C" {
#include "ctypes.h"
#include "stack.h"
}

using namespace llvm;
using namespace llvm::orc;

void optimizeModule(Module &mod, LLVMContext &ctx) {
    PassBuilder pb;
    FunctionAnalysisManager fam;
    LoopAnalysisManager lam;
    CGSCCAnalysisManager cgam;
    ModuleAnalysisManager mam;

    pb.registerModuleAnalyses(mam);
    pb.registerCGSCCAnalyses(cgam);
    pb.registerFunctionAnalyses(fam);
    pb.registerLoopAnalyses(lam);
    pb.crossRegisterProxies(lam, fam, cgam, mam);

    ModulePassManager mpm = pb.buildPerModuleDefaultPipeline(OptimizationLevel::O3);
    mpm.run(mod, mam);
}

int main() {
    InitializeNativeTarget();
    InitializeNativeTargetAsmPrinter();

    auto jit = cantFail(LLJITBuilder().create());

    LLVMContext ctx;
    SMDiagnostic err;
    // Step 1: Parse coreMod
std::unique_ptr<Module> coreMod = parseAssemblyFile("vm.ll", err, ctx);
if (!coreMod) {
    err.print("jit_test", errs());
    return 1;
}

// Step 2: Create wrapper module
std::unique_ptr<Module> mod = std::make_unique<Module>("test_wrapper", ctx);

// Step 3: Link coreMod into mod
if (Linker::linkModules(*mod, std::move(coreMod))) {
    errs() << "Failed to link core module into wrapper module.\n";
    return 1;
}

// Continue building wrapperFn inside `mod`...

    IRBuilder<> builder(ctx);

    Type *codeTy = StructType::get(PointerType::getUnqual(Type::getInt8Ty(ctx)), PointerType::getUnqual(Type::getInt8Ty(ctx)));
    Type *voidptr = PointerType::getUnqual(Type::getInt8Ty(ctx));
    FunctionType *fnTy = FunctionType::get(voidptr, {voidptr, voidptr}, false);

    Function *wrapperFn = Function::Create(fnTy, Function::ExternalLinkage, "jit_func", mod.get());
    auto args = wrapperFn->args().begin();
    Value *llvm_code = &*args++;
    Value *llvm_vm = &*args;

    BasicBlock *entry = BasicBlock::Create(ctx, "entry", wrapperFn);
    builder.SetInsertPoint(entry);

    Value *codeVar = builder.CreateAlloca(codeTy);
    auto setCode = [&](int64_t imm) {
        Value *xtField = builder.CreateStructGEP(codeTy, codeVar, 0);
        Value *valField = builder.CreateStructGEP(codeTy, codeVar, 1);
        builder.CreateStore(Constant::getNullValue(voidptr), xtField);
        builder.CreateStore(ConstantExpr::getIntToPtr(ConstantInt::get(Type::getInt64Ty(ctx), imm), voidptr), valField);
        return codeVar;
    };

    auto callVM = [&](const char *name, int64_t imm) {
        FunctionCallee callee = mod->getOrInsertFunction(name, fnTy);
        if (Function *f = dyn_cast<Function>(callee.getCallee())) {
            f->addFnAttr(Attribute::AlwaysInline);
            f->setLinkage(GlobalValue::InternalLinkage);
        }
        Value *arg = setCode(imm);
        return builder.CreateCall(callee, {arg, llvm_vm});
    };


    callVM("frame_alloc", 5);
    callVM("push_local", 0);
    callVM("pick", 1);
    callVM("inject", 5 * sizeof(Word));
    callVM("param_drop", 1);
    callVM("pick", 1);
    callVM("push_local", 0);
    callVM("inject", 5 * sizeof(Word));
    callVM("param_drop", 1);
    Value *retVal = callVM("frame_free", 5);

    builder.CreateRet(retVal);

    // Print IR before optimization
    std::cout << "=== LLVM IR Before Optimization ===\n";
    mod->print(outs(), nullptr);

    // Run optimization
    optimizeModule(*mod, ctx);

    std::cout << "\n=== LLVM IR After Optimization ===\n";
    mod->print(outs(), nullptr);

    cantFail(jit->addIRModule(ThreadSafeModule(std::move(mod), std::make_unique<LLVMContext>())));

    auto sym = cantFail(jit->lookup("jit_func"));
    auto compiled_fn = (Code *(*)(Code *, VM *))static_cast<uintptr_t>(sym.getValue());

    void *buffer[10];
    void *buffer2[10];
    VM vm = {};
    vm.data_stack = STACK_LIT(buffer, 10);
    vm.param_stack = STACK_LIT(buffer2, 10);

    Word src[5] = {(Word *)1, (Word *)3, (Word *)1, (Word *)1, (Word *)-1};
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
