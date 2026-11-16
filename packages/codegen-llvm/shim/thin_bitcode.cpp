// Writes a module as ThinLTO bitcode: the bitcode with its module summary
// index, which `ld.lld` and `wasm-ld` need to optimize a program across its
// units in parallel (design/parallel-pipeline.md §3.3). The C API of LLVM 18
// only writes bitcode without a summary (a regular, whole-program LTO input).

#include "llvm-c/Core.h"
#include "llvm-c/TargetMachine.h"
#include "llvm/Analysis/ModuleSummaryAnalysis.h"
#include "llvm/Analysis/ProfileSummaryInfo.h"
#include "llvm/Bitcode/BitcodeWriter.h"
#include "llvm/IR/Function.h"
#include "llvm/IR/Module.h"
#include "llvm/IR/ModuleSummaryIndex.h"
#include "llvm/Support/raw_ostream.h"
#include "llvm/Target/TargetMachine.h"

extern "C" int fa_write_thin_bitcode(LLVMModuleRef module, const char *path) {
	llvm::Module *m = llvm::unwrap(module);
	std::error_code error;
	llvm::raw_fd_ostream out(path, error);
	if (error) {
		return 1;
	}
	llvm::ProfileSummaryInfo profile(*m);
	llvm::ModuleSummaryIndex index = llvm::buildModuleSummaryIndex(*m, nullptr, &profile);
	// the hash names the promoted local symbols (`.llvm.<hash>`): unique per module
	llvm::WriteBitcodeToFile(*m, out, false, &index, true);
	out.close();
	return out.has_error() ? 1 : 0;
}

// Makes a target machine put the constructors and destructors of a module
// (`llvm.global_ctors`, `llvm.global_dtors`: the load and unload functions
// of a C library) in `.init_array` and `.fini_array` on ELF, as clang does:
// `ld.lld` runs nothing from the `.ctors` sections LLVM writes by default.
extern "C" void fa_use_init_array(LLVMTargetMachineRef machine) {
	reinterpret_cast<llvm::TargetMachine *>(machine)->Options.UseInitArray = true;
}

// Makes a target machine compile the thread-local variables of a module as
// emulated ones (calls of `__emutls_get_address`, of libgcc or compiler-rt):
// the object is then loaded in memory by the JIT (`src/jit.rs`), which has
// no thread-local storage segment to give it.
extern "C" void fa_use_emulated_tls(LLVMTargetMachineRef machine) {
	reinterpret_cast<llvm::TargetMachine *>(machine)->Options.EmulatedTLS = true;
}

// Drop all cross-block references before destroying any instructions. Deleting
// blocks individually through LLVMDeleteBasicBlock leaves later blocks holding
// uses of freed instructions (allocas, PHIs and loop-carried values).
extern "C" void fa_delete_function_body(LLVMValueRef function) {
	llvm::cast<llvm::Function>(llvm::unwrap(function))->deleteBody();
}

// Guarded variants of already-lowered methods. No analyzer identities or
// ownership decisions are recreated: the two bodies retain the original IR.
#include <string>
#include <utility>
#include <vector>
#include "llvm/ADT/DenseMap.h"
#include "llvm/ADT/SmallVector.h"
#include "llvm/IR/IRBuilder.h"
#include "llvm/IR/IntrinsicInst.h"
#include "llvm/Transforms/Utils/Cloning.h"

