// Writes a module as ThinLTO bitcode: the bitcode with its module summary
// index, which `ld.lld` and `wasm-ld` need to optimize a program across its
// units in parallel (design/parallel-pipeline.md §3.3). The C API of LLVM 18
// only writes bitcode without a summary (a regular, whole-program LTO input).

#include "llvm-c/Core.h"
#include "llvm-c/TargetMachine.h"
#include "llvm/Analysis/ModuleSummaryAnalysis.h"
#include "llvm/Analysis/ProfileSummaryInfo.h"
#include "llvm/Bitcode/BitcodeWriter.h"
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
