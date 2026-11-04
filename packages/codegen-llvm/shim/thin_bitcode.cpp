// Writes a module as ThinLTO bitcode: the bitcode with its module summary
// index, which `ld.lld` and `wasm-ld` need to optimize a program across its
// units in parallel (design/parallel-pipeline.md §3.3). The C API of LLVM 18
// only writes bitcode without a summary (a regular, whole-program LTO input).

#include "llvm-c/Core.h"
#include "llvm/Analysis/ModuleSummaryAnalysis.h"
#include "llvm/Analysis/ProfileSummaryInfo.h"
#include "llvm/Bitcode/BitcodeWriter.h"
#include "llvm/IR/Module.h"
#include "llvm/IR/ModuleSummaryIndex.h"
#include "llvm/Support/raw_ostream.h"

extern "C" int fa_write_thin_bitcode(LLVMModuleRef module, const char *path) {
	llvm::Module *m = llvm::unwrap(module);
	std::error_code error;
	llvm::raw_fd_ostream out(path, error);
	if (error) {
		return 1;
	}
	llvm::ProfileSummaryInfo profile(*m);
	llvm::ModuleSummaryIndex index = llvm::buildModuleSummaryIndex(*m, nullptr, &profile);
	llvm::WriteBitcodeToFile(*m, out, false, &index);
	out.close();
	return out.has_error() ? 1 : 0;
}