namespace {
constexpr const char *FrozenAscii = "fa.frozen.ascii";
struct FrozenVariant {
	llvm::Function *original;
	unsigned receiver;
	uint64_t capacity_offset;
	std::string field;
	llvm::Function *ascii = nullptr;
	llvm::Function *unicode = nullptr;
};

llvm::Value *frozen_ascii_test(llvm::IRBuilder<> &builder, llvm::Function &function,
                             const FrozenVariant &variant) {
	auto &layout = function.getParent()->getDataLayout();
	auto *word = builder.getIntNTy(layout.getPointerSizeInBits());
	auto *address = builder.CreateInBoundsGEP(builder.getInt8Ty(),
		function.getArg(variant.receiver), builder.getInt64(variant.capacity_offset));
	auto *capacity = builder.CreateLoad(word, address, "frozen.capacity");
	return builder.CreateICmpSLT(capacity, llvm::ConstantInt::get(word, 0), "frozen.ascii");
}

llvm::Function *frozen_clone(FrozenVariant &variant, bool ascii) {
	llvm::ValueToValueMapTy bindings;
	bindings[variant.original] = variant.original;
	auto *copy = llvm::CloneFunction(variant.original, bindings);
	copy->setName(variant.original->getName() + (ascii ? ".ascii" : ".unicode"));
	copy->setLinkage(llvm::GlobalValue::InternalLinkage);
	copy->setCallingConv(variant.original->getCallingConv());
	copy->removeFnAttr(FrozenAscii);
	llvm::IRBuilder<> builder(&*copy->getEntryBlock().getFirstInsertionPt());
	auto *test = frozen_ascii_test(builder, *copy, variant);
	auto *assume = llvm::Intrinsic::getDeclaration(copy->getParent(), llvm::Intrinsic::assume);
	builder.CreateCall(assume, {ascii ? test : builder.CreateNot(test)});
	return copy;
}

void redirect_frozen_calls(llvm::Function &body, const FrozenVariant &variant, bool ascii,
                          const llvm::DenseMap<llvm::Function *, size_t> &indices,
                          const std::vector<FrozenVariant> &variants) {
	for (auto &block : body) {
		for (auto &instruction : block) {
			auto *call = llvm::dyn_cast<llvm::CallBase>(&instruction);
			if (!call) { continue; }
			auto found = indices.find(call->getCalledFunction());
			if (found == indices.end()) { continue; }
			const auto &callee = variants[found->second];
			if (callee.field != variant.field || callee.capacity_offset != variant.capacity_offset ||
				callee.receiver >= call->arg_size() ||
				call->getArgOperand(callee.receiver) != body.getArg(variant.receiver)) { continue; }
			call->setCalledFunction(ascii ? callee.ascii : callee.unicode);
		}
	}
}

void frozen_wrapper(FrozenVariant &variant) {
	auto &function = *variant.original;
	auto linkage = function.getLinkage();
	function.deleteBody();
	function.setLinkage(linkage);
	function.removeFnAttr(FrozenAscii);
	auto &context = function.getContext();
	auto *entry = llvm::BasicBlock::Create(context, "dispatch", &function);
	auto *ascii = llvm::BasicBlock::Create(context, "ascii", &function);
	auto *unicode = llvm::BasicBlock::Create(context, "unicode", &function);
	llvm::IRBuilder<> builder(entry);
	builder.CreateCondBr(frozen_ascii_test(builder, function, variant), ascii, unicode);
	llvm::SmallVector<llvm::Value *, 8> arguments;
	llvm::SmallVector<llvm::AttributeSet, 8> parameter_attributes;
	for (auto &argument : function.args()) {
		arguments.push_back(&argument);
		parameter_attributes.push_back(function.getAttributes().getParamAttrs(argument.getArgNo()));
	}
	auto attributes = llvm::AttributeList::get(context, llvm::AttributeSet(),
		function.getAttributes().getRetAttrs(), parameter_attributes);
	for (auto [block, target] : {std::pair{ascii, variant.ascii}, std::pair{unicode, variant.unicode}}) {
		builder.SetInsertPoint(block);
		auto *call = builder.CreateCall(target, arguments);
		call->setCallingConv(function.getCallingConv());
		call->setAttributes(attributes);
		// Identical prototypes, calling conventions and ABI attributes.
		call->setTailCallKind(llvm::CallInst::TCK_MustTail);
		if (function.getReturnType()->isVoidTy()) { builder.CreateRetVoid(); }
		else { builder.CreateRet(call); }
	}
}
}

extern "C" unsigned fa_specialize_frozen_strings(LLVMModuleRef module) {
	auto &m = *llvm::unwrap(module);
	std::vector<FrozenVariant> variants;
	llvm::DenseMap<llvm::Function *, size_t> indices;
	for (auto &function : m) {
		auto attribute = function.getFnAttribute(FrozenAscii);
		if (!attribute.isStringAttribute()) { continue; }
		llvm::SmallVector<llvm::StringRef, 3> parts;
		attribute.getValueAsString().split(parts, ',', 2);
		function.removeFnAttr(FrozenAscii);
		unsigned receiver;
		uint64_t offset;
		if (parts.size() != 3 || parts[0].getAsInteger(10, receiver) || parts[1].getAsInteger(10, offset) ||
			function.isDeclaration() || function.isVarArg() || receiver >= function.arg_size() ||
			function.hasFnAttribute(llvm::Attribute::OptimizeNone) ||
			!function.getArg(receiver)->getType()->isPointerTy()) { continue; }
		unsigned instructions = 0;
		bool reads_units = false;
		for (const auto &block : function) {
			instructions += block.size();
			for (const auto &instruction : block) {
				const auto *call = llvm::dyn_cast<llvm::CallBase>(&instruction);
				if (call && call->getCalledFunction() && call->getCalledFunction()->getName() == "string_code_unit_slow") {
					reads_units = true;
				}
			}
		}
		if (!reads_units || instructions > 2048) { continue; }
		indices[&function] = variants.size();
		variants.push_back({&function, receiver, offset, parts[2].str()});
	}
	// Self recursion can already become one loop. The benefit here is
	// carrying a field fact around a mutually recursive method group and
	// through its callers; do not duplicate unrelated single-method loops.
	std::vector<std::vector<size_t>> edges(variants.size());
	for (size_t i = 0; i < variants.size(); ++i) {
		const auto &source = variants[i];
		for (const auto &block : *source.original) {
			for (const auto &instruction : block) {
				const auto *call = llvm::dyn_cast<llvm::CallBase>(&instruction);
				if (!call) { continue; }
				auto found = indices.find(call->getCalledFunction());
				if (found == indices.end() || found->second == i) { continue; }
				const auto &target = variants[found->second];
				if (source.field == target.field && source.capacity_offset == target.capacity_offset &&
					call->getArgOperand(target.receiver) == source.original->getArg(source.receiver)) {
					edges[i].push_back(found->second);
				}
			}
		}
	}
	std::vector<bool> keep(variants.size(), false);
	for (size_t root = 0; root < variants.size(); ++root) {
		std::vector<bool> seen(variants.size(), false);
		auto pending = edges[root];
		while (!pending.empty()) {
			auto next = pending.back(); pending.pop_back();
			if (next == root) { keep[root] = true; break; }
			if (seen[next]) { continue; }
			seen[next] = true;
			pending.insert(pending.end(), edges[next].begin(), edges[next].end());
		}
	}
	bool changed = true;
	while (changed) {
		changed = false;
		for (size_t i = 0; i < variants.size(); ++i) {
			if (keep[i]) { continue; }
			for (auto next : edges[i]) {
				if (keep[next]) { keep[i] = true; changed = true; break; }
			}
		}
	}
	std::vector<FrozenVariant> selected;
	for (size_t i = 0; i < variants.size(); ++i) {
		if (keep[i]) { selected.push_back(std::move(variants[i])); }
	}
	variants = std::move(selected);
	indices.clear();
	for (size_t i = 0; i < variants.size(); ++i) { indices[variants[i].original] = i; }
	// Predeclare every body before redirecting mutual recursion. CloneFunction
	// does not remap function addresses: only the exact direct calls below do.
	for (auto &variant : variants) {
		variant.ascii = frozen_clone(variant, true);
		variant.unicode = frozen_clone(variant, false);
	}
	for (auto &variant : variants) {
		redirect_frozen_calls(*variant.ascii, variant, true, indices, variants);
		redirect_frozen_calls(*variant.unicode, variant, false, indices, variants);
	}
	for (auto &variant : variants) { frozen_wrapper(variant); }
	return variants.size();
}
