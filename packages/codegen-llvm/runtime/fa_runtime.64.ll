source_filename = "fa_runtime.c"

%struct.AlternateStack = type { ptr, i32, i64 }
%struct.SignalAction = type { ptr, [16 x i64], i32, ptr }
%struct.BareError = type { i64 }
%struct.anon = type { %struct.FaSystemLock, %struct.FaSystemLock, ptr, i64, i32, i32, i32 }
%struct.FaSystemLock = type { [64 x i8] }
%struct.Range = type { i32, i32 }
%struct.FaEpollEvent = type <{ i32, i64 }>
%struct.FaString = type { ptr, i64, i64 }
%struct.Boundaries = type { ptr, i64, i64 }
%struct.Replacing = type { ptr, i64, i64, ptr, i64, i32, i32 }
%struct.FaArrayHeader = type { ptr, i64, i64 }
%struct.Splitting = type { ptr, i64, i64, ptr }
%struct.IndexError = type { i64, i64, i64 }
%struct.FaSegmented = type { i64, i64, [48 x ptr] }
%struct.PageHeader = type { i32, i32 }
%struct.FreeSpan = type { ptr, i64 }
%struct.FaPool = type { i64, ptr, ptr, ptr, i32, i32, i32, i32, i32, i32 }
%struct.PoolChunk = type { ptr, i32, i32, i32, i32, [0 x i64] }
%struct.FaLinks = type { i64, ptr, i64, i64, ptr, i64 }
%struct.FaAddressMap = type { ptr, ptr, i64 }
%struct.FaSource = type { i64, i64, ptr, i64, i64, ptr, ptr, i64, i64 }
%struct.FaChild = type { i64, i64, ptr }
%struct.FaObserver = type { %struct.FaSource, ptr, ptr, ptr, ptr, ptr, ptr, ptr, i64, i64, i64, i32, i32, i32, i32, i32, i32, i32, i32, i64, [2 x ptr] }
%struct.MessageError = type { i64, %struct.FaString }
%struct.ParseError = type { i64, %struct.FaString, %struct.FaString }
%struct.FaTable = type { ptr, i64, i64, i64, ptr, i64, i64 }
%struct.FaShared = type { i64, i64, ptr, i64 }
%struct.FaAtomicBlock = type { %struct.FaShared, i64 }
%struct.FaExecutor = type { ptr, ptr, i64, ptr, i64, i64, i64, ptr, i64, ptr, ptr, %struct.FaTask, i32, i32, i32, i32, i64, ptr, ptr, [32 x ptr], i32 }
%struct.FaTask = type { ptr, ptr, ptr, i32, i32 }
%struct.FaJoin = type { ptr, i64, ptr, ptr }
%struct.FaWait = type { ptr, ptr, ptr, ptr, ptr, ptr, i64, i64, i64, i32, i32 }
%struct.FaMutex = type { %struct.FaShared, i32, i32, ptr, %struct.FaSystemLock, %struct.FaSystemLock, [0 x i8] }
%struct.FaChannel = type { %struct.FaShared, %struct.FaSystemLock, %struct.FaSystemLock, i64, i64, ptr, i64, i64, i64, i32, i32, ptr, ptr, ptr }
%struct.ParallelJob = type { ptr, ptr, i64, i64, i64, i32, i32, i32, i32, ptr, i64, ptr }

@output_lock = internal global i32 0, align 4
@output_length = internal unnamed_addr global i64 0, align 8
@output = internal global [8192 x i8] zeroinitializer, align 16
@builder_length = internal thread_local(initialexec) global i64 0, align 8
@builder = internal thread_local(initialexec) global ptr null, align 8
@.str = private unnamed_addr constant [5 x i8] c"true\00", align 1
@.str.1 = private unnamed_addr constant [6 x i8] c"false\00", align 1
@fa_b_quoted.hex = internal unnamed_addr constant [17 x i8] c"0123456789ABCDEF\00", align 16
@.str.2 = private unnamed_addr constant [3 x i8] c"\\\\\00", align 1
@.str.3 = private unnamed_addr constant [3 x i8] c"\\\22\00", align 1
@.str.4 = private unnamed_addr constant [3 x i8] c"\\n\00", align 1
@.str.5 = private unnamed_addr constant [3 x i8] c"\\r\00", align 1
@.str.6 = private unnamed_addr constant [3 x i8] c"\\t\00", align 1
@.str.7 = private unnamed_addr constant [3 x i8] c"\\0\00", align 1
@.str.8 = private unnamed_addr constant [3 x i8] c"{{\00", align 1
@.str.9 = private unnamed_addr constant [3 x i8] c"}}\00", align 1
@.str.10 = private unnamed_addr constant [3 x i8] c"\\x\00", align 1
@.str.11 = private unnamed_addr constant [3 x i8] c", \00", align 1
@.str.12 = private unnamed_addr constant [3 x i8] c"SS\00", align 1
@argument_count = internal unnamed_addr global i64 0, align 8
@argument_items = internal unnamed_addr global ptr null, align 8
@free_blocks = internal thread_local(initialexec) global [40 x ptr] zeroinitializer, align 16
@class_next = internal thread_local(initialexec) global [40 x ptr] zeroinitializer, align 16
@class_end = internal thread_local(initialexec) global [40 x ptr] zeroinitializer, align 16
@builder_capacity = internal thread_local(initialexec) global i64 0, align 8
@fa_live_blocks = global i64 0, align 8
@.str.13 = private unnamed_addr constant [7 x i8] c"leak: \00", align 1
@.str.14 = private unnamed_addr constant [8 x i8] c" blocks\00", align 1
@argument_sources = internal unnamed_addr global [16 x ptr] zeroinitializer, align 16
@field_source_count = internal unnamed_addr global i64 0, align 8
@field_sources = internal unnamed_addr global ptr null, align 8
@field_source_capacity = internal global i64 0, align 8
@reactive_observer = internal unnamed_addr global ptr null, align 8
@reactive_sequence = internal unnamed_addr global i64 0, align 8
@reactive_flushing = internal unnamed_addr global i1 false, align 4
@reactive_pending_count = internal unnamed_addr global i64 0, align 8
@.str.15 = private unnamed_addr constant [74 x i8] c"the `watch` blocks kept changing what they read: stopped after 100 rounds\00", align 1
@reactive_pending = internal unnamed_addr global ptr null, align 8
@reactive_made_count = internal unnamed_addr global i64 0, align 8
@reactive_made = internal unnamed_addr global ptr null, align 8
@reactive_made_capacity = internal global i64 0, align 8
@reactive_pending_capacity = internal global i64 0, align 8
@.str.16 = private unnamed_addr constant [19 x i8] c"integer overflow: \00", align 1
@.str.17 = private unnamed_addr constant [17 x i8] c" doesn't fit in \00", align 1
@.str.18 = private unnamed_addr constant [44 x i8] c"integer overflow: the value doesn't fit in \00", align 1
@.str.19 = private unnamed_addr constant [19 x i8] c"assertion failed: \00", align 1
@signal_stack = internal global [16384 x i8] zeroinitializer, align 16
@__const.fa_guard_stack.stack = private unnamed_addr constant %struct.AlternateStack { ptr @signal_stack, i32 0, i64 16384 }, align 8
@__const.fa_guard_stack.action = private unnamed_addr constant %struct.SignalAction { ptr @on_fault, [16 x i64] zeroinitializer, i32 134217728, ptr null }, align 8
@.str.20 = private unnamed_addr constant [13 x i8] c"\0A       \E2\86\B3 \00", align 1
@.str.21 = private unnamed_addr constant [8 x i8] c"error: \00", align 1
@.str.22 = private unnamed_addr constant [2 x i8] c"\0A\00", align 1
@division_by_zero = internal constant %struct.BareError { i64 3 }, align 8
@.str.23 = private unnamed_addr constant [20 x i8] c"negative exponent: \00", align 1
@.str.24 = private unnamed_addr constant [24 x i8] c" doesn't fit in Integer\00", align 1
@.str.25 = private unnamed_addr constant [24 x i8] c" is not a finite number\00", align 1
@.str.26 = private unnamed_addr constant [14 x i8] c"shift amount \00", align 1
@.str.27 = private unnamed_addr constant [16 x i8] c" is outside 0..\00", align 1
@.str.28 = private unnamed_addr constant [18 x i8] c"the text is empty\00", align 1
@.str.29 = private unnamed_addr constant [20 x i8] c"\22 is not an integer\00", align 1
@current_executor = internal thread_local(initialexec) global ptr null, align 8
@frame_to_free = internal thread_local(initialexec) global ptr null, align 8
@next_frame = internal thread_local(initialexec) global ptr null, align 8
@fa_coop_budget = thread_local(initialexec) global i32 0, align 4
@.str.30 = private unnamed_addr constant [49 x i8] c"a blocking call waits for a task that isn't done\00", align 1
@.str.31 = private unnamed_addr constant [38 x i8] c"`lock()` on a mutex this thread holds\00", align 1
@.str.32 = private unnamed_addr constant [60 x i8] c"`send` on a full channel that no other thread receives from\00", align 1
@.str.33 = private unnamed_addr constant [60 x i8] c"`receive` on an empty channel that no other thread sends to\00", align 1
@in_parallel = internal thread_local(initialexec) global i32 0, align 4
@pool = internal global %struct.anon zeroinitializer, align 16
@fa_blocking_depth = thread_local(initialexec) local_unnamed_addr global i32 0, align 4
@fa_turn_site = thread_local(initialexec) global ptr null, align 8
@.str.34 = private unnamed_addr constant [1 x i8] zeroinitializer, align 1
@.str.35 = private unnamed_addr constant [4 x i8] c"NaN\00", align 1
@.str.36 = private unnamed_addr constant [9 x i8] c"Infinity\00", align 1
@.str.37 = private unnamed_addr constant [3 x i8] c"0.\00", align 1
@.str.38 = private unnamed_addr constant [5 x i8] c"%.*e\00", align 1
@.str.39 = private unnamed_addr constant [8 x i8] c"%llde%d\00", align 1
@.str.40 = private unnamed_addr constant [6 x i8] c"%.14e\00", align 1
@EXTEND = internal unnamed_addr constant [153 x %struct.Range] [%struct.Range { i32 768, i32 879 }, %struct.Range { i32 1155, i32 1161 }, %struct.Range { i32 1425, i32 1469 }, %struct.Range { i32 1471, i32 1471 }, %struct.Range { i32 1473, i32 1474 }, %struct.Range { i32 1476, i32 1477 }, %struct.Range { i32 1479, i32 1479 }, %struct.Range { i32 1552, i32 1562 }, %struct.Range { i32 1611, i32 1631 }, %struct.Range { i32 1648, i32 1648 }, %struct.Range { i32 1750, i32 1756 }, %struct.Range { i32 1759, i32 1764 }, %struct.Range { i32 1767, i32 1768 }, %struct.Range { i32 1770, i32 1773 }, %struct.Range { i32 1809, i32 1809 }, %struct.Range { i32 1840, i32 1866 }, %struct.Range { i32 1958, i32 1968 }, %struct.Range { i32 2027, i32 2035 }, %struct.Range { i32 2070, i32 2093 }, %struct.Range { i32 2137, i32 2139 }, %struct.Range { i32 2200, i32 2207 }, %struct.Range { i32 2250, i32 2273 }, %struct.Range { i32 2275, i32 2307 }, %struct.Range { i32 2362, i32 2364 }, %struct.Range { i32 2366, i32 2383 }, %struct.Range { i32 2385, i32 2391 }, %struct.Range { i32 2402, i32 2403 }, %struct.Range { i32 2433, i32 2435 }, %struct.Range { i32 2492, i32 2492 }, %struct.Range { i32 2494, i32 2509 }, %struct.Range { i32 2519, i32 2519 }, %struct.Range { i32 2530, i32 2531 }, %struct.Range { i32 2561, i32 2563 }, %struct.Range { i32 2620, i32 2641 }, %struct.Range { i32 2672, i32 2673 }, %struct.Range { i32 2677, i32 2677 }, %struct.Range { i32 2689, i32 2691 }, %struct.Range { i32 2748, i32 2748 }, %struct.Range { i32 2750, i32 2765 }, %struct.Range { i32 2786, i32 2787 }, %struct.Range { i32 2817, i32 2819 }, %struct.Range { i32 2876, i32 2876 }, %struct.Range { i32 2878, i32 2903 }, %struct.Range { i32 2914, i32 2915 }, %struct.Range { i32 2946, i32 2946 }, %struct.Range { i32 3006, i32 3021 }, %struct.Range { i32 3031, i32 3031 }, %struct.Range { i32 3072, i32 3076 }, %struct.Range { i32 3132, i32 3132 }, %struct.Range { i32 3134, i32 3158 }, %struct.Range { i32 3170, i32 3171 }, %struct.Range { i32 3201, i32 3203 }, %struct.Range { i32 3260, i32 3260 }, %struct.Range { i32 3262, i32 3286 }, %struct.Range { i32 3298, i32 3299 }, %struct.Range { i32 3328, i32 3331 }, %struct.Range { i32 3387, i32 3388 }, %struct.Range { i32 3390, i32 3405 }, %struct.Range { i32 3415, i32 3415 }, %struct.Range { i32 3426, i32 3427 }, %struct.Range { i32 3457, i32 3459 }, %struct.Range { i32 3530, i32 3551 }, %struct.Range { i32 3570, i32 3571 }, %struct.Range { i32 3633, i32 3633 }, %struct.Range { i32 3635, i32 3642 }, %struct.Range { i32 3655, i32 3662 }, %struct.Range { i32 3761, i32 3761 }, %struct.Range { i32 3763, i32 3772 }, %struct.Range { i32 3784, i32 3790 }, %struct.Range { i32 3864, i32 3865 }, %struct.Range { i32 3893, i32 3893 }, %struct.Range { i32 3895, i32 3895 }, %struct.Range { i32 3897, i32 3897 }, %struct.Range { i32 3902, i32 3903 }, %struct.Range { i32 3953, i32 3972 }, %struct.Range { i32 3974, i32 3975 }, %struct.Range { i32 3981, i32 4028 }, %struct.Range { i32 4038, i32 4038 }, %struct.Range { i32 4139, i32 4158 }, %struct.Range { i32 4182, i32 4185 }, %struct.Range { i32 4190, i32 4192 }, %struct.Range { i32 4194, i32 4196 }, %struct.Range { i32 4199, i32 4205 }, %struct.Range { i32 4209, i32 4212 }, %struct.Range { i32 4226, i32 4237 }, %struct.Range { i32 4239, i32 4239 }, %struct.Range { i32 4250, i32 4253 }, %struct.Range { i32 4957, i32 4959 }, %struct.Range { i32 5906, i32 5909 }, %struct.Range { i32 5938, i32 5940 }, %struct.Range { i32 5970, i32 5971 }, %struct.Range { i32 6002, i32 6003 }, %struct.Range { i32 6068, i32 6099 }, %struct.Range { i32 6109, i32 6109 }, %struct.Range { i32 6155, i32 6157 }, %struct.Range { i32 6159, i32 6159 }, %struct.Range { i32 6277, i32 6278 }, %struct.Range { i32 6313, i32 6313 }, %struct.Range { i32 6432, i32 6459 }, %struct.Range { i32 6679, i32 6683 }, %struct.Range { i32 6741, i32 6783 }, %struct.Range { i32 6832, i32 6911 }, %struct.Range { i32 6912, i32 6916 }, %struct.Range { i32 6964, i32 6980 }, %struct.Range { i32 7019, i32 7027 }, %struct.Range { i32 7040, i32 7042 }, %struct.Range { i32 7073, i32 7085 }, %struct.Range { i32 7142, i32 7155 }, %struct.Range { i32 7204, i32 7223 }, %struct.Range { i32 7376, i32 7378 }, %struct.Range { i32 7380, i32 7400 }, %struct.Range { i32 7405, i32 7405 }, %struct.Range { i32 7412, i32 7412 }, %struct.Range { i32 7415, i32 7417 }, %struct.Range { i32 7616, i32 7679 }, %struct.Range { i32 8204, i32 8204 }, %struct.Range { i32 8400, i32 8432 }, %struct.Range { i32 11503, i32 11505 }, %struct.Range { i32 11647, i32 11647 }, %struct.Range { i32 11744, i32 11775 }, %struct.Range { i32 12330, i32 12335 }, %struct.Range { i32 12441, i32 12442 }, %struct.Range { i32 42607, i32 42610 }, %struct.Range { i32 42612, i32 42621 }, %struct.Range { i32 42654, i32 42655 }, %struct.Range { i32 42736, i32 42737 }, %struct.Range { i32 43010, i32 43010 }, %struct.Range { i32 43014, i32 43014 }, %struct.Range { i32 43019, i32 43019 }, %struct.Range { i32 43043, i32 43047 }, %struct.Range { i32 43052, i32 43052 }, %struct.Range { i32 43136, i32 43137 }, %struct.Range { i32 43188, i32 43205 }, %struct.Range { i32 43232, i32 43249 }, %struct.Range { i32 43263, i32 43263 }, %struct.Range { i32 43302, i32 43309 }, %struct.Range { i32 43335, i32 43347 }, %struct.Range { i32 43392, i32 43395 }, %struct.Range { i32 43443, i32 43456 }, %struct.Range { i32 43561, i32 43574 }, %struct.Range { i32 43587, i32 43587 }, %struct.Range { i32 43596, i32 43597 }, %struct.Range { i32 43755, i32 43759 }, %struct.Range { i32 43765, i32 43766 }, %struct.Range { i32 44003, i32 44010 }, %struct.Range { i32 44012, i32 44013 }, %struct.Range { i32 64286, i32 64286 }, %struct.Range { i32 65024, i32 65039 }, %struct.Range { i32 65056, i32 65071 }, %struct.Range { i32 65438, i32 65439 }, %struct.Range { i32 127995, i32 127999 }, %struct.Range { i32 917536, i32 917631 }, %struct.Range { i32 917760, i32 917999 }], align 16
@PICTOGRAPHIC = internal unnamed_addr constant [22 x %struct.Range] [%struct.Range { i32 169, i32 169 }, %struct.Range { i32 174, i32 174 }, %struct.Range { i32 8252, i32 8252 }, %struct.Range { i32 8265, i32 8265 }, %struct.Range { i32 8482, i32 8482 }, %struct.Range { i32 8505, i32 8505 }, %struct.Range { i32 8596, i32 8618 }, %struct.Range { i32 8986, i32 9215 }, %struct.Range { i32 9410, i32 9410 }, %struct.Range { i32 9642, i32 9726 }, %struct.Range { i32 9728, i32 10175 }, %struct.Range { i32 10548, i32 10549 }, %struct.Range { i32 11013, i32 11093 }, %struct.Range { i32 12336, i32 12336 }, %struct.Range { i32 12349, i32 12349 }, %struct.Range { i32 12951, i32 12951 }, %struct.Range { i32 12953, i32 12953 }, %struct.Range { i32 126976, i32 127231 }, %struct.Range { i32 127245, i32 127405 }, %struct.Range { i32 127489, i32 127994 }, %struct.Range { i32 128000, i32 129791 }, %struct.Range { i32 130048, i32 131069 }], align 16
@out_of_memory.message = internal constant [21 x i8] c"stop: out of memory\0A\00", align 16
@spans_lock = internal global i32 0, align 4
@free_spans = internal unnamed_addr global ptr null, align 8
@.str.41 = private unnamed_addr constant [34 x i8] c"a derived value depends on itself\00", align 1
@.str.42 = private unnamed_addr constant [7 x i8] c"stop: \00", align 1
@.str.43 = private unnamed_addr constant [7 x i8] c"\0A  at \00", align 1
@on_fault.message = internal constant [22 x i8] c"stop: stack overflow\0A\00", align 16
@__const.executor.event = private unnamed_addr constant %struct.FaEpollEvent <{ i32 1, i64 0 }>, align 1
@.str.44 = private unnamed_addr constant [17 x i8] c"stop: deadlock: \00", align 1
@.str.45 = private unnamed_addr constant [37 x i8] c"warning: a task kept the thread for \00", align 1
@.str.46 = private unnamed_addr constant [6 x i8] c" ms (\00", align 1
@.str.47 = private unnamed_addr constant [7 x i8] c"a task\00", align 1
@.str.48 = private unnamed_addr constant [51 x i8] c"): await Task.yield() or move the work to a worker\00", align 1
@.str.49 = private unnamed_addr constant [33 x i8] c"stop: deadlock: every task waits\00", align 1
@.str.50 = private unnamed_addr constant [17 x i8] c" (main waits on \00", align 1
@.str.51 = private unnamed_addr constant [2 x i8] c")\00", align 1
@.str.52 = private unnamed_addr constant [19 x i8] c" (a task waits on \00", align 1

; Function Attrs: nounwind optsize
define void @fa_flush() local_unnamed_addr #0 {
  %1 = atomicrmw xchg ptr @output_lock, i32 1 acquire, align 4
  %2 = icmp eq i32 %1, 0
  br i1 %2, label %12, label %6

3:                                                ; preds = %9, %6
  %4 = atomicrmw xchg ptr @output_lock, i32 1 acquire, align 4
  %5 = icmp eq i32 %4, 0
  br i1 %5, label %12, label %6, !llvm.loop !2

6:                                                ; preds = %0, %3
  %7 = load atomic i32, ptr @output_lock monotonic, align 4
  %8 = icmp eq i32 %7, 0
  br i1 %8, label %3, label %9

9:                                                ; preds = %6, %9
  tail call void @llvm.x86.sse2.pause()
  %10 = load atomic i32, ptr @output_lock monotonic, align 4
  %11 = icmp eq i32 %10, 0
  br i1 %11, label %3, label %9, !llvm.loop !4

12:                                               ; preds = %3, %0
  %13 = load i64, ptr @output_length, align 8, !tbaa !5
  %14 = icmp eq i64 %13, 0
  br i1 %14, label %16, label %15

15:                                               ; preds = %12
  tail call void @fa_write(i32 noundef 1, ptr noundef nonnull @output, i64 noundef %13) #34
  store i64 0, ptr @output_length, align 8, !tbaa !5
  br label %16

16:                                               ; preds = %12, %15
  store atomic i32 0, ptr @output_lock release, align 4
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_out(i32 noundef %0, ptr noundef %1, i64 noundef %2) local_unnamed_addr #0 {
  %4 = atomicrmw xchg ptr @output_lock, i32 1 acquire, align 4
  %5 = icmp eq i32 %4, 0
  br i1 %5, label %15, label %9

6:                                                ; preds = %12, %9
  %7 = atomicrmw xchg ptr @output_lock, i32 1 acquire, align 4
  %8 = icmp eq i32 %7, 0
  br i1 %8, label %15, label %9, !llvm.loop !2

9:                                                ; preds = %3, %6
  %10 = load atomic i32, ptr @output_lock monotonic, align 4
  %11 = icmp eq i32 %10, 0
  br i1 %11, label %6, label %12

12:                                               ; preds = %9, %12
  tail call void @llvm.x86.sse2.pause()
  %13 = load atomic i32, ptr @output_lock monotonic, align 4
  %14 = icmp eq i32 %13, 0
  br i1 %14, label %6, label %12, !llvm.loop !4

15:                                               ; preds = %6, %3
  %16 = icmp eq i32 %0, 1
  %17 = load i64, ptr @output_length, align 8, !tbaa !5
  br i1 %16, label %22, label %18

18:                                               ; preds = %15
  %19 = icmp eq i64 %17, 0
  br i1 %19, label %21, label %20

20:                                               ; preds = %18
  tail call void @fa_write(i32 noundef 1, ptr noundef nonnull @output, i64 noundef %17) #34
  store i64 0, ptr @output_length, align 8, !tbaa !5
  br label %21

21:                                               ; preds = %18, %20
  tail call void @fa_write(i32 noundef %0, ptr noundef %1, i64 noundef %2) #34
  br label %35

22:                                               ; preds = %15
  %23 = add i64 %17, %2
  %24 = icmp ugt i64 %23, 8192
  br i1 %24, label %25, label %31

25:                                               ; preds = %22
  %26 = icmp eq i64 %17, 0
  br i1 %26, label %28, label %27

27:                                               ; preds = %25
  tail call void @fa_write(i32 noundef 1, ptr noundef nonnull @output, i64 noundef %17) #34
  store i64 0, ptr @output_length, align 8, !tbaa !5
  br label %28

28:                                               ; preds = %25, %27
  %29 = icmp ugt i64 %2, 8192
  br i1 %29, label %30, label %31

30:                                               ; preds = %28
  tail call void @fa_write(i32 noundef 1, ptr noundef %1, i64 noundef %2) #34
  br label %35

31:                                               ; preds = %28, %22
  %32 = phi i64 [ %17, %22 ], [ 0, %28 ]
  %33 = getelementptr inbounds i8, ptr @output, i64 %32
  tail call void @llvm.memcpy.p0.p0.i64(ptr nonnull align 1 %33, ptr align 1 %1, i64 %2, i1 false)
  %34 = add i64 %32, %2
  store i64 %34, ptr @output_length, align 8, !tbaa !5
  br label %35

35:                                               ; preds = %31, %30, %21
  store atomic i32 0, ptr @output_lock release, align 4
  ret void
}

; Function Attrs: optsize
declare void @fa_write(i32 noundef, ptr noundef, i64 noundef) local_unnamed_addr #1

; Function Attrs: noreturn nounwind optsize
define void @fa_terminate(i32 noundef %0) local_unnamed_addr #2 {
  tail call void @fa_flush() #35
  tail call void @fa_exit(i32 noundef %0) #36
  unreachable
}

; Function Attrs: noreturn optsize
declare void @fa_exit(i32 noundef) local_unnamed_addr #3

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, inaccessiblemem: none)
define void @fa_b_reset() local_unnamed_addr #4 {
  %1 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %1, align 8, !tbaa !5
  ret void
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare nonnull ptr @llvm.threadlocal.address.p0(ptr nonnull) #5

; Function Attrs: nounwind optsize
define void @fa_b_bytes(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  tail call fastcc void @reserve(i64 noundef %1) #35
  %3 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %4 = load ptr, ptr %3, align 8, !tbaa !9
  %5 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %6 = load i64, ptr %5, align 8, !tbaa !5
  %7 = getelementptr inbounds i8, ptr %4, i64 %6
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %7, ptr align 1 %0, i64 %1, i1 false)
  %8 = load i64, ptr %5, align 8, !tbaa !5
  %9 = add i64 %8, %1
  store i64 %9, ptr %5, align 8, !tbaa !5
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @reserve(i64 noundef %0) unnamed_addr #0 {
  %2 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %3 = load i64, ptr %2, align 8, !tbaa !5
  %4 = add i64 %3, %0
  %5 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_capacity)
  %6 = load i64, ptr %5, align 8, !tbaa !5
  %7 = icmp ugt i64 %4, %6
  br i1 %7, label %8, label %26

8:                                                ; preds = %1
  %9 = icmp ult i64 %6, 256
  %10 = shl i64 %6, 1
  %11 = select i1 %9, i64 256, i64 %10
  br label %12

12:                                               ; preds = %12, %8
  %13 = phi i64 [ %11, %8 ], [ %15, %12 ]
  %14 = icmp ult i64 %13, %4
  %15 = shl i64 %13, 1
  br i1 %14, label %12, label %16, !llvm.loop !11

16:                                               ; preds = %12
  %17 = tail call ptr @fa_alloc(i64 noundef %13, i64 noundef 1) #34
  %18 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %19 = load ptr, ptr %18, align 8, !tbaa !9
  %20 = load i64, ptr %2, align 8, !tbaa !5
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %17, ptr align 1 %19, i64 %20, i1 false)
  %21 = load ptr, ptr %18, align 8, !tbaa !9
  %22 = icmp eq ptr %21, null
  br i1 %22, label %25, label %23

23:                                               ; preds = %16
  %24 = load i64, ptr %5, align 8, !tbaa !5
  tail call void @fa_free(ptr noundef nonnull %21, i64 noundef %24, i64 noundef 1) #34
  br label %25

25:                                               ; preds = %23, %16
  store ptr %17, ptr %18, align 8, !tbaa !9
  store i64 %13, ptr %5, align 8, !tbaa !5
  br label %26

26:                                               ; preds = %1, %25
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_b_line(i32 noundef %0) local_unnamed_addr #0 {
  tail call fastcc void @reserve(i64 noundef 1) #35
  %2 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %3 = load ptr, ptr %2, align 8, !tbaa !9
  %4 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %5 = load i64, ptr %4, align 8, !tbaa !5
  %6 = getelementptr inbounds i8, ptr %3, i64 %5
  store i8 10, ptr %6, align 1, !tbaa !12
  %7 = load i64, ptr %4, align 8, !tbaa !5
  %8 = add i64 %7, 1
  store i64 %8, ptr %4, align 8, !tbaa !5
  %9 = load ptr, ptr %2, align 8, !tbaa !9
  tail call void @fa_out(i32 noundef %0, ptr noundef %9, i64 noundef %8) #35
  store i64 0, ptr %4, align 8, !tbaa !5
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_b_take(ptr nocapture noundef writeonly %0) local_unnamed_addr #0 {
  %2 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %3 = load i64, ptr %2, align 8, !tbaa !5
  %4 = icmp eq i64 %3, 0
  br i1 %4, label %9, label %5

5:                                                ; preds = %1
  %6 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %7 = load ptr, ptr %6, align 8, !tbaa !9
  %8 = tail call ptr @fa_alloc(i64 noundef %3, i64 noundef 1) #34, !noalias !13
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %8, ptr align 1 %7, i64 %3, i1 false), !noalias !13
  br label %9

9:                                                ; preds = %1, %5
  %10 = phi ptr [ %8, %5 ], [ @.str.34, %1 ]
  store ptr %10, ptr %0, align 8, !tbaa.struct !16
  %11 = getelementptr inbounds i8, ptr %0, i64 8
  store i64 %3, ptr %11, align 8, !tbaa.struct !17
  %12 = getelementptr inbounds i8, ptr %0, i64 16
  store i64 %3, ptr %12, align 8, !tbaa.struct !18
  store i64 0, ptr %2, align 8, !tbaa !5
  ret void
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.start.p0(i64 immarg, ptr nocapture) #6

; Function Attrs: mustprogress nocallback nofree nounwind willreturn memory(argmem: readwrite)
declare void @llvm.memcpy.p0.p0.i64(ptr noalias nocapture writeonly, ptr noalias nocapture readonly, i64, i1 immarg) #7

; Function Attrs: mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.end.p0(i64 immarg, ptr nocapture) #6

; Function Attrs: nounwind optsize
define void @fa_b_i64(i64 noundef %0) local_unnamed_addr #0 {
  %2 = tail call i64 @llvm.abs.i64(i64 %0, i1 false)
  %3 = lshr i64 %0, 63
  %4 = trunc i64 %3 to i32
  tail call fastcc void @append_u64(i64 noundef %2, i32 noundef %4) #35
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @append_u64(i64 noundef %0, i32 noundef %1) unnamed_addr #0 {
  %3 = alloca [24 x i8], align 16
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %3) #31
  br label %4

4:                                                ; preds = %4, %2
  %5 = phi i64 [ %0, %2 ], [ %12, %4 ]
  %6 = phi i64 [ 24, %2 ], [ %7, %4 ]
  %7 = add i64 %6, -1
  %8 = urem i64 %5, 10
  %9 = trunc i64 %8 to i8
  %10 = or disjoint i8 %9, 48
  %11 = getelementptr inbounds [24 x i8], ptr %3, i64 0, i64 %7
  store i8 %10, ptr %11, align 1, !tbaa !12
  %12 = udiv i64 %5, 10
  %13 = icmp ult i64 %5, 10
  br i1 %13, label %14, label %4, !llvm.loop !19

14:                                               ; preds = %4
  %15 = icmp eq i32 %1, 0
  br i1 %15, label %19, label %16

16:                                               ; preds = %14
  %17 = add i64 %6, -2
  %18 = getelementptr inbounds [24 x i8], ptr %3, i64 0, i64 %17
  store i8 45, ptr %18, align 1, !tbaa !12
  br label %19

19:                                               ; preds = %16, %14
  %20 = phi i64 [ %17, %16 ], [ %7, %14 ]
  %21 = getelementptr inbounds i8, ptr %3, i64 %20
  %22 = sub i64 24, %20
  tail call fastcc void @reserve(i64 noundef %22) #35
  %23 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %24 = load ptr, ptr %23, align 8, !tbaa !9
  %25 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %26 = load i64, ptr %25, align 8, !tbaa !5
  %27 = getelementptr inbounds i8, ptr %24, i64 %26
  call void @llvm.memcpy.p0.p0.i64(ptr align 1 %27, ptr nonnull align 1 %21, i64 %22, i1 false)
  %28 = load i64, ptr %25, align 8, !tbaa !5
  %29 = add i64 %28, %22
  store i64 %29, ptr %25, align 8, !tbaa !5
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %3) #31
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_b_u64(i64 noundef %0) local_unnamed_addr #0 {
  tail call fastcc void @append_u64(i64 noundef %0, i32 noundef 0) #35
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_b_i128(i64 noundef %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = zext i64 %1 to i128
  %4 = shl nuw i128 %3, 64
  %5 = zext i64 %0 to i128
  %6 = or disjoint i128 %4, %5
  %7 = icmp slt i64 %1, 0
  %8 = lshr i64 %1, 63
  %9 = trunc i64 %8 to i32
  %10 = sub i128 0, %6
  %11 = select i1 %7, i128 %10, i128 %6
  %12 = trunc i128 %11 to i64
  %13 = lshr i128 %11, 64
  %14 = trunc i128 %13 to i64
  tail call fastcc void @append_u128(i64 noundef %12, i64 noundef %14, i32 noundef %9) #35
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @append_u128(i64 noundef %0, i64 noundef %1, i32 noundef %2) unnamed_addr #0 {
  %4 = alloca [4 x i32], align 16
  %5 = alloca [48 x i8], align 16
  %6 = icmp eq i64 %1, 0
  br i1 %6, label %7, label %8

7:                                                ; preds = %3
  tail call fastcc void @append_u64(i64 noundef %0, i32 noundef %2) #35
  br label %73

8:                                                ; preds = %3
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %4) #31
  %9 = lshr i64 %1, 32
  %10 = trunc i64 %9 to i32
  store i32 %10, ptr %4, align 16, !tbaa !20
  %11 = getelementptr inbounds i32, ptr %4, i64 1
  %12 = trunc i64 %1 to i32
  store i32 %12, ptr %11, align 4, !tbaa !20
  %13 = getelementptr inbounds i32, ptr %4, i64 2
  %14 = lshr i64 %0, 32
  %15 = trunc i64 %14 to i32
  store i32 %15, ptr %13, align 8, !tbaa !20
  %16 = getelementptr inbounds i32, ptr %4, i64 3
  %17 = trunc i64 %0 to i32
  store i32 %17, ptr %16, align 4, !tbaa !20
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %5) #31
  br label %18

18:                                               ; preds = %40, %8
  %19 = phi i64 [ 48, %8 ], [ %45, %40 ]
  br label %20

20:                                               ; preds = %20, %18
  %21 = phi i64 [ 0, %18 ], [ %31, %20 ]
  %22 = phi i64 [ 0, %18 ], [ %30, %20 ]
  %23 = shl nuw nsw i64 %22, 32
  %24 = getelementptr inbounds i32, ptr %4, i64 %21
  %25 = load i32, ptr %24, align 4, !tbaa !20
  %26 = zext i32 %25 to i64
  %27 = or disjoint i64 %23, %26
  %28 = udiv i64 %27, 1000000000
  %29 = trunc i64 %28 to i32
  store i32 %29, ptr %24, align 4, !tbaa !20
  %30 = urem i64 %27, 1000000000
  %31 = add nuw nsw i64 %21, 1
  %32 = icmp eq i64 %31, 4
  br i1 %32, label %33, label %20, !llvm.loop !22

33:                                               ; preds = %20
  %34 = trunc i64 %30 to i32
  %35 = load <4 x i32>, ptr %4, align 16, !tbaa !20
  %36 = tail call i32 @llvm.vector.reduce.or.v4i32(<4 x i32> %35)
  %37 = icmp ne i32 %36, 0
  %38 = icmp ne i32 %34, 0
  %39 = select i1 %37, i1 true, i1 %38
  br i1 %39, label %41, label %56

40:                                               ; preds = %41
  br i1 %37, label %18, label %56

41:                                               ; preds = %33, %41
  %42 = phi i32 [ %51, %41 ], [ 0, %33 ]
  %43 = phi i32 [ %50, %41 ], [ %34, %33 ]
  %44 = phi i64 [ %45, %41 ], [ %19, %33 ]
  %45 = add i64 %44, -1
  %46 = urem i32 %43, 10
  %47 = trunc i32 %46 to i8
  %48 = or disjoint i8 %47, 48
  %49 = getelementptr inbounds [48 x i8], ptr %5, i64 0, i64 %45
  store i8 %48, ptr %49, align 1, !tbaa !12
  %50 = udiv i32 %43, 10
  %51 = add nuw nsw i32 %42, 1
  %52 = icmp ult i32 %42, 8
  %53 = icmp ugt i32 %43, 9
  %54 = select i1 %37, i1 true, i1 %53
  %55 = select i1 %52, i1 %54, i1 false
  br i1 %55, label %41, label %40, !llvm.loop !23

56:                                               ; preds = %33, %40
  %57 = phi i64 [ %45, %40 ], [ %19, %33 ]
  %58 = icmp eq i32 %2, 0
  br i1 %58, label %62, label %59

59:                                               ; preds = %56
  %60 = add i64 %57, -1
  %61 = getelementptr inbounds [48 x i8], ptr %5, i64 0, i64 %60
  store i8 45, ptr %61, align 1, !tbaa !12
  br label %62

62:                                               ; preds = %59, %56
  %63 = phi i64 [ %60, %59 ], [ %57, %56 ]
  %64 = getelementptr inbounds i8, ptr %5, i64 %63
  %65 = sub i64 48, %63
  tail call fastcc void @reserve(i64 noundef %65) #35
  %66 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %67 = load ptr, ptr %66, align 8, !tbaa !9
  %68 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %69 = load i64, ptr %68, align 8, !tbaa !5
  %70 = getelementptr inbounds i8, ptr %67, i64 %69
  call void @llvm.memcpy.p0.p0.i64(ptr align 1 %70, ptr nonnull align 1 %64, i64 %65, i1 false)
  %71 = load i64, ptr %68, align 8, !tbaa !5
  %72 = add i64 %71, %65
  store i64 %72, ptr %68, align 8, !tbaa !5
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %5) #31
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %4) #31
  br label %73

73:                                               ; preds = %62, %7
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_b_u128(i64 noundef %0, i64 noundef %1) local_unnamed_addr #0 {
  tail call fastcc void @append_u128(i64 noundef %0, i64 noundef %1, i32 noundef 0) #35
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_b_bool(i32 noundef %0) local_unnamed_addr #0 {
  %2 = icmp eq i32 %0, 0
  %3 = select i1 %2, ptr @.str.1, ptr @.str
  tail call fastcc void @append_c(ptr noundef nonnull %3) #35
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @append_c(ptr nocapture noundef readonly %0) unnamed_addr #0 {
  br label %2

2:                                                ; preds = %2, %1
  %3 = phi i64 [ 0, %1 ], [ %7, %2 ]
  %4 = getelementptr inbounds i8, ptr %0, i64 %3
  %5 = load i8, ptr %4, align 1, !tbaa !12
  %6 = icmp eq i8 %5, 0
  %7 = add i64 %3, 1
  br i1 %6, label %8, label %2, !llvm.loop !24

8:                                                ; preds = %2
  tail call fastcc void @reserve(i64 noundef %3) #35
  %9 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %10 = load ptr, ptr %9, align 8, !tbaa !9
  %11 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %12 = load i64, ptr %11, align 8, !tbaa !5
  %13 = getelementptr inbounds i8, ptr %10, i64 %12
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %13, ptr nonnull align 1 %0, i64 %3, i1 false)
  %14 = load i64, ptr %11, align 8, !tbaa !5
  %15 = add i64 %14, %3
  store i64 %15, ptr %11, align 8, !tbaa !5
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_b_f64(double noundef %0) local_unnamed_addr #0 {
  tail call fastcc void @append_double(double noundef %0) #35
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @append_double(double noundef %0) unnamed_addr #0 {
  %2 = alloca [40 x i8], align 16
  %3 = alloca [24 x i8], align 16
  %4 = fcmp ord double %0, 0.000000e+00
  br i1 %4, label %6, label %5

5:                                                ; preds = %1
  tail call fastcc void @append_c(ptr noundef nonnull @.str.35) #35
  br label %205

6:                                                ; preds = %1
  %7 = fcmp oeq double %0, 0.000000e+00
  br i1 %7, label %8, label %16

8:                                                ; preds = %6
  tail call fastcc void @reserve(i64 noundef 1) #35
  %9 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %10 = load ptr, ptr %9, align 8, !tbaa !9
  %11 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %12 = load i64, ptr %11, align 8, !tbaa !5
  %13 = getelementptr inbounds i8, ptr %10, i64 %12
  store i8 48, ptr %13, align 1, !tbaa !12
  %14 = load i64, ptr %11, align 8, !tbaa !5
  %15 = add i64 %14, 1
  store i64 %15, ptr %11, align 8, !tbaa !5
  br label %205

16:                                               ; preds = %6
  %17 = fcmp olt double %0, 0.000000e+00
  br i1 %17, label %18, label %27

18:                                               ; preds = %16
  tail call fastcc void @reserve(i64 noundef 1) #35
  %19 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %20 = load ptr, ptr %19, align 8, !tbaa !9
  %21 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %22 = load i64, ptr %21, align 8, !tbaa !5
  %23 = getelementptr inbounds i8, ptr %20, i64 %22
  store i8 45, ptr %23, align 1, !tbaa !12
  %24 = load i64, ptr %21, align 8, !tbaa !5
  %25 = add i64 %24, 1
  store i64 %25, ptr %21, align 8, !tbaa !5
  %26 = fneg double %0
  br label %27

27:                                               ; preds = %18, %16
  %28 = phi double [ %26, %18 ], [ %0, %16 ]
  %29 = fcmp ogt double %28, 0x7FEFFFFFFFFFFFFF
  br i1 %29, label %30, label %31

30:                                               ; preds = %27
  tail call fastcc void @append_c(ptr noundef nonnull @.str.36) #35
  br label %205

31:                                               ; preds = %27
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %3) #31
  call void @llvm.lifetime.start.p0(i64 40, ptr nonnull %2) #31
  br label %32

32:                                               ; preds = %32, %31
  %33 = phi i32 [ 1, %31 ], [ %38, %32 ]
  %34 = add nsw i32 %33, -1
  %35 = call i32 (ptr, i64, ptr, ...) @snprintf(ptr noundef nonnull %2, i64 noundef 40, ptr noundef nonnull @.str.38, i32 noundef %34, double noundef %28) #34
  %36 = call double @strtod(ptr noundef nonnull %2, ptr noundef null) #34
  %37 = fcmp oeq double %36, %28
  %38 = add nuw nsw i32 %33, 1
  %39 = icmp eq i32 %38, 18
  %40 = select i1 %37, i1 true, i1 %39
  br i1 %40, label %41, label %32, !llvm.loop !25

41:                                               ; preds = %32
  %42 = load i8, ptr %2, align 16, !tbaa !12
  %43 = icmp eq i8 %42, 101
  br i1 %43, label %59, label %44

44:                                               ; preds = %41, %54
  %45 = phi i8 [ %57, %54 ], [ %42, %41 ]
  %46 = phi ptr [ %56, %54 ], [ %2, %41 ]
  %47 = phi i32 [ %55, %54 ], [ 0, %41 ]
  %48 = add i8 %45, -48
  %49 = icmp ult i8 %48, 10
  br i1 %49, label %50, label %54

50:                                               ; preds = %44
  %51 = sext i32 %47 to i64
  %52 = getelementptr inbounds i8, ptr %3, i64 %51
  store i8 %45, ptr %52, align 1, !tbaa !12
  %53 = add nsw i32 %47, 1
  br label %54

54:                                               ; preds = %50, %44
  %55 = phi i32 [ %53, %50 ], [ %47, %44 ]
  %56 = getelementptr inbounds i8, ptr %46, i64 1
  %57 = load i8, ptr %56, align 1, !tbaa !12
  %58 = icmp eq i8 %57, 101
  br i1 %58, label %59, label %44, !llvm.loop !26

59:                                               ; preds = %54, %41
  %60 = phi i32 [ 0, %41 ], [ %55, %54 ]
  %61 = phi ptr [ %2, %41 ], [ %56, %54 ]
  %62 = getelementptr inbounds i8, ptr %61, i64 1
  %63 = load i8, ptr %62, align 1, !tbaa !12
  %64 = getelementptr inbounds i8, ptr %61, i64 2
  %65 = load i8, ptr %64, align 1, !tbaa !12
  %66 = icmp eq i8 %65, 0
  br i1 %66, label %67, label %71

67:                                               ; preds = %71, %59
  %68 = phi i32 [ 0, %59 ], [ %78, %71 ]
  %69 = zext i32 %60 to i64
  %70 = call i32 @llvm.smin.i32(i32 %60, i32 1)
  br label %82

71:                                               ; preds = %59, %71
  %72 = phi i8 [ %80, %71 ], [ %65, %59 ]
  %73 = phi i32 [ %78, %71 ], [ 0, %59 ]
  %74 = phi ptr [ %79, %71 ], [ %64, %59 ]
  %75 = sext i8 %72 to i32
  %76 = mul nsw i32 %73, 10
  %77 = add nsw i32 %75, -48
  %78 = add i32 %77, %76
  %79 = getelementptr inbounds i8, ptr %74, i64 1
  %80 = load i8, ptr %79, align 1, !tbaa !12
  %81 = icmp eq i8 %80, 0
  br i1 %81, label %67, label %71, !llvm.loop !27

82:                                               ; preds = %86, %67
  %83 = phi i64 [ %69, %67 ], [ %87, %86 ]
  %84 = trunc i64 %83 to i32
  %85 = icmp sgt i32 %84, 1
  br i1 %85, label %86, label %91

86:                                               ; preds = %82
  %87 = add nsw i64 %83, -1
  %88 = getelementptr inbounds i8, ptr %3, i64 %87
  %89 = load i8, ptr %88, align 1, !tbaa !12
  %90 = icmp eq i8 %89, 48
  br i1 %90, label %82, label %91, !llvm.loop !28

91:                                               ; preds = %82, %86
  %92 = phi i32 [ %70, %82 ], [ %84, %86 ]
  %93 = icmp eq i8 %63, 45
  %94 = sub nsw i32 0, %68
  %95 = select i1 %93, i32 %94, i32 %68
  %96 = add nsw i32 %95, 1
  call void @llvm.lifetime.end.p0(i64 40, ptr nonnull %2) #31
  %97 = icmp sle i32 %92, %96
  %98 = icmp slt i32 %95, 21
  %99 = and i1 %98, %97
  br i1 %99, label %100, label %119

100:                                              ; preds = %91
  %101 = sext i32 %92 to i64
  call fastcc void @reserve(i64 noundef %101) #35
  %102 = call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %103 = load ptr, ptr %102, align 8, !tbaa !9
  %104 = call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %105 = load i64, ptr %104, align 8, !tbaa !5
  %106 = getelementptr inbounds i8, ptr %103, i64 %105
  call void @llvm.memcpy.p0.p0.i64(ptr align 1 %106, ptr nonnull align 16 %3, i64 %101, i1 false)
  %107 = load i64, ptr %104, align 8, !tbaa !5
  %108 = add i64 %107, %101
  store i64 %108, ptr %104, align 8, !tbaa !5
  %109 = icmp sgt i32 %92, %95
  br i1 %109, label %204, label %110

110:                                              ; preds = %100, %110
  %111 = phi i32 [ %117, %110 ], [ %92, %100 ]
  call fastcc void @reserve(i64 noundef 1) #35
  %112 = load ptr, ptr %102, align 8, !tbaa !9
  %113 = load i64, ptr %104, align 8, !tbaa !5
  %114 = getelementptr inbounds i8, ptr %112, i64 %113
  store i8 48, ptr %114, align 1, !tbaa !12
  %115 = load i64, ptr %104, align 8, !tbaa !5
  %116 = add i64 %115, 1
  store i64 %116, ptr %104, align 8, !tbaa !5
  %117 = add i32 %111, 1
  %118 = icmp eq i32 %111, %95
  br i1 %118, label %204, label %110, !llvm.loop !29

119:                                              ; preds = %91
  %120 = icmp ult i32 %95, 21
  br i1 %120, label %121, label %143

121:                                              ; preds = %119
  %122 = zext nneg i32 %96 to i64
  call fastcc void @reserve(i64 noundef %122) #35
  %123 = call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %124 = load ptr, ptr %123, align 8, !tbaa !9
  %125 = call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %126 = load i64, ptr %125, align 8, !tbaa !5
  %127 = getelementptr inbounds i8, ptr %124, i64 %126
  call void @llvm.memcpy.p0.p0.i64(ptr align 1 %127, ptr nonnull align 16 %3, i64 %122, i1 false)
  %128 = load i64, ptr %125, align 8, !tbaa !5
  %129 = add i64 %128, %122
  store i64 %129, ptr %125, align 8, !tbaa !5
  call fastcc void @reserve(i64 noundef 1) #35
  %130 = load ptr, ptr %123, align 8, !tbaa !9
  %131 = load i64, ptr %125, align 8, !tbaa !5
  %132 = getelementptr inbounds i8, ptr %130, i64 %131
  store i8 46, ptr %132, align 1, !tbaa !12
  %133 = load i64, ptr %125, align 8, !tbaa !5
  %134 = add i64 %133, 1
  store i64 %134, ptr %125, align 8, !tbaa !5
  %135 = getelementptr inbounds i8, ptr %3, i64 %122
  %136 = sub nsw i32 %92, %96
  %137 = sext i32 %136 to i64
  call fastcc void @reserve(i64 noundef %137) #35
  %138 = load ptr, ptr %123, align 8, !tbaa !9
  %139 = load i64, ptr %125, align 8, !tbaa !5
  %140 = getelementptr inbounds i8, ptr %138, i64 %139
  call void @llvm.memcpy.p0.p0.i64(ptr align 1 %140, ptr nonnull align 1 %135, i64 %137, i1 false)
  %141 = load i64, ptr %125, align 8, !tbaa !5
  %142 = add i64 %141, %137
  store i64 %142, ptr %125, align 8, !tbaa !5
  br label %204

143:                                              ; preds = %119
  %144 = icmp ugt i32 %95, -7
  %145 = call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  br i1 %144, label %146, label %166

146:                                              ; preds = %143
  call fastcc void @append_c(ptr noundef nonnull @.str.37) #35
  %147 = icmp eq i32 %95, -1
  %148 = call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  br i1 %147, label %149, label %156

149:                                              ; preds = %156, %146
  %150 = sext i32 %92 to i64
  call fastcc void @reserve(i64 noundef %150) #35
  %151 = load ptr, ptr %145, align 8, !tbaa !9
  %152 = load i64, ptr %148, align 8, !tbaa !5
  %153 = getelementptr inbounds i8, ptr %151, i64 %152
  call void @llvm.memcpy.p0.p0.i64(ptr align 1 %153, ptr nonnull align 16 %3, i64 %150, i1 false)
  %154 = load i64, ptr %148, align 8, !tbaa !5
  %155 = add i64 %154, %150
  store i64 %155, ptr %148, align 8, !tbaa !5
  br label %204

156:                                              ; preds = %146, %156
  %157 = phi i32 [ %163, %156 ], [ 0, %146 ]
  call fastcc void @reserve(i64 noundef 1) #35
  %158 = load ptr, ptr %145, align 8, !tbaa !9
  %159 = load i64, ptr %148, align 8, !tbaa !5
  %160 = getelementptr inbounds i8, ptr %158, i64 %159
  store i8 48, ptr %160, align 1, !tbaa !12
  %161 = load i64, ptr %148, align 8, !tbaa !5
  %162 = add i64 %161, 1
  store i64 %162, ptr %148, align 8, !tbaa !5
  %163 = add nuw nsw i32 %157, 1
  %164 = xor i32 %95, %163
  %165 = icmp eq i32 %164, -1
  br i1 %165, label %149, label %156, !llvm.loop !30

166:                                              ; preds = %143
  %167 = load i8, ptr %3, align 16, !tbaa !12
  call fastcc void @reserve(i64 noundef 1) #35
  %168 = load ptr, ptr %145, align 8, !tbaa !9
  %169 = call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %170 = load i64, ptr %169, align 8, !tbaa !5
  %171 = getelementptr inbounds i8, ptr %168, i64 %170
  store i8 %167, ptr %171, align 1, !tbaa !12
  %172 = load i64, ptr %169, align 8, !tbaa !5
  %173 = add i64 %172, 1
  store i64 %173, ptr %169, align 8, !tbaa !5
  %174 = icmp sgt i32 %92, 1
  br i1 %174, label %175, label %189

175:                                              ; preds = %166
  call fastcc void @reserve(i64 noundef 1) #35
  %176 = load ptr, ptr %145, align 8, !tbaa !9
  %177 = load i64, ptr %169, align 8, !tbaa !5
  %178 = getelementptr inbounds i8, ptr %176, i64 %177
  store i8 46, ptr %178, align 1, !tbaa !12
  %179 = load i64, ptr %169, align 8, !tbaa !5
  %180 = add i64 %179, 1
  store i64 %180, ptr %169, align 8, !tbaa !5
  %181 = getelementptr inbounds i8, ptr %3, i64 1
  %182 = add nsw i32 %92, -1
  %183 = zext nneg i32 %182 to i64
  call fastcc void @reserve(i64 noundef %183) #35
  %184 = load ptr, ptr %145, align 8, !tbaa !9
  %185 = load i64, ptr %169, align 8, !tbaa !5
  %186 = getelementptr inbounds i8, ptr %184, i64 %185
  call void @llvm.memcpy.p0.p0.i64(ptr align 1 %186, ptr nonnull align 1 %181, i64 %183, i1 false)
  %187 = load i64, ptr %169, align 8, !tbaa !5
  %188 = add i64 %187, %183
  store i64 %188, ptr %169, align 8, !tbaa !5
  br label %189

189:                                              ; preds = %175, %166
  call fastcc void @reserve(i64 noundef 1) #35
  %190 = load ptr, ptr %145, align 8, !tbaa !9
  %191 = load i64, ptr %169, align 8, !tbaa !5
  %192 = getelementptr inbounds i8, ptr %190, i64 %191
  store i8 101, ptr %192, align 1, !tbaa !12
  %193 = load i64, ptr %169, align 8, !tbaa !5
  %194 = add i64 %193, 1
  store i64 %194, ptr %169, align 8, !tbaa !5
  %195 = icmp slt i32 %95, 0
  %196 = select i1 %195, i8 45, i8 43
  call fastcc void @reserve(i64 noundef 1) #35
  %197 = load ptr, ptr %145, align 8, !tbaa !9
  %198 = load i64, ptr %169, align 8, !tbaa !5
  %199 = getelementptr inbounds i8, ptr %197, i64 %198
  store i8 %196, ptr %199, align 1, !tbaa !12
  %200 = load i64, ptr %169, align 8, !tbaa !5
  %201 = add i64 %200, 1
  store i64 %201, ptr %169, align 8, !tbaa !5
  %202 = call i32 @llvm.abs.i32(i32 %68, i1 true)
  %203 = zext nneg i32 %202 to i64
  call fastcc void @append_u64(i64 noundef %203, i32 noundef 0) #35
  br label %204

204:                                              ; preds = %110, %100, %121, %189, %149
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %3) #31
  br label %205

205:                                              ; preds = %204, %30, %8, %5
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_b_f32(float noundef %0) local_unnamed_addr #0 {
  %2 = alloca [48 x i8], align 16
  %3 = alloca [40 x i8], align 16
  %4 = alloca [3 x i64], align 16
  %5 = fcmp ueq float %0, 0.000000e+00
  %6 = fsub float %0, %0
  %7 = fcmp une float %6, 0.000000e+00
  %8 = or i1 %5, %7
  %9 = fpext float %0 to double
  br i1 %8, label %10, label %11

10:                                               ; preds = %1
  tail call fastcc void @append_double(double noundef %9) #35
  br label %107

11:                                               ; preds = %1
  call void @llvm.lifetime.start.p0(i64 40, ptr nonnull %3) #31
  %12 = getelementptr inbounds i64, ptr %4, i64 1
  %13 = getelementptr inbounds i64, ptr %4, i64 2
  %14 = insertelement <2 x double> poison, double %9, i64 0
  %15 = shufflevector <2 x double> %14, <2 x double> poison, <2 x i32> zeroinitializer
  br label %16

16:                                               ; preds = %102, %11
  %17 = phi i32 [ 1, %11 ], [ %103, %102 ]
  %18 = add nsw i32 %17, -1
  %19 = call i32 (ptr, i64, ptr, ...) @snprintf(ptr noundef nonnull %3, i64 noundef 40, ptr noundef nonnull @.str.38, i32 noundef %18, double noundef %9) #34
  %20 = load i8, ptr %3, align 16, !tbaa !12
  %21 = icmp eq i8 %20, 45
  %22 = zext i1 %21 to i64
  %23 = getelementptr inbounds i8, ptr %3, i64 %22
  %24 = load i8, ptr %23, align 1, !tbaa !12
  %25 = icmp eq i8 %24, 101
  br i1 %25, label %39, label %26

26:                                               ; preds = %16, %26
  %27 = phi i8 [ %37, %26 ], [ %24, %16 ]
  %28 = phi i64 [ %35, %26 ], [ 0, %16 ]
  %29 = phi ptr [ %36, %26 ], [ %23, %16 ]
  %30 = add i8 %27, -48
  %31 = icmp ult i8 %30, 10
  %32 = mul nsw i64 %28, 10
  %33 = zext nneg i8 %30 to i64
  %34 = add nsw i64 %32, %33
  %35 = select i1 %31, i64 %34, i64 %28
  %36 = getelementptr inbounds i8, ptr %29, i64 1
  %37 = load i8, ptr %36, align 1, !tbaa !12
  %38 = icmp eq i8 %37, 101
  br i1 %38, label %39, label %26, !llvm.loop !31

39:                                               ; preds = %26, %16
  %40 = phi ptr [ %23, %16 ], [ %36, %26 ]
  %41 = phi i64 [ 0, %16 ], [ %35, %26 ]
  %42 = getelementptr inbounds i8, ptr %40, i64 1
  %43 = load i8, ptr %42, align 1, !tbaa !12
  %44 = icmp eq i8 %43, 45
  %45 = getelementptr inbounds i8, ptr %40, i64 2
  %46 = load i8, ptr %45, align 1, !tbaa !12
  %47 = icmp eq i8 %46, 0
  br i1 %47, label %59, label %48

48:                                               ; preds = %39, %48
  %49 = phi i8 [ %57, %48 ], [ %46, %39 ]
  %50 = phi i32 [ %55, %48 ], [ 0, %39 ]
  %51 = phi ptr [ %56, %48 ], [ %45, %39 ]
  %52 = sext i8 %49 to i32
  %53 = mul nsw i32 %50, 10
  %54 = add nsw i32 %52, -48
  %55 = add i32 %54, %53
  %56 = getelementptr inbounds i8, ptr %51, i64 1
  %57 = load i8, ptr %56, align 1, !tbaa !12
  %58 = icmp eq i8 %57, 0
  br i1 %58, label %59, label %48, !llvm.loop !32

59:                                               ; preds = %48, %39
  %60 = phi i32 [ 0, %39 ], [ %55, %48 ]
  %61 = sub nsw i32 0, %60
  %62 = select i1 %44, i32 %61, i32 %60
  %63 = sub nsw i32 %62, %18
  %64 = sub nsw i64 0, %41
  %65 = select i1 %21, i64 %64, i64 %41
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %4) #31
  store i64 %65, ptr %4, align 16, !tbaa !5
  %66 = add nsw i64 %65, -1
  store i64 %66, ptr %12, align 8, !tbaa !5
  %67 = add nsw i64 %65, 1
  store i64 %67, ptr %13, align 16, !tbaa !5
  br label %71

68:                                               ; preds = %94
  %69 = add nuw nsw i64 %72, 1
  %70 = icmp eq i64 %69, 3
  br i1 %70, label %100, label %71, !llvm.loop !33

71:                                               ; preds = %68, %59
  %72 = phi i64 [ 0, %59 ], [ %69, %68 ]
  %73 = phi double [ 0.000000e+00, %59 ], [ %96, %68 ]
  %74 = phi i32 [ 0, %59 ], [ %95, %68 ]
  %75 = getelementptr inbounds [3 x i64], ptr %4, i64 0, i64 %72
  %76 = load i64, ptr %75, align 8, !tbaa !5
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %2) #31
  %77 = call i32 (ptr, i64, ptr, ...) @snprintf(ptr noundef nonnull %2, i64 noundef 48, ptr noundef nonnull @.str.39, i64 noundef %76, i32 noundef %63) #34
  %78 = call double @strtod(ptr noundef nonnull %2, ptr noundef null) #34
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %2) #31
  %79 = fptrunc double %78 to float
  %80 = fcmp oeq float %79, %0
  br i1 %80, label %81, label %94

81:                                               ; preds = %71
  %82 = insertelement <2 x double> poison, double %78, i64 0
  %83 = insertelement <2 x double> %82, double %73, i64 1
  %84 = fsub <2 x double> %83, %15
  %85 = fsub <2 x double> %15, %83
  %86 = fcmp olt <2 x double> %84, zeroinitializer
  %87 = select <2 x i1> %86, <2 x double> %85, <2 x double> %84
  %88 = icmp eq i32 %74, 0
  %89 = extractelement <2 x double> %87, i64 0
  %90 = extractelement <2 x double> %87, i64 1
  %91 = fcmp olt double %89, %90
  %92 = select i1 %88, i1 true, i1 %91
  br i1 %92, label %93, label %94

93:                                               ; preds = %81
  br label %94

94:                                               ; preds = %93, %81, %71
  %95 = phi i32 [ 1, %93 ], [ %74, %71 ], [ 1, %81 ]
  %96 = phi double [ %78, %93 ], [ %73, %71 ], [ %73, %81 ]
  %97 = icmp eq i32 %95, 0
  %98 = icmp ne i64 %72, 0
  %99 = or i1 %98, %97
  br i1 %99, label %68, label %101

100:                                              ; preds = %68
  br i1 %97, label %102, label %101

101:                                              ; preds = %100, %94
  call fastcc void @append_double(double noundef %96) #35
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %4) #31
  br label %106

102:                                              ; preds = %100
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %4) #31
  %103 = add nuw nsw i32 %17, 1
  %104 = icmp eq i32 %103, 10
  br i1 %104, label %105, label %16, !llvm.loop !34

105:                                              ; preds = %102
  call fastcc void @append_double(double noundef %9) #35
  br label %106

106:                                              ; preds = %105, %101
  call void @llvm.lifetime.end.p0(i64 40, ptr nonnull %3) #31
  br label %107

107:                                              ; preds = %10, %106
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_b_percentage(double noundef %0) local_unnamed_addr #0 {
  %2 = alloca [40 x i8], align 16
  %3 = fmul double %0, 1.000000e+02
  %4 = fcmp ueq double %3, 0.000000e+00
  %5 = fsub double %3, %3
  %6 = fcmp une double %5, 0.000000e+00
  %7 = or i1 %4, %6
  br i1 %7, label %8, label %9

8:                                                ; preds = %1
  tail call fastcc void @append_double(double noundef %3) #35
  br label %12

9:                                                ; preds = %1
  call void @llvm.lifetime.start.p0(i64 40, ptr nonnull %2) #31
  %10 = call i32 (ptr, i64, ptr, ...) @snprintf(ptr noundef nonnull %2, i64 noundef 40, ptr noundef nonnull @.str.40, double noundef %3) #34
  %11 = call double @strtod(ptr noundef nonnull %2, ptr noundef null) #34
  call fastcc void @append_double(double noundef %11) #35
  call void @llvm.lifetime.end.p0(i64 40, ptr nonnull %2) #31
  br label %12

12:                                               ; preds = %8, %9
  call fastcc void @reserve(i64 noundef 1) #35
  %13 = call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %14 = load ptr, ptr %13, align 8, !tbaa !9
  %15 = call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %16 = load i64, ptr %15, align 8, !tbaa !5
  %17 = getelementptr inbounds i8, ptr %14, i64 %16
  store i8 37, ptr %17, align 1, !tbaa !12
  %18 = load i64, ptr %15, align 8, !tbaa !5
  %19 = add i64 %18, 1
  store i64 %19, ptr %15, align 8, !tbaa !5
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_b_quoted(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  tail call fastcc void @reserve(i64 noundef 1) #35
  %3 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %4 = load ptr, ptr %3, align 8, !tbaa !9
  %5 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %6 = load i64, ptr %5, align 8, !tbaa !5
  %7 = getelementptr inbounds i8, ptr %4, i64 %6
  store i8 34, ptr %7, align 1, !tbaa !12
  %8 = load i64, ptr %5, align 8, !tbaa !5
  %9 = add i64 %8, 1
  store i64 %9, ptr %5, align 8, !tbaa !5
  %10 = icmp eq i64 %1, 0
  br i1 %10, label %11, label %17

11:                                               ; preds = %59, %2
  tail call fastcc void @reserve(i64 noundef 1) #35
  %12 = load ptr, ptr %3, align 8, !tbaa !9
  %13 = load i64, ptr %5, align 8, !tbaa !5
  %14 = getelementptr inbounds i8, ptr %12, i64 %13
  store i8 34, ptr %14, align 1, !tbaa !12
  %15 = load i64, ptr %5, align 8, !tbaa !5
  %16 = add i64 %15, 1
  store i64 %16, ptr %5, align 8, !tbaa !5
  ret void

17:                                               ; preds = %2, %59
  %18 = phi i64 [ %60, %59 ], [ 0, %2 ]
  %19 = getelementptr inbounds i8, ptr %0, i64 %18
  %20 = load i8, ptr %19, align 1, !tbaa !12
  %21 = zext i8 %20 to i32
  switch i8 %20, label %30 [
    i8 92, label %22
    i8 34, label %23
    i8 10, label %24
    i8 13, label %25
    i8 9, label %26
    i8 0, label %27
    i8 123, label %28
    i8 125, label %29
  ]

22:                                               ; preds = %17
  tail call fastcc void @append_c(ptr noundef nonnull @.str.2) #35
  br label %59

23:                                               ; preds = %17
  tail call fastcc void @append_c(ptr noundef nonnull @.str.3) #35
  br label %59

24:                                               ; preds = %17
  tail call fastcc void @append_c(ptr noundef nonnull @.str.4) #35
  br label %59

25:                                               ; preds = %17
  tail call fastcc void @append_c(ptr noundef nonnull @.str.5) #35
  br label %59

26:                                               ; preds = %17
  tail call fastcc void @append_c(ptr noundef nonnull @.str.6) #35
  br label %59

27:                                               ; preds = %17
  tail call fastcc void @append_c(ptr noundef nonnull @.str.7) #35
  br label %59

28:                                               ; preds = %17
  tail call fastcc void @append_c(ptr noundef nonnull @.str.8) #35
  br label %59

29:                                               ; preds = %17
  tail call fastcc void @append_c(ptr noundef nonnull @.str.9) #35
  br label %59

30:                                               ; preds = %17
  %31 = icmp ult i8 %20, 32
  %32 = icmp eq i8 %20, 127
  %33 = or i1 %31, %32
  br i1 %33, label %34, label %53

34:                                               ; preds = %30
  tail call fastcc void @append_c(ptr noundef nonnull @.str.10) #35
  %35 = lshr i32 %21, 4
  %36 = zext nneg i32 %35 to i64
  %37 = getelementptr inbounds [17 x i8], ptr @fa_b_quoted.hex, i64 0, i64 %36
  %38 = load i8, ptr %37, align 1, !tbaa !12
  tail call fastcc void @reserve(i64 noundef 1) #35
  %39 = load ptr, ptr %3, align 8, !tbaa !9
  %40 = load i64, ptr %5, align 8, !tbaa !5
  %41 = getelementptr inbounds i8, ptr %39, i64 %40
  store i8 %38, ptr %41, align 1, !tbaa !12
  %42 = load i64, ptr %5, align 8, !tbaa !5
  %43 = add i64 %42, 1
  store i64 %43, ptr %5, align 8, !tbaa !5
  %44 = and i32 %21, 15
  %45 = zext nneg i32 %44 to i64
  %46 = getelementptr inbounds [17 x i8], ptr @fa_b_quoted.hex, i64 0, i64 %45
  %47 = load i8, ptr %46, align 1, !tbaa !12
  tail call fastcc void @reserve(i64 noundef 1) #35
  %48 = load ptr, ptr %3, align 8, !tbaa !9
  %49 = load i64, ptr %5, align 8, !tbaa !5
  %50 = getelementptr inbounds i8, ptr %48, i64 %49
  store i8 %47, ptr %50, align 1, !tbaa !12
  %51 = load i64, ptr %5, align 8, !tbaa !5
  %52 = add i64 %51, 1
  store i64 %52, ptr %5, align 8, !tbaa !5
  br label %59

53:                                               ; preds = %30
  tail call fastcc void @reserve(i64 noundef 1) #35
  %54 = load ptr, ptr %3, align 8, !tbaa !9
  %55 = load i64, ptr %5, align 8, !tbaa !5
  %56 = getelementptr inbounds i8, ptr %54, i64 %55
  store i8 %20, ptr %56, align 1, !tbaa !12
  %57 = load i64, ptr %5, align 8, !tbaa !5
  %58 = add i64 %57, 1
  store i64 %58, ptr %5, align 8, !tbaa !5
  br label %59

59:                                               ; preds = %34, %53, %29, %28, %27, %26, %25, %24, %23, %22
  %60 = add nuw i64 %18, 1
  %61 = icmp eq i64 %60, %1
  br i1 %61, label %11, label %17, !llvm.loop !35
}

; Function Attrs: nounwind optsize
define void @fa_b_string_array(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  tail call fastcc void @reserve(i64 noundef 1) #35
  %3 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %4 = load ptr, ptr %3, align 8, !tbaa !9
  %5 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %6 = load i64, ptr %5, align 8, !tbaa !5
  %7 = getelementptr inbounds i8, ptr %4, i64 %6
  store i8 91, ptr %7, align 1, !tbaa !12
  %8 = load i64, ptr %5, align 8, !tbaa !5
  %9 = add i64 %8, 1
  store i64 %9, ptr %5, align 8, !tbaa !5
  %10 = icmp eq i64 %1, 0
  br i1 %10, label %11, label %17

11:                                               ; preds = %21, %2
  tail call fastcc void @reserve(i64 noundef 1) #35
  %12 = load ptr, ptr %3, align 8, !tbaa !9
  %13 = load i64, ptr %5, align 8, !tbaa !5
  %14 = getelementptr inbounds i8, ptr %12, i64 %13
  store i8 93, ptr %14, align 1, !tbaa !12
  %15 = load i64, ptr %5, align 8, !tbaa !5
  %16 = add i64 %15, 1
  store i64 %16, ptr %5, align 8, !tbaa !5
  ret void

17:                                               ; preds = %2, %21
  %18 = phi i64 [ %26, %21 ], [ 0, %2 ]
  %19 = icmp eq i64 %18, 0
  br i1 %19, label %21, label %20

20:                                               ; preds = %17
  tail call fastcc void @append_c(ptr noundef nonnull @.str.11) #35
  br label %21

21:                                               ; preds = %20, %17
  %22 = getelementptr inbounds %struct.FaString, ptr %0, i64 %18
  %23 = load ptr, ptr %22, align 8, !tbaa !36
  %24 = getelementptr inbounds %struct.FaString, ptr %0, i64 %18, i32 1
  %25 = load i64, ptr %24, align 8, !tbaa !38
  tail call void @fa_b_quoted(ptr noundef %23, i64 noundef %25) #35
  %26 = add nuw i64 %18, 1
  %27 = icmp eq i64 %26, %1
  br i1 %27, label %11, label %17, !llvm.loop !39
}

; Function Attrs: nounwind optsize
define void @fa_concat(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef readonly %2, i64 noundef %3, ptr nocapture noundef writeonly %4) local_unnamed_addr #0 {
  %6 = add i64 %3, %1
  %7 = icmp eq i64 %6, 0
  br i1 %7, label %8, label %11

8:                                                ; preds = %5
  store ptr @.str.34, ptr %4, align 8, !tbaa.struct !16
  %9 = getelementptr inbounds i8, ptr %4, i64 8
  store i64 0, ptr %9, align 8, !tbaa.struct !17
  %10 = getelementptr inbounds i8, ptr %4, i64 16
  store i64 0, ptr %10, align 8, !tbaa.struct !18
  br label %16

11:                                               ; preds = %5
  %12 = tail call ptr @fa_alloc(i64 noundef %6, i64 noundef 1) #34
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %12, ptr align 1 %0, i64 %1, i1 false)
  %13 = getelementptr inbounds i8, ptr %12, i64 %1
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %13, ptr align 1 %2, i64 %3, i1 false)
  store ptr %12, ptr %4, align 8, !tbaa !36
  %14 = getelementptr inbounds %struct.FaString, ptr %4, i64 0, i32 1
  store i64 %6, ptr %14, align 8, !tbaa !38
  %15 = getelementptr inbounds %struct.FaString, ptr %4, i64 0, i32 2
  store i64 %6, ptr %15, align 8, !tbaa !40
  br label %16

16:                                               ; preds = %11, %8
  ret void
}

; Function Attrs: optsize
declare ptr @fa_alloc(i64 noundef, i64 noundef) local_unnamed_addr #1

; Function Attrs: nounwind optsize
define void @fa_string_append(ptr nocapture noundef %0, ptr nocapture noundef readonly %1, i64 noundef %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %2, 0
  br i1 %4, label %28, label %5

5:                                                ; preds = %3
  %6 = getelementptr inbounds %struct.FaString, ptr %0, i64 0, i32 1
  %7 = load i64, ptr %6, align 8, !tbaa !38
  %8 = add i64 %7, %2
  %9 = getelementptr inbounds %struct.FaString, ptr %0, i64 0, i32 2
  %10 = load i64, ptr %9, align 8, !tbaa !40
  %11 = add i64 %10, -1
  %12 = icmp ult i64 %11, -2
  %13 = icmp ule i64 %8, %10
  %14 = select i1 %12, i1 %13, i1 false
  br i1 %14, label %15, label %18

15:                                               ; preds = %5
  %16 = load ptr, ptr %0, align 8, !tbaa !36
  %17 = getelementptr inbounds i8, ptr %16, i64 %7
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %17, ptr align 1 %1, i64 %2, i1 false)
  store i64 %8, ptr %6, align 8, !tbaa !38
  br label %28

18:                                               ; preds = %5
  %19 = shl i64 %10, 1
  %20 = tail call i64 @llvm.umax.i64(i64 %19, i64 %8)
  %21 = select i1 %12, i64 %20, i64 %8
  %22 = tail call ptr @fa_alloc(i64 noundef %21, i64 noundef 1) #34
  %23 = load ptr, ptr %0, align 8, !tbaa !36
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %22, ptr align 1 %23, i64 %7, i1 false)
  %24 = getelementptr inbounds i8, ptr %22, i64 %7
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %24, ptr align 1 %1, i64 %2, i1 false)
  br i1 %12, label %25, label %27

25:                                               ; preds = %18
  %26 = load ptr, ptr %0, align 8, !tbaa !36
  tail call void @fa_free(ptr noundef %26, i64 noundef %10, i64 noundef 1) #34
  br label %27

27:                                               ; preds = %25, %18
  store ptr %22, ptr %0, align 8, !tbaa !36
  store i64 %8, ptr %6, align 8, !tbaa !38
  store i64 %21, ptr %9, align 8, !tbaa !40
  br label %28

28:                                               ; preds = %15, %27, %3
  ret void
}

; Function Attrs: optsize
declare void @fa_free(ptr noundef, i64 noundef, i64 noundef) local_unnamed_addr #1

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: read)
define i32 @fa_string_equal(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef readonly %2, i64 noundef %3) local_unnamed_addr #8 {
  %5 = icmp eq i64 %1, %3
  br i1 %5, label %6, label %25

6:                                                ; preds = %4
  %7 = icmp eq i64 %1, 0
  br i1 %7, label %25, label %8

8:                                                ; preds = %6
  %9 = load i8, ptr %0, align 1, !tbaa !12
  %10 = load i8, ptr %2, align 1, !tbaa !12
  %11 = icmp eq i8 %9, %10
  br i1 %11, label %12, label %25

12:                                               ; preds = %8, %16
  %13 = phi i64 [ %14, %16 ], [ 0, %8 ]
  %14 = add nuw i64 %13, 1
  %15 = icmp eq i64 %14, %1
  br i1 %15, label %22, label %16, !llvm.loop !41

16:                                               ; preds = %12
  %17 = getelementptr inbounds i8, ptr %0, i64 %14
  %18 = load i8, ptr %17, align 1, !tbaa !12
  %19 = getelementptr inbounds i8, ptr %2, i64 %14
  %20 = load i8, ptr %19, align 1, !tbaa !12
  %21 = icmp eq i8 %18, %20
  br i1 %21, label %12, label %22, !llvm.loop !41

22:                                               ; preds = %16, %12
  %23 = icmp uge i64 %14, %1
  %24 = zext i1 %23 to i32
  br label %25

25:                                               ; preds = %8, %22, %6, %4
  %26 = phi i32 [ 0, %4 ], [ 1, %6 ], [ 0, %8 ], [ %24, %22 ]
  ret i32 %26
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: read)
define i32 @fa_string_compare(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef readonly %2, i64 noundef %3) local_unnamed_addr #8 {
  %5 = tail call i64 @llvm.umin.i64(i64 %1, i64 %3)
  %6 = icmp eq i64 %5, 0
  br i1 %6, label %11, label %7

7:                                                ; preds = %4
  %8 = load i8, ptr %0, align 1, !tbaa !12
  %9 = load i8, ptr %2, align 1, !tbaa !12
  %10 = icmp eq i8 %8, %9
  br i1 %10, label %16, label %28

11:                                               ; preds = %4
  %12 = icmp ult i64 %1, %3
  %13 = icmp eq i64 %1, %3
  %14 = select i1 %12, i32 -1, i32 1
  %15 = select i1 %13, i32 0, i32 %14
  br label %44

16:                                               ; preds = %7, %20
  %17 = phi i64 [ %18, %20 ], [ 0, %7 ]
  %18 = add i64 %17, 1
  %19 = icmp eq i64 %18, %5
  br i1 %19, label %39, label %20, !llvm.loop !42

20:                                               ; preds = %16
  %21 = getelementptr inbounds i8, ptr %0, i64 %18
  %22 = load i8, ptr %21, align 1, !tbaa !12
  %23 = getelementptr inbounds i8, ptr %2, i64 %18
  %24 = load i8, ptr %23, align 1, !tbaa !12
  %25 = icmp eq i8 %22, %24
  br i1 %25, label %16, label %26, !llvm.loop !42

26:                                               ; preds = %20
  %27 = icmp ult i64 %18, %5
  br label %28

28:                                               ; preds = %26, %7
  %29 = phi i1 [ true, %7 ], [ %27, %26 ]
  %30 = phi i8 [ %8, %7 ], [ %22, %26 ]
  %31 = phi i8 [ %9, %7 ], [ %24, %26 ]
  %32 = icmp ult i8 %30, %31
  %33 = select i1 %32, i32 -1, i32 1
  %34 = icmp ult i64 %1, %3
  %35 = icmp eq i64 %1, %3
  %36 = select i1 %34, i32 -1, i32 1
  %37 = select i1 %35, i32 0, i32 %36
  %38 = select i1 %29, i32 %33, i32 %37
  br label %44

39:                                               ; preds = %16
  %40 = icmp ult i64 %1, %3
  %41 = icmp eq i64 %1, %3
  %42 = select i1 %40, i32 -1, i32 1
  %43 = select i1 %41, i32 0, i32 %42
  br label %44

44:                                               ; preds = %28, %39, %11
  %45 = phi i32 [ %43, %39 ], [ %15, %11 ], [ %38, %28 ]
  ret i32 %45
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define i64 @fa_next_grapheme(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #9 {
  %4 = tail call fastcc i64 @next_boundary(ptr noundef %0, i64 noundef %1, i64 noundef %2) #35
  ret i64 %4
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define internal fastcc i64 @next_boundary(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2) unnamed_addr #9 {
  %4 = alloca i64, align 8
  %5 = alloca i64, align 8
  store i64 %2, ptr %4, align 8, !tbaa !5
  %6 = getelementptr inbounds i8, ptr %0, i64 %2
  %7 = load i8, ptr %6, align 1, !tbaa !12
  %8 = icmp sgt i8 %7, -1
  br i1 %8, label %9, label %24

9:                                                ; preds = %3
  %10 = add i64 %2, 1
  %11 = icmp ult i64 %10, %1
  br i1 %11, label %12, label %21

12:                                               ; preds = %9
  %13 = getelementptr inbounds i8, ptr %0, i64 %10
  %14 = load i8, ptr %13, align 1, !tbaa !12
  %15 = icmp sgt i8 %14, -1
  br i1 %15, label %16, label %24

16:                                               ; preds = %12
  %17 = icmp eq i8 %7, 13
  br i1 %17, label %18, label %21

18:                                               ; preds = %16
  %19 = icmp eq i8 %14, 10
  %20 = zext i1 %19 to i64
  br label %21

21:                                               ; preds = %9, %18, %16
  %22 = phi i64 [ 0, %16 ], [ %20, %18 ], [ 0, %9 ]
  %23 = add i64 %22, %10
  br label %106

24:                                               ; preds = %12, %3
  %25 = call fastcc i32 @decode(ptr noundef nonnull %0, i64 noundef %1, ptr noundef nonnull %4) #35
  %26 = tail call fastcc i32 @break_kind(i32 noundef %25) #35, !range !43
  %27 = load i64, ptr %4, align 8, !tbaa !5
  %28 = icmp ult i64 %27, %1
  br i1 %28, label %29, label %106

29:                                               ; preds = %24
  %30 = icmp eq i32 %26, 7
  %31 = zext i1 %30 to i32
  %32 = icmp eq i32 %26, 6
  %33 = zext i1 %32 to i32
  br label %34

34:                                               ; preds = %29, %101
  %35 = phi i32 [ %104, %101 ], [ %31, %29 ]
  %36 = phi i32 [ %103, %101 ], [ %33, %29 ]
  %37 = phi i32 [ %40, %101 ], [ %26, %29 ]
  %38 = phi i64 [ %102, %101 ], [ %27, %29 ]
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %5) #31
  store i64 %38, ptr %5, align 8, !tbaa !5
  %39 = call fastcc i32 @decode(ptr noundef %0, i64 noundef %1, ptr noundef nonnull %5) #35
  %40 = tail call fastcc i32 @break_kind(i32 noundef %39) #35, !range !43
  %41 = icmp eq i32 %37, 1
  %42 = icmp eq i32 %40, 2
  %43 = and i1 %41, %42
  br i1 %43, label %86, label %44

44:                                               ; preds = %34
  %45 = add nsw i32 %37, -1
  %46 = icmp ult i32 %45, 3
  %47 = add nsw i32 %40, -1
  %48 = icmp ult i32 %47, 3
  %49 = or i1 %46, %48
  br i1 %49, label %100, label %50

50:                                               ; preds = %44
  %51 = icmp eq i32 %37, 8
  br i1 %51, label %52, label %53

52:                                               ; preds = %50
  switch i32 %40, label %59 [
    i32 12, label %86
    i32 11, label %86
    i32 9, label %86
    i32 8, label %86
  ]

53:                                               ; preds = %50
  %54 = and i32 %37, 13
  %55 = icmp eq i32 %54, 9
  %56 = add nsw i32 %40, -9
  %57 = icmp ult i32 %56, 2
  %58 = and i1 %55, %57
  br i1 %58, label %86, label %59

59:                                               ; preds = %53, %52
  %60 = icmp eq i32 %37, 12
  %61 = icmp eq i32 %37, 10
  %62 = or i1 %60, %61
  %63 = icmp eq i32 %40, 10
  %64 = and i1 %62, %63
  %65 = and i32 %40, 14
  %66 = icmp eq i32 %65, 4
  %67 = or i1 %64, %66
  br i1 %67, label %88, label %68

68:                                               ; preds = %59
  %69 = icmp eq i32 %37, 5
  %70 = icmp eq i32 %40, 7
  %71 = and i1 %69, %70
  %72 = icmp ne i32 %35, 0
  %73 = and i1 %72, %71
  br i1 %73, label %74, label %76

74:                                               ; preds = %68
  %75 = load i64, ptr %5, align 8, !tbaa !5
  br label %101

76:                                               ; preds = %68
  %77 = icmp eq i32 %37, 6
  %78 = icmp eq i32 %40, 6
  %79 = and i1 %77, %78
  %80 = and i32 %36, -2147483647
  %81 = icmp eq i32 %80, 1
  %82 = and i1 %81, %79
  br i1 %82, label %83, label %100

83:                                               ; preds = %76
  %84 = load i64, ptr %5, align 8, !tbaa !5
  %85 = add nuw nsw i32 %36, 1
  br label %94

86:                                               ; preds = %53, %52, %52, %52, %52, %34
  %87 = load i64, ptr %5, align 8, !tbaa !5
  br label %94

88:                                               ; preds = %59
  %89 = load i64, ptr %5, align 8, !tbaa !5
  %90 = icmp eq i32 %40, 6
  %91 = add nsw i32 %36, 1
  %92 = select i1 %90, i32 %91, i32 0
  %93 = icmp eq i32 %40, 7
  br i1 %93, label %101, label %94

94:                                               ; preds = %83, %86, %88
  %95 = phi i64 [ %87, %86 ], [ %89, %88 ], [ %84, %83 ]
  %96 = phi i32 [ 0, %86 ], [ %92, %88 ], [ %85, %83 ]
  %97 = add nsw i32 %40, -6
  %98 = icmp ult i32 %97, -2
  %99 = select i1 %98, i32 0, i32 %35
  br label %101

100:                                              ; preds = %76, %44
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %5) #31
  br label %106

101:                                              ; preds = %74, %94, %88
  %102 = phi i64 [ %89, %88 ], [ %95, %94 ], [ %75, %74 ]
  %103 = phi i32 [ %92, %88 ], [ %96, %94 ], [ 0, %74 ]
  %104 = phi i32 [ 1, %88 ], [ %99, %94 ], [ 1, %74 ]
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %5) #31
  %105 = icmp ult i64 %102, %1
  br i1 %105, label %34, label %106

106:                                              ; preds = %101, %24, %100, %21
  %107 = phi i64 [ %23, %21 ], [ %38, %100 ], [ %27, %24 ], [ %102, %101 ]
  ret i64 %107
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define i64 @fa_graphemes(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #9 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %29, label %4

4:                                                ; preds = %2, %25
  %5 = phi i64 [ %27, %25 ], [ 0, %2 ]
  %6 = phi i64 [ %26, %25 ], [ 0, %2 ]
  %7 = add i64 %6, 8
  %8 = icmp ult i64 %7, %1
  br i1 %8, label %9, label %22

9:                                                ; preds = %4
  %10 = getelementptr inbounds i8, ptr %0, i64 %7
  %11 = load i8, ptr %10, align 1, !tbaa !12
  %12 = icmp sgt i8 %11, -1
  br i1 %12, label %13, label %22

13:                                               ; preds = %9
  %14 = getelementptr inbounds i8, ptr %0, i64 %6
  %15 = load i64, ptr %14, align 1
  %16 = xor i64 %15, 940422246894996749
  %17 = add i64 %16, -72340172838076673
  %18 = or i64 %17, %15
  %19 = and i64 %18, -9187201950435737472
  %20 = icmp eq i64 %19, 0
  %21 = add i64 %5, 8
  br i1 %20, label %25, label %22

22:                                               ; preds = %13, %9, %4
  %23 = tail call fastcc i64 @next_boundary(ptr noundef %0, i64 noundef %1, i64 noundef %6) #35
  %24 = add i64 %5, 1
  br label %25

25:                                               ; preds = %22, %13
  %26 = phi i64 [ %7, %13 ], [ %23, %22 ]
  %27 = phi i64 [ %21, %13 ], [ %24, %22 ]
  %28 = icmp ult i64 %26, %1
  br i1 %28, label %4, label %29, !llvm.loop !44

29:                                               ; preds = %25, %2
  %30 = phi i64 [ 0, %2 ], [ %27, %25 ]
  ret i64 %30
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define i32 @fa_string_includes(ptr noundef %0, i64 noundef %1, ptr nocapture noundef readonly %2, i64 noundef %3) local_unnamed_addr #10 {
  %5 = tail call fastcc i64 @find(ptr noundef %0, i64 noundef %1, ptr noundef %2, i64 noundef %3, i32 noundef 0) #35
  %6 = icmp sgt i64 %5, -1
  %7 = zext i1 %6 to i32
  ret i32 %7
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc i64 @find(ptr noundef %0, i64 noundef %1, ptr nocapture noundef readonly %2, i64 noundef %3, i32 noundef %4) unnamed_addr #10 {
  %6 = alloca %struct.Boundaries, align 8
  %7 = alloca %struct.Boundaries, align 8
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %6) #31
  store ptr %0, ptr %6, align 8, !tbaa !36
  %8 = getelementptr inbounds %struct.Boundaries, ptr %6, i64 0, i32 1
  store i64 %1, ptr %8, align 8, !tbaa !38
  %9 = getelementptr inbounds %struct.Boundaries, ptr %6, i64 0, i32 2
  store i64 0, ptr %9, align 8, !tbaa !40
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %7) #31
  store ptr %0, ptr %7, align 8, !tbaa !36
  %10 = getelementptr inbounds %struct.Boundaries, ptr %7, i64 0, i32 1
  store i64 %1, ptr %10, align 8, !tbaa !38
  %11 = getelementptr inbounds %struct.Boundaries, ptr %7, i64 0, i32 2
  store i64 0, ptr %11, align 8, !tbaa !40
  %12 = icmp ugt i64 %3, %1
  br i1 %12, label %50, label %13

13:                                               ; preds = %5
  %14 = icmp eq i64 %3, 0
  %15 = icmp ne i32 %4, 0
  br label %16

16:                                               ; preds = %13, %45
  %17 = phi i64 [ %3, %13 ], [ %48, %45 ]
  %18 = phi i64 [ 0, %13 ], [ %47, %45 ]
  %19 = phi i64 [ -1, %13 ], [ %46, %45 ]
  %20 = getelementptr i8, ptr %0, i64 %18
  br i1 %14, label %37, label %21

21:                                               ; preds = %16
  %22 = load i8, ptr %20, align 1, !tbaa !12
  %23 = load i8, ptr %2, align 1, !tbaa !12
  %24 = icmp eq i8 %22, %23
  br i1 %24, label %25, label %45

25:                                               ; preds = %21, %29
  %26 = phi i64 [ %27, %29 ], [ 0, %21 ]
  %27 = add nuw i64 %26, 1
  %28 = icmp eq i64 %27, %3
  br i1 %28, label %35, label %29, !llvm.loop !45

29:                                               ; preds = %25
  %30 = getelementptr i8, ptr %20, i64 %27
  %31 = load i8, ptr %30, align 1, !tbaa !12
  %32 = getelementptr inbounds i8, ptr %2, i64 %27
  %33 = load i8, ptr %32, align 1, !tbaa !12
  %34 = icmp eq i8 %31, %33
  br i1 %34, label %25, label %35, !llvm.loop !45

35:                                               ; preds = %29, %25
  %36 = icmp ult i64 %27, %3
  br i1 %36, label %45, label %37

37:                                               ; preds = %16, %35
  %38 = call fastcc i32 @at_boundary(ptr noundef nonnull %6, i64 noundef %18) #35, !range !46
  %39 = icmp eq i32 %38, 0
  br i1 %39, label %45, label %40

40:                                               ; preds = %37
  %41 = call fastcc i32 @at_boundary(ptr noundef nonnull %7, i64 noundef %17) #35, !range !46
  %42 = icmp eq i32 %41, 0
  %43 = or i1 %42, %15
  %44 = select i1 %42, i64 %19, i64 %18
  br i1 %43, label %45, label %50

45:                                               ; preds = %21, %40, %35, %37
  %46 = phi i64 [ %44, %40 ], [ %19, %37 ], [ %19, %35 ], [ %19, %21 ]
  %47 = add i64 %18, 1
  %48 = add i64 %47, %3
  %49 = icmp ugt i64 %48, %1
  br i1 %49, label %50, label %16, !llvm.loop !47

50:                                               ; preds = %45, %40, %5
  %51 = phi i64 [ -1, %5 ], [ %18, %40 ], [ %46, %45 ]
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %7) #31
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %6) #31
  ret i64 %51
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define i32 @fa_string_starts_with(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef readonly %2, i64 noundef %3) local_unnamed_addr #9 {
  %5 = icmp ugt i64 %3, %1
  br i1 %5, label %32, label %6

6:                                                ; preds = %4
  %7 = icmp eq i64 %3, 0
  br i1 %7, label %28, label %8

8:                                                ; preds = %6
  %9 = load i8, ptr %0, align 1, !tbaa !12
  %10 = load i8, ptr %2, align 1, !tbaa !12
  %11 = icmp eq i8 %9, %10
  br i1 %11, label %12, label %32

12:                                               ; preds = %8, %16
  %13 = phi i64 [ %14, %16 ], [ 0, %8 ]
  %14 = add nuw i64 %13, 1
  %15 = icmp eq i64 %14, %3
  br i1 %15, label %22, label %16, !llvm.loop !45

16:                                               ; preds = %12
  %17 = getelementptr i8, ptr %0, i64 %14
  %18 = load i8, ptr %17, align 1, !tbaa !12
  %19 = getelementptr inbounds i8, ptr %2, i64 %14
  %20 = load i8, ptr %19, align 1, !tbaa !12
  %21 = icmp eq i8 %18, %20
  br i1 %21, label %12, label %22, !llvm.loop !45

22:                                               ; preds = %16, %12
  %23 = icmp ult i64 %14, %3
  br i1 %23, label %32, label %24

24:                                               ; preds = %22, %24
  %25 = phi i64 [ %26, %24 ], [ 0, %22 ]
  %26 = tail call fastcc i64 @next_boundary(ptr noundef nonnull %0, i64 noundef %1, i64 noundef %25) #35
  %27 = icmp ult i64 %26, %3
  br i1 %27, label %24, label %28, !llvm.loop !48

28:                                               ; preds = %24, %6
  %29 = phi i64 [ 0, %6 ], [ %26, %24 ]
  %30 = icmp eq i64 %29, %3
  %31 = zext i1 %30 to i32
  br label %32

32:                                               ; preds = %8, %4, %28, %22
  %33 = phi i32 [ 0, %22 ], [ %31, %28 ], [ 0, %4 ], [ 0, %8 ]
  ret i32 %33
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define i32 @fa_string_ends_with(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef readonly %2, i64 noundef %3) local_unnamed_addr #9 {
  %5 = icmp ugt i64 %3, %1
  br i1 %5, label %36, label %6

6:                                                ; preds = %4
  %7 = sub i64 %1, %3
  %8 = getelementptr i8, ptr %0, i64 %7
  %9 = icmp eq i64 %3, 0
  br i1 %9, label %26, label %10

10:                                               ; preds = %6
  %11 = load i8, ptr %8, align 1, !tbaa !12
  %12 = load i8, ptr %2, align 1, !tbaa !12
  %13 = icmp eq i8 %11, %12
  br i1 %13, label %14, label %36

14:                                               ; preds = %10, %18
  %15 = phi i64 [ %16, %18 ], [ 0, %10 ]
  %16 = add nuw i64 %15, 1
  %17 = icmp eq i64 %16, %3
  br i1 %17, label %24, label %18, !llvm.loop !45

18:                                               ; preds = %14
  %19 = getelementptr i8, ptr %8, i64 %16
  %20 = load i8, ptr %19, align 1, !tbaa !12
  %21 = getelementptr inbounds i8, ptr %2, i64 %16
  %22 = load i8, ptr %21, align 1, !tbaa !12
  %23 = icmp eq i8 %20, %22
  br i1 %23, label %14, label %24, !llvm.loop !45

24:                                               ; preds = %18, %14
  %25 = icmp ult i64 %16, %3
  br i1 %25, label %36, label %26

26:                                               ; preds = %6, %24
  %27 = icmp eq i64 %1, %3
  br i1 %27, label %32, label %28

28:                                               ; preds = %26, %28
  %29 = phi i64 [ %30, %28 ], [ 0, %26 ]
  %30 = tail call fastcc i64 @next_boundary(ptr noundef %0, i64 noundef %1, i64 noundef %29) #35
  %31 = icmp ult i64 %30, %7
  br i1 %31, label %28, label %32, !llvm.loop !48

32:                                               ; preds = %28, %26
  %33 = phi i64 [ 0, %26 ], [ %30, %28 ]
  %34 = icmp eq i64 %33, %7
  %35 = zext i1 %34 to i32
  br label %36

36:                                               ; preds = %10, %32, %24, %4
  %37 = phi i32 [ 0, %24 ], [ 0, %4 ], [ %35, %32 ], [ 0, %10 ]
  ret i32 %37
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define i64 @fa_string_index_of(ptr noundef %0, i64 noundef %1, ptr nocapture noundef readonly %2, i64 noundef %3, i32 noundef %4) local_unnamed_addr #10 {
  %6 = tail call fastcc i64 @find(ptr noundef %0, i64 noundef %1, ptr noundef %2, i64 noundef %3, i32 noundef %4) #35
  %7 = icmp slt i64 %6, 0
  br i1 %7, label %16, label %8

8:                                                ; preds = %5
  %9 = icmp eq i64 %6, 0
  br i1 %9, label %16, label %10

10:                                               ; preds = %8, %10
  %11 = phi i64 [ %14, %10 ], [ 0, %8 ]
  %12 = phi i64 [ %13, %10 ], [ 0, %8 ]
  %13 = add i64 %12, 1
  %14 = tail call fastcc i64 @next_boundary(ptr noundef %0, i64 noundef %1, i64 noundef %11) #35
  %15 = icmp ult i64 %14, %6
  br i1 %15, label %10, label %16, !llvm.loop !49

16:                                               ; preds = %10, %8, %5
  %17 = phi i64 [ -1, %5 ], [ 0, %8 ], [ %13, %10 ]
  ret i64 %17
}

; Function Attrs: nounwind optsize
define void @fa_string_replace(ptr noundef %0, i64 noundef %1, ptr nocapture noundef readonly %2, i64 noundef %3, ptr noundef %4, i64 noundef %5, i32 noundef %6, ptr nocapture noundef writeonly %7) local_unnamed_addr #0 {
  %9 = alloca %struct.Replacing, align 8
  %10 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %10, align 8, !tbaa !5
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %9) #31
  store ptr %0, ptr %9, align 8, !tbaa !50
  %11 = getelementptr inbounds %struct.Replacing, ptr %9, i64 0, i32 1
  store i64 0, ptr %11, align 8, !tbaa !52
  %12 = getelementptr inbounds %struct.Replacing, ptr %9, i64 0, i32 2
  store i64 %3, ptr %12, align 8, !tbaa !53
  %13 = getelementptr inbounds %struct.Replacing, ptr %9, i64 0, i32 3
  store ptr %4, ptr %13, align 8, !tbaa !54
  %14 = getelementptr inbounds %struct.Replacing, ptr %9, i64 0, i32 4
  store i64 %5, ptr %14, align 8, !tbaa !55
  %15 = getelementptr inbounds %struct.Replacing, ptr %9, i64 0, i32 5
  store i32 %6, ptr %15, align 8, !tbaa !56
  %16 = getelementptr inbounds %struct.Replacing, ptr %9, i64 0, i32 6
  store i32 0, ptr %16, align 4, !tbaa !57
  call fastcc void @each_match(ptr noundef %0, i64 noundef %1, ptr noundef %2, i64 noundef %3, ptr noundef nonnull @replace_one, ptr noundef nonnull %9) #35
  %17 = load i64, ptr %11, align 8, !tbaa !52
  %18 = getelementptr inbounds i8, ptr %0, i64 %17
  %19 = sub i64 %1, %17
  call fastcc void @reserve(i64 noundef %19) #35
  %20 = call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %21 = load ptr, ptr %20, align 8, !tbaa !9
  %22 = load i64, ptr %10, align 8, !tbaa !5
  %23 = getelementptr inbounds i8, ptr %21, i64 %22
  call void @llvm.memcpy.p0.p0.i64(ptr align 1 %23, ptr align 1 %18, i64 %19, i1 false)
  %24 = load i64, ptr %10, align 8, !tbaa !5
  %25 = add i64 %24, %19
  store i64 %25, ptr %10, align 8, !tbaa !5
  call void @fa_b_take(ptr noundef %7) #35
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %9) #31
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @each_match(ptr noundef %0, i64 noundef %1, ptr nocapture noundef readonly %2, i64 noundef %3, ptr nocapture noundef readonly %4, ptr noundef %5) unnamed_addr #0 {
  %7 = alloca %struct.Boundaries, align 8
  %8 = alloca %struct.Boundaries, align 8
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %7) #31
  store ptr %0, ptr %7, align 8, !tbaa !36
  %9 = getelementptr inbounds %struct.Boundaries, ptr %7, i64 0, i32 1
  store i64 %1, ptr %9, align 8, !tbaa !38
  %10 = getelementptr inbounds %struct.Boundaries, ptr %7, i64 0, i32 2
  store i64 0, ptr %10, align 8, !tbaa !40
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %8) #31
  store ptr %0, ptr %8, align 8, !tbaa !36
  %11 = getelementptr inbounds %struct.Boundaries, ptr %8, i64 0, i32 1
  store i64 %1, ptr %11, align 8, !tbaa !38
  %12 = getelementptr inbounds %struct.Boundaries, ptr %8, i64 0, i32 2
  store i64 0, ptr %12, align 8, !tbaa !40
  %13 = icmp ugt i64 %3, %1
  br i1 %13, label %49, label %14

14:                                               ; preds = %6
  %15 = icmp eq i64 %3, 0
  %16 = tail call i64 @llvm.umax.i64(i64 %3, i64 1)
  br label %17

17:                                               ; preds = %14, %44
  %18 = phi i64 [ %3, %14 ], [ %47, %44 ]
  %19 = phi i64 [ 0, %14 ], [ %46, %44 ]
  %20 = getelementptr i8, ptr %0, i64 %19
  br i1 %15, label %37, label %21

21:                                               ; preds = %17
  %22 = load i8, ptr %20, align 1, !tbaa !12
  %23 = load i8, ptr %2, align 1, !tbaa !12
  %24 = icmp eq i8 %22, %23
  br i1 %24, label %25, label %44

25:                                               ; preds = %21, %29
  %26 = phi i64 [ %27, %29 ], [ 0, %21 ]
  %27 = add nuw i64 %26, 1
  %28 = icmp eq i64 %27, %3
  br i1 %28, label %35, label %29, !llvm.loop !45

29:                                               ; preds = %25
  %30 = getelementptr i8, ptr %20, i64 %27
  %31 = load i8, ptr %30, align 1, !tbaa !12
  %32 = getelementptr inbounds i8, ptr %2, i64 %27
  %33 = load i8, ptr %32, align 1, !tbaa !12
  %34 = icmp eq i8 %31, %33
  br i1 %34, label %25, label %35, !llvm.loop !45

35:                                               ; preds = %29, %25
  %36 = icmp ult i64 %27, %3
  br i1 %36, label %44, label %37

37:                                               ; preds = %17, %35
  %38 = call fastcc i32 @at_boundary(ptr noundef nonnull %7, i64 noundef %19) #35, !range !46
  %39 = icmp eq i32 %38, 0
  br i1 %39, label %44, label %40

40:                                               ; preds = %37
  %41 = call fastcc i32 @at_boundary(ptr noundef nonnull %8, i64 noundef %18) #35, !range !46
  %42 = icmp eq i32 %41, 0
  br i1 %42, label %44, label %43

43:                                               ; preds = %40
  tail call void %4(ptr noundef %5, i64 noundef %19) #34, !callees !58
  br label %44

44:                                               ; preds = %21, %35, %37, %40, %43
  %45 = phi i64 [ %16, %43 ], [ 1, %40 ], [ 1, %37 ], [ 1, %35 ], [ 1, %21 ]
  %46 = add i64 %45, %19
  %47 = add i64 %46, %3
  %48 = icmp ugt i64 %47, %1
  br i1 %48, label %49, label %17, !llvm.loop !59

49:                                               ; preds = %44, %6
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %8) #31
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %7) #31
  ret void
}

; Function Attrs: nounwind optsize
define internal void @replace_one(ptr nocapture noundef %0, i64 noundef %1) #0 {
  %3 = getelementptr inbounds %struct.Replacing, ptr %0, i64 0, i32 6
  %4 = load i32, ptr %3, align 4, !tbaa !57
  %5 = icmp eq i32 %4, 0
  br i1 %5, label %6, label %35

6:                                                ; preds = %2
  %7 = load ptr, ptr %0, align 8, !tbaa !50
  %8 = getelementptr inbounds %struct.Replacing, ptr %0, i64 0, i32 1
  %9 = load i64, ptr %8, align 8, !tbaa !52
  %10 = getelementptr inbounds i8, ptr %7, i64 %9
  %11 = sub i64 %1, %9
  tail call fastcc void @reserve(i64 noundef %11) #35
  %12 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %13 = load ptr, ptr %12, align 8, !tbaa !9
  %14 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %15 = load i64, ptr %14, align 8, !tbaa !5
  %16 = getelementptr inbounds i8, ptr %13, i64 %15
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %16, ptr align 1 %10, i64 %11, i1 false)
  %17 = load i64, ptr %14, align 8, !tbaa !5
  %18 = add i64 %17, %11
  store i64 %18, ptr %14, align 8, !tbaa !5
  %19 = getelementptr inbounds %struct.Replacing, ptr %0, i64 0, i32 3
  %20 = load ptr, ptr %19, align 8, !tbaa !54
  %21 = getelementptr inbounds %struct.Replacing, ptr %0, i64 0, i32 4
  %22 = load i64, ptr %21, align 8, !tbaa !55
  tail call fastcc void @reserve(i64 noundef %22) #35
  %23 = load ptr, ptr %12, align 8, !tbaa !9
  %24 = load i64, ptr %14, align 8, !tbaa !5
  %25 = getelementptr inbounds i8, ptr %23, i64 %24
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %25, ptr align 1 %20, i64 %22, i1 false)
  %26 = load i64, ptr %14, align 8, !tbaa !5
  %27 = add i64 %26, %22
  store i64 %27, ptr %14, align 8, !tbaa !5
  %28 = getelementptr inbounds %struct.Replacing, ptr %0, i64 0, i32 2
  %29 = load i64, ptr %28, align 8, !tbaa !53
  %30 = add i64 %29, %1
  store i64 %30, ptr %8, align 8, !tbaa !52
  %31 = getelementptr inbounds %struct.Replacing, ptr %0, i64 0, i32 5
  %32 = load i32, ptr %31, align 8, !tbaa !56
  %33 = icmp eq i32 %32, 0
  %34 = zext i1 %33 to i32
  store i32 %34, ptr %3, align 4, !tbaa !57
  br label %35

35:                                               ; preds = %2, %6
  ret void
}

; Function Attrs: nounwind optsize
define ptr @fa_string_characters(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = tail call ptr @fa_alloc(i64 noundef 24, i64 noundef 8) #34
  %4 = tail call ptr @fa_alloc(i64 noundef 96, i64 noundef 16) #34
  store ptr %4, ptr %3, align 8, !tbaa !36
  %5 = getelementptr inbounds %struct.FaArrayHeader, ptr %3, i64 0, i32 1
  store i64 0, ptr %5, align 8, !tbaa !38
  %6 = getelementptr inbounds %struct.FaArrayHeader, ptr %3, i64 0, i32 2
  store i64 4, ptr %6, align 8, !tbaa !40
  %7 = icmp eq i64 %1, 0
  br i1 %7, label %8, label %9

8:                                                ; preds = %18, %2
  ret ptr %3

9:                                                ; preds = %2, %18
  %10 = phi i64 [ %11, %18 ], [ 0, %2 ]
  %11 = tail call fastcc i64 @next_boundary(ptr noundef %0, i64 noundef %1, i64 noundef %10) #35
  %12 = tail call ptr @fa_array_push_slot(ptr noundef nonnull %3, i64 noundef 24) #35
  %13 = sub i64 %11, %10
  %14 = icmp eq i64 %11, %10
  br i1 %14, label %18, label %15

15:                                               ; preds = %9
  %16 = getelementptr inbounds i8, ptr %0, i64 %10
  %17 = tail call ptr @fa_alloc(i64 noundef %13, i64 noundef 1) #34, !noalias !60
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %17, ptr align 1 %16, i64 %13, i1 false), !noalias !60
  br label %18

18:                                               ; preds = %9, %15
  %19 = phi ptr [ %17, %15 ], [ @.str.34, %9 ]
  store ptr %19, ptr %12, align 8, !tbaa.struct !16
  %20 = getelementptr inbounds i8, ptr %12, i64 8
  store i64 %13, ptr %20, align 8, !tbaa.struct !17
  %21 = getelementptr inbounds i8, ptr %12, i64 16
  store i64 %13, ptr %21, align 8, !tbaa.struct !18
  %22 = icmp ult i64 %11, %1
  br i1 %22, label %9, label %8, !llvm.loop !65
}

; Function Attrs: nounwind optsize
define ptr @fa_array_new(i64 noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #0 {
  %4 = tail call ptr @fa_alloc(i64 noundef 24, i64 noundef 8) #34
  %5 = tail call i64 @llvm.smax.i64(i64 %1, i64 %0)
  %6 = mul nsw i64 %5, %2
  %7 = tail call i64 @llvm.umax.i64(i64 %6, i64 1)
  %8 = tail call ptr @fa_alloc(i64 noundef %7, i64 noundef 16) #34
  store ptr %8, ptr %4, align 8, !tbaa !36
  %9 = getelementptr inbounds %struct.FaArrayHeader, ptr %4, i64 0, i32 1
  store i64 %0, ptr %9, align 8, !tbaa !38
  %10 = getelementptr inbounds %struct.FaArrayHeader, ptr %4, i64 0, i32 2
  store i64 %5, ptr %10, align 8, !tbaa !40
  ret ptr %4
}

; Function Attrs: nounwind optsize
define ptr @fa_array_push_slot(ptr nocapture noundef %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %4 = load i64, ptr %3, align 8, !tbaa !38
  %5 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 2
  %6 = load i64, ptr %5, align 8, !tbaa !40
  %7 = icmp eq i64 %4, %6
  br i1 %7, label %10, label %8

8:                                                ; preds = %2
  %9 = load ptr, ptr %0, align 8, !tbaa !36
  br label %23

10:                                               ; preds = %2
  %11 = icmp slt i64 %4, 4
  %12 = shl nuw nsw i64 %4, 1
  %13 = select i1 %11, i64 4, i64 %12
  %14 = mul nsw i64 %13, %1
  %15 = tail call ptr @fa_alloc(i64 noundef %14, i64 noundef 16) #34
  %16 = load ptr, ptr %0, align 8, !tbaa !36
  %17 = load i64, ptr %3, align 8, !tbaa !38
  %18 = mul nsw i64 %17, %1
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %15, ptr align 1 %16, i64 %18, i1 false)
  %19 = load ptr, ptr %0, align 8, !tbaa !36
  %20 = load i64, ptr %5, align 8, !tbaa !40
  %21 = mul nsw i64 %20, %1
  tail call void @fa_free(ptr noundef %19, i64 noundef %21, i64 noundef 16) #34
  store ptr %15, ptr %0, align 8, !tbaa !36
  store i64 %13, ptr %5, align 8, !tbaa !40
  %22 = load i64, ptr %3, align 8, !tbaa !38
  br label %23

23:                                               ; preds = %8, %10
  %24 = phi i64 [ %4, %8 ], [ %22, %10 ]
  %25 = phi ptr [ %9, %8 ], [ %15, %10 ]
  %26 = mul nsw i64 %24, %1
  %27 = getelementptr inbounds i8, ptr %25, i64 %26
  %28 = add nsw i64 %24, 1
  store i64 %28, ptr %3, align 8, !tbaa !38
  ret ptr %27
}

; Function Attrs: nounwind optsize
define ptr @fa_string_split(ptr noundef %0, i64 noundef %1, ptr nocapture noundef readonly %2, i64 noundef %3) local_unnamed_addr #0 {
  %5 = alloca %struct.Splitting, align 8
  %6 = icmp eq i64 %3, 0
  br i1 %6, label %7, label %9

7:                                                ; preds = %4
  %8 = tail call ptr @fa_string_characters(ptr noundef %0, i64 noundef %1) #35
  br label %30

9:                                                ; preds = %4
  call void @llvm.lifetime.start.p0(i64 32, ptr nonnull %5) #31
  store ptr %0, ptr %5, align 8, !tbaa !66
  %10 = getelementptr inbounds %struct.Splitting, ptr %5, i64 0, i32 1
  store i64 0, ptr %10, align 8, !tbaa !68
  %11 = getelementptr inbounds %struct.Splitting, ptr %5, i64 0, i32 2
  store i64 %3, ptr %11, align 8, !tbaa !69
  %12 = getelementptr inbounds %struct.Splitting, ptr %5, i64 0, i32 3
  %13 = tail call ptr @fa_alloc(i64 noundef 24, i64 noundef 8) #34
  %14 = tail call ptr @fa_alloc(i64 noundef 96, i64 noundef 16) #34
  store ptr %14, ptr %13, align 8, !tbaa !36
  %15 = getelementptr inbounds %struct.FaArrayHeader, ptr %13, i64 0, i32 1
  store i64 0, ptr %15, align 8, !tbaa !38
  %16 = getelementptr inbounds %struct.FaArrayHeader, ptr %13, i64 0, i32 2
  store i64 4, ptr %16, align 8, !tbaa !40
  store ptr %13, ptr %12, align 8, !tbaa !70
  call fastcc void @each_match(ptr noundef %0, i64 noundef %1, ptr noundef %2, i64 noundef %3, ptr noundef nonnull @split_one, ptr noundef nonnull %5) #35
  %17 = load ptr, ptr %12, align 8, !tbaa !70
  %18 = call ptr @fa_array_push_slot(ptr noundef %17, i64 noundef 24) #35
  %19 = load i64, ptr %10, align 8, !tbaa !68
  %20 = sub i64 %1, %19
  %21 = icmp eq i64 %19, %1
  br i1 %21, label %25, label %22

22:                                               ; preds = %9
  %23 = getelementptr inbounds i8, ptr %0, i64 %19
  %24 = call ptr @fa_alloc(i64 noundef %20, i64 noundef 1) #34, !noalias !71
  call void @llvm.memcpy.p0.p0.i64(ptr align 1 %24, ptr align 1 %23, i64 %20, i1 false), !noalias !71
  br label %25

25:                                               ; preds = %9, %22
  %26 = phi ptr [ %24, %22 ], [ @.str.34, %9 ]
  store ptr %26, ptr %18, align 8, !tbaa.struct !16
  %27 = getelementptr inbounds i8, ptr %18, i64 8
  store i64 %20, ptr %27, align 8, !tbaa.struct !17
  %28 = getelementptr inbounds i8, ptr %18, i64 16
  store i64 %20, ptr %28, align 8, !tbaa.struct !18
  %29 = load ptr, ptr %12, align 8, !tbaa !70
  call void @llvm.lifetime.end.p0(i64 32, ptr nonnull %5) #31
  br label %30

30:                                               ; preds = %25, %7
  %31 = phi ptr [ %8, %7 ], [ %29, %25 ]
  ret ptr %31
}

; Function Attrs: nounwind optsize
define internal void @split_one(ptr nocapture noundef %0, i64 noundef %1) #0 {
  %3 = getelementptr inbounds %struct.Splitting, ptr %0, i64 0, i32 3
  %4 = load ptr, ptr %3, align 8, !tbaa !70
  %5 = tail call ptr @fa_array_push_slot(ptr noundef %4, i64 noundef 24) #35
  %6 = getelementptr inbounds %struct.Splitting, ptr %0, i64 0, i32 1
  %7 = load i64, ptr %6, align 8, !tbaa !68
  %8 = sub i64 %1, %7
  %9 = icmp eq i64 %7, %1
  br i1 %9, label %14, label %10

10:                                               ; preds = %2
  %11 = load ptr, ptr %0, align 8, !tbaa !66
  %12 = getelementptr inbounds i8, ptr %11, i64 %7
  %13 = tail call ptr @fa_alloc(i64 noundef %8, i64 noundef 1) #34, !noalias !76
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %13, ptr align 1 %12, i64 %8, i1 false), !noalias !76
  br label %14

14:                                               ; preds = %2, %10
  %15 = phi ptr [ %13, %10 ], [ @.str.34, %2 ]
  store ptr %15, ptr %5, align 8, !tbaa.struct !16
  %16 = getelementptr inbounds i8, ptr %5, i64 8
  store i64 %8, ptr %16, align 8, !tbaa.struct !17
  %17 = getelementptr inbounds i8, ptr %5, i64 16
  store i64 %8, ptr %17, align 8, !tbaa.struct !18
  %18 = getelementptr inbounds %struct.Splitting, ptr %0, i64 0, i32 2
  %19 = load i64, ptr %18, align 8, !tbaa !69
  %20 = add i64 %19, %1
  store i64 %20, ptr %6, align 8, !tbaa !68
  ret void
}

; Function Attrs: nounwind optsize
define ptr @fa_string_code_points(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = alloca i64, align 8
  %4 = tail call ptr @fa_alloc(i64 noundef 24, i64 noundef 8) #34
  %5 = tail call ptr @fa_alloc(i64 noundef 16, i64 noundef 16) #34
  store ptr %5, ptr %4, align 8, !tbaa !36
  %6 = getelementptr inbounds %struct.FaArrayHeader, ptr %4, i64 0, i32 1
  store i64 0, ptr %6, align 8, !tbaa !38
  %7 = getelementptr inbounds %struct.FaArrayHeader, ptr %4, i64 0, i32 2
  store i64 4, ptr %7, align 8, !tbaa !40
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %3) #31
  store i64 0, ptr %3, align 8, !tbaa !5
  %8 = icmp eq i64 %1, 0
  br i1 %8, label %9, label %10

9:                                                ; preds = %10, %2
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %3) #31
  ret ptr %4

10:                                               ; preds = %2, %10
  %11 = tail call ptr @fa_array_push_slot(ptr noundef nonnull %4, i64 noundef 4) #35
  %12 = call fastcc i32 @decode(ptr noundef %0, i64 noundef %1, ptr noundef nonnull %3) #35
  store i32 %12, ptr %11, align 4, !tbaa !20
  %13 = load i64, ptr %3, align 8, !tbaa !5
  %14 = icmp ult i64 %13, %1
  br i1 %14, label %10, label %9, !llvm.loop !81
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define internal fastcc i32 @decode(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef %2) unnamed_addr #9 {
  %4 = load i64, ptr %2, align 8, !tbaa !5
  %5 = getelementptr inbounds i8, ptr %0, i64 %4
  %6 = load i8, ptr %5, align 1, !tbaa !12
  %7 = zext i8 %6 to i32
  %8 = icmp sgt i8 %6, -1
  br i1 %8, label %22, label %9

9:                                                ; preds = %3
  %10 = icmp ult i8 %6, -32
  %11 = icmp ult i8 %6, -16
  %12 = select i1 %11, i32 3, i32 4
  %13 = select i1 %10, i32 2, i32 %12
  %14 = zext nneg i32 %13 to i64
  %15 = lshr i32 127, %13
  %16 = and i32 %15, %7
  br label %17

17:                                               ; preds = %9, %26
  %18 = phi i64 [ %33, %26 ], [ 1, %9 ]
  %19 = phi i32 [ %32, %26 ], [ %16, %9 ]
  %20 = add i64 %18, %4
  %21 = icmp ult i64 %20, %1
  br i1 %21, label %26, label %22

22:                                               ; preds = %17, %26, %3
  %23 = phi i64 [ 1, %3 ], [ %14, %26 ], [ %14, %17 ]
  %24 = phi i32 [ %7, %3 ], [ %19, %17 ], [ %32, %26 ]
  %25 = add i64 %23, %4
  store i64 %25, ptr %2, align 8, !tbaa !5
  ret i32 %24

26:                                               ; preds = %17
  %27 = shl i32 %19, 6
  %28 = getelementptr inbounds i8, ptr %0, i64 %20
  %29 = load i8, ptr %28, align 1, !tbaa !12
  %30 = and i8 %29, 63
  %31 = zext nneg i8 %30 to i32
  %32 = or disjoint i32 %27, %31
  %33 = add nuw nsw i64 %18, 1
  %34 = icmp eq i64 %33, %14
  br i1 %34, label %22, label %17, !llvm.loop !82
}

; Function Attrs: nounwind optsize
define noundef i32 @fa_string_get(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2, ptr nocapture noundef writeonly %3) local_unnamed_addr #0 {
  %5 = icmp slt i64 %2, 0
  br i1 %5, label %6, label %10

6:                                                ; preds = %4
  %7 = tail call i64 @fa_graphemes(ptr noundef %0, i64 noundef %1) #35
  %8 = add nsw i64 %7, %2
  %9 = icmp slt i64 %8, 0
  br i1 %9, label %31, label %10

10:                                               ; preds = %4, %6
  %11 = phi i64 [ %8, %6 ], [ %2, %4 ]
  %12 = icmp eq i64 %1, 0
  br i1 %12, label %31, label %16

13:                                               ; preds = %16
  %14 = add nuw nsw i64 %17, 1
  %15 = icmp ult i64 %19, %1
  br i1 %15, label %16, label %31, !llvm.loop !83

16:                                               ; preds = %10, %13
  %17 = phi i64 [ %14, %13 ], [ 0, %10 ]
  %18 = phi i64 [ %19, %13 ], [ 0, %10 ]
  %19 = tail call fastcc i64 @next_boundary(ptr noundef %0, i64 noundef %1, i64 noundef %18) #35
  %20 = icmp eq i64 %17, %11
  br i1 %20, label %21, label %13

21:                                               ; preds = %16
  %22 = sub i64 %19, %18
  %23 = icmp eq i64 %19, %18
  br i1 %23, label %27, label %24

24:                                               ; preds = %21
  %25 = getelementptr inbounds i8, ptr %0, i64 %18
  %26 = tail call ptr @fa_alloc(i64 noundef %22, i64 noundef 1) #34, !noalias !84
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %26, ptr align 1 %25, i64 %22, i1 false), !noalias !84
  br label %27

27:                                               ; preds = %24, %21
  %28 = phi ptr [ %26, %24 ], [ @.str.34, %21 ]
  store ptr %28, ptr %3, align 8, !tbaa.struct !16
  %29 = getelementptr inbounds i8, ptr %3, i64 8
  store i64 %22, ptr %29, align 8, !tbaa.struct !17
  %30 = getelementptr inbounds i8, ptr %3, i64 16
  store i64 %22, ptr %30, align 8, !tbaa.struct !18
  br label %31

31:                                               ; preds = %13, %27, %10, %6
  %32 = phi i32 [ 0, %6 ], [ 1, %27 ], [ 0, %10 ], [ 0, %13 ]
  ret i32 %32
}

; Function Attrs: nounwind optsize
define void @fa_string_case(ptr nocapture noundef readonly %0, i64 noundef %1, i32 noundef %2, ptr nocapture noundef writeonly %3) local_unnamed_addr #0 {
  %5 = alloca i64, align 8
  %6 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %6, align 8, !tbaa !5
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %5) #31
  store i64 0, ptr %5, align 8, !tbaa !5
  %7 = icmp eq i64 %1, 0
  br i1 %7, label %145, label %8

8:                                                ; preds = %4
  %9 = icmp eq i32 %2, 0
  %10 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  br label %11

11:                                               ; preds = %8, %142
  %12 = call fastcc i32 @decode(ptr noundef %0, i64 noundef %1, ptr noundef nonnull %5) #35
  br i1 %9, label %40, label %13

13:                                               ; preds = %11
  %14 = add i32 %12, -97
  %15 = icmp ult i32 %14, 26
  br i1 %15, label %29, label %16

16:                                               ; preds = %13
  %17 = add i32 %12, -224
  %18 = icmp ult i32 %17, 31
  %19 = icmp ne i32 %12, 247
  %20 = and i1 %19, %18
  br i1 %20, label %29, label %21

21:                                               ; preds = %16
  %22 = add i32 %12, -945
  %23 = icmp ult i32 %22, 25
  %24 = icmp ne i32 %12, 962
  %25 = and i1 %24, %23
  %26 = add i32 %12, -1072
  %27 = icmp ult i32 %26, 32
  %28 = or i1 %27, %25
  br i1 %28, label %29, label %31

29:                                               ; preds = %21, %16, %13
  %30 = add nsw i32 %12, -32
  br label %65

31:                                               ; preds = %21
  %32 = and i32 %12, -16
  %33 = icmp eq i32 %32, 1104
  br i1 %33, label %34, label %36

34:                                               ; preds = %31
  %35 = add nsw i32 %12, -80
  br label %72

36:                                               ; preds = %31
  switch i32 %12, label %65 [
    i32 255, label %72
    i32 181, label %37
    i32 962, label %38
    i32 223, label %39
  ]

37:                                               ; preds = %36
  br label %72

38:                                               ; preds = %36
  br label %72

39:                                               ; preds = %36
  tail call fastcc void @append_c(ptr noundef nonnull @.str.12) #35
  br label %142, !llvm.loop !89

40:                                               ; preds = %11
  %41 = add i32 %12, -65
  %42 = icmp ult i32 %41, 26
  br i1 %42, label %56, label %43

43:                                               ; preds = %40
  %44 = add i32 %12, -192
  %45 = icmp ult i32 %44, 31
  %46 = icmp ne i32 %12, 215
  %47 = and i1 %46, %45
  br i1 %47, label %56, label %48

48:                                               ; preds = %43
  %49 = add i32 %12, -913
  %50 = icmp ult i32 %49, 25
  %51 = icmp ne i32 %12, 930
  %52 = and i1 %51, %50
  %53 = add i32 %12, -1040
  %54 = icmp ult i32 %53, 32
  %55 = or i1 %54, %52
  br i1 %55, label %56, label %58

56:                                               ; preds = %48, %43, %40
  %57 = add nuw nsw i32 %12, 32
  br label %65

58:                                               ; preds = %48
  %59 = and i32 %12, -16
  %60 = icmp eq i32 %59, 1024
  br i1 %60, label %61, label %63

61:                                               ; preds = %58
  %62 = or disjoint i32 %12, 80
  br label %72

63:                                               ; preds = %58
  %64 = icmp eq i32 %12, 376
  br i1 %64, label %72, label %65

65:                                               ; preds = %36, %63, %56, %29
  %66 = phi i32 [ %30, %29 ], [ %57, %56 ], [ %12, %63 ], [ %12, %36 ]
  %67 = icmp ult i32 %66, 128
  br i1 %67, label %68, label %70

68:                                               ; preds = %65
  %69 = trunc i32 %66 to i8
  br label %135

70:                                               ; preds = %65
  %71 = icmp ult i32 %66, 2048
  br i1 %71, label %72, label %85

72:                                               ; preds = %34, %37, %38, %61, %36, %63, %70
  %73 = phi i32 [ %66, %70 ], [ 376, %36 ], [ %62, %61 ], [ 931, %38 ], [ 924, %37 ], [ %35, %34 ], [ 255, %63 ]
  %74 = lshr i32 %73, 6
  %75 = trunc i32 %74 to i8
  %76 = or disjoint i8 %75, -64
  tail call fastcc void @reserve(i64 noundef 1) #35
  %77 = load ptr, ptr %10, align 8, !tbaa !9
  %78 = load i64, ptr %6, align 8, !tbaa !5
  %79 = getelementptr inbounds i8, ptr %77, i64 %78
  store i8 %76, ptr %79, align 1, !tbaa !12
  %80 = load i64, ptr %6, align 8, !tbaa !5
  %81 = add i64 %80, 1
  store i64 %81, ptr %6, align 8, !tbaa !5
  %82 = trunc i32 %73 to i8
  %83 = and i8 %82, 63
  %84 = or disjoint i8 %83, -128
  br label %135

85:                                               ; preds = %70
  %86 = icmp ult i32 %66, 65536
  tail call fastcc void @reserve(i64 noundef 1) #35
  %87 = load ptr, ptr %10, align 8, !tbaa !9
  %88 = load i64, ptr %6, align 8, !tbaa !5
  %89 = getelementptr inbounds i8, ptr %87, i64 %88
  br i1 %86, label %90, label %108

90:                                               ; preds = %85
  %91 = lshr i32 %66, 12
  %92 = trunc i32 %91 to i8
  %93 = or disjoint i8 %92, -32
  store i8 %93, ptr %89, align 1, !tbaa !12
  %94 = load i64, ptr %6, align 8, !tbaa !5
  %95 = add i64 %94, 1
  store i64 %95, ptr %6, align 8, !tbaa !5
  %96 = lshr i32 %66, 6
  %97 = trunc i32 %96 to i8
  %98 = and i8 %97, 63
  %99 = or disjoint i8 %98, -128
  tail call fastcc void @reserve(i64 noundef 1) #35
  %100 = load ptr, ptr %10, align 8, !tbaa !9
  %101 = load i64, ptr %6, align 8, !tbaa !5
  %102 = getelementptr inbounds i8, ptr %100, i64 %101
  store i8 %99, ptr %102, align 1, !tbaa !12
  %103 = load i64, ptr %6, align 8, !tbaa !5
  %104 = add i64 %103, 1
  store i64 %104, ptr %6, align 8, !tbaa !5
  %105 = trunc i32 %66 to i8
  %106 = and i8 %105, 63
  %107 = or disjoint i8 %106, -128
  br label %135

108:                                              ; preds = %85
  %109 = lshr i32 %66, 18
  %110 = trunc i32 %109 to i8
  %111 = or i8 %110, -16
  store i8 %111, ptr %89, align 1, !tbaa !12
  %112 = load i64, ptr %6, align 8, !tbaa !5
  %113 = add i64 %112, 1
  store i64 %113, ptr %6, align 8, !tbaa !5
  %114 = lshr i32 %66, 12
  %115 = trunc i32 %114 to i8
  %116 = and i8 %115, 63
  %117 = or disjoint i8 %116, -128
  tail call fastcc void @reserve(i64 noundef 1) #35
  %118 = load ptr, ptr %10, align 8, !tbaa !9
  %119 = load i64, ptr %6, align 8, !tbaa !5
  %120 = getelementptr inbounds i8, ptr %118, i64 %119
  store i8 %117, ptr %120, align 1, !tbaa !12
  %121 = load i64, ptr %6, align 8, !tbaa !5
  %122 = add i64 %121, 1
  store i64 %122, ptr %6, align 8, !tbaa !5
  %123 = lshr i32 %66, 6
  %124 = trunc i32 %123 to i8
  %125 = and i8 %124, 63
  %126 = or disjoint i8 %125, -128
  tail call fastcc void @reserve(i64 noundef 1) #35
  %127 = load ptr, ptr %10, align 8, !tbaa !9
  %128 = load i64, ptr %6, align 8, !tbaa !5
  %129 = getelementptr inbounds i8, ptr %127, i64 %128
  store i8 %126, ptr %129, align 1, !tbaa !12
  %130 = load i64, ptr %6, align 8, !tbaa !5
  %131 = add i64 %130, 1
  store i64 %131, ptr %6, align 8, !tbaa !5
  %132 = trunc i32 %66 to i8
  %133 = and i8 %132, 63
  %134 = or disjoint i8 %133, -128
  br label %135

135:                                              ; preds = %68, %72, %90, %108
  %136 = phi i8 [ %84, %72 ], [ %134, %108 ], [ %107, %90 ], [ %69, %68 ]
  tail call fastcc void @reserve(i64 noundef 1) #35
  %137 = load ptr, ptr %10, align 8, !tbaa !9
  %138 = load i64, ptr %6, align 8, !tbaa !5
  %139 = getelementptr inbounds i8, ptr %137, i64 %138
  store i8 %136, ptr %139, align 1, !tbaa !12
  %140 = load i64, ptr %6, align 8, !tbaa !5
  %141 = add i64 %140, 1
  store i64 %141, ptr %6, align 8, !tbaa !5
  br label %142

142:                                              ; preds = %135, %39
  %143 = load i64, ptr %5, align 8, !tbaa !5
  %144 = icmp ult i64 %143, %1
  br i1 %144, label %11, label %145

145:                                              ; preds = %142, %4
  tail call void @fa_b_take(ptr noundef %3) #35
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %5) #31
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_string_trim(ptr nocapture noundef readonly %0, i64 noundef %1, i32 noundef %2, ptr nocapture noundef writeonly %3) local_unnamed_addr #0 {
  %5 = alloca i64, align 8
  %6 = alloca i64, align 8
  %7 = and i32 %2, 1
  %8 = icmp eq i32 %7, 0
  br i1 %8, label %17, label %9

9:                                                ; preds = %4, %12
  %10 = phi i64 [ %16, %12 ], [ 0, %4 ]
  %11 = icmp ult i64 %10, %1
  br i1 %11, label %12, label %17

12:                                               ; preds = %9
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %5) #31
  store i64 %10, ptr %5, align 8, !tbaa !5
  %13 = call fastcc i32 @decode(ptr noundef %0, i64 noundef %1, ptr noundef nonnull %5) #35
  %14 = tail call fastcc i32 @is_space(i32 noundef %13) #35, !range !46
  %15 = icmp eq i32 %14, 0
  %16 = load i64, ptr %5, align 8
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %5) #31
  br i1 %15, label %17, label %9

17:                                               ; preds = %12, %9, %4
  %18 = phi i64 [ 0, %4 ], [ %10, %9 ], [ %10, %12 ]
  %19 = and i32 %2, 2
  %20 = icmp eq i32 %19, 0
  br i1 %20, label %38, label %21

21:                                               ; preds = %17, %33
  %22 = phi i64 [ %34, %33 ], [ %1, %17 ]
  %23 = icmp ugt i64 %22, %18
  br i1 %23, label %24, label %38

24:                                               ; preds = %21, %28
  %25 = phi i64 [ %26, %28 ], [ %22, %21 ]
  %26 = add i64 %25, -1
  %27 = icmp ugt i64 %26, %18
  br i1 %27, label %28, label %33

28:                                               ; preds = %24
  %29 = getelementptr inbounds i8, ptr %0, i64 %26
  %30 = load i8, ptr %29, align 1, !tbaa !12
  %31 = and i8 %30, -64
  %32 = icmp eq i8 %31, -128
  br i1 %32, label %24, label %33, !llvm.loop !90

33:                                               ; preds = %24, %28
  %34 = phi i64 [ %18, %24 ], [ %26, %28 ]
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %6) #31
  store i64 %34, ptr %6, align 8, !tbaa !5
  %35 = call fastcc i32 @decode(ptr noundef %0, i64 noundef %22, ptr noundef nonnull %6) #35
  %36 = tail call fastcc i32 @is_space(i32 noundef %35) #35, !range !46
  %37 = icmp eq i32 %36, 0
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %6) #31
  br i1 %37, label %38, label %21

38:                                               ; preds = %33, %21, %17
  %39 = phi i64 [ %1, %17 ], [ %22, %21 ], [ %22, %33 ]
  %40 = sub i64 %39, %18
  %41 = icmp eq i64 %39, %18
  br i1 %41, label %45, label %42

42:                                               ; preds = %38
  %43 = getelementptr inbounds i8, ptr %0, i64 %18
  %44 = tail call ptr @fa_alloc(i64 noundef %40, i64 noundef 1) #34, !noalias !91
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %44, ptr align 1 %43, i64 %40, i1 false), !noalias !91
  br label %45

45:                                               ; preds = %38, %42
  %46 = phi ptr [ %44, %42 ], [ @.str.34, %38 ]
  store ptr %46, ptr %3, align 8, !tbaa.struct !16
  %47 = getelementptr inbounds i8, ptr %3, i64 8
  store i64 %40, ptr %47, align 8, !tbaa.struct !17
  %48 = getelementptr inbounds i8, ptr %3, i64 16
  store i64 %40, ptr %48, align 8, !tbaa.struct !18
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(none)
define internal fastcc noundef i32 @is_space(i32 noundef %0) unnamed_addr #11 {
  switch i32 %0, label %2 [
    i32 5760, label %9
    i32 160, label %9
    i32 32, label %9
    i32 13, label %9
    i32 12, label %9
    i32 11, label %9
    i32 10, label %9
    i32 9, label %9
  ]

2:                                                ; preds = %1
  %3 = add i32 %0, -8192
  %4 = icmp ult i32 %3, 11
  br i1 %4, label %9, label %5

5:                                                ; preds = %2
  switch i32 %0, label %6 [
    i32 12288, label %9
    i32 8287, label %9
    i32 8239, label %9
    i32 8233, label %9
    i32 8232, label %9
  ]

6:                                                ; preds = %5
  %7 = icmp eq i32 %0, 65279
  %8 = zext i1 %7 to i32
  br label %9

9:                                                ; preds = %5, %5, %5, %5, %5, %2, %1, %1, %1, %1, %1, %1, %1, %1, %6
  %10 = phi i32 [ %8, %6 ], [ 1, %1 ], [ 1, %5 ], [ 1, %1 ], [ 1, %1 ], [ 1, %1 ], [ 1, %1 ], [ 1, %1 ], [ 1, %1 ], [ 1, %1 ], [ 1, %2 ], [ 1, %5 ], [ 1, %5 ], [ 1, %5 ], [ 1, %5 ]
  ret i32 %10
}

; Function Attrs: nounwind optsize
define void @fa_string_repeat(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2, ptr nocapture noundef writeonly %3) local_unnamed_addr #0 {
  %5 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %5, align 8, !tbaa !5
  %6 = icmp sgt i64 %2, 0
  br i1 %6, label %7, label %9

7:                                                ; preds = %4
  %8 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  br label %10

9:                                                ; preds = %10, %4
  tail call void @fa_b_take(ptr noundef %3) #35
  ret void

10:                                               ; preds = %7, %10
  %11 = phi i64 [ 0, %7 ], [ %17, %10 ]
  tail call fastcc void @reserve(i64 noundef %1) #35
  %12 = load ptr, ptr %8, align 8, !tbaa !9
  %13 = load i64, ptr %5, align 8, !tbaa !5
  %14 = getelementptr inbounds i8, ptr %12, i64 %13
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %14, ptr align 1 %0, i64 %1, i1 false)
  %15 = load i64, ptr %5, align 8, !tbaa !5
  %16 = add i64 %15, %1
  store i64 %16, ptr %5, align 8, !tbaa !5
  %17 = add nuw nsw i64 %11, 1
  %18 = icmp eq i64 %17, %2
  br i1 %18, label %9, label %10, !llvm.loop !94
}

; Function Attrs: nounwind optsize
define void @fa_array_join(ptr nocapture noundef readonly %0, ptr nocapture noundef readonly %1, i64 noundef %2, ptr nocapture noundef writeonly %3) local_unnamed_addr #0 {
  %5 = load ptr, ptr %0, align 8, !tbaa !36
  %6 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %6, align 8, !tbaa !5
  %7 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %8 = load i64, ptr %7, align 8, !tbaa !38
  %9 = icmp sgt i64 %8, 0
  br i1 %9, label %10, label %12

10:                                               ; preds = %4
  %11 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  br label %13

12:                                               ; preds = %22, %4
  tail call void @fa_b_take(ptr noundef %3) #35
  ret void

13:                                               ; preds = %10, %22
  %14 = phi i64 [ 0, %10 ], [ %32, %22 ]
  %15 = icmp eq i64 %14, 0
  br i1 %15, label %22, label %16

16:                                               ; preds = %13
  tail call fastcc void @reserve(i64 noundef %2) #35
  %17 = load ptr, ptr %11, align 8, !tbaa !9
  %18 = load i64, ptr %6, align 8, !tbaa !5
  %19 = getelementptr inbounds i8, ptr %17, i64 %18
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %19, ptr align 1 %1, i64 %2, i1 false)
  %20 = load i64, ptr %6, align 8, !tbaa !5
  %21 = add i64 %20, %2
  store i64 %21, ptr %6, align 8, !tbaa !5
  br label %22

22:                                               ; preds = %16, %13
  %23 = getelementptr inbounds %struct.FaString, ptr %5, i64 %14
  %24 = load ptr, ptr %23, align 8, !tbaa !36
  %25 = getelementptr inbounds %struct.FaString, ptr %5, i64 %14, i32 1
  %26 = load i64, ptr %25, align 8, !tbaa !38
  tail call fastcc void @reserve(i64 noundef %26) #35
  %27 = load ptr, ptr %11, align 8, !tbaa !9
  %28 = load i64, ptr %6, align 8, !tbaa !5
  %29 = getelementptr inbounds i8, ptr %27, i64 %28
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %29, ptr align 1 %24, i64 %26, i1 false)
  %30 = load i64, ptr %6, align 8, !tbaa !5
  %31 = add i64 %30, %26
  store i64 %31, ptr %6, align 8, !tbaa !5
  %32 = add nuw nsw i64 %14, 1
  %33 = load i64, ptr %7, align 8, !tbaa !38
  %34 = icmp slt i64 %32, %33
  br i1 %34, label %13, label %12, !llvm.loop !95
}

; Function Attrs: nounwind optsize
define void @fa_string_pad(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2, ptr nocapture noundef readonly %3, i64 noundef %4, i32 noundef %5, ptr nocapture noundef writeonly %6) local_unnamed_addr #0 {
  %8 = tail call i64 @fa_graphemes(ptr noundef %0, i64 noundef %1) #35
  %9 = sub nsw i64 %2, %8
  %10 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %10, align 8, !tbaa !5
  %11 = icmp eq i32 %5, 0
  br i1 %11, label %12, label %19

12:                                               ; preds = %7
  tail call fastcc void @reserve(i64 noundef %1) #35
  %13 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %14 = load ptr, ptr %13, align 8, !tbaa !9
  %15 = load i64, ptr %10, align 8, !tbaa !5
  %16 = getelementptr inbounds i8, ptr %14, i64 %15
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %16, ptr align 1 %0, i64 %1, i1 false)
  %17 = load i64, ptr %10, align 8, !tbaa !5
  %18 = add i64 %17, %1
  store i64 %18, ptr %10, align 8, !tbaa !5
  br label %19

19:                                               ; preds = %12, %7
  %20 = icmp ne i64 %4, 0
  %21 = icmp sgt i64 %9, 0
  %22 = and i1 %20, %21
  br i1 %22, label %23, label %25

23:                                               ; preds = %19
  %24 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  br label %26

25:                                               ; preds = %26, %19
  br i1 %11, label %48, label %41

26:                                               ; preds = %23, %26
  %27 = phi i64 [ 0, %23 ], [ %38, %26 ]
  %28 = phi i64 [ 0, %23 ], [ %39, %26 ]
  %29 = tail call fastcc i64 @next_boundary(ptr noundef %3, i64 noundef %4, i64 noundef %27) #35
  %30 = getelementptr inbounds i8, ptr %3, i64 %27
  %31 = sub i64 %29, %27
  tail call fastcc void @reserve(i64 noundef %31) #35
  %32 = load ptr, ptr %24, align 8, !tbaa !9
  %33 = load i64, ptr %10, align 8, !tbaa !5
  %34 = getelementptr inbounds i8, ptr %32, i64 %33
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %34, ptr align 1 %30, i64 %31, i1 false)
  %35 = load i64, ptr %10, align 8, !tbaa !5
  %36 = add i64 %35, %31
  store i64 %36, ptr %10, align 8, !tbaa !5
  %37 = icmp eq i64 %29, %4
  %38 = select i1 %37, i64 0, i64 %29
  %39 = add nuw nsw i64 %28, 1
  %40 = icmp slt i64 %39, %9
  br i1 %40, label %26, label %25, !llvm.loop !96

41:                                               ; preds = %25
  tail call fastcc void @reserve(i64 noundef %1) #35
  %42 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %43 = load ptr, ptr %42, align 8, !tbaa !9
  %44 = load i64, ptr %10, align 8, !tbaa !5
  %45 = getelementptr inbounds i8, ptr %43, i64 %44
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %45, ptr align 1 %0, i64 %1, i1 false)
  %46 = load i64, ptr %10, align 8, !tbaa !5
  %47 = add i64 %46, %1
  store i64 %47, ptr %10, align 8, !tbaa !5
  br label %48

48:                                               ; preds = %41, %25
  tail call void @fa_b_take(ptr noundef %6) #35
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_arguments_init(i32 noundef %0, ptr nocapture noundef readonly %1) local_unnamed_addr #0 {
  %3 = icmp sgt i32 %0, 1
  %4 = zext nneg i32 %0 to i64
  %5 = add nsw i64 %4, -1
  %6 = select i1 %3, i64 %5, i64 0
  store i64 %6, ptr @argument_count, align 8, !tbaa !5
  %7 = mul nsw i64 %6, 24
  %8 = add nsw i64 %7, 24
  %9 = tail call ptr @fa_alloc(i64 noundef %8, i64 noundef 8) #34
  store ptr %9, ptr @argument_items, align 8, !tbaa !9
  %10 = load i64, ptr @argument_count, align 8, !tbaa !5
  %11 = icmp eq i64 %10, 0
  br i1 %11, label %12, label %13

12:                                               ; preds = %25, %2
  ret void

13:                                               ; preds = %2, %25
  %14 = phi i64 [ %15, %25 ], [ 0, %2 ]
  %15 = add nuw i64 %14, 1
  %16 = getelementptr inbounds ptr, ptr %1, i64 %15
  %17 = load ptr, ptr %16, align 8, !tbaa !9
  %18 = getelementptr inbounds %struct.FaString, ptr %9, i64 %14
  store ptr %17, ptr %18, align 8, !tbaa !36
  br label %19

19:                                               ; preds = %19, %13
  %20 = phi i64 [ 0, %13 ], [ %24, %19 ]
  %21 = getelementptr inbounds i8, ptr %17, i64 %20
  %22 = load i8, ptr %21, align 1, !tbaa !12
  %23 = icmp eq i8 %22, 0
  %24 = add i64 %20, 1
  br i1 %23, label %25, label %19, !llvm.loop !24

25:                                               ; preds = %19
  %26 = getelementptr inbounds %struct.FaString, ptr %9, i64 %14, i32 1
  store i64 %20, ptr %26, align 8, !tbaa !38
  %27 = getelementptr inbounds %struct.FaString, ptr %9, i64 %14, i32 2
  store i64 0, ptr %27, align 8, !tbaa !40
  %28 = icmp eq i64 %15, %10
  br i1 %28, label %12, label %13, !llvm.loop !97
}

; Function Attrs: nounwind optsize
define void @fa_array_append(ptr nocapture noundef %0, ptr nocapture noundef readonly %1, i64 noundef %2) local_unnamed_addr #0 {
  %4 = getelementptr inbounds %struct.FaArrayHeader, ptr %1, i64 0, i32 1
  %5 = load i64, ptr %4, align 8, !tbaa !38
  %6 = icmp sgt i64 %5, 0
  br i1 %6, label %8, label %7

7:                                                ; preds = %8, %3
  ret void

8:                                                ; preds = %3, %8
  %9 = phi i64 [ %14, %8 ], [ 0, %3 ]
  %10 = tail call ptr @fa_array_push_slot(ptr noundef %0, i64 noundef %2) #35
  %11 = load ptr, ptr %1, align 8, !tbaa !36
  %12 = mul nsw i64 %9, %2
  %13 = getelementptr inbounds i8, ptr %11, i64 %12
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %10, ptr align 1 %13, i64 %2, i1 false)
  %14 = add nuw nsw i64 %9, 1
  %15 = icmp eq i64 %14, %5
  br i1 %15, label %7, label %8, !llvm.loop !98
}

; Function Attrs: nounwind optsize
define noundef ptr @fa_array_slice(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2, i8 noundef signext %3, i64 noundef %4, ptr nocapture noundef writeonly %5) local_unnamed_addr #0 {
  %7 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %8 = load i64, ptr %7, align 8, !tbaa !38
  %9 = zext i8 %3 to i32
  %10 = and i32 %9, 1
  %11 = icmp eq i32 %10, 0
  br i1 %11, label %23, label %12

12:                                               ; preds = %6
  %13 = sub nsw i64 0, %8
  %14 = icmp sgt i64 %13, %1
  %15 = icmp slt i64 %8, %1
  %16 = or i1 %14, %15
  br i1 %16, label %17, label %19

17:                                               ; preds = %12
  %18 = tail call ptr @fa_error_index(i64 noundef %1, i64 noundef %8) #35
  br label %59

19:                                               ; preds = %12
  %20 = icmp slt i64 %1, 0
  %21 = select i1 %20, i64 %8, i64 0
  %22 = add nsw i64 %21, %1
  br label %23

23:                                               ; preds = %19, %6
  %24 = phi i64 [ %22, %19 ], [ 0, %6 ]
  %25 = and i32 %9, 2
  %26 = icmp eq i32 %25, 0
  br i1 %26, label %46, label %27

27:                                               ; preds = %23
  %28 = and i32 %9, 4
  %29 = lshr exact i32 %28, 2
  %30 = sub nsw i64 0, %8
  %31 = icmp sgt i64 %30, %2
  br i1 %31, label %44, label %32

32:                                               ; preds = %27
  %33 = shl nuw i32 %28, 29
  %34 = ashr exact i32 %33, 31
  %35 = sext i32 %34 to i64
  %36 = add nsw i64 %8, %35
  %37 = icmp slt i64 %36, %2
  br i1 %37, label %44, label %38

38:                                               ; preds = %32
  %39 = icmp slt i64 %2, 0
  %40 = select i1 %39, i64 %8, i64 0
  %41 = zext nneg i32 %29 to i64
  %42 = add i64 %41, %2
  %43 = add i64 %42, %40
  br label %46

44:                                               ; preds = %27, %32
  %45 = tail call ptr @fa_error_index(i64 noundef %2, i64 noundef %8) #35
  br label %59

46:                                               ; preds = %38, %23
  %47 = phi i64 [ %8, %23 ], [ %43, %38 ]
  %48 = tail call i64 @llvm.smax.i64(i64 %47, i64 %24)
  %49 = sub nsw i64 %48, %24
  %50 = tail call ptr @fa_alloc(i64 noundef 24, i64 noundef 8) #34
  %51 = mul nsw i64 %49, %4
  %52 = tail call i64 @llvm.umax.i64(i64 %51, i64 1)
  %53 = tail call ptr @fa_alloc(i64 noundef %52, i64 noundef 16) #34
  store ptr %53, ptr %50, align 8, !tbaa !36
  %54 = getelementptr inbounds %struct.FaArrayHeader, ptr %50, i64 0, i32 1
  store i64 %49, ptr %54, align 8, !tbaa !38
  %55 = getelementptr inbounds %struct.FaArrayHeader, ptr %50, i64 0, i32 2
  store i64 %49, ptr %55, align 8, !tbaa !40
  %56 = load ptr, ptr %0, align 8, !tbaa !36
  %57 = mul nsw i64 %24, %4
  %58 = getelementptr inbounds i8, ptr %56, i64 %57
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %53, ptr align 1 %58, i64 %51, i1 false)
  store ptr %50, ptr %5, align 8, !tbaa !9
  br label %59

59:                                               ; preds = %44, %46, %17
  %60 = phi ptr [ %18, %17 ], [ null, %46 ], [ %45, %44 ]
  ret ptr %60
}

; Function Attrs: cold noinline nounwind optsize
define noundef ptr @fa_error_index(i64 noundef %0, i64 noundef %1) local_unnamed_addr #12 {
  %3 = tail call ptr @fa_alloc(i64 noundef 24, i64 noundef 8) #34
  store i64 1, ptr %3, align 8, !tbaa !99
  %4 = getelementptr inbounds %struct.IndexError, ptr %3, i64 0, i32 1
  store i64 %0, ptr %4, align 8, !tbaa !101
  %5 = getelementptr inbounds %struct.IndexError, ptr %3, i64 0, i32 2
  store i64 %1, ptr %5, align 8, !tbaa !102
  ret ptr %3
}

; Function Attrs: nounwind optsize
define void @fa_array_reserve(ptr nocapture noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #0 {
  %4 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 2
  %5 = load i64, ptr %4, align 8, !tbaa !40
  %6 = icmp slt i64 %5, %1
  br i1 %6, label %7, label %23

7:                                                ; preds = %3
  %8 = icmp sgt i64 %2, 0
  br i1 %8, label %9, label %13

9:                                                ; preds = %7
  %10 = udiv i64 9223372036854775807, %2
  %11 = icmp slt i64 %10, %1
  br i1 %11, label %12, label %13

12:                                               ; preds = %9
  tail call fastcc void @out_of_memory() #37
  unreachable

13:                                               ; preds = %9, %7
  %14 = mul nsw i64 %2, %1
  %15 = tail call ptr @fa_alloc(i64 noundef %14, i64 noundef 16) #34
  %16 = load ptr, ptr %0, align 8, !tbaa !36
  %17 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %18 = load i64, ptr %17, align 8, !tbaa !38
  %19 = mul nsw i64 %18, %2
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %15, ptr align 1 %16, i64 %19, i1 false)
  %20 = load ptr, ptr %0, align 8, !tbaa !36
  %21 = load i64, ptr %4, align 8, !tbaa !40
  %22 = mul nsw i64 %21, %2
  tail call void @fa_free(ptr noundef %20, i64 noundef %22, i64 noundef 16) #34
  store ptr %15, ptr %0, align 8, !tbaa !36
  store i64 %1, ptr %4, align 8, !tbaa !40
  br label %23

23:                                               ; preds = %3, %13
  ret void
}

; Function Attrs: noreturn nounwind optsize
define internal fastcc void @out_of_memory() unnamed_addr #2 {
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @out_of_memory.message, i64 noundef 20) #34
  tail call void @fa_exit(i32 noundef 101) #36
  unreachable
}

; Function Attrs: nounwind optsize
define void @fa_array_assign(ptr noundef %0, ptr noundef readonly %1, i64 noundef %2) local_unnamed_addr #0 {
  %4 = icmp eq ptr %0, %1
  br i1 %4, label %27, label %5

5:                                                ; preds = %3
  %6 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 2
  %7 = load i64, ptr %6, align 8, !tbaa !40
  %8 = getelementptr inbounds %struct.FaArrayHeader, ptr %1, i64 0, i32 1
  %9 = load i64, ptr %8, align 8, !tbaa !38
  %10 = icmp slt i64 %7, %9
  br i1 %10, label %13, label %11

11:                                               ; preds = %5
  %12 = load ptr, ptr %0, align 8, !tbaa !36
  br label %20

13:                                               ; preds = %5
  %14 = mul nsw i64 %9, %2
  %15 = tail call ptr @fa_alloc(i64 noundef %14, i64 noundef 16) #34
  %16 = load ptr, ptr %0, align 8, !tbaa !36
  %17 = load i64, ptr %6, align 8, !tbaa !40
  %18 = mul nsw i64 %17, %2
  tail call void @fa_free(ptr noundef %16, i64 noundef %18, i64 noundef 16) #34
  store ptr %15, ptr %0, align 8, !tbaa !36
  %19 = load i64, ptr %8, align 8, !tbaa !38
  store i64 %19, ptr %6, align 8, !tbaa !40
  br label %20

20:                                               ; preds = %11, %13
  %21 = phi i64 [ %9, %11 ], [ %19, %13 ]
  %22 = phi ptr [ %12, %11 ], [ %15, %13 ]
  %23 = load ptr, ptr %1, align 8, !tbaa !36
  %24 = mul nsw i64 %21, %2
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %22, ptr align 1 %23, i64 %24, i1 false)
  %25 = load i64, ptr %8, align 8, !tbaa !38
  %26 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  store i64 %25, ptr %26, align 8, !tbaa !38
  br label %27

27:                                               ; preds = %3, %20
  ret void
}

; Function Attrs: nounwind optsize
define ptr @fa_array_copy(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %4 = load i64, ptr %3, align 8, !tbaa !38
  %5 = tail call ptr @fa_alloc(i64 noundef 24, i64 noundef 8) #34
  %6 = mul nsw i64 %4, %1
  %7 = tail call i64 @llvm.umax.i64(i64 %6, i64 1)
  %8 = tail call ptr @fa_alloc(i64 noundef %7, i64 noundef 16) #34
  store ptr %8, ptr %5, align 8, !tbaa !36
  %9 = getelementptr inbounds %struct.FaArrayHeader, ptr %5, i64 0, i32 1
  store i64 %4, ptr %9, align 8, !tbaa !38
  %10 = getelementptr inbounds %struct.FaArrayHeader, ptr %5, i64 0, i32 2
  store i64 %4, ptr %10, align 8, !tbaa !40
  %11 = load ptr, ptr %0, align 8, !tbaa !36
  %12 = load i64, ptr %3, align 8, !tbaa !38
  %13 = mul nsw i64 %12, %1
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %8, ptr align 1 %11, i64 %13, i1 false)
  ret ptr %5
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define void @fa_array_reverse(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #10 {
  %3 = alloca [64 x i8], align 16
  %4 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %5 = load i64, ptr %4, align 8, !tbaa !38
  %6 = icmp slt i64 %5, 2
  br i1 %6, label %32, label %7

7:                                                ; preds = %2
  call void @llvm.lifetime.start.p0(i64 64, ptr nonnull %3)
  %8 = add nsw i64 %5, -1
  %9 = mul nsw i64 %8, %1
  %10 = icmp sgt i64 %9, 0
  br i1 %10, label %11, label %31

11:                                               ; preds = %7
  %12 = load ptr, ptr %0, align 8, !tbaa !36
  %13 = getelementptr inbounds i8, ptr %12, i64 %9
  %14 = icmp sgt i64 %1, 0
  %15 = sub i64 0, %1
  br label %16

16:                                               ; preds = %11, %19
  %17 = phi ptr [ %12, %11 ], [ %20, %19 ]
  %18 = phi ptr [ %13, %11 ], [ %21, %19 ]
  br i1 %14, label %23, label %19

19:                                               ; preds = %23, %16
  %20 = getelementptr inbounds i8, ptr %17, i64 %1
  %21 = getelementptr inbounds i8, ptr %18, i64 %15
  %22 = icmp ult ptr %20, %21
  br i1 %22, label %16, label %31, !llvm.loop !103

23:                                               ; preds = %16, %23
  %24 = phi i64 [ %29, %23 ], [ 0, %16 ]
  %25 = sub nsw i64 %1, %24
  %26 = tail call i64 @llvm.umin.i64(i64 %25, i64 64)
  %27 = getelementptr inbounds i8, ptr %17, i64 %24
  call void @llvm.memcpy.p0.p0.i64(ptr nonnull align 16 %3, ptr align 1 %27, i64 %26, i1 false)
  %28 = getelementptr inbounds i8, ptr %18, i64 %24
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %27, ptr nonnull align 1 %28, i64 %26, i1 false)
  call void @llvm.memcpy.p0.p0.i64(ptr nonnull align 1 %28, ptr nonnull align 16 %3, i64 %26, i1 false)
  %29 = add nuw nsw i64 %24, 64
  %30 = icmp slt i64 %29, %1
  br i1 %30, label %23, label %19, !llvm.loop !104

31:                                               ; preds = %19, %7
  call void @llvm.lifetime.end.p0(i64 64, ptr nonnull %3)
  br label %32

32:                                               ; preds = %2, %31
  ret void
}

; Function Attrs: nounwind optsize
define ptr @fa_array_insert_slot(ptr nocapture noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #0 {
  %4 = tail call ptr @fa_array_push_slot(ptr noundef %0, i64 noundef %2) #35
  %5 = load ptr, ptr %0, align 8, !tbaa !36
  %6 = mul nsw i64 %2, %1
  %7 = getelementptr inbounds i8, ptr %5, i64 %6
  %8 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %9 = load i64, ptr %8, align 8, !tbaa !38
  %10 = xor i64 %1, -1
  %11 = add i64 %9, %10
  %12 = mul nsw i64 %11, %2
  %13 = getelementptr inbounds i8, ptr %7, i64 %2
  tail call void @llvm.memmove.p0.p0.i64(ptr align 1 %13, ptr align 1 %7, i64 %12, i1 false)
  ret ptr %7
}

; Function Attrs: mustprogress nocallback nofree nounwind willreturn memory(argmem: readwrite)
declare void @llvm.memmove.p0.p0.i64(ptr nocapture writeonly, ptr nocapture readonly, i64, i1 immarg) #7

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define void @fa_array_remove(ptr nocapture noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #13 {
  %4 = load ptr, ptr %0, align 8, !tbaa !36
  %5 = mul nsw i64 %2, %1
  %6 = getelementptr inbounds i8, ptr %4, i64 %5
  %7 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %8 = load i64, ptr %7, align 8, !tbaa !38
  %9 = xor i64 %1, -1
  %10 = add i64 %8, %9
  %11 = mul nsw i64 %10, %2
  %12 = getelementptr inbounds i8, ptr %6, i64 %2
  tail call void @llvm.memmove.p0.p0.i64(ptr align 1 %6, ptr align 1 %12, i64 %11, i1 false)
  %13 = load i64, ptr %7, align 8, !tbaa !38
  %14 = add nsw i64 %13, -1
  store i64 %14, ptr %7, align 8, !tbaa !38
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_array_sort(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef readonly %2, ptr noundef %3) local_unnamed_addr #0 {
  %5 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %6 = load i64, ptr %5, align 8, !tbaa !38
  %7 = icmp slt i64 %6, 2
  br i1 %7, label %75, label %8

8:                                                ; preds = %4
  %9 = mul nsw i64 %6, %1
  %10 = load ptr, ptr %0, align 8, !tbaa !36
  %11 = tail call ptr @fa_alloc(i64 noundef %9, i64 noundef 16) #34
  br label %12

12:                                               ; preds = %20, %8
  %13 = phi ptr [ %10, %8 ], [ %14, %20 ]
  %14 = phi ptr [ %11, %8 ], [ %13, %20 ]
  %15 = phi i64 [ 1, %8 ], [ %16, %20 ]
  %16 = shl nsw i64 %15, 1
  br label %22

17:                                               ; preds = %20
  %18 = load ptr, ptr %0, align 8, !tbaa !36
  %19 = icmp eq ptr %14, %18
  br i1 %19, label %73, label %72

20:                                               ; preds = %54
  %21 = icmp slt i64 %16, %6
  br i1 %21, label %12, label %17, !llvm.loop !105

22:                                               ; preds = %12, %54
  %23 = phi i64 [ 0, %12 ], [ %26, %54 ]
  %24 = add nsw i64 %23, %15
  %25 = tail call i64 @llvm.smin.i64(i64 %24, i64 %6)
  %26 = add nsw i64 %23, %16
  %27 = tail call i64 @llvm.smin.i64(i64 %26, i64 %6)
  %28 = icmp slt i64 %23, %25
  %29 = icmp slt i64 %24, %27
  %30 = select i1 %28, i1 %29, i1 false
  br i1 %30, label %31, label %54

31:                                               ; preds = %22, %47
  %32 = phi i64 [ %50, %47 ], [ %23, %22 ]
  %33 = phi i64 [ %49, %47 ], [ %25, %22 ]
  %34 = phi i64 [ %48, %47 ], [ %23, %22 ]
  %35 = mul nsw i64 %33, %1
  %36 = getelementptr inbounds i8, ptr %13, i64 %35
  %37 = mul nsw i64 %34, %1
  %38 = getelementptr inbounds i8, ptr %13, i64 %37
  %39 = tail call i32 %2(ptr noundef %3, ptr noundef %36, ptr noundef %38) #34
  %40 = icmp slt i32 %39, 0
  %41 = mul nsw i64 %32, %1
  %42 = getelementptr inbounds i8, ptr %14, i64 %41
  br i1 %40, label %43, label %45

43:                                               ; preds = %31
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %42, ptr align 1 %36, i64 %1, i1 false)
  %44 = add nsw i64 %33, 1
  br label %47

45:                                               ; preds = %31
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %42, ptr align 1 %38, i64 %1, i1 false)
  %46 = add nsw i64 %34, 1
  br label %47

47:                                               ; preds = %45, %43
  %48 = phi i64 [ %34, %43 ], [ %46, %45 ]
  %49 = phi i64 [ %44, %43 ], [ %33, %45 ]
  %50 = add nsw i64 %32, 1
  %51 = icmp slt i64 %48, %25
  %52 = icmp slt i64 %49, %27
  %53 = select i1 %51, i1 %52, i1 false
  br i1 %53, label %31, label %54, !llvm.loop !106

54:                                               ; preds = %47, %22
  %55 = phi i64 [ %23, %22 ], [ %48, %47 ]
  %56 = phi i64 [ %25, %22 ], [ %49, %47 ]
  %57 = phi i64 [ %23, %22 ], [ %50, %47 ]
  %58 = mul nsw i64 %57, %1
  %59 = getelementptr inbounds i8, ptr %14, i64 %58
  %60 = mul nsw i64 %55, %1
  %61 = getelementptr inbounds i8, ptr %13, i64 %60
  %62 = sub nsw i64 %25, %55
  %63 = mul nsw i64 %62, %1
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %59, ptr align 1 %61, i64 %63, i1 false)
  %64 = add nsw i64 %57, %62
  %65 = mul nsw i64 %64, %1
  %66 = getelementptr inbounds i8, ptr %14, i64 %65
  %67 = mul nsw i64 %56, %1
  %68 = getelementptr inbounds i8, ptr %13, i64 %67
  %69 = sub nsw i64 %27, %56
  %70 = mul nsw i64 %69, %1
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %66, ptr align 1 %68, i64 %70, i1 false)
  %71 = icmp slt i64 %26, %6
  br i1 %71, label %22, label %20, !llvm.loop !107

72:                                               ; preds = %17
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %18, ptr align 1 %14, i64 %9, i1 false)
  br label %73

73:                                               ; preds = %17, %72
  %74 = phi ptr [ %14, %72 ], [ %13, %17 ]
  tail call void @fa_free(ptr noundef %74, i64 noundef %9, i64 noundef 16) #34
  br label %75

75:                                               ; preds = %73, %4
  ret void
}

; Function Attrs: nounwind optsize
define ptr @fa_arguments() local_unnamed_addr #0 {
  %1 = load i64, ptr @argument_count, align 8, !tbaa !5
  %2 = tail call ptr @fa_alloc(i64 noundef 24, i64 noundef 8) #34
  %3 = mul nsw i64 %1, 24
  %4 = tail call i64 @llvm.umax.i64(i64 %3, i64 1)
  %5 = tail call ptr @fa_alloc(i64 noundef %4, i64 noundef 16) #34
  store ptr %5, ptr %2, align 8, !tbaa !36
  %6 = getelementptr inbounds %struct.FaArrayHeader, ptr %2, i64 0, i32 1
  store i64 %1, ptr %6, align 8, !tbaa !38
  %7 = getelementptr inbounds %struct.FaArrayHeader, ptr %2, i64 0, i32 2
  store i64 %1, ptr %7, align 8, !tbaa !40
  %8 = load ptr, ptr @argument_items, align 8, !tbaa !9
  %9 = load i64, ptr @argument_count, align 8, !tbaa !5
  %10 = mul nsw i64 %9, 24
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %5, ptr align 1 %8, i64 %10, i1 false)
  ret ptr %2
}

; Function Attrs: nounwind optsize
define void @fa_array_free(ptr noundef %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = load ptr, ptr %0, align 8, !tbaa !36
  %4 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 2
  %5 = load i64, ptr %4, align 8, !tbaa !40
  %6 = mul nsw i64 %5, %1
  tail call void @fa_free(ptr noundef %3, i64 noundef %6, i64 noundef 16) #34
  tail call void @fa_free(ptr noundef nonnull %0, i64 noundef 24, i64 noundef 8) #34
  ret void
}

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read)
define ptr @fa_segmented_at(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2, i32 noundef %3) local_unnamed_addr #14 {
  %5 = zext nneg i32 %3 to i64
  %6 = lshr i64 %1, %5
  %7 = icmp eq i64 %6, 0
  br i1 %7, label %15, label %8

8:                                                ; preds = %4
  %9 = tail call i64 @llvm.ctlz.i64(i64 %6, i1 true), !range !108
  %10 = sub nuw nsw i64 64, %9
  %11 = sext i32 %3 to i64
  %12 = sub nsw i64 %11, %9
  %13 = add nsw i64 %12, 63
  %14 = shl nsw i64 -1, %13
  br label %15

15:                                               ; preds = %4, %8
  %16 = phi i64 [ %14, %8 ], [ 0, %4 ]
  %17 = phi i64 [ %10, %8 ], [ 0, %4 ]
  %18 = getelementptr inbounds %struct.FaSegmented, ptr %0, i64 0, i32 2, i64 %17
  %19 = load ptr, ptr %18, align 8, !tbaa !9
  %20 = add i64 %16, %1
  %21 = mul nsw i64 %20, %2
  %22 = getelementptr inbounds i8, ptr %19, i64 %21
  ret ptr %22
}

; Function Attrs: nounwind optsize
define noundef ptr @fa_segmented_new() local_unnamed_addr #0 {
  %1 = tail call ptr @fa_alloc(i64 noundef 400, i64 noundef 8) #34
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(400) %1, i8 0, i64 400, i1 false)
  ret ptr %1
}

; Function Attrs: mustprogress nocallback nofree nounwind willreturn memory(argmem: write)
declare void @llvm.memset.p0.i64(ptr nocapture writeonly, i8, i64, i1 immarg) #15

; Function Attrs: nounwind optsize
define ptr @fa_segmented_push_slot(ptr nocapture noundef %0, i64 noundef %1, i32 noundef %2) local_unnamed_addr #0 {
  %4 = load i64, ptr %0, align 8, !tbaa !109
  %5 = zext nneg i32 %2 to i64
  %6 = lshr i64 %4, %5
  %7 = icmp eq i64 %6, 0
  br i1 %7, label %15, label %8

8:                                                ; preds = %3
  %9 = tail call i64 @llvm.ctlz.i64(i64 %6, i1 true), !range !108
  %10 = sub nuw nsw i64 64, %9
  %11 = sext i32 %2 to i64
  %12 = sub nsw i64 %11, %9
  %13 = add nsw i64 %12, 63
  %14 = shl nsw i64 -1, %13
  br label %15

15:                                               ; preds = %3, %8
  %16 = phi i64 [ %14, %8 ], [ 0, %3 ]
  %17 = phi i64 [ %10, %8 ], [ 0, %3 ]
  %18 = getelementptr inbounds %struct.FaSegmented, ptr %0, i64 0, i32 1
  %19 = load i64, ptr %18, align 8, !tbaa !111
  %20 = icmp slt i64 %17, %19
  br i1 %20, label %21, label %24

21:                                               ; preds = %15
  %22 = getelementptr inbounds %struct.FaSegmented, ptr %0, i64 0, i32 2, i64 %17
  %23 = load ptr, ptr %22, align 8, !tbaa !9
  br label %32

24:                                               ; preds = %15
  %25 = sext i32 %2 to i64
  %26 = tail call i64 @llvm.usub.sat.i64(i64 %17, i64 1)
  %27 = add nsw i64 %26, %25
  %28 = shl i64 %1, %27
  %29 = tail call ptr @fa_alloc(i64 noundef %28, i64 noundef 16) #34
  %30 = getelementptr inbounds %struct.FaSegmented, ptr %0, i64 0, i32 2, i64 %17
  store ptr %29, ptr %30, align 8, !tbaa !9
  %31 = add nuw nsw i64 %17, 1
  store i64 %31, ptr %18, align 8, !tbaa !111
  br label %32

32:                                               ; preds = %21, %24
  %33 = phi ptr [ %23, %21 ], [ %29, %24 ]
  %34 = add nsw i64 %4, 1
  store i64 %34, ptr %0, align 8, !tbaa !109
  %35 = add i64 %16, %4
  %36 = mul nsw i64 %35, %1
  %37 = getelementptr inbounds i8, ptr %33, i64 %36
  ret ptr %37
}

; Function Attrs: nounwind optsize
define void @fa_segmented_free(ptr noundef %0, i64 noundef %1, i32 noundef %2) local_unnamed_addr #0 {
  %4 = getelementptr inbounds %struct.FaSegmented, ptr %0, i64 0, i32 1
  %5 = load i64, ptr %4, align 8, !tbaa !111
  %6 = icmp sgt i64 %5, 0
  br i1 %6, label %7, label %9

7:                                                ; preds = %3
  %8 = sext i32 %2 to i64
  br label %10

9:                                                ; preds = %10, %3
  tail call void @fa_free(ptr noundef nonnull %0, i64 noundef 400, i64 noundef 8) #34
  ret void

10:                                               ; preds = %7, %10
  %11 = phi i64 [ 0, %7 ], [ %17, %10 ]
  %12 = getelementptr inbounds %struct.FaSegmented, ptr %0, i64 0, i32 2, i64 %11
  %13 = load ptr, ptr %12, align 8, !tbaa !9
  %14 = tail call i64 @llvm.usub.sat.i64(i64 %11, i64 1)
  %15 = add i64 %14, %8
  %16 = shl i64 %1, %15
  tail call void @fa_free(ptr noundef %13, i64 noundef %16, i64 noundef 16) #34
  %17 = add nuw nsw i64 %11, 1
  %18 = load i64, ptr %4, align 8, !tbaa !111
  %19 = icmp slt i64 %17, %18
  br i1 %19, label %10, label %9, !llvm.loop !112
}

; Function Attrs: nounwind optsize
define ptr @fa_segmented_flatten(ptr nocapture noundef readonly %0, i64 noundef %1, i32 noundef %2) local_unnamed_addr #0 {
  %4 = load i64, ptr %0, align 8, !tbaa !109
  %5 = tail call ptr @fa_alloc(i64 noundef 24, i64 noundef 8) #34
  %6 = mul nsw i64 %4, %1
  %7 = tail call i64 @llvm.umax.i64(i64 %6, i64 1)
  %8 = tail call ptr @fa_alloc(i64 noundef %7, i64 noundef 16) #34
  store ptr %8, ptr %5, align 8, !tbaa !36
  %9 = getelementptr inbounds %struct.FaArrayHeader, ptr %5, i64 0, i32 1
  store i64 %4, ptr %9, align 8, !tbaa !38
  %10 = getelementptr inbounds %struct.FaArrayHeader, ptr %5, i64 0, i32 2
  store i64 %4, ptr %10, align 8, !tbaa !40
  %11 = load i64, ptr %0, align 8, !tbaa !109
  %12 = icmp sgt i64 %11, 0
  br i1 %12, label %13, label %15

13:                                               ; preds = %3
  %14 = sext i32 %2 to i64
  br label %16

15:                                               ; preds = %16, %3
  ret ptr %5

16:                                               ; preds = %13, %16
  %17 = phi i64 [ %11, %13 ], [ %33, %16 ]
  %18 = phi i64 [ 0, %13 ], [ %32, %16 ]
  %19 = phi i64 [ 0, %13 ], [ %31, %16 ]
  %20 = tail call i64 @llvm.usub.sat.i64(i64 %18, i64 1)
  %21 = add i64 %20, %14
  %22 = shl nuw i64 1, %21
  %23 = sub nsw i64 %17, %19
  %24 = tail call i64 @llvm.smin.i64(i64 %22, i64 %23)
  %25 = load ptr, ptr %5, align 8, !tbaa !36
  %26 = mul nsw i64 %19, %1
  %27 = getelementptr inbounds i8, ptr %25, i64 %26
  %28 = getelementptr inbounds %struct.FaSegmented, ptr %0, i64 0, i32 2, i64 %18
  %29 = load ptr, ptr %28, align 8, !tbaa !9
  %30 = mul nsw i64 %24, %1
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %27, ptr align 1 %29, i64 %30, i1 false)
  %31 = add nsw i64 %24, %19
  %32 = add nuw nsw i64 %18, 1
  %33 = load i64, ptr %0, align 8, !tbaa !109
  %34 = icmp slt i64 %31, %33
  br i1 %34, label %16, label %15, !llvm.loop !113
}

; Function Attrs: nounwind optsize
define noundef ptr @fa_segmented_from_array(ptr noundef %0, i64 noundef %1, i32 noundef %2) local_unnamed_addr #0 {
  %4 = tail call noundef ptr @fa_alloc(i64 noundef 400, i64 noundef 8) #34
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(400) %4, i8 0, i64 400, i1 false)
  %5 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %6 = load i64, ptr %5, align 8, !tbaa !38
  %7 = icmp sgt i64 %6, 0
  br i1 %7, label %13, label %8

8:                                                ; preds = %13, %3
  %9 = load ptr, ptr %0, align 8, !tbaa !36
  %10 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 2
  %11 = load i64, ptr %10, align 8, !tbaa !40
  %12 = mul nsw i64 %11, %1
  tail call void @fa_free(ptr noundef %9, i64 noundef %12, i64 noundef 16) #34
  tail call void @fa_free(ptr noundef nonnull %0, i64 noundef 24, i64 noundef 8) #34
  ret ptr %4

13:                                               ; preds = %3, %13
  %14 = phi i64 [ %19, %13 ], [ 0, %3 ]
  %15 = tail call ptr @fa_segmented_push_slot(ptr noundef %4, i64 noundef %1, i32 noundef %2) #35
  %16 = load ptr, ptr %0, align 8, !tbaa !36
  %17 = mul nsw i64 %14, %1
  %18 = getelementptr inbounds i8, ptr %16, i64 %17
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %15, ptr align 1 %18, i64 %1, i1 false)
  %19 = add nuw nsw i64 %14, 1
  %20 = load i64, ptr %5, align 8, !tbaa !38
  %21 = icmp slt i64 %19, %20
  br i1 %21, label %13, label %8, !llvm.loop !114
}

; Function Attrs: nounwind optsize
define ptr @fa_heap_alloc(i64 noundef %0) local_unnamed_addr #0 {
  %2 = icmp ugt i64 %0, 8192
  br i1 %2, label %3, label %14

3:                                                ; preds = %1
  %4 = add i64 %0, 65551
  %5 = lshr i64 %4, 16
  %6 = icmp ult i64 %4, 65536
  br i1 %6, label %73, label %7

7:                                                ; preds = %3
  %8 = tail call fastcc ptr @take_pages(i64 noundef %5) #35
  %9 = icmp eq ptr %8, null
  br i1 %9, label %73, label %10

10:                                               ; preds = %7
  store i32 65535, ptr %8, align 4, !tbaa !115
  %11 = trunc i64 %5 to i32
  %12 = getelementptr inbounds %struct.PageHeader, ptr %8, i64 0, i32 1
  store i32 %11, ptr %12, align 4, !tbaa !117
  %13 = getelementptr inbounds i8, ptr %8, i64 16
  br label %73

14:                                               ; preds = %1
  %15 = icmp ult i64 %0, 129
  br i1 %15, label %16, label %24

16:                                               ; preds = %14
  %17 = add nuw nsw i64 %0, 15
  %18 = and i64 %17, 496
  %19 = icmp ugt i64 %0, 16
  %20 = select i1 %19, i64 %18, i64 16
  %21 = lshr exact i64 %20, 4
  %22 = trunc i64 %21 to i32
  %23 = add nsw i32 %22, -1
  br label %41

24:                                               ; preds = %14, %39
  %25 = phi i32 [ %28, %39 ], [ 8, %14 ]
  %26 = phi i64 [ %40, %39 ], [ 128, %14 ]
  %27 = lshr exact i64 %26, 2
  %28 = add i32 %25, 4
  br label %29

29:                                               ; preds = %35, %24
  %30 = phi i64 [ 1, %24 ], [ %37, %35 ]
  %31 = phi i32 [ %25, %24 ], [ %36, %35 ]
  %32 = mul nuw i64 %30, %27
  %33 = add i64 %32, %26
  %34 = icmp ult i64 %33, %0
  br i1 %34, label %35, label %41

35:                                               ; preds = %29
  %36 = add i32 %31, 1
  %37 = add nuw nsw i64 %30, 1
  %38 = icmp eq i64 %37, 5
  br i1 %38, label %39, label %29, !llvm.loop !118

39:                                               ; preds = %35
  %40 = shl i64 %26, 1
  br label %24

41:                                               ; preds = %29, %16
  %42 = phi i64 [ %20, %16 ], [ %33, %29 ]
  %43 = phi i32 [ %23, %16 ], [ %31, %29 ]
  %44 = tail call align 16 ptr @llvm.threadlocal.address.p0(ptr align 16 @free_blocks)
  %45 = zext i32 %43 to i64
  %46 = getelementptr inbounds [40 x ptr], ptr %44, i64 0, i64 %45
  %47 = load ptr, ptr %46, align 8, !tbaa !9
  %48 = icmp eq ptr %47, null
  br i1 %48, label %51, label %49

49:                                               ; preds = %41
  %50 = load ptr, ptr %47, align 8, !tbaa !119
  store ptr %50, ptr %46, align 8, !tbaa !9
  br label %73

51:                                               ; preds = %41
  %52 = tail call align 16 ptr @llvm.threadlocal.address.p0(ptr align 16 @class_next)
  %53 = getelementptr inbounds [40 x ptr], ptr %52, i64 0, i64 %45
  %54 = load ptr, ptr %53, align 8, !tbaa !9
  %55 = getelementptr inbounds i8, ptr %54, i64 %42
  %56 = tail call align 16 ptr @llvm.threadlocal.address.p0(ptr align 16 @class_end)
  %57 = getelementptr inbounds [40 x ptr], ptr %56, i64 0, i64 %45
  %58 = load ptr, ptr %57, align 8, !tbaa !9
  %59 = icmp ugt ptr %55, %58
  %60 = icmp eq ptr %54, null
  %61 = or i1 %60, %59
  br i1 %61, label %62, label %70

62:                                               ; preds = %51
  %63 = tail call fastcc ptr @take_pages(i64 noundef 1) #35
  %64 = icmp eq ptr %63, null
  br i1 %64, label %73, label %65

65:                                               ; preds = %62
  store i32 %43, ptr %63, align 4, !tbaa !115
  %66 = getelementptr inbounds %struct.PageHeader, ptr %63, i64 0, i32 1
  store i32 1, ptr %66, align 4, !tbaa !117
  %67 = getelementptr inbounds i8, ptr %63, i64 16
  store ptr %67, ptr %53, align 8, !tbaa !9
  %68 = getelementptr inbounds i8, ptr %63, i64 65536
  store ptr %68, ptr %57, align 8, !tbaa !9
  %69 = load ptr, ptr %53, align 8, !tbaa !9
  br label %70

70:                                               ; preds = %65, %51
  %71 = phi ptr [ %69, %65 ], [ %54, %51 ]
  %72 = getelementptr inbounds i8, ptr %71, i64 %42
  store ptr %72, ptr %53, align 8, !tbaa !9
  br label %73

73:                                               ; preds = %49, %70, %62, %3, %7, %10
  %74 = phi ptr [ null, %3 ], [ %13, %10 ], [ null, %7 ], [ %47, %49 ], [ %71, %70 ], [ null, %62 ]
  ret ptr %74
}

; Function Attrs: nounwind optsize
define internal fastcc ptr @take_pages(i64 noundef %0) unnamed_addr #0 {
  %2 = atomicrmw xchg ptr @spans_lock, i32 1 acquire, align 4
  %3 = icmp eq i32 %2, 0
  br i1 %3, label %4, label %8

4:                                                ; preds = %5, %1
  br label %14

5:                                                ; preds = %11, %8
  %6 = atomicrmw xchg ptr @spans_lock, i32 1 acquire, align 4
  %7 = icmp eq i32 %6, 0
  br i1 %7, label %4, label %8, !llvm.loop !2

8:                                                ; preds = %1, %5
  %9 = load atomic i32, ptr @spans_lock monotonic, align 4
  %10 = icmp eq i32 %9, 0
  br i1 %10, label %5, label %11

11:                                               ; preds = %8, %11
  tail call void @llvm.x86.sse2.pause()
  %12 = load atomic i32, ptr @spans_lock monotonic, align 4
  %13 = icmp eq i32 %12, 0
  br i1 %13, label %5, label %11, !llvm.loop !4

14:                                               ; preds = %4, %18
  %15 = phi ptr [ %16, %18 ], [ @free_spans, %4 ]
  %16 = load ptr, ptr %15, align 8, !tbaa !9
  %17 = icmp eq ptr %16, null
  br i1 %17, label %34, label %18

18:                                               ; preds = %14
  %19 = getelementptr inbounds %struct.FreeSpan, ptr %16, i64 0, i32 1
  %20 = load i64, ptr %19, align 8, !tbaa !121
  %21 = icmp ult i64 %20, %0
  br i1 %21, label %14, label %22

22:                                               ; preds = %18
  %23 = icmp eq i64 %20, %0
  br i1 %23, label %24, label %26

24:                                               ; preds = %22
  %25 = load ptr, ptr %16, align 8, !tbaa !123
  br label %32

26:                                               ; preds = %22
  %27 = shl i64 %0, 16
  %28 = getelementptr inbounds i8, ptr %16, i64 %27
  %29 = load ptr, ptr %16, align 8, !tbaa !123
  store ptr %29, ptr %28, align 8, !tbaa !123
  %30 = sub i64 %20, %0
  %31 = getelementptr inbounds %struct.FreeSpan, ptr %28, i64 0, i32 1
  store i64 %30, ptr %31, align 8, !tbaa !121
  br label %32

32:                                               ; preds = %26, %24
  %33 = phi ptr [ %28, %26 ], [ %25, %24 ]
  store ptr %33, ptr %15, align 8, !tbaa !9
  br label %52

34:                                               ; preds = %14
  %35 = tail call i64 @llvm.umax.i64(i64 %0, i64 16)
  %36 = tail call ptr @fa_pages(i64 noundef %35) #34
  %37 = icmp eq ptr %36, null
  br i1 %37, label %52, label %38

38:                                               ; preds = %34
  %39 = icmp ult i64 %0, 16
  br i1 %39, label %40, label %52

40:                                               ; preds = %38
  %41 = shl nuw nsw i64 %0, 16
  %42 = getelementptr inbounds i8, ptr %36, i64 %41
  %43 = sub nuw nsw i64 %35, %0
  %44 = getelementptr inbounds %struct.FreeSpan, ptr %42, i64 0, i32 1
  store i64 %43, ptr %44, align 8, !tbaa !121
  store ptr null, ptr %42, align 8, !tbaa !123
  br label %45

45:                                               ; preds = %45, %40
  %46 = phi ptr [ @free_spans, %40 ], [ %47, %45 ]
  %47 = load ptr, ptr %46, align 8, !tbaa !9
  %48 = icmp ne ptr %47, null
  %49 = icmp ult ptr %47, %42
  %50 = and i1 %48, %49
  br i1 %50, label %45, label %51, !llvm.loop !124

51:                                               ; preds = %45
  store ptr %47, ptr %42, align 8, !tbaa !123
  store ptr %42, ptr %46, align 8, !tbaa !9
  br label %52

52:                                               ; preds = %32, %34, %38, %51
  %53 = phi ptr [ null, %34 ], [ %36, %51 ], [ %36, %38 ], [ %16, %32 ]
  store atomic i32 0, ptr @spans_lock release, align 4
  ret ptr %53
}

; Function Attrs: nounwind optsize
define void @fa_heap_free(ptr noundef %0) local_unnamed_addr #0 {
  %2 = ptrtoint ptr %0 to i64
  %3 = and i64 %2, -65536
  %4 = inttoptr i64 %3 to ptr
  %5 = load i32, ptr %4, align 65536, !tbaa !115
  %6 = icmp eq i32 %5, 65535
  br i1 %6, label %7, label %11

7:                                                ; preds = %1
  %8 = getelementptr inbounds %struct.PageHeader, ptr %4, i64 0, i32 1
  %9 = load i32, ptr %8, align 4, !tbaa !117
  %10 = zext i32 %9 to i64
  tail call fastcc void @give_pages(ptr noundef nonnull %4, i64 noundef %10) #35
  br label %16

11:                                               ; preds = %1
  %12 = tail call align 16 ptr @llvm.threadlocal.address.p0(ptr align 16 @free_blocks)
  %13 = zext i32 %5 to i64
  %14 = getelementptr inbounds [40 x ptr], ptr %12, i64 0, i64 %13
  %15 = load ptr, ptr %14, align 8, !tbaa !9
  store ptr %15, ptr %0, align 8, !tbaa !119
  store ptr %0, ptr %14, align 8, !tbaa !9
  br label %16

16:                                               ; preds = %11, %7
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @give_pages(ptr noundef %0, i64 noundef %1) unnamed_addr #0 {
  %3 = atomicrmw xchg ptr @spans_lock, i32 1 acquire, align 4
  %4 = icmp eq i32 %3, 0
  br i1 %4, label %14, label %8

5:                                                ; preds = %11, %8
  %6 = atomicrmw xchg ptr @spans_lock, i32 1 acquire, align 4
  %7 = icmp eq i32 %6, 0
  br i1 %7, label %14, label %8, !llvm.loop !2

8:                                                ; preds = %2, %5
  %9 = load atomic i32, ptr @spans_lock monotonic, align 4
  %10 = icmp eq i32 %9, 0
  br i1 %10, label %5, label %11

11:                                               ; preds = %8, %11
  tail call void @llvm.x86.sse2.pause()
  %12 = load atomic i32, ptr @spans_lock monotonic, align 4
  %13 = icmp eq i32 %12, 0
  br i1 %13, label %5, label %11, !llvm.loop !4

14:                                               ; preds = %5, %2
  %15 = getelementptr inbounds %struct.FreeSpan, ptr %0, i64 0, i32 1
  store i64 %1, ptr %15, align 8, !tbaa !121
  br label %16

16:                                               ; preds = %16, %14
  %17 = phi ptr [ null, %14 ], [ %19, %16 ]
  %18 = phi ptr [ @free_spans, %14 ], [ %19, %16 ]
  %19 = load ptr, ptr %18, align 8, !tbaa !9
  %20 = icmp ne ptr %19, null
  %21 = icmp ult ptr %19, %0
  %22 = and i1 %20, %21
  br i1 %22, label %16, label %23, !llvm.loop !125

23:                                               ; preds = %16
  store ptr %19, ptr %0, align 8, !tbaa !123
  %24 = shl i64 %1, 16
  %25 = getelementptr inbounds i8, ptr %0, i64 %24
  %26 = icmp eq ptr %25, %19
  %27 = select i1 %20, i1 %26, i1 false
  br i1 %27, label %28, label %33

28:                                               ; preds = %23
  %29 = getelementptr inbounds %struct.FreeSpan, ptr %19, i64 0, i32 1
  %30 = load i64, ptr %29, align 8, !tbaa !121
  %31 = add i64 %30, %1
  store i64 %31, ptr %15, align 8, !tbaa !121
  %32 = load ptr, ptr %19, align 8, !tbaa !123
  store ptr %32, ptr %0, align 8, !tbaa !123
  br label %33

33:                                               ; preds = %28, %23
  %34 = phi ptr [ %32, %28 ], [ %19, %23 ]
  %35 = phi i64 [ %31, %28 ], [ %1, %23 ]
  %36 = icmp eq ptr %17, null
  br i1 %36, label %46, label %37

37:                                               ; preds = %33
  %38 = getelementptr inbounds %struct.FreeSpan, ptr %17, i64 0, i32 1
  %39 = load i64, ptr %38, align 8, !tbaa !121
  %40 = shl i64 %39, 16
  %41 = getelementptr inbounds i8, ptr %17, i64 %40
  %42 = icmp eq ptr %41, %0
  br i1 %42, label %43, label %45

43:                                               ; preds = %37
  %44 = add i64 %39, %35
  store i64 %44, ptr %38, align 8, !tbaa !121
  store ptr %34, ptr %17, align 8, !tbaa !123
  br label %47

45:                                               ; preds = %37
  store ptr %0, ptr %17, align 8, !tbaa !123
  br label %47

46:                                               ; preds = %33
  store ptr %0, ptr @free_spans, align 8, !tbaa !9
  br label %47

47:                                               ; preds = %43, %45, %46
  store atomic i32 0, ptr @spans_lock release, align 4
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_release_builder() local_unnamed_addr #0 {
  %1 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %2 = load ptr, ptr %1, align 8, !tbaa !9
  %3 = icmp eq ptr %2, null
  br i1 %3, label %8, label %4

4:                                                ; preds = %0
  %5 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_capacity)
  %6 = load i64, ptr %5, align 8, !tbaa !5
  tail call void @fa_free(ptr noundef nonnull %2, i64 noundef %6, i64 noundef 1) #34
  store ptr null, ptr %1, align 8, !tbaa !9
  store i64 0, ptr %5, align 8, !tbaa !5
  %7 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %7, align 8, !tbaa !5
  br label %8

8:                                                ; preds = %4, %0
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_leak_check() local_unnamed_addr #0 {
  tail call void @fa_flush() #35
  %1 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %2 = load ptr, ptr %1, align 8, !tbaa !9
  %3 = icmp eq ptr %2, null
  br i1 %3, label %7, label %4

4:                                                ; preds = %0
  %5 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_capacity)
  %6 = load i64, ptr %5, align 8, !tbaa !5
  tail call void @fa_free(ptr noundef nonnull %2, i64 noundef %6, i64 noundef 1) #34
  store ptr null, ptr %1, align 8, !tbaa !9
  store i64 0, ptr %5, align 8, !tbaa !5
  br label %7

7:                                                ; preds = %4, %0
  %8 = load ptr, ptr @argument_items, align 8, !tbaa !9
  %9 = icmp eq ptr %8, null
  br i1 %9, label %14, label %10

10:                                               ; preds = %7
  %11 = load i64, ptr @argument_count, align 8, !tbaa !5
  %12 = mul i64 %11, 24
  %13 = add i64 %12, 24
  tail call void @fa_free(ptr noundef nonnull %8, i64 noundef %13, i64 noundef 8) #34
  store ptr null, ptr @argument_items, align 8, !tbaa !9
  br label %14

14:                                               ; preds = %10, %7
  %15 = load i64, ptr @fa_live_blocks, align 8, !tbaa !5
  %16 = icmp eq i64 %15, 0
  br i1 %16, label %20, label %17

17:                                               ; preds = %14
  %18 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %18, align 8, !tbaa !5
  tail call fastcc void @append_c(ptr noundef nonnull @.str.13) #35
  %19 = load i64, ptr @fa_live_blocks, align 8, !tbaa !5
  tail call void @fa_b_u64(i64 noundef %19) #35
  tail call fastcc void @append_c(ptr noundef nonnull @.str.14) #35
  tail call void @fa_b_line(i32 noundef 2) #35
  tail call void @fa_exit(i32 noundef 102) #36
  unreachable

20:                                               ; preds = %14
  ret void
}

; Function Attrs: nounwind optsize
define noundef ptr @fa_pool_new(i64 noundef %0) local_unnamed_addr #0 {
  %2 = tail call ptr @fa_alloc(i64 noundef 56, i64 noundef 8) #34
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(56) %2, i8 0, i64 40, i1 false)
  %3 = tail call i64 @llvm.smax.i64(i64 %0, i64 4)
  %4 = trunc i64 %3 to i32
  %5 = getelementptr inbounds %struct.FaPool, ptr %2, i64 0, i32 6
  store i32 %4, ptr %5, align 8, !tbaa !126
  %6 = shl i64 %3, 6
  %7 = add i64 %6, 104
  br label %8

8:                                                ; preds = %8, %1
  %9 = phi i64 [ 1, %1 ], [ %12, %8 ]
  %10 = shl i64 %9, 16
  %11 = icmp ult i64 %10, %7
  %12 = shl i64 %9, 1
  br i1 %11, label %8, label %13, !llvm.loop !128

13:                                               ; preds = %8
  %14 = shl i64 %9, 19
  %15 = add i64 %14, -320
  %16 = shl i64 %3, 3
  %17 = or disjoint i64 %16, 1
  %18 = udiv i64 %15, %17
  br label %19

19:                                               ; preds = %19, %13
  %20 = phi i64 [ %18, %13 ], [ %29, %19 ]
  %21 = add i64 %20, 63
  %22 = lshr i64 %21, 3
  %23 = and i64 %22, 2305843009213693944
  %24 = add nuw nsw i64 %23, 39
  %25 = and i64 %24, 4611686018427387888
  %26 = mul i64 %20, %3
  %27 = add i64 %25, %26
  %28 = icmp ugt i64 %27, %10
  %29 = add i64 %20, -1
  br i1 %28, label %19, label %30

30:                                               ; preds = %19
  %31 = trunc i64 %20 to i32
  %32 = getelementptr inbounds %struct.FaPool, ptr %2, i64 0, i32 7
  store i32 %31, ptr %32, align 4, !tbaa !129
  %33 = trunc i64 %9 to i32
  %34 = getelementptr inbounds %struct.FaPool, ptr %2, i64 0, i32 8
  store i32 %33, ptr %34, align 8, !tbaa !130
  %35 = trunc i64 %25 to i32
  %36 = getelementptr inbounds %struct.FaPool, ptr %2, i64 0, i32 9
  store i32 %35, ptr %36, align 4, !tbaa !131
  ret ptr %2
}

; Function Attrs: nounwind optsize
define noundef ptr @fa_columnar_new(i64 noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #0 {
  %4 = tail call ptr @fa_alloc(i64 noundef 56, i64 noundef 8) #34
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(56) %4, i8 0, i64 40, i1 false)
  %5 = getelementptr inbounds %struct.FaPool, ptr %4, i64 0, i32 6
  store i32 4, ptr %5, align 8, !tbaa !126
  %6 = trunc i64 %0 to i32
  %7 = getelementptr inbounds %struct.FaPool, ptr %4, i64 0, i32 7
  store i32 %6, ptr %7, align 4, !tbaa !129
  %8 = trunc i64 %1 to i32
  %9 = getelementptr inbounds %struct.FaPool, ptr %4, i64 0, i32 8
  store i32 %8, ptr %9, align 8, !tbaa !130
  %10 = trunc i64 %2 to i32
  %11 = getelementptr inbounds %struct.FaPool, ptr %4, i64 0, i32 9
  store i32 %10, ptr %11, align 4, !tbaa !131
  ret ptr %4
}

; Function Attrs: nounwind optsize
define ptr @fa_pool_add_slot(ptr nocapture noundef %0) local_unnamed_addr #0 {
  %2 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 4
  %3 = load i32, ptr %2, align 8, !tbaa !132
  %4 = add i32 %3, 63
  %5 = icmp ult i32 %4, 64
  br i1 %5, label %29, label %6

6:                                                ; preds = %1
  %7 = lshr i32 %4, 6
  %8 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 2
  %9 = load ptr, ptr %8, align 8, !tbaa !133
  %10 = zext nneg i32 %7 to i64
  br label %11

11:                                               ; preds = %6, %16
  %12 = phi i64 [ 0, %6 ], [ %17, %16 ]
  %13 = getelementptr inbounds i64, ptr %9, i64 %12
  %14 = load i64, ptr %13, align 8, !tbaa !5
  %15 = icmp eq i64 %14, 0
  br i1 %15, label %16, label %19

16:                                               ; preds = %11
  %17 = add nuw nsw i64 %12, 1
  %18 = icmp eq i64 %17, %10
  br i1 %18, label %29, label %11, !llvm.loop !134

19:                                               ; preds = %11
  %20 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 1
  %21 = load ptr, ptr %20, align 8, !tbaa !135
  %22 = shl i64 %12, 6
  %23 = tail call i64 @llvm.cttz.i64(i64 %14, i1 true), !range !108
  %24 = and i64 %22, 4294967232
  %25 = or disjoint i64 %24, %23
  %26 = getelementptr inbounds ptr, ptr %21, i64 %25
  %27 = load ptr, ptr %26, align 8, !tbaa !9
  %28 = icmp eq ptr %27, null
  br i1 %28, label %29, label %31

29:                                               ; preds = %16, %1, %19
  %30 = tail call fastcc ptr @pool_new_chunk(ptr noundef nonnull %0) #38
  br label %31

31:                                               ; preds = %29, %19
  %32 = phi ptr [ %30, %29 ], [ %27, %19 ]
  %33 = getelementptr inbounds %struct.PoolChunk, ptr %32, i64 0, i32 4
  %34 = load i32, ptr %33, align 4, !tbaa !20
  %35 = icmp eq i32 %34, 0
  br i1 %35, label %47, label %36

36:                                               ; preds = %31
  %37 = add i32 %34, -1
  %38 = load ptr, ptr %32, align 8, !tbaa !9
  %39 = zext i32 %37 to i64
  %40 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 6
  %41 = load i32, ptr %40, align 8, !tbaa !126
  %42 = zext i32 %41 to i64
  %43 = mul nuw i64 %42, %39
  %44 = getelementptr inbounds i8, ptr %38, i64 %43
  %45 = load i32, ptr %44, align 1
  store i32 %45, ptr %33, align 4, !tbaa !20
  %46 = icmp eq i32 %45, 0
  br label %51

47:                                               ; preds = %31
  %48 = getelementptr inbounds %struct.PoolChunk, ptr %32, i64 0, i32 3
  %49 = load i32, ptr %48, align 8, !tbaa !20
  %50 = add i32 %49, 1
  store i32 %50, ptr %48, align 8, !tbaa !20
  br label %51

51:                                               ; preds = %47, %36
  %52 = phi i1 [ %46, %36 ], [ true, %47 ]
  %53 = phi i32 [ %37, %36 ], [ %49, %47 ]
  %54 = and i32 %53, 63
  %55 = zext nneg i32 %54 to i64
  %56 = shl nuw i64 1, %55
  %57 = lshr i32 %53, 6
  %58 = zext nneg i32 %57 to i64
  %59 = getelementptr inbounds %struct.PoolChunk, ptr %32, i64 0, i32 5, i64 %58
  %60 = load i64, ptr %59, align 8, !tbaa !5
  %61 = or i64 %56, %60
  store i64 %61, ptr %59, align 8, !tbaa !5
  %62 = getelementptr inbounds %struct.PoolChunk, ptr %32, i64 0, i32 2
  %63 = load i32, ptr %62, align 4, !tbaa !20
  %64 = add i32 %63, 1
  store i32 %64, ptr %62, align 4, !tbaa !20
  %65 = load i64, ptr %0, align 8, !tbaa !136
  %66 = add nsw i64 %65, 1
  store i64 %66, ptr %0, align 8, !tbaa !136
  br i1 %52, label %67, label %87

67:                                               ; preds = %51
  %68 = getelementptr inbounds %struct.PoolChunk, ptr %32, i64 0, i32 3
  %69 = load i32, ptr %68, align 8, !tbaa !20
  %70 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 7
  %71 = load i32, ptr %70, align 4, !tbaa !129
  %72 = icmp eq i32 %69, %71
  br i1 %72, label %73, label %87

73:                                               ; preds = %67
  %74 = getelementptr inbounds %struct.PoolChunk, ptr %32, i64 0, i32 1
  %75 = load i32, ptr %74, align 8, !tbaa !20
  %76 = and i32 %75, 63
  %77 = zext nneg i32 %76 to i64
  %78 = shl nuw i64 1, %77
  %79 = xor i64 %78, -1
  %80 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 2
  %81 = load ptr, ptr %80, align 8, !tbaa !133
  %82 = lshr i32 %75, 6
  %83 = zext nneg i32 %82 to i64
  %84 = getelementptr inbounds i64, ptr %81, i64 %83
  %85 = load i64, ptr %84, align 8, !tbaa !5
  %86 = and i64 %85, %79
  store i64 %86, ptr %84, align 8, !tbaa !5
  br label %87

87:                                               ; preds = %73, %67, %51
  %88 = load ptr, ptr %32, align 8, !tbaa !9
  %89 = zext i32 %53 to i64
  %90 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 6
  %91 = load i32, ptr %90, align 8, !tbaa !126
  %92 = zext i32 %91 to i64
  %93 = mul nuw i64 %92, %89
  %94 = getelementptr inbounds i8, ptr %88, i64 %93
  ret ptr %94
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.cttz.i64(i64, i1 immarg) #5

; Function Attrs: cold noinline nounwind optsize
define internal fastcc noundef ptr @pool_new_chunk(ptr nocapture noundef %0) unnamed_addr #12 {
  %2 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 3
  %3 = load ptr, ptr %2, align 8, !tbaa !137
  store ptr null, ptr %2, align 8, !tbaa !137
  %4 = icmp eq ptr %3, null
  br i1 %4, label %5, label %41

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 8
  %7 = load i32, ptr %6, align 8, !tbaa !130
  %8 = zext i32 %7 to i64
  %9 = icmp eq i32 %7, 1
  br i1 %9, label %10, label %12

10:                                               ; preds = %5
  %11 = tail call fastcc ptr @take_pages(i64 noundef 1) #35
  br label %35

12:                                               ; preds = %5
  %13 = shl nuw nsw i64 %8, 1
  %14 = add nsw i64 %13, -1
  %15 = tail call fastcc ptr @take_pages(i64 noundef %14) #35
  %16 = icmp eq ptr %15, null
  br i1 %16, label %38, label %17

17:                                               ; preds = %12
  %18 = shl nuw nsw i64 %8, 16
  %19 = ptrtoint ptr %15 to i64
  %20 = add nsw i64 %18, -1
  %21 = add i64 %20, %19
  %22 = sub nsw i64 0, %18
  %23 = and i64 %21, %22
  %24 = inttoptr i64 %23 to ptr
  %25 = sub i64 %23, %19
  %26 = lshr i64 %25, 16
  %27 = icmp ult i64 %25, 65536
  br i1 %27, label %29, label %28

28:                                               ; preds = %17
  tail call fastcc void @give_pages(ptr noundef nonnull %15, i64 noundef %26) #35
  br label %29

29:                                               ; preds = %28, %17
  %30 = xor i64 %26, -1
  %31 = add nsw i64 %30, %8
  %32 = icmp eq i64 %31, 0
  br i1 %32, label %35, label %33

33:                                               ; preds = %29
  %34 = getelementptr inbounds i8, ptr %24, i64 %18
  tail call fastcc void @give_pages(ptr noundef %34, i64 noundef %31) #35
  br label %35

35:                                               ; preds = %10, %29, %33
  %36 = phi ptr [ %11, %10 ], [ %24, %33 ], [ %24, %29 ]
  %37 = icmp eq ptr %36, null
  br i1 %37, label %38, label %39

38:                                               ; preds = %12, %35
  tail call fastcc void @out_of_memory() #37
  unreachable

39:                                               ; preds = %35
  %40 = atomicrmw add ptr @fa_live_blocks, i64 1 monotonic, align 8
  br label %41

41:                                               ; preds = %39, %1
  %42 = phi ptr [ %36, %39 ], [ %3, %1 ]
  %43 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 7
  %44 = load i32, ptr %43, align 4, !tbaa !129
  %45 = add i32 %44, 63
  %46 = lshr i32 %45, 3
  %47 = and i32 %46, 536870904
  %48 = add nuw nsw i32 %47, 24
  %49 = zext nneg i32 %48 to i64
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(1) %42, i8 0, i64 %49, i1 false)
  %50 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 9
  %51 = load i32, ptr %50, align 4, !tbaa !131
  %52 = zext i32 %51 to i64
  %53 = getelementptr inbounds i8, ptr %42, i64 %52
  store ptr %53, ptr %42, align 8, !tbaa !9
  %54 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 4
  %55 = load i32, ptr %54, align 8, !tbaa !132
  %56 = icmp eq i32 %55, 0
  br i1 %56, label %71, label %57

57:                                               ; preds = %41
  %58 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 1
  %59 = load ptr, ptr %58, align 8, !tbaa !135
  %60 = zext i32 %55 to i64
  br label %61

61:                                               ; preds = %57, %66
  %62 = phi i64 [ 0, %57 ], [ %67, %66 ]
  %63 = getelementptr inbounds ptr, ptr %59, i64 %62
  %64 = load ptr, ptr %63, align 8, !tbaa !9
  %65 = icmp eq ptr %64, null
  br i1 %65, label %69, label %66

66:                                               ; preds = %61
  %67 = add nuw nsw i64 %62, 1
  %68 = icmp eq i64 %67, %60
  br i1 %68, label %74, label %61, !llvm.loop !138

69:                                               ; preds = %61
  %70 = trunc i64 %62 to i32
  br label %71

71:                                               ; preds = %69, %41
  %72 = phi i32 [ 0, %41 ], [ %70, %69 ]
  %73 = icmp eq i32 %72, %55
  br i1 %73, label %74, label %122

74:                                               ; preds = %66, %71
  %75 = phi i32 [ %72, %71 ], [ %55, %66 ]
  %76 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 5
  %77 = load i32, ptr %76, align 4, !tbaa !139
  %78 = icmp eq i32 %55, %77
  br i1 %78, label %79, label %119

79:                                               ; preds = %74
  %80 = shl i32 %55, 1
  %81 = select i1 %56, i32 4, i32 %80
  %82 = zext i32 %81 to i64
  %83 = shl nuw nsw i64 %82, 3
  %84 = tail call ptr @fa_alloc(i64 noundef %83, i64 noundef 8) #34
  %85 = add i32 %81, 63
  %86 = lshr i32 %85, 3
  %87 = and i32 %86, 536870904
  %88 = zext nneg i32 %87 to i64
  %89 = tail call ptr @fa_alloc(i64 noundef %88, i64 noundef 8) #34
  tail call void @llvm.memset.p0.i64(ptr align 8 %84, i8 0, i64 %83, i1 false)
  tail call void @llvm.memset.p0.i64(ptr align 8 %89, i8 0, i64 %88, i1 false)
  %90 = load i32, ptr %76, align 4, !tbaa !139
  %91 = icmp eq i32 %90, 0
  br i1 %91, label %115, label %92

92:                                               ; preds = %79
  %93 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 1
  %94 = load ptr, ptr %93, align 8, !tbaa !135
  %95 = load i32, ptr %54, align 8, !tbaa !132
  %96 = zext i32 %95 to i64
  %97 = shl nuw nsw i64 %96, 3
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %84, ptr align 1 %94, i64 %97, i1 false)
  %98 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 2
  %99 = load ptr, ptr %98, align 8, !tbaa !133
  %100 = load i32, ptr %76, align 4, !tbaa !139
  %101 = add i32 %100, 63
  %102 = lshr i32 %101, 3
  %103 = and i32 %102, 536870904
  %104 = zext nneg i32 %103 to i64
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %89, ptr align 1 %99, i64 %104, i1 false)
  %105 = load ptr, ptr %93, align 8, !tbaa !135
  %106 = load i32, ptr %76, align 4, !tbaa !139
  %107 = zext i32 %106 to i64
  %108 = shl nuw nsw i64 %107, 3
  tail call void @fa_free(ptr noundef %105, i64 noundef %108, i64 noundef 8) #34
  %109 = load ptr, ptr %98, align 8, !tbaa !133
  %110 = load i32, ptr %76, align 4, !tbaa !139
  %111 = add i32 %110, 63
  %112 = lshr i32 %111, 3
  %113 = and i32 %112, 536870904
  %114 = zext nneg i32 %113 to i64
  tail call void @fa_free(ptr noundef %109, i64 noundef %114, i64 noundef 8) #34
  br label %115

115:                                              ; preds = %92, %79
  %116 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 1
  store ptr %84, ptr %116, align 8, !tbaa !135
  %117 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 2
  store ptr %89, ptr %117, align 8, !tbaa !133
  store i32 %81, ptr %76, align 4, !tbaa !139
  %118 = load i32, ptr %54, align 8, !tbaa !132
  br label %119

119:                                              ; preds = %115, %74
  %120 = phi i32 [ %118, %115 ], [ %55, %74 ]
  %121 = add i32 %120, 1
  store i32 %121, ptr %54, align 8, !tbaa !132
  br label %122

122:                                              ; preds = %119, %71
  %123 = phi i32 [ %75, %119 ], [ %72, %71 ]
  %124 = getelementptr inbounds %struct.PoolChunk, ptr %42, i64 0, i32 1
  store i32 %123, ptr %124, align 8, !tbaa !20
  %125 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 1
  %126 = load ptr, ptr %125, align 8, !tbaa !135
  %127 = zext i32 %123 to i64
  %128 = getelementptr inbounds ptr, ptr %126, i64 %127
  store ptr %42, ptr %128, align 8, !tbaa !9
  %129 = and i32 %123, 63
  %130 = zext nneg i32 %129 to i64
  %131 = shl nuw i64 1, %130
  %132 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 2
  %133 = load ptr, ptr %132, align 8, !tbaa !133
  %134 = lshr i32 %123, 6
  %135 = zext nneg i32 %134 to i64
  %136 = getelementptr inbounds i64, ptr %133, i64 %135
  %137 = load i64, ptr %136, align 8, !tbaa !5
  %138 = or i64 %137, %131
  store i64 %138, ptr %136, align 8, !tbaa !5
  ret ptr %42
}

; Function Attrs: nounwind optsize
define void @fa_pool_remove(ptr nocapture noundef %0, ptr noundef %1) local_unnamed_addr #0 {
  %3 = ptrtoint ptr %1 to i64
  %4 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 8
  %5 = load i32, ptr %4, align 8, !tbaa !130
  %6 = zext i32 %5 to i64
  %7 = mul nsw i64 %6, -65536
  %8 = and i64 %7, %3
  %9 = inttoptr i64 %8 to ptr
  %10 = load ptr, ptr %9, align 65536, !tbaa !9
  %11 = ptrtoint ptr %10 to i64
  %12 = sub i64 %3, %11
  %13 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 6
  %14 = load i32, ptr %13, align 8, !tbaa !126
  %15 = zext i32 %14 to i64
  %16 = sdiv i64 %12, %15
  %17 = trunc i64 %16 to i32
  %18 = and i64 %16, 63
  %19 = shl nuw i64 1, %18
  %20 = xor i64 %19, -1
  %21 = lshr i64 %16, 6
  %22 = and i64 %21, 67108863
  %23 = getelementptr inbounds %struct.PoolChunk, ptr %9, i64 0, i32 5, i64 %22
  %24 = load i64, ptr %23, align 8, !tbaa !5
  %25 = and i64 %24, %20
  store i64 %25, ptr %23, align 8, !tbaa !5
  %26 = getelementptr inbounds %struct.PoolChunk, ptr %9, i64 0, i32 4
  %27 = load i32, ptr %26, align 4
  store i32 %27, ptr %1, align 1
  %28 = add i32 %17, 1
  store i32 %28, ptr %26, align 4, !tbaa !20
  %29 = getelementptr inbounds %struct.PoolChunk, ptr %9, i64 0, i32 2
  %30 = load i32, ptr %29, align 4, !tbaa !20
  %31 = add i32 %30, -1
  store i32 %31, ptr %29, align 4, !tbaa !20
  %32 = load i64, ptr %0, align 8, !tbaa !136
  %33 = add nsw i64 %32, -1
  store i64 %33, ptr %0, align 8, !tbaa !136
  %34 = getelementptr inbounds %struct.PoolChunk, ptr %9, i64 0, i32 1
  %35 = load i32, ptr %34, align 8, !tbaa !20
  %36 = icmp eq i32 %31, 0
  br i1 %36, label %37, label %78

37:                                               ; preds = %2
  %38 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 1
  %39 = load ptr, ptr %38, align 8, !tbaa !135
  %40 = zext i32 %35 to i64
  %41 = getelementptr inbounds ptr, ptr %39, i64 %40
  store ptr null, ptr %41, align 8, !tbaa !9
  %42 = and i32 %35, 63
  %43 = zext nneg i32 %42 to i64
  %44 = shl nuw i64 1, %43
  %45 = xor i64 %44, -1
  %46 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 2
  %47 = load ptr, ptr %46, align 8, !tbaa !133
  %48 = lshr i32 %35, 6
  %49 = zext nneg i32 %48 to i64
  %50 = getelementptr inbounds i64, ptr %47, i64 %49
  %51 = load i64, ptr %50, align 8, !tbaa !5
  %52 = and i64 %51, %45
  store i64 %52, ptr %50, align 8, !tbaa !5
  %53 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 4
  %54 = load i32, ptr %53, align 8, !tbaa !132
  %55 = icmp eq i32 %54, 0
  br i1 %55, label %69, label %56

56:                                               ; preds = %37
  %57 = load ptr, ptr %38, align 8, !tbaa !135
  %58 = zext i32 %54 to i64
  br label %59

59:                                               ; preds = %56, %66
  %60 = phi i64 [ %58, %56 ], [ %61, %66 ]
  %61 = add nsw i64 %60, -1
  %62 = and i64 %61, 4294967295
  %63 = getelementptr inbounds ptr, ptr %57, i64 %62
  %64 = load ptr, ptr %63, align 8, !tbaa !9
  %65 = icmp eq ptr %64, null
  br i1 %65, label %66, label %69

66:                                               ; preds = %59
  %67 = trunc i64 %61 to i32
  store i32 %67, ptr %53, align 8, !tbaa !132
  %68 = icmp eq i32 %67, 0
  br i1 %68, label %69, label %59, !llvm.loop !140

69:                                               ; preds = %59, %66, %37
  %70 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 3
  %71 = load ptr, ptr %70, align 8, !tbaa !137
  %72 = icmp eq ptr %71, null
  br i1 %72, label %73, label %74

73:                                               ; preds = %69
  store ptr %9, ptr %70, align 8, !tbaa !137
  br label %89

74:                                               ; preds = %69
  %75 = load i32, ptr %4, align 8, !tbaa !130
  %76 = zext i32 %75 to i64
  tail call fastcc void @give_pages(ptr noundef nonnull %9, i64 noundef %76) #35
  %77 = atomicrmw sub ptr @fa_live_blocks, i64 1 monotonic, align 8
  br label %89

78:                                               ; preds = %2
  %79 = and i32 %35, 63
  %80 = zext nneg i32 %79 to i64
  %81 = shl nuw i64 1, %80
  %82 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 2
  %83 = load ptr, ptr %82, align 8, !tbaa !133
  %84 = lshr i32 %35, 6
  %85 = zext nneg i32 %84 to i64
  %86 = getelementptr inbounds i64, ptr %83, i64 %85
  %87 = load i64, ptr %86, align 8, !tbaa !5
  %88 = or i64 %87, %81
  store i64 %88, ptr %86, align 8, !tbaa !5
  br label %89

89:                                               ; preds = %73, %74, %78
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(read, inaccessiblemem: none)
define i64 @fa_pool_seek(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #16 {
  %3 = lshr i64 %1, 32
  %4 = trunc i64 %3 to i32
  %5 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 4
  %6 = load i32, ptr %5, align 8, !tbaa !132
  %7 = icmp ugt i32 %6, %4
  br i1 %7, label %8, label %49

8:                                                ; preds = %2
  %9 = trunc i64 %1 to i32
  %10 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 1
  %11 = load ptr, ptr %10, align 8, !tbaa !135
  %12 = zext i32 %6 to i64
  br label %13

13:                                               ; preds = %8, %46
  %14 = phi i64 [ %3, %8 ], [ %47, %46 ]
  %15 = phi i32 [ %9, %8 ], [ 0, %46 ]
  %16 = getelementptr inbounds ptr, ptr %11, i64 %14
  %17 = load ptr, ptr %16, align 8, !tbaa !9
  %18 = icmp eq ptr %17, null
  br i1 %18, label %46, label %19

19:                                               ; preds = %13
  %20 = getelementptr inbounds %struct.PoolChunk, ptr %17, i64 0, i32 3
  %21 = load i32, ptr %20, align 8, !tbaa !20
  %22 = icmp ult i32 %15, %21
  br i1 %22, label %23, label %46

23:                                               ; preds = %19, %38
  %24 = phi i32 [ %40, %38 ], [ %15, %19 ]
  %25 = lshr i32 %24, 6
  %26 = zext nneg i32 %25 to i64
  %27 = getelementptr inbounds %struct.PoolChunk, ptr %17, i64 0, i32 5, i64 %26
  %28 = load i64, ptr %27, align 8, !tbaa !5
  %29 = and i32 %24, 63
  %30 = zext nneg i32 %29 to i64
  %31 = lshr i64 %28, %30
  %32 = icmp eq i64 %31, 0
  br i1 %32, label %38, label %33

33:                                               ; preds = %23
  %34 = tail call i64 @llvm.cttz.i64(i64 %31, i1 true), !range !108
  %35 = trunc i64 %34 to i32
  %36 = add i32 %24, %35
  %37 = icmp ult i32 %36, %21
  br i1 %37, label %42, label %46

38:                                               ; preds = %23
  %39 = and i32 %24, -64
  %40 = add i32 %39, 64
  %41 = icmp ult i32 %40, %21
  br i1 %41, label %23, label %46

42:                                               ; preds = %33
  %43 = shl nuw i64 %14, 32
  %44 = zext i32 %36 to i64
  %45 = or disjoint i64 %43, %44
  br label %49

46:                                               ; preds = %38, %19, %33, %13
  %47 = add nuw nsw i64 %14, 1
  %48 = icmp eq i64 %47, %12
  br i1 %48, label %49, label %13, !llvm.loop !141

49:                                               ; preds = %46, %2, %42
  %50 = phi i64 [ %45, %42 ], [ -1, %2 ], [ -1, %46 ]
  ret i64 %50
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(read, inaccessiblemem: none)
define ptr @fa_pool_at(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #17 {
  %3 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 1
  %4 = load ptr, ptr %3, align 8, !tbaa !135
  %5 = lshr i64 %1, 32
  %6 = getelementptr inbounds ptr, ptr %4, i64 %5
  %7 = load ptr, ptr %6, align 8, !tbaa !9
  %8 = load ptr, ptr %7, align 8, !tbaa !9
  %9 = and i64 %1, 4294967295
  %10 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 6
  %11 = load i32, ptr %10, align 8, !tbaa !126
  %12 = zext i32 %11 to i64
  %13 = mul nuw i64 %9, %12
  %14 = getelementptr inbounds i8, ptr %8, i64 %13
  ret ptr %14
}

; Function Attrs: nounwind optsize
define void @fa_pool_clear(ptr nocapture noundef %0) local_unnamed_addr #0 {
  tail call fastcc void @pool_release_chunks(ptr noundef %0) #35
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @pool_release_chunks(ptr nocapture noundef %0) unnamed_addr #0 {
  %2 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 4
  %3 = load i32, ptr %2, align 8, !tbaa !132
  %4 = icmp eq i32 %3, 0
  br i1 %4, label %8, label %5

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 1
  %7 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 8
  br label %12

8:                                                ; preds = %24, %1
  %9 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 5
  %10 = load i32, ptr %9, align 4, !tbaa !139
  %11 = icmp eq i32 %10, 0
  br i1 %11, label %41, label %29

12:                                               ; preds = %5, %24
  %13 = phi i32 [ %3, %5 ], [ %25, %24 ]
  %14 = phi i64 [ 0, %5 ], [ %26, %24 ]
  %15 = load ptr, ptr %6, align 8, !tbaa !135
  %16 = getelementptr inbounds ptr, ptr %15, i64 %14
  %17 = load ptr, ptr %16, align 8, !tbaa !9
  %18 = icmp eq ptr %17, null
  br i1 %18, label %24, label %19

19:                                               ; preds = %12
  %20 = load i32, ptr %7, align 8, !tbaa !130
  %21 = zext i32 %20 to i64
  tail call fastcc void @give_pages(ptr noundef nonnull %17, i64 noundef %21) #35
  %22 = atomicrmw sub ptr @fa_live_blocks, i64 1 monotonic, align 8
  %23 = load i32, ptr %2, align 8, !tbaa !132
  br label %24

24:                                               ; preds = %12, %19
  %25 = phi i32 [ %13, %12 ], [ %23, %19 ]
  %26 = add nuw nsw i64 %14, 1
  %27 = zext i32 %25 to i64
  %28 = icmp ult i64 %26, %27
  br i1 %28, label %12, label %8, !llvm.loop !142

29:                                               ; preds = %8
  %30 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 1
  %31 = load ptr, ptr %30, align 8, !tbaa !135
  %32 = zext i32 %10 to i64
  %33 = shl nuw nsw i64 %32, 3
  tail call void @llvm.memset.p0.i64(ptr align 8 %31, i8 0, i64 %33, i1 false)
  %34 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 2
  %35 = load ptr, ptr %34, align 8, !tbaa !133
  %36 = load i32, ptr %9, align 4, !tbaa !139
  %37 = add i32 %36, 63
  %38 = lshr i32 %37, 3
  %39 = and i32 %38, 536870904
  %40 = zext nneg i32 %39 to i64
  tail call void @llvm.memset.p0.i64(ptr align 8 %35, i8 0, i64 %40, i1 false)
  br label %41

41:                                               ; preds = %29, %8
  store i32 0, ptr %2, align 8, !tbaa !132
  store i64 0, ptr %0, align 8, !tbaa !136
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_pool_free(ptr noundef %0) local_unnamed_addr #0 {
  tail call fastcc void @pool_release_chunks(ptr noundef %0) #35
  %2 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 3
  %3 = load ptr, ptr %2, align 8, !tbaa !137
  %4 = icmp eq ptr %3, null
  br i1 %4, label %10, label %5

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 8
  %7 = load i32, ptr %6, align 8, !tbaa !130
  %8 = zext i32 %7 to i64
  tail call fastcc void @give_pages(ptr noundef nonnull %3, i64 noundef %8) #35
  %9 = atomicrmw sub ptr @fa_live_blocks, i64 1 monotonic, align 8
  br label %10

10:                                               ; preds = %5, %1
  %11 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 5
  %12 = load i32, ptr %11, align 4, !tbaa !139
  %13 = icmp eq i32 %12, 0
  br i1 %13, label %26, label %14

14:                                               ; preds = %10
  %15 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 1
  %16 = load ptr, ptr %15, align 8, !tbaa !135
  %17 = zext i32 %12 to i64
  %18 = shl nuw nsw i64 %17, 3
  tail call void @fa_free(ptr noundef %16, i64 noundef %18, i64 noundef 8) #34
  %19 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 2
  %20 = load ptr, ptr %19, align 8, !tbaa !133
  %21 = load i32, ptr %11, align 4, !tbaa !139
  %22 = add i32 %21, 63
  %23 = lshr i32 %22, 3
  %24 = and i32 %23, 536870904
  %25 = zext nneg i32 %24 to i64
  tail call void @fa_free(ptr noundef %20, i64 noundef %25, i64 noundef 8) #34
  br label %26

26:                                               ; preds = %14, %10
  tail call void @fa_free(ptr noundef nonnull %0, i64 noundef 56, i64 noundef 8) #34
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(read, inaccessiblemem: none)
define i32 @fa_links_has(ptr noundef readonly %0, ptr noundef readonly %1) local_unnamed_addr #16 {
  %3 = tail call fastcc i64 @links_find(ptr noundef %0, ptr noundef %1) #35
  %4 = icmp sgt i64 %3, -1
  %5 = zext i1 %4 to i32
  ret i32 %5
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(read, inaccessiblemem: none)
define internal fastcc i64 @links_find(ptr noundef readonly %0, ptr noundef %1) unnamed_addr #16 {
  %3 = icmp eq ptr %0, null
  br i1 %3, label %56, label %4

4:                                                ; preds = %2
  %5 = getelementptr inbounds %struct.FaLinks, ptr %0, i64 0, i32 4
  %6 = load ptr, ptr %5, align 8, !tbaa !143
  %7 = icmp eq ptr %6, null
  br i1 %7, label %8, label %23

8:                                                ; preds = %4
  %9 = getelementptr inbounds %struct.FaLinks, ptr %0, i64 0, i32 2
  %10 = load i64, ptr %9, align 8, !tbaa !145
  %11 = icmp sgt i64 %10, 0
  br i1 %11, label %12, label %56

12:                                               ; preds = %8
  %13 = getelementptr inbounds %struct.FaLinks, ptr %0, i64 0, i32 1
  %14 = load ptr, ptr %13, align 8, !tbaa !146
  br label %15

15:                                               ; preds = %12, %20
  %16 = phi i64 [ 0, %12 ], [ %21, %20 ]
  %17 = getelementptr inbounds ptr, ptr %14, i64 %16
  %18 = load ptr, ptr %17, align 8, !tbaa !9
  %19 = icmp eq ptr %18, %1
  br i1 %19, label %56, label %20

20:                                               ; preds = %15
  %21 = add nuw nsw i64 %16, 1
  %22 = icmp eq i64 %21, %10
  br i1 %22, label %56, label %15, !llvm.loop !147

23:                                               ; preds = %4
  %24 = getelementptr inbounds %struct.FaLinks, ptr %0, i64 0, i32 5
  %25 = load i64, ptr %24, align 8, !tbaa !148
  %26 = ptrtoint ptr %1 to i64
  %27 = lshr i64 %26, 33
  %28 = xor i64 %27, %26
  %29 = mul i64 %28, -49064778989728563
  %30 = lshr i64 %29, 33
  %31 = xor i64 %30, %29
  %32 = and i64 %31, %25
  %33 = getelementptr inbounds i32, ptr %6, i64 %32
  %34 = load i32, ptr %33, align 4, !tbaa !20
  %35 = icmp eq i32 %34, 0
  br i1 %35, label %56, label %36

36:                                               ; preds = %23
  %37 = getelementptr inbounds %struct.FaLinks, ptr %0, i64 0, i32 1
  %38 = load ptr, ptr %37, align 8, !tbaa !146
  br label %45

39:                                               ; preds = %45
  %40 = add i64 %47, 1
  %41 = and i64 %40, %25
  %42 = getelementptr inbounds i32, ptr %6, i64 %41
  %43 = load i32, ptr %42, align 4, !tbaa !20
  %44 = icmp eq i32 %43, 0
  br i1 %44, label %56, label %45

45:                                               ; preds = %36, %39
  %46 = phi i32 [ %34, %36 ], [ %43, %39 ]
  %47 = phi i64 [ %32, %36 ], [ %41, %39 ]
  %48 = add i32 %46, -1
  %49 = zext i32 %48 to i64
  %50 = getelementptr inbounds ptr, ptr %38, i64 %49
  %51 = load ptr, ptr %50, align 8, !tbaa !9
  %52 = icmp eq ptr %51, %1
  br i1 %52, label %53, label %39

53:                                               ; preds = %45
  %54 = zext i32 %46 to i64
  %55 = add nsw i64 %54, -1
  br label %56

56:                                               ; preds = %39, %15, %20, %23, %8, %53, %2
  %57 = phi i64 [ -1, %2 ], [ %55, %53 ], [ -1, %8 ], [ -1, %23 ], [ %16, %15 ], [ -1, %20 ], [ -1, %39 ]
  ret i64 %57
}

; Function Attrs: nounwind optsize
define noundef i32 @fa_links_add(ptr nocapture noundef %0, ptr noundef %1) local_unnamed_addr #0 {
  %3 = load ptr, ptr %0, align 8, !tbaa !9
  %4 = icmp eq ptr %3, null
  br i1 %4, label %5, label %7

5:                                                ; preds = %2
  %6 = tail call ptr @fa_alloc(i64 noundef 48, i64 noundef 8) #34
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %6, i8 0, i64 48, i1 false)
  store ptr %6, ptr %0, align 8, !tbaa !9
  br label %10

7:                                                ; preds = %2
  %8 = tail call fastcc i64 @links_find(ptr noundef nonnull %3, ptr noundef %1) #35
  %9 = icmp sgt i64 %8, -1
  br i1 %9, label %152, label %10

10:                                               ; preds = %7, %5
  %11 = phi ptr [ %6, %5 ], [ %3, %7 ]
  %12 = getelementptr inbounds %struct.FaLinks, ptr %11, i64 0, i32 2
  %13 = load i64, ptr %12, align 8, !tbaa !145
  %14 = getelementptr inbounds %struct.FaLinks, ptr %11, i64 0, i32 3
  %15 = load i64, ptr %14, align 8, !tbaa !149
  %16 = icmp eq i64 %13, %15
  br i1 %16, label %17, label %118

17:                                               ; preds = %10
  %18 = icmp sgt i64 %13, 0
  br i1 %18, label %19, label %40

19:                                               ; preds = %17
  %20 = load i64, ptr %11, align 8, !tbaa !150
  %21 = shl nsw i64 %20, 1
  %22 = icmp sgt i64 %21, %13
  br i1 %22, label %40, label %23

23:                                               ; preds = %19
  %24 = getelementptr inbounds %struct.FaLinks, ptr %11, i64 0, i32 1
  br label %26

25:                                               ; preds = %36
  store i64 %37, ptr %12, align 8, !tbaa !145
  br label %58

26:                                               ; preds = %23, %36
  %27 = phi i64 [ 0, %23 ], [ %38, %36 ]
  %28 = phi i64 [ 0, %23 ], [ %37, %36 ]
  %29 = load ptr, ptr %24, align 8, !tbaa !146
  %30 = getelementptr inbounds ptr, ptr %29, i64 %27
  %31 = load ptr, ptr %30, align 8, !tbaa !9
  %32 = icmp eq ptr %31, null
  br i1 %32, label %36, label %33

33:                                               ; preds = %26
  %34 = getelementptr inbounds ptr, ptr %29, i64 %28
  store ptr %31, ptr %34, align 8, !tbaa !9
  %35 = add nsw i64 %28, 1
  br label %36

36:                                               ; preds = %26, %33
  %37 = phi i64 [ %35, %33 ], [ %28, %26 ]
  %38 = add nuw nsw i64 %27, 1
  %39 = icmp eq i64 %38, %13
  br i1 %39, label %25, label %26, !llvm.loop !151

40:                                               ; preds = %19, %17
  %41 = icmp eq i64 %13, 0
  %42 = shl nsw i64 %13, 1
  %43 = select i1 %41, i64 4, i64 %42
  %44 = shl i64 %43, 3
  %45 = tail call ptr @fa_alloc(i64 noundef %44, i64 noundef 8) #34
  %46 = load i64, ptr %14, align 8, !tbaa !149
  %47 = icmp sgt i64 %46, 0
  br i1 %47, label %48, label %56

48:                                               ; preds = %40
  %49 = getelementptr inbounds %struct.FaLinks, ptr %11, i64 0, i32 1
  %50 = load ptr, ptr %49, align 8, !tbaa !146
  %51 = load i64, ptr %12, align 8, !tbaa !145
  %52 = shl i64 %51, 3
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %45, ptr align 1 %50, i64 %52, i1 false)
  %53 = load ptr, ptr %49, align 8, !tbaa !146
  %54 = load i64, ptr %14, align 8, !tbaa !149
  %55 = shl i64 %54, 3
  tail call void @fa_free(ptr noundef %53, i64 noundef %55, i64 noundef 8) #34
  br label %56

56:                                               ; preds = %48, %40
  %57 = getelementptr inbounds %struct.FaLinks, ptr %11, i64 0, i32 1
  store ptr %45, ptr %57, align 8, !tbaa !146
  store i64 %43, ptr %14, align 8, !tbaa !149
  br label %58

58:                                               ; preds = %56, %25
  %59 = phi i64 [ %43, %56 ], [ %13, %25 ]
  %60 = getelementptr inbounds %struct.FaLinks, ptr %11, i64 0, i32 4
  %61 = load ptr, ptr %60, align 8, !tbaa !143
  %62 = icmp eq ptr %61, null
  br i1 %62, label %69, label %63

63:                                               ; preds = %58
  %64 = getelementptr inbounds %struct.FaLinks, ptr %11, i64 0, i32 5
  %65 = load i64, ptr %64, align 8, !tbaa !148
  %66 = shl i64 %65, 2
  %67 = add i64 %66, 4
  tail call void @fa_free(ptr noundef nonnull %61, i64 noundef %67, i64 noundef 4) #34
  store ptr null, ptr %60, align 8, !tbaa !143
  %68 = load i64, ptr %14, align 8, !tbaa !149
  br label %69

69:                                               ; preds = %63, %58
  %70 = phi i64 [ %68, %63 ], [ %59, %58 ]
  %71 = icmp slt i64 %70, 9
  br i1 %71, label %72, label %74

72:                                               ; preds = %69
  %73 = load i64, ptr %12, align 8, !tbaa !145
  br label %118

74:                                               ; preds = %69
  %75 = shl nuw nsw i64 %70, 1
  br label %76

76:                                               ; preds = %76, %74
  %77 = phi i64 [ %79, %76 ], [ 16, %74 ]
  %78 = icmp slt i64 %77, %75
  %79 = shl nsw i64 %77, 1
  br i1 %78, label %76, label %80, !llvm.loop !152

80:                                               ; preds = %76
  %81 = add nsw i64 %77, -1
  %82 = getelementptr inbounds %struct.FaLinks, ptr %11, i64 0, i32 5
  store i64 %81, ptr %82, align 8, !tbaa !148
  %83 = shl i64 %77, 2
  %84 = tail call ptr @fa_alloc(i64 noundef %83, i64 noundef 4) #34
  store ptr %84, ptr %60, align 8, !tbaa !143
  tail call void @llvm.memset.p0.i64(ptr align 4 %84, i8 0, i64 %83, i1 false)
  %85 = load i64, ptr %12, align 8, !tbaa !145
  %86 = icmp sgt i64 %85, 0
  br i1 %86, label %87, label %118

87:                                               ; preds = %80
  %88 = getelementptr inbounds %struct.FaLinks, ptr %11, i64 0, i32 1
  %89 = load ptr, ptr %88, align 8, !tbaa !146
  br label %90

90:                                               ; preds = %115, %87
  %91 = phi i64 [ 0, %87 ], [ %116, %115 ]
  %92 = getelementptr inbounds ptr, ptr %89, i64 %91
  %93 = load ptr, ptr %92, align 8, !tbaa !9
  %94 = icmp eq ptr %93, null
  br i1 %94, label %115, label %95

95:                                               ; preds = %90
  %96 = load i64, ptr %82, align 8, !tbaa !148
  %97 = ptrtoint ptr %93 to i64
  %98 = lshr i64 %97, 33
  %99 = xor i64 %98, %97
  %100 = mul i64 %99, -49064778989728563
  %101 = lshr i64 %100, 33
  %102 = xor i64 %101, %100
  %103 = load ptr, ptr %60, align 8, !tbaa !143
  br label %104

104:                                              ; preds = %104, %95
  %105 = phi i64 [ %102, %95 ], [ %110, %104 ]
  %106 = and i64 %105, %96
  %107 = getelementptr inbounds i32, ptr %103, i64 %106
  %108 = load i32, ptr %107, align 4, !tbaa !20
  %109 = icmp eq i32 %108, 0
  %110 = add i64 %106, 1
  br i1 %109, label %111, label %104, !llvm.loop !153

111:                                              ; preds = %104
  %112 = getelementptr inbounds i32, ptr %103, i64 %106
  %113 = trunc i64 %91 to i32
  %114 = add i32 %113, 1
  store i32 %114, ptr %112, align 4, !tbaa !20
  br label %115

115:                                              ; preds = %111, %90
  %116 = add nuw nsw i64 %91, 1
  %117 = icmp eq i64 %116, %85
  br i1 %117, label %118, label %90, !llvm.loop !154

118:                                              ; preds = %115, %72, %80, %10
  %119 = phi i64 [ %73, %72 ], [ %85, %80 ], [ %13, %10 ], [ %85, %115 ]
  %120 = getelementptr inbounds %struct.FaLinks, ptr %11, i64 0, i32 1
  %121 = load ptr, ptr %120, align 8, !tbaa !146
  %122 = getelementptr inbounds ptr, ptr %121, i64 %119
  store ptr %1, ptr %122, align 8, !tbaa !9
  %123 = add nsw i64 %119, 1
  store i64 %123, ptr %12, align 8, !tbaa !145
  %124 = load i64, ptr %11, align 8, !tbaa !150
  %125 = add nsw i64 %124, 1
  store i64 %125, ptr %11, align 8, !tbaa !150
  %126 = getelementptr inbounds %struct.FaLinks, ptr %11, i64 0, i32 4
  %127 = load ptr, ptr %126, align 8, !tbaa !143
  %128 = icmp eq ptr %127, null
  br i1 %128, label %152, label %129

129:                                              ; preds = %118
  %130 = getelementptr inbounds %struct.FaLinks, ptr %11, i64 0, i32 5
  %131 = load i64, ptr %130, align 8, !tbaa !148
  %132 = load ptr, ptr %120, align 8, !tbaa !146
  %133 = getelementptr inbounds ptr, ptr %132, i64 %119
  %134 = load ptr, ptr %133, align 8, !tbaa !9
  %135 = ptrtoint ptr %134 to i64
  %136 = lshr i64 %135, 33
  %137 = xor i64 %136, %135
  %138 = mul i64 %137, -49064778989728563
  %139 = lshr i64 %138, 33
  %140 = xor i64 %139, %138
  br label %141

141:                                              ; preds = %141, %129
  %142 = phi i64 [ %140, %129 ], [ %147, %141 ]
  %143 = and i64 %142, %131
  %144 = getelementptr inbounds i32, ptr %127, i64 %143
  %145 = load i32, ptr %144, align 4, !tbaa !20
  %146 = icmp eq i32 %145, 0
  %147 = add i64 %143, 1
  br i1 %146, label %148, label %141, !llvm.loop !153

148:                                              ; preds = %141
  %149 = getelementptr inbounds i32, ptr %127, i64 %143
  %150 = trunc i64 %119 to i32
  %151 = add i32 %150, 1
  store i32 %151, ptr %149, align 4, !tbaa !20
  br label %152

152:                                              ; preds = %118, %148, %7
  %153 = phi i32 [ 0, %7 ], [ 1, %148 ], [ 1, %118 ]
  ret i32 %153
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define noundef i32 @fa_links_delete(ptr noundef %0, ptr noundef %1) local_unnamed_addr #10 {
  %3 = tail call fastcc i64 @links_find(ptr noundef %0, ptr noundef %1) #35
  %4 = icmp slt i64 %3, 0
  br i1 %4, label %75, label %5

5:                                                ; preds = %2
  %6 = getelementptr inbounds %struct.FaLinks, ptr %0, i64 0, i32 1
  %7 = load ptr, ptr %6, align 8, !tbaa !146
  %8 = getelementptr inbounds ptr, ptr %7, i64 %3
  store ptr null, ptr %8, align 8, !tbaa !9
  %9 = load i64, ptr %0, align 8, !tbaa !150
  %10 = add nsw i64 %9, -1
  store i64 %10, ptr %0, align 8, !tbaa !150
  %11 = getelementptr inbounds %struct.FaLinks, ptr %0, i64 0, i32 4
  %12 = load ptr, ptr %11, align 8, !tbaa !143
  %13 = icmp eq ptr %12, null
  br i1 %13, label %71, label %14

14:                                               ; preds = %5
  %15 = getelementptr inbounds %struct.FaLinks, ptr %0, i64 0, i32 5
  %16 = load i64, ptr %15, align 8, !tbaa !148
  %17 = ptrtoint ptr %1 to i64
  %18 = lshr i64 %17, 33
  %19 = xor i64 %18, %17
  %20 = mul i64 %19, -49064778989728563
  %21 = lshr i64 %20, 33
  %22 = xor i64 %21, %20
  %23 = trunc i64 %3 to i32
  %24 = add i32 %23, 1
  br label %25

25:                                               ; preds = %25, %14
  %26 = phi i64 [ %22, %14 ], [ %31, %25 ]
  %27 = and i64 %26, %16
  %28 = getelementptr inbounds i32, ptr %12, i64 %27
  %29 = load i32, ptr %28, align 4, !tbaa !20
  %30 = icmp eq i32 %29, %24
  %31 = add i64 %27, 1
  br i1 %30, label %32, label %25, !llvm.loop !155

32:                                               ; preds = %25
  %33 = and i64 %31, %16
  %34 = getelementptr inbounds i32, ptr %12, i64 %33
  %35 = load i32, ptr %34, align 4, !tbaa !20
  %36 = icmp eq i32 %35, 0
  br i1 %36, label %68, label %37

37:                                               ; preds = %32
  %38 = load ptr, ptr %6, align 8, !tbaa !146
  br label %39

39:                                               ; preds = %37, %61
  %40 = phi i32 [ %35, %37 ], [ %66, %61 ]
  %41 = phi i64 [ %33, %37 ], [ %64, %61 ]
  %42 = phi i64 [ %27, %37 ], [ %62, %61 ]
  %43 = add i32 %40, -1
  %44 = zext i32 %43 to i64
  %45 = getelementptr inbounds ptr, ptr %38, i64 %44
  %46 = load ptr, ptr %45, align 8, !tbaa !9
  %47 = ptrtoint ptr %46 to i64
  %48 = lshr i64 %47, 33
  %49 = xor i64 %48, %47
  %50 = mul i64 %49, -49064778989728563
  %51 = lshr i64 %50, 33
  %52 = xor i64 %51, %50
  %53 = and i64 %52, %16
  %54 = sub i64 %41, %53
  %55 = and i64 %54, %16
  %56 = sub i64 %41, %42
  %57 = and i64 %56, %16
  %58 = icmp ult i64 %55, %57
  br i1 %58, label %61, label %59

59:                                               ; preds = %39
  %60 = getelementptr inbounds i32, ptr %12, i64 %42
  store i32 %40, ptr %60, align 4, !tbaa !20
  br label %61

61:                                               ; preds = %39, %59
  %62 = phi i64 [ %41, %59 ], [ %42, %39 ]
  %63 = add i64 %41, 1
  %64 = and i64 %63, %16
  %65 = getelementptr inbounds i32, ptr %12, i64 %64
  %66 = load i32, ptr %65, align 4, !tbaa !20
  %67 = icmp eq i32 %66, 0
  br i1 %67, label %68, label %39

68:                                               ; preds = %61, %32
  %69 = phi i64 [ %27, %32 ], [ %62, %61 ]
  %70 = getelementptr inbounds i32, ptr %12, i64 %69
  store i32 0, ptr %70, align 4, !tbaa !20
  br label %71

71:                                               ; preds = %68, %5
  %72 = icmp eq i64 %10, 0
  br i1 %72, label %73, label %75

73:                                               ; preds = %71
  %74 = getelementptr inbounds %struct.FaLinks, ptr %0, i64 0, i32 2
  store i64 0, ptr %74, align 8, !tbaa !145
  br label %75

75:                                               ; preds = %71, %73, %2
  %76 = phi i32 [ 0, %2 ], [ 1, %73 ], [ 1, %71 ]
  ret i32 %76
}

; Function Attrs: nounwind optsize
define void @fa_links_free(ptr noundef %0) local_unnamed_addr #0 {
  %2 = icmp eq ptr %0, null
  br i1 %2, label %21, label %3

3:                                                ; preds = %1
  %4 = getelementptr inbounds %struct.FaLinks, ptr %0, i64 0, i32 4
  %5 = load ptr, ptr %4, align 8, !tbaa !143
  %6 = icmp eq ptr %5, null
  br i1 %6, label %12, label %7

7:                                                ; preds = %3
  %8 = getelementptr inbounds %struct.FaLinks, ptr %0, i64 0, i32 5
  %9 = load i64, ptr %8, align 8, !tbaa !148
  %10 = shl i64 %9, 2
  %11 = add i64 %10, 4
  tail call void @fa_free(ptr noundef nonnull %5, i64 noundef %11, i64 noundef 4) #34
  br label %12

12:                                               ; preds = %7, %3
  %13 = getelementptr inbounds %struct.FaLinks, ptr %0, i64 0, i32 3
  %14 = load i64, ptr %13, align 8, !tbaa !149
  %15 = icmp sgt i64 %14, 0
  br i1 %15, label %16, label %20

16:                                               ; preds = %12
  %17 = getelementptr inbounds %struct.FaLinks, ptr %0, i64 0, i32 1
  %18 = load ptr, ptr %17, align 8, !tbaa !146
  %19 = shl i64 %14, 3
  tail call void @fa_free(ptr noundef %18, i64 noundef %19, i64 noundef 8) #34
  br label %20

20:                                               ; preds = %16, %12
  tail call void @fa_free(ptr noundef nonnull %0, i64 noundef 48, i64 noundef 8) #34
  br label %21

21:                                               ; preds = %1, %20
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(read, inaccessiblemem: none)
define i64 @fa_links_seek(ptr noundef readonly %0, i64 noundef %1) local_unnamed_addr #16 {
  %3 = icmp eq ptr %0, null
  br i1 %3, label %19, label %4

4:                                                ; preds = %2
  %5 = getelementptr inbounds %struct.FaLinks, ptr %0, i64 0, i32 2
  %6 = load i64, ptr %5, align 8, !tbaa !145
  %7 = icmp ugt i64 %6, %1
  br i1 %7, label %8, label %19

8:                                                ; preds = %4
  %9 = getelementptr inbounds %struct.FaLinks, ptr %0, i64 0, i32 1
  %10 = load ptr, ptr %9, align 8, !tbaa !146
  br label %11

11:                                               ; preds = %8, %16
  %12 = phi i64 [ %1, %8 ], [ %17, %16 ]
  %13 = getelementptr inbounds ptr, ptr %10, i64 %12
  %14 = load ptr, ptr %13, align 8, !tbaa !9
  %15 = icmp eq ptr %14, null
  br i1 %15, label %16, label %19

16:                                               ; preds = %11
  %17 = add i64 %12, 1
  %18 = icmp eq i64 %17, %6
  br i1 %18, label %19, label %11, !llvm.loop !156

19:                                               ; preds = %11, %16, %4, %2
  %20 = phi i64 [ -1, %2 ], [ -1, %4 ], [ %12, %11 ], [ -1, %16 ]
  ret i64 %20
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(read, inaccessiblemem: none)
define ptr @fa_links_at(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #17 {
  %3 = getelementptr inbounds %struct.FaLinks, ptr %0, i64 0, i32 1
  %4 = load ptr, ptr %3, align 8, !tbaa !146
  %5 = getelementptr inbounds ptr, ptr %4, i64 %1
  %6 = load ptr, ptr %5, align 8, !tbaa !9
  ret ptr %6
}

; Function Attrs: nounwind optsize
define void @fa_relate_one(ptr noundef %0, i64 noundef %1, i64 noundef %2, ptr noundef %3, i32 noundef %4) local_unnamed_addr #0 {
  %6 = getelementptr inbounds i8, ptr %0, i64 %1
  %7 = load ptr, ptr %6, align 8, !tbaa !9
  %8 = icmp eq ptr %7, %3
  br i1 %8, label %35, label %9

9:                                                ; preds = %5
  %10 = icmp eq i32 %4, 0
  %11 = icmp eq ptr %7, null
  br i1 %10, label %22, label %12

12:                                               ; preds = %9
  br i1 %11, label %17, label %13

13:                                               ; preds = %12
  %14 = getelementptr inbounds i8, ptr %7, i64 %2
  %15 = load ptr, ptr %14, align 8, !tbaa !9
  %16 = tail call i32 @fa_links_delete(ptr noundef %15, ptr noundef nonnull %0) #35, !range !46
  br label %17

17:                                               ; preds = %13, %12
  store ptr %3, ptr %6, align 8, !tbaa !9
  %18 = icmp eq ptr %3, null
  br i1 %18, label %35, label %19

19:                                               ; preds = %17
  %20 = getelementptr inbounds i8, ptr %3, i64 %2
  %21 = tail call i32 @fa_links_add(ptr noundef nonnull %20, ptr noundef nonnull %0) #35, !range !46
  br label %35

22:                                               ; preds = %9
  br i1 %11, label %25, label %23

23:                                               ; preds = %22
  %24 = getelementptr inbounds i8, ptr %7, i64 %2
  store ptr null, ptr %24, align 8, !tbaa !9
  br label %25

25:                                               ; preds = %23, %22
  %26 = icmp eq ptr %3, null
  br i1 %26, label %34, label %27

27:                                               ; preds = %25
  %28 = getelementptr inbounds i8, ptr %3, i64 %2
  %29 = load ptr, ptr %28, align 8, !tbaa !9
  %30 = icmp eq ptr %29, null
  br i1 %30, label %33, label %31

31:                                               ; preds = %27
  %32 = getelementptr inbounds i8, ptr %29, i64 %1
  store ptr null, ptr %32, align 8, !tbaa !9
  br label %33

33:                                               ; preds = %31, %27
  store ptr %0, ptr %28, align 8, !tbaa !9
  br label %34

34:                                               ; preds = %33, %25
  store ptr %3, ptr %6, align 8, !tbaa !9
  br label %35

35:                                               ; preds = %17, %19, %5, %34
  ret void
}

; Function Attrs: nounwind optsize
define noundef i32 @fa_relate_add(ptr noundef %0, i64 noundef %1, i64 noundef %2, ptr noundef %3) local_unnamed_addr #0 {
  %5 = getelementptr inbounds i8, ptr %0, i64 %1
  %6 = tail call i32 @fa_links_add(ptr noundef %5, ptr noundef %3) #35, !range !46
  %7 = icmp eq i32 %6, 0
  br i1 %7, label %11, label %8

8:                                                ; preds = %4
  %9 = getelementptr inbounds i8, ptr %3, i64 %2
  %10 = tail call i32 @fa_links_add(ptr noundef %9, ptr noundef %0) #35, !range !46
  br label %11

11:                                               ; preds = %4, %8
  %12 = phi i32 [ 1, %8 ], [ 0, %4 ]
  ret i32 %12
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define noundef i32 @fa_relate_delete(ptr noundef %0, i64 noundef %1, i64 noundef %2, ptr noundef %3) local_unnamed_addr #10 {
  %5 = getelementptr inbounds i8, ptr %0, i64 %1
  %6 = load ptr, ptr %5, align 8, !tbaa !9
  %7 = tail call i32 @fa_links_delete(ptr noundef %6, ptr noundef %3) #35, !range !46
  %8 = icmp eq i32 %7, 0
  br i1 %8, label %13, label %9

9:                                                ; preds = %4
  %10 = getelementptr inbounds i8, ptr %3, i64 %2
  %11 = load ptr, ptr %10, align 8, !tbaa !9
  %12 = tail call i32 @fa_links_delete(ptr noundef %11, ptr noundef nonnull %0) #35, !range !46
  br label %13

13:                                               ; preds = %4, %9
  %14 = phi i32 [ 1, %9 ], [ 0, %4 ]
  ret i32 %14
}

; Function Attrs: nounwind optsize
define void @fa_relate_clear(ptr noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #0 {
  %4 = getelementptr inbounds i8, ptr %0, i64 %1
  %5 = load ptr, ptr %4, align 8, !tbaa !9
  %6 = icmp eq ptr %5, null
  br i1 %6, label %30, label %7

7:                                                ; preds = %3
  %8 = getelementptr inbounds %struct.FaLinks, ptr %5, i64 0, i32 2
  %9 = load i64, ptr %8, align 8, !tbaa !145
  %10 = icmp sgt i64 %9, 0
  br i1 %10, label %11, label %13

11:                                               ; preds = %7
  %12 = getelementptr inbounds %struct.FaLinks, ptr %5, i64 0, i32 1
  br label %14

13:                                               ; preds = %26, %7
  tail call void @fa_links_free(ptr noundef nonnull %5) #35
  store ptr null, ptr %4, align 8, !tbaa !9
  br label %30

14:                                               ; preds = %11, %26
  %15 = phi i64 [ %9, %11 ], [ %27, %26 ]
  %16 = phi i64 [ 0, %11 ], [ %28, %26 ]
  %17 = load ptr, ptr %12, align 8, !tbaa !146
  %18 = getelementptr inbounds ptr, ptr %17, i64 %16
  %19 = load ptr, ptr %18, align 8, !tbaa !9
  %20 = icmp eq ptr %19, null
  br i1 %20, label %26, label %21

21:                                               ; preds = %14
  %22 = getelementptr inbounds i8, ptr %19, i64 %2
  %23 = load ptr, ptr %22, align 8, !tbaa !9
  %24 = tail call i32 @fa_links_delete(ptr noundef %23, ptr noundef %0) #35, !range !46
  %25 = load i64, ptr %8, align 8, !tbaa !145
  br label %26

26:                                               ; preds = %21, %14
  %27 = phi i64 [ %25, %21 ], [ %15, %14 ]
  %28 = add nuw nsw i64 %16, 1
  %29 = icmp slt i64 %28, %27
  br i1 %29, label %14, label %13, !llvm.loop !157

30:                                               ; preds = %3, %13
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define void @fa_unlink(ptr noundef %0, i32 noundef %1, i32 noundef %2, i64 noundef %3, i64 noundef %4, ptr noundef %5) local_unnamed_addr #10 {
  %7 = icmp eq i32 %2, 0
  br i1 %7, label %42, label %8

8:                                                ; preds = %6
  %9 = getelementptr inbounds i8, ptr %0, i64 %3
  %10 = load ptr, ptr %9, align 8, !tbaa !9
  %11 = icmp eq i32 %1, 2
  br i1 %11, label %12, label %33

12:                                               ; preds = %8
  %13 = getelementptr inbounds %struct.FaLinks, ptr %10, i64 0, i32 2
  %14 = icmp eq ptr %10, null
  br i1 %14, label %89, label %15

15:                                               ; preds = %12
  %16 = getelementptr inbounds %struct.FaLinks, ptr %10, i64 0, i32 1
  %17 = load i64, ptr %13, align 8, !tbaa !145
  %18 = icmp sgt i64 %17, 0
  br i1 %18, label %19, label %89

19:                                               ; preds = %15, %29
  %20 = phi i64 [ %30, %29 ], [ 0, %15 ]
  %21 = load ptr, ptr %16, align 8, !tbaa !146
  %22 = getelementptr inbounds ptr, ptr %21, i64 %20
  %23 = load ptr, ptr %22, align 8, !tbaa !9
  %24 = icmp eq ptr %23, null
  br i1 %24, label %29, label %25

25:                                               ; preds = %19
  %26 = getelementptr inbounds i8, ptr %23, i64 %4
  %27 = load ptr, ptr %26, align 8, !tbaa !9
  %28 = tail call i32 @fa_links_delete(ptr noundef %27, ptr noundef %0) #35, !range !46
  br label %29

29:                                               ; preds = %25, %19
  %30 = add nuw nsw i64 %20, 1
  %31 = load i64, ptr %13, align 8, !tbaa !145
  %32 = icmp slt i64 %30, %31
  br i1 %32, label %19, label %89

33:                                               ; preds = %8
  %34 = icmp eq ptr %10, null
  br i1 %34, label %89, label %35

35:                                               ; preds = %33
  %36 = icmp eq i32 %1, 0
  %37 = getelementptr inbounds i8, ptr %10, i64 %4
  br i1 %36, label %38, label %41

38:                                               ; preds = %35
  %39 = load ptr, ptr %37, align 8, !tbaa !9
  %40 = tail call i32 @fa_links_delete(ptr noundef %39, ptr noundef nonnull %0) #35, !range !46
  br label %89

41:                                               ; preds = %35
  store ptr null, ptr %37, align 8, !tbaa !9
  br label %89

42:                                               ; preds = %6
  switch i32 %1, label %83 [
    i32 0, label %43
    i32 2, label %43
    i32 1, label %69
    i32 3, label %75
  ]

43:                                               ; preds = %42, %42
  %44 = getelementptr inbounds i8, ptr %0, i64 %4
  %45 = load ptr, ptr %44, align 8, !tbaa !9
  %46 = getelementptr inbounds %struct.FaLinks, ptr %45, i64 0, i32 2
  %47 = icmp eq ptr %45, null
  br i1 %47, label %89, label %48

48:                                               ; preds = %43
  %49 = getelementptr inbounds %struct.FaLinks, ptr %45, i64 0, i32 1
  %50 = icmp eq i32 %1, 0
  %51 = load i64, ptr %46, align 8, !tbaa !145
  %52 = icmp sgt i64 %51, 0
  br i1 %52, label %53, label %89

53:                                               ; preds = %48, %65
  %54 = phi i64 [ %66, %65 ], [ 0, %48 ]
  %55 = load ptr, ptr %49, align 8, !tbaa !146
  %56 = getelementptr inbounds ptr, ptr %55, i64 %54
  %57 = load ptr, ptr %56, align 8, !tbaa !9
  %58 = icmp eq ptr %57, null
  br i1 %58, label %65, label %59

59:                                               ; preds = %53
  %60 = getelementptr inbounds i8, ptr %57, i64 %3
  br i1 %50, label %61, label %62

61:                                               ; preds = %59
  store ptr null, ptr %60, align 8, !tbaa !9
  br label %65

62:                                               ; preds = %59
  %63 = load ptr, ptr %60, align 8, !tbaa !9
  %64 = tail call i32 @fa_links_delete(ptr noundef %63, ptr noundef %0) #35, !range !46
  br label %65

65:                                               ; preds = %61, %62, %53
  %66 = add nuw nsw i64 %54, 1
  %67 = load i64, ptr %46, align 8, !tbaa !145
  %68 = icmp slt i64 %66, %67
  br i1 %68, label %53, label %89

69:                                               ; preds = %42
  %70 = getelementptr inbounds i8, ptr %0, i64 %4
  %71 = load ptr, ptr %70, align 8, !tbaa !9
  %72 = icmp eq ptr %71, null
  br i1 %72, label %89, label %73

73:                                               ; preds = %69
  %74 = getelementptr inbounds i8, ptr %71, i64 %3
  store ptr null, ptr %74, align 8, !tbaa !9
  br label %89

75:                                               ; preds = %42
  %76 = getelementptr inbounds i8, ptr %0, i64 %3
  %77 = load ptr, ptr %76, align 8, !tbaa !9
  %78 = icmp eq ptr %77, null
  %79 = icmp eq ptr %77, %0
  %80 = or i1 %78, %79
  br i1 %80, label %89, label %81

81:                                               ; preds = %75
  %82 = getelementptr inbounds i8, ptr %77, i64 %3
  store ptr null, ptr %82, align 8, !tbaa !9
  br label %89

83:                                               ; preds = %42
  %84 = icmp eq ptr %5, null
  br i1 %84, label %89, label %85

85:                                               ; preds = %83
  %86 = load ptr, ptr %5, align 8, !tbaa !9
  %87 = icmp eq ptr %86, %0
  br i1 %87, label %88, label %89

88:                                               ; preds = %85
  store ptr null, ptr %5, align 8, !tbaa !9
  br label %89

89:                                               ; preds = %29, %65, %15, %48, %12, %43, %75, %81, %69, %73, %38, %41, %33, %83, %85, %88
  ret void
}

; Function Attrs: nounwind optsize
define ptr @fa_address_map_new(i64 noundef %0) local_unnamed_addr #0 {
  %2 = tail call ptr @fa_alloc(i64 noundef 24, i64 noundef 8) #34
  %3 = shl nsw i64 %0, 1
  br label %4

4:                                                ; preds = %4, %1
  %5 = phi i64 [ 16, %1 ], [ %7, %4 ]
  %6 = icmp slt i64 %5, %3
  %7 = shl nsw i64 %5, 1
  br i1 %6, label %4, label %8, !llvm.loop !158

8:                                                ; preds = %4
  %9 = add nsw i64 %5, -1
  %10 = getelementptr inbounds %struct.FaAddressMap, ptr %2, i64 0, i32 2
  store i64 %9, ptr %10, align 8, !tbaa !159
  %11 = shl i64 %5, 3
  %12 = tail call ptr @fa_alloc(i64 noundef %11, i64 noundef 8) #34
  store ptr %12, ptr %2, align 8, !tbaa !161
  %13 = tail call ptr @fa_alloc(i64 noundef %11, i64 noundef 8) #34
  %14 = getelementptr inbounds %struct.FaAddressMap, ptr %2, i64 0, i32 1
  store ptr %13, ptr %14, align 8, !tbaa !162
  %15 = load ptr, ptr %2, align 8, !tbaa !161
  tail call void @llvm.memset.p0.i64(ptr align 8 %15, i8 0, i64 %11, i1 false)
  ret ptr %2
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define void @fa_address_map_put(ptr nocapture noundef readonly %0, ptr noundef %1, ptr noundef %2) local_unnamed_addr #10 {
  %4 = getelementptr inbounds %struct.FaAddressMap, ptr %0, i64 0, i32 2
  %5 = load i64, ptr %4, align 8, !tbaa !159
  %6 = ptrtoint ptr %1 to i64
  %7 = lshr i64 %6, 33
  %8 = xor i64 %7, %6
  %9 = mul i64 %8, -49064778989728563
  %10 = lshr i64 %9, 33
  %11 = xor i64 %10, %9
  %12 = load ptr, ptr %0, align 8, !tbaa !161
  br label %13

13:                                               ; preds = %13, %3
  %14 = phi i64 [ %11, %3 ], [ %21, %13 ]
  %15 = and i64 %14, %5
  %16 = getelementptr inbounds ptr, ptr %12, i64 %15
  %17 = load ptr, ptr %16, align 8, !tbaa !9
  %18 = icmp eq ptr %17, null
  %19 = icmp eq ptr %17, %1
  %20 = or i1 %18, %19
  %21 = add i64 %15, 1
  br i1 %20, label %22, label %13, !llvm.loop !163

22:                                               ; preds = %13
  %23 = getelementptr inbounds ptr, ptr %12, i64 %15
  store ptr %1, ptr %23, align 8, !tbaa !9
  %24 = getelementptr inbounds %struct.FaAddressMap, ptr %0, i64 0, i32 1
  %25 = load ptr, ptr %24, align 8, !tbaa !162
  %26 = getelementptr inbounds ptr, ptr %25, i64 %15
  store ptr %2, ptr %26, align 8, !tbaa !9
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(read, inaccessiblemem: none)
define ptr @fa_address_map_get(ptr nocapture noundef readonly %0, ptr noundef %1) local_unnamed_addr #16 {
  %3 = icmp eq ptr %1, null
  br i1 %3, label %33, label %4

4:                                                ; preds = %2
  %5 = getelementptr inbounds %struct.FaAddressMap, ptr %0, i64 0, i32 2
  %6 = load i64, ptr %5, align 8, !tbaa !159
  %7 = ptrtoint ptr %1 to i64
  %8 = lshr i64 %7, 33
  %9 = xor i64 %8, %7
  %10 = mul i64 %9, -49064778989728563
  %11 = lshr i64 %10, 33
  %12 = xor i64 %11, %10
  %13 = load ptr, ptr %0, align 8, !tbaa !161
  %14 = and i64 %12, %6
  %15 = getelementptr inbounds ptr, ptr %13, i64 %14
  %16 = load ptr, ptr %15, align 8, !tbaa !9
  %17 = icmp eq ptr %16, null
  br i1 %17, label %33, label %18

18:                                               ; preds = %4, %27
  %19 = phi ptr [ %31, %27 ], [ %16, %4 ]
  %20 = phi i64 [ %29, %27 ], [ %14, %4 ]
  %21 = icmp eq ptr %19, %1
  br i1 %21, label %22, label %27

22:                                               ; preds = %18
  %23 = getelementptr inbounds %struct.FaAddressMap, ptr %0, i64 0, i32 1
  %24 = load ptr, ptr %23, align 8, !tbaa !162
  %25 = getelementptr inbounds ptr, ptr %24, i64 %20
  %26 = load ptr, ptr %25, align 8, !tbaa !9
  br label %33

27:                                               ; preds = %18
  %28 = add i64 %20, 1
  %29 = and i64 %28, %6
  %30 = getelementptr inbounds ptr, ptr %13, i64 %29
  %31 = load ptr, ptr %30, align 8, !tbaa !9
  %32 = icmp eq ptr %31, null
  br i1 %32, label %33, label %18, !llvm.loop !164

33:                                               ; preds = %27, %4, %22, %2
  %34 = phi ptr [ null, %2 ], [ %26, %22 ], [ null, %4 ], [ null, %27 ]
  ret ptr %34
}

; Function Attrs: nounwind optsize
define ptr @fa_links_relink(ptr noundef readonly %0, ptr nocapture noundef readonly %1) local_unnamed_addr #0 {
  %3 = alloca ptr, align 8
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %3) #31
  store ptr null, ptr %3, align 8, !tbaa !9
  %4 = getelementptr inbounds %struct.FaLinks, ptr %0, i64 0, i32 2
  %5 = icmp eq ptr %0, null
  br i1 %5, label %12, label %6

6:                                                ; preds = %2
  %7 = getelementptr inbounds %struct.FaLinks, ptr %0, i64 0, i32 1
  %8 = load i64, ptr %4, align 8, !tbaa !145
  %9 = icmp sgt i64 %8, 0
  br i1 %9, label %14, label %10

10:                                               ; preds = %23, %6
  %11 = load ptr, ptr %3, align 8, !tbaa !9
  br label %12

12:                                               ; preds = %10, %2
  %13 = phi ptr [ %11, %10 ], [ null, %2 ]
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %3) #31
  ret ptr %13

14:                                               ; preds = %6, %23
  %15 = phi i64 [ %24, %23 ], [ 0, %6 ]
  %16 = load ptr, ptr %7, align 8, !tbaa !146
  %17 = getelementptr inbounds ptr, ptr %16, i64 %15
  %18 = load ptr, ptr %17, align 8, !tbaa !9
  %19 = tail call ptr @fa_address_map_get(ptr noundef %1, ptr noundef %18) #35
  %20 = icmp eq ptr %19, null
  br i1 %20, label %23, label %21

21:                                               ; preds = %14
  %22 = call i32 @fa_links_add(ptr noundef nonnull %3, ptr noundef nonnull %19) #35, !range !46
  br label %23

23:                                               ; preds = %21, %14
  %24 = add nuw nsw i64 %15, 1
  %25 = load i64, ptr %4, align 8, !tbaa !145
  %26 = icmp slt i64 %24, %25
  br i1 %26, label %14, label %10
}

; Function Attrs: nounwind optsize
define void @fa_address_map_free(ptr noundef %0) local_unnamed_addr #0 {
  %2 = load ptr, ptr %0, align 8, !tbaa !161
  %3 = getelementptr inbounds %struct.FaAddressMap, ptr %0, i64 0, i32 2
  %4 = load i64, ptr %3, align 8, !tbaa !159
  %5 = shl i64 %4, 3
  %6 = add i64 %5, 8
  tail call void @fa_free(ptr noundef %2, i64 noundef %6, i64 noundef 8) #34
  %7 = getelementptr inbounds %struct.FaAddressMap, ptr %0, i64 0, i32 1
  %8 = load ptr, ptr %7, align 8, !tbaa !162
  %9 = load i64, ptr %3, align 8, !tbaa !159
  %10 = shl i64 %9, 3
  %11 = add i64 %10, 8
  tail call void @fa_free(ptr noundef %8, i64 noundef %11, i64 noundef 8) #34
  tail call void @fa_free(ptr noundef nonnull %0, i64 noundef 24, i64 noundef 8) #34
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, argmem: none, inaccessiblemem: none)
define void @fa_argument_source_set(i32 noundef %0, ptr noundef %1) local_unnamed_addr #18 {
  %3 = and i32 %0, 15
  %4 = zext nneg i32 %3 to i64
  %5 = getelementptr inbounds [16 x ptr], ptr @argument_sources, i64 0, i64 %4
  store ptr %1, ptr %5, align 8, !tbaa !9
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(read, argmem: none, inaccessiblemem: none)
define ptr @fa_argument_source_get(i32 noundef %0) local_unnamed_addr #19 {
  %2 = and i32 %0, 15
  %3 = zext nneg i32 %2 to i64
  %4 = getelementptr inbounds [16 x ptr], ptr @argument_sources, i64 0, i64 %3
  %5 = load ptr, ptr %4, align 8, !tbaa !9
  ret ptr %5
}

; Function Attrs: nounwind optsize
define noundef ptr @fa_source_new() local_unnamed_addr #0 {
  %1 = tail call ptr @fa_alloc(i64 noundef 72, i64 noundef 8) #34
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %1, i8 0, i64 72, i1 false)
  %2 = load ptr, ptr @reactive_made, align 8, !tbaa !9
  %3 = load i64, ptr @reactive_made_count, align 8, !tbaa !5
  %4 = tail call fastcc ptr @grow_items(ptr noundef %2, i64 noundef %3, ptr noundef nonnull @reactive_made_capacity, i64 noundef 8) #35
  store ptr %4, ptr @reactive_made, align 8, !tbaa !9
  %5 = load i64, ptr @reactive_made_count, align 8, !tbaa !5
  %6 = getelementptr inbounds ptr, ptr %4, i64 %5
  store ptr %1, ptr %6, align 8, !tbaa !9
  %7 = add nsw i64 %5, 1
  store i64 %7, ptr @reactive_made_count, align 8, !tbaa !5
  ret ptr %1
}

; Function Attrs: nounwind optsize
define ptr @fa_source_child(ptr noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #0 {
  %4 = icmp eq ptr %0, null
  br i1 %4, label %47, label %5

5:                                                ; preds = %3
  %6 = getelementptr inbounds %struct.FaSource, ptr %0, i64 0, i32 7
  %7 = load i64, ptr %6, align 8, !tbaa !165
  %8 = icmp sgt i64 %7, 0
  br i1 %8, label %9, label %27

9:                                                ; preds = %5
  %10 = getelementptr inbounds %struct.FaSource, ptr %0, i64 0, i32 6
  %11 = load ptr, ptr %10, align 8, !tbaa !167
  br label %12

12:                                               ; preds = %9, %21
  %13 = phi i64 [ 0, %9 ], [ %22, %21 ]
  %14 = getelementptr inbounds %struct.FaChild, ptr %11, i64 %13
  %15 = load i64, ptr %14, align 8, !tbaa !168
  %16 = icmp eq i64 %15, %1
  br i1 %16, label %17, label %21

17:                                               ; preds = %12
  %18 = getelementptr inbounds %struct.FaChild, ptr %11, i64 %13, i32 1
  %19 = load i64, ptr %18, align 8, !tbaa !170
  %20 = icmp eq i64 %19, %2
  br i1 %20, label %24, label %21

21:                                               ; preds = %17, %12
  %22 = add nuw nsw i64 %13, 1
  %23 = icmp eq i64 %22, %7
  br i1 %23, label %27, label %12, !llvm.loop !171

24:                                               ; preds = %17
  %25 = getelementptr inbounds %struct.FaChild, ptr %11, i64 %13, i32 2
  %26 = load ptr, ptr %25, align 8, !tbaa !172
  br label %47

27:                                               ; preds = %21, %5
  %28 = tail call ptr @fa_alloc(i64 noundef 72, i64 noundef 8) #34
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %28, i8 0, i64 72, i1 false)
  %29 = getelementptr inbounds %struct.FaSource, ptr %28, i64 0, i32 5
  store ptr %0, ptr %29, align 8, !tbaa !173
  %30 = load ptr, ptr @reactive_made, align 8, !tbaa !9
  %31 = load i64, ptr @reactive_made_count, align 8, !tbaa !5
  %32 = tail call fastcc ptr @grow_items(ptr noundef %30, i64 noundef %31, ptr noundef nonnull @reactive_made_capacity, i64 noundef 8) #35
  store ptr %32, ptr @reactive_made, align 8, !tbaa !9
  %33 = load i64, ptr @reactive_made_count, align 8, !tbaa !5
  %34 = getelementptr inbounds ptr, ptr %32, i64 %33
  store ptr %28, ptr %34, align 8, !tbaa !9
  %35 = add nsw i64 %33, 1
  store i64 %35, ptr @reactive_made_count, align 8, !tbaa !5
  %36 = getelementptr inbounds %struct.FaSource, ptr %0, i64 0, i32 6
  %37 = load ptr, ptr %36, align 8, !tbaa !167
  %38 = load i64, ptr %6, align 8, !tbaa !165
  %39 = getelementptr inbounds %struct.FaSource, ptr %0, i64 0, i32 8
  %40 = tail call fastcc ptr @grow_items(ptr noundef %37, i64 noundef %38, ptr noundef nonnull %39, i64 noundef 24) #35
  store ptr %40, ptr %36, align 8, !tbaa !167
  %41 = load i64, ptr %6, align 8, !tbaa !165
  %42 = getelementptr inbounds %struct.FaChild, ptr %40, i64 %41
  store i64 %1, ptr %42, align 8, !tbaa.struct !174
  %43 = getelementptr inbounds i8, ptr %42, i64 8
  store i64 %2, ptr %43, align 8, !tbaa.struct !175
  %44 = getelementptr inbounds i8, ptr %42, i64 16
  store ptr %28, ptr %44, align 8, !tbaa.struct !176
  %45 = load i64, ptr %6, align 8, !tbaa !165
  %46 = add nsw i64 %45, 1
  store i64 %46, ptr %6, align 8, !tbaa !165
  br label %47

47:                                               ; preds = %24, %3, %27
  %48 = phi ptr [ %26, %24 ], [ %28, %27 ], [ null, %3 ]
  ret ptr %48
}

; Function Attrs: nounwind optsize
define internal fastcc ptr @grow_items(ptr noundef %0, i64 noundef %1, ptr nocapture noundef %2, i64 noundef %3) unnamed_addr #0 {
  %5 = load i64, ptr %2, align 8, !tbaa !5
  %6 = icmp sgt i64 %5, %1
  br i1 %6, label %20, label %7

7:                                                ; preds = %4
  %8 = icmp eq i64 %5, 0
  %9 = shl nsw i64 %5, 1
  %10 = select i1 %8, i64 4, i64 %9
  %11 = mul i64 %10, %3
  %12 = tail call ptr @fa_alloc(i64 noundef %11, i64 noundef 8) #34
  %13 = load i64, ptr %2, align 8, !tbaa !5
  %14 = icmp sgt i64 %13, 0
  br i1 %14, label %15, label %19

15:                                               ; preds = %7
  %16 = mul i64 %3, %1
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %12, ptr align 1 %0, i64 %16, i1 false)
  %17 = load i64, ptr %2, align 8, !tbaa !5
  %18 = mul i64 %17, %3
  tail call void @fa_free(ptr noundef %0, i64 noundef %18, i64 noundef 8) #34
  br label %19

19:                                               ; preds = %15, %7
  store i64 %10, ptr %2, align 8, !tbaa !5
  br label %20

20:                                               ; preds = %4, %19
  %21 = phi ptr [ %12, %19 ], [ %0, %4 ]
  ret ptr %21
}

; Function Attrs: nounwind optsize
define ptr @fa_field_source(ptr noundef %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = ptrtoint ptr %0 to i64
  %4 = load i64, ptr @field_source_count, align 8, !tbaa !5
  %5 = icmp sgt i64 %4, 0
  br i1 %5, label %6, label %23

6:                                                ; preds = %2
  %7 = load ptr, ptr @field_sources, align 8, !tbaa !9
  br label %8

8:                                                ; preds = %6, %20
  %9 = phi i64 [ 0, %6 ], [ %21, %20 ]
  %10 = getelementptr inbounds %struct.FaChild, ptr %7, i64 %9
  %11 = load i64, ptr %10, align 8, !tbaa !168
  %12 = icmp eq i64 %11, %3
  br i1 %12, label %13, label %20

13:                                               ; preds = %8
  %14 = getelementptr inbounds %struct.FaChild, ptr %7, i64 %9, i32 1
  %15 = load i64, ptr %14, align 8, !tbaa !170
  %16 = icmp eq i64 %15, %1
  br i1 %16, label %17, label %20

17:                                               ; preds = %13
  %18 = getelementptr inbounds %struct.FaChild, ptr %7, i64 %9, i32 2
  %19 = load ptr, ptr %18, align 8, !tbaa !172
  br label %33

20:                                               ; preds = %13, %8
  %21 = add nuw nsw i64 %9, 1
  %22 = icmp eq i64 %21, %4
  br i1 %22, label %23, label %8, !llvm.loop !177

23:                                               ; preds = %20, %2
  %24 = tail call ptr @fa_source_new() #35
  %25 = load ptr, ptr @field_sources, align 8, !tbaa !9
  %26 = load i64, ptr @field_source_count, align 8, !tbaa !5
  %27 = tail call fastcc ptr @grow_items(ptr noundef %25, i64 noundef %26, ptr noundef nonnull @field_source_capacity, i64 noundef 24) #35
  store ptr %27, ptr @field_sources, align 8, !tbaa !9
  %28 = load i64, ptr @field_source_count, align 8, !tbaa !5
  %29 = getelementptr inbounds %struct.FaChild, ptr %27, i64 %28
  store i64 %3, ptr %29, align 8, !tbaa.struct !174
  %30 = getelementptr inbounds i8, ptr %29, i64 8
  store i64 %1, ptr %30, align 8, !tbaa.struct !175
  %31 = getelementptr inbounds i8, ptr %29, i64 16
  store ptr %24, ptr %31, align 8, !tbaa.struct !176
  %32 = add nsw i64 %28, 1
  store i64 %32, ptr @field_source_count, align 8, !tbaa !5
  br label %33

33:                                               ; preds = %17, %23
  %34 = phi ptr [ %24, %23 ], [ %19, %17 ]
  ret ptr %34
}

; Function Attrs: nounwind optsize
define void @fa_track(ptr noundef %0) local_unnamed_addr #0 {
  %2 = alloca i64, align 8
  %3 = load ptr, ptr @reactive_observer, align 8, !tbaa !9
  %4 = icmp eq ptr %3, null
  %5 = icmp eq ptr %0, null
  %6 = or i1 %5, %4
  br i1 %6, label %62, label %7

7:                                                ; preds = %1
  %8 = getelementptr inbounds %struct.FaObserver, ptr %3, i64 0, i32 8
  %9 = load i64, ptr %8, align 8, !tbaa !178
  %10 = icmp sgt i64 %9, 0
  %11 = getelementptr inbounds %struct.FaObserver, ptr %3, i64 0, i32 6
  %12 = load ptr, ptr %11, align 8, !tbaa !180
  br i1 %10, label %16, label %21

13:                                               ; preds = %16
  %14 = add nuw nsw i64 %17, 1
  %15 = icmp eq i64 %14, %9
  br i1 %15, label %21, label %16, !llvm.loop !181

16:                                               ; preds = %7, %13
  %17 = phi i64 [ %14, %13 ], [ 0, %7 ]
  %18 = getelementptr inbounds ptr, ptr %12, i64 %17
  %19 = load ptr, ptr %18, align 8, !tbaa !9
  %20 = icmp eq ptr %19, %0
  br i1 %20, label %62, label %13

21:                                               ; preds = %13, %7
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %2) #31
  %22 = getelementptr inbounds %struct.FaObserver, ptr %3, i64 0, i32 9
  %23 = load i64, ptr %22, align 8, !tbaa !182
  store i64 %23, ptr %2, align 8, !tbaa !5
  %24 = getelementptr inbounds %struct.FaObserver, ptr %3, i64 0, i32 6
  %25 = call fastcc ptr @grow_items(ptr noundef %12, i64 noundef %9, ptr noundef nonnull %2, i64 noundef 8) #35
  store ptr %25, ptr %24, align 8, !tbaa !180
  %26 = load i64, ptr %22, align 8, !tbaa !182
  store i64 %26, ptr %2, align 8, !tbaa !5
  %27 = getelementptr inbounds %struct.FaObserver, ptr %3, i64 0, i32 7
  %28 = load ptr, ptr %27, align 8, !tbaa !183
  %29 = load i64, ptr %8, align 8, !tbaa !178
  %30 = call fastcc ptr @grow_items(ptr noundef %28, i64 noundef %29, ptr noundef nonnull %2, i64 noundef 8) #35
  store ptr %30, ptr %27, align 8, !tbaa !183
  %31 = load i64, ptr %2, align 8, !tbaa !5
  store i64 %31, ptr %22, align 8, !tbaa !182
  %32 = load ptr, ptr %24, align 8, !tbaa !180
  %33 = load i64, ptr %8, align 8, !tbaa !178
  %34 = getelementptr inbounds ptr, ptr %32, i64 %33
  store ptr %0, ptr %34, align 8, !tbaa !9
  %35 = getelementptr inbounds %struct.FaSource, ptr %0, i64 0, i32 1
  %36 = load i64, ptr %35, align 8, !tbaa !184
  %37 = load ptr, ptr %27, align 8, !tbaa !183
  %38 = getelementptr inbounds i64, ptr %37, i64 %33
  store i64 %36, ptr %38, align 8, !tbaa !5
  %39 = load i64, ptr %8, align 8, !tbaa !178
  %40 = add nsw i64 %39, 1
  store i64 %40, ptr %8, align 8, !tbaa !178
  %41 = getelementptr inbounds %struct.FaSource, ptr %0, i64 0, i32 3
  %42 = load i64, ptr %41, align 8, !tbaa !185
  %43 = icmp sgt i64 %42, 0
  %44 = getelementptr inbounds %struct.FaSource, ptr %0, i64 0, i32 2
  %45 = load ptr, ptr %44, align 8, !tbaa !186
  br i1 %43, label %49, label %54

46:                                               ; preds = %49
  %47 = add nuw nsw i64 %50, 1
  %48 = icmp eq i64 %47, %42
  br i1 %48, label %54, label %49, !llvm.loop !187

49:                                               ; preds = %21, %46
  %50 = phi i64 [ %47, %46 ], [ 0, %21 ]
  %51 = getelementptr inbounds ptr, ptr %45, i64 %50
  %52 = load ptr, ptr %51, align 8, !tbaa !9
  %53 = icmp eq ptr %52, %3
  br i1 %53, label %61, label %46

54:                                               ; preds = %46, %21
  %55 = getelementptr inbounds %struct.FaSource, ptr %0, i64 0, i32 2
  %56 = getelementptr inbounds %struct.FaSource, ptr %0, i64 0, i32 4
  %57 = tail call fastcc ptr @grow_items(ptr noundef %45, i64 noundef %42, ptr noundef nonnull %56, i64 noundef 8) #35
  store ptr %57, ptr %55, align 8, !tbaa !186
  %58 = load i64, ptr %41, align 8, !tbaa !185
  %59 = getelementptr inbounds ptr, ptr %57, i64 %58
  store ptr %3, ptr %59, align 8, !tbaa !9
  %60 = add nsw i64 %58, 1
  store i64 %60, ptr %41, align 8, !tbaa !185
  br label %61

61:                                               ; preds = %49, %54
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %2) #31
  br label %62

62:                                               ; preds = %16, %1, %61
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_notify(ptr noundef %0) local_unnamed_addr #0 {
  %2 = icmp eq ptr %0, null
  br i1 %2, label %29, label %3

3:                                                ; preds = %1
  tail call fastcc void @bump(ptr noundef nonnull %0) #35
  %4 = getelementptr inbounds %struct.FaSource, ptr %0, i64 0, i32 5
  %5 = load ptr, ptr %4, align 8, !tbaa !173
  %6 = icmp eq ptr %5, null
  br i1 %6, label %29, label %7

7:                                                ; preds = %3, %25
  %8 = phi ptr [ %27, %25 ], [ %5, %3 ]
  %9 = getelementptr inbounds %struct.FaSource, ptr %8, i64 0, i32 1
  %10 = load i64, ptr %9, align 8, !tbaa !184
  %11 = add nsw i64 %10, 1
  store i64 %11, ptr %9, align 8, !tbaa !184
  %12 = getelementptr inbounds %struct.FaSource, ptr %8, i64 0, i32 3
  %13 = load i64, ptr %12, align 8, !tbaa !185
  %14 = icmp sgt i64 %13, 0
  br i1 %14, label %15, label %25

15:                                               ; preds = %7
  %16 = getelementptr inbounds %struct.FaSource, ptr %8, i64 0, i32 2
  br label %17

17:                                               ; preds = %17, %15
  %18 = phi i64 [ 0, %15 ], [ %22, %17 ]
  %19 = load ptr, ptr %16, align 8, !tbaa !186
  %20 = getelementptr inbounds ptr, ptr %19, i64 %18
  %21 = load ptr, ptr %20, align 8, !tbaa !9
  tail call fastcc void @mark(ptr noundef %21) #35
  %22 = add nuw nsw i64 %18, 1
  %23 = load i64, ptr %12, align 8, !tbaa !185
  %24 = icmp slt i64 %22, %23
  br i1 %24, label %17, label %25, !llvm.loop !188

25:                                               ; preds = %17, %7
  %26 = getelementptr inbounds %struct.FaSource, ptr %8, i64 0, i32 5
  %27 = load ptr, ptr %26, align 8, !tbaa !173
  %28 = icmp eq ptr %27, null
  br i1 %28, label %29, label %7, !llvm.loop !189

29:                                               ; preds = %25, %3, %1
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @bump(ptr nocapture noundef %0) unnamed_addr #0 {
  %2 = getelementptr inbounds %struct.FaSource, ptr %0, i64 0, i32 1
  %3 = load i64, ptr %2, align 8, !tbaa !184
  %4 = add nsw i64 %3, 1
  store i64 %4, ptr %2, align 8, !tbaa !184
  %5 = getelementptr inbounds %struct.FaSource, ptr %0, i64 0, i32 3
  %6 = load i64, ptr %5, align 8, !tbaa !185
  %7 = icmp sgt i64 %6, 0
  br i1 %7, label %8, label %18

8:                                                ; preds = %1
  %9 = getelementptr inbounds %struct.FaSource, ptr %0, i64 0, i32 2
  br label %10

10:                                               ; preds = %10, %8
  %11 = phi i64 [ 0, %8 ], [ %15, %10 ]
  %12 = load ptr, ptr %9, align 8, !tbaa !186
  %13 = getelementptr inbounds ptr, ptr %12, i64 %11
  %14 = load ptr, ptr %13, align 8, !tbaa !9
  tail call fastcc void @mark(ptr noundef %14) #35
  %15 = add nuw nsw i64 %11, 1
  %16 = load i64, ptr %5, align 8, !tbaa !185
  %17 = icmp slt i64 %15, %16
  br i1 %17, label %10, label %18, !llvm.loop !188

18:                                               ; preds = %10, %1
  %19 = getelementptr inbounds %struct.FaSource, ptr %0, i64 0, i32 7
  %20 = load i64, ptr %19, align 8, !tbaa !165
  %21 = icmp sgt i64 %20, 0
  br i1 %21, label %22, label %24

22:                                               ; preds = %18
  %23 = getelementptr inbounds %struct.FaSource, ptr %0, i64 0, i32 6
  br label %25

24:                                               ; preds = %25, %18
  ret void

25:                                               ; preds = %22, %25
  %26 = phi i64 [ 0, %22 ], [ %30, %25 ]
  %27 = load ptr, ptr %23, align 8, !tbaa !167
  %28 = getelementptr inbounds %struct.FaChild, ptr %27, i64 %26, i32 2
  %29 = load ptr, ptr %28, align 8, !tbaa !172
  tail call fastcc void @bump(ptr noundef %29) #35
  %30 = add nuw nsw i64 %26, 1
  %31 = load i64, ptr %19, align 8, !tbaa !165
  %32 = icmp slt i64 %30, %31
  br i1 %32, label %25, label %24, !llvm.loop !190
}

; Function Attrs: nounwind optsize
define noundef ptr @fa_derived_new(ptr noundef %0, ptr noundef %1, ptr noundef %2, i64 noundef %3) local_unnamed_addr #0 {
  %5 = tail call fastcc ptr @observer_new(ptr noundef %0, ptr noundef null, ptr noundef %1, ptr noundef %2, i64 noundef %3) #35
  store i64 1, ptr %5, align 8, !tbaa !191
  %6 = getelementptr inbounds %struct.FaObserver, ptr %5, i64 0, i32 11
  store i32 1, ptr %6, align 8, !tbaa !192
  ret ptr %5
}

; Function Attrs: nounwind optsize
define internal fastcc noundef ptr @observer_new(ptr noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef %3, i64 noundef %4) unnamed_addr #0 {
  %6 = tail call ptr @fa_alloc(i64 noundef 208, i64 noundef 8) #34
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(208) %6, i8 0, i64 208, i1 false)
  %7 = getelementptr inbounds %struct.FaObserver, ptr %6, i64 0, i32 3
  store ptr %0, ptr %7, align 8, !tbaa !193
  %8 = getelementptr inbounds %struct.FaObserver, ptr %6, i64 0, i32 4
  store ptr %1, ptr %8, align 8, !tbaa !194
  %9 = getelementptr inbounds %struct.FaObserver, ptr %6, i64 0, i32 5
  store ptr %2, ptr %9, align 8, !tbaa !195
  %10 = getelementptr inbounds %struct.FaObserver, ptr %6, i64 0, i32 2
  store ptr %3, ptr %10, align 8, !tbaa !196
  %11 = icmp sgt i64 %4, 0
  br i1 %11, label %12, label %16

12:                                               ; preds = %5
  %13 = tail call ptr @fa_alloc(i64 noundef %4, i64 noundef 16) #34
  %14 = getelementptr inbounds %struct.FaObserver, ptr %6, i64 0, i32 1
  store ptr %13, ptr %14, align 8, !tbaa !197
  tail call void @llvm.memset.p0.i64(ptr align 1 %13, i8 0, i64 %4, i1 false)
  %15 = getelementptr inbounds %struct.FaObserver, ptr %6, i64 0, i32 19
  store i64 %4, ptr %15, align 8, !tbaa !198
  br label %16

16:                                               ; preds = %12, %5
  %17 = load ptr, ptr @reactive_made, align 8, !tbaa !9
  %18 = load i64, ptr @reactive_made_count, align 8, !tbaa !5
  %19 = tail call fastcc ptr @grow_items(ptr noundef %17, i64 noundef %18, ptr noundef nonnull @reactive_made_capacity, i64 noundef 8) #35
  store ptr %19, ptr @reactive_made, align 8, !tbaa !9
  %20 = load i64, ptr @reactive_made_count, align 8, !tbaa !5
  %21 = getelementptr inbounds ptr, ptr %19, i64 %20
  store ptr %6, ptr %21, align 8, !tbaa !9
  %22 = add nsw i64 %20, 1
  store i64 %22, ptr @reactive_made_count, align 8, !tbaa !5
  ret ptr %6
}

; Function Attrs: nounwind optsize
define ptr @fa_derived_read(ptr noundef %0) local_unnamed_addr #0 {
  tail call fastcc void @refresh(ptr noundef %0) #35
  tail call void @fa_track(ptr noundef %0) #35
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 1
  %3 = load ptr, ptr %2, align 8, !tbaa !197
  ret ptr %3
}

; Function Attrs: nounwind optsize
define internal fastcc void @refresh(ptr noundef %0) unnamed_addr #0 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 11
  %3 = load i32, ptr %2, align 8, !tbaa !192
  %4 = icmp eq i32 %3, 0
  br i1 %4, label %29, label %5

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 12
  %7 = load i32, ptr %6, align 4, !tbaa !199
  %8 = icmp eq i32 %7, 0
  br i1 %8, label %13, label %9

9:                                                ; preds = %5
  %10 = tail call fastcc i32 @outdated(ptr noundef nonnull %0) #35
  %11 = icmp eq i32 %10, 0
  br i1 %11, label %12, label %13

12:                                               ; preds = %9
  store i32 0, ptr %2, align 8, !tbaa !192
  br label %29

13:                                               ; preds = %9, %5
  %14 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 13
  %15 = load i32, ptr %14, align 8, !tbaa !200
  %16 = icmp eq i32 %15, 0
  br i1 %16, label %18, label %17

17:                                               ; preds = %13
  tail call fastcc void @reactive_stop(ptr noundef nonnull @.str.41) #37
  unreachable

18:                                               ; preds = %13
  tail call fastcc void @unsubscribe(ptr noundef nonnull %0) #35
  %19 = load ptr, ptr @reactive_observer, align 8, !tbaa !9
  store ptr %0, ptr @reactive_observer, align 8, !tbaa !9
  store i32 1, ptr %14, align 8, !tbaa !200
  %20 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 16
  store i32 1, ptr %20, align 4, !tbaa !201
  %21 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 3
  %22 = load ptr, ptr %21, align 8, !tbaa !193
  tail call void %22(ptr noundef nonnull %0) #34
  store i32 0, ptr %14, align 8, !tbaa !200
  store ptr %19, ptr @reactive_observer, align 8, !tbaa !9
  store i32 0, ptr %2, align 8, !tbaa !192
  store i32 1, ptr %6, align 4, !tbaa !199
  %23 = load i32, ptr %20, align 4, !tbaa !201
  %24 = icmp eq i32 %23, 0
  br i1 %24, label %29, label %25

25:                                               ; preds = %18
  %26 = getelementptr inbounds %struct.FaSource, ptr %0, i64 0, i32 1
  %27 = load i64, ptr %26, align 8, !tbaa !202
  %28 = add nsw i64 %27, 1
  store i64 %28, ptr %26, align 8, !tbaa !202
  br label %29

29:                                               ; preds = %18, %25, %1, %12
  ret void
}

; Function Attrs: nounwind optsize
define noundef ptr @fa_watch_new(ptr noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef %3) local_unnamed_addr #0 {
  %5 = tail call fastcc ptr @observer_new(ptr noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef %3, i64 noundef 0) #35
  store i64 2, ptr %5, align 8, !tbaa !191
  %6 = getelementptr inbounds %struct.FaObserver, ptr %5, i64 0, i32 17
  store i32 1, ptr %6, align 8, !tbaa !203
  %7 = load i64, ptr @reactive_sequence, align 8, !tbaa !5
  %8 = add nsw i64 %7, 1
  store i64 %8, ptr @reactive_sequence, align 8, !tbaa !5
  %9 = getelementptr inbounds %struct.FaObserver, ptr %5, i64 0, i32 10
  store i64 %8, ptr %9, align 8, !tbaa !204
  tail call fastcc void @unsubscribe(ptr noundef nonnull %5) #35
  %10 = load ptr, ptr @reactive_observer, align 8, !tbaa !9
  store ptr null, ptr @reactive_observer, align 8, !tbaa !9
  %11 = getelementptr inbounds %struct.FaObserver, ptr %5, i64 0, i32 4
  %12 = load ptr, ptr %11, align 8, !tbaa !194
  tail call void %12(ptr noundef nonnull %5) #34
  store ptr %5, ptr @reactive_observer, align 8, !tbaa !9
  %13 = getelementptr inbounds %struct.FaObserver, ptr %5, i64 0, i32 3
  %14 = load ptr, ptr %13, align 8, !tbaa !193
  tail call void %14(ptr noundef nonnull %5) #34
  store ptr %10, ptr @reactive_observer, align 8, !tbaa !9
  ret ptr %5
}

; Function Attrs: nounwind optsize
define void @fa_dispose(ptr noundef %0) local_unnamed_addr #0 {
  %2 = icmp eq ptr %0, null
  br i1 %2, label %11, label %3

3:                                                ; preds = %1
  %4 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 15
  %5 = load i32, ptr %4, align 8, !tbaa !205
  %6 = icmp eq i32 %5, 0
  br i1 %6, label %7, label %11

7:                                                ; preds = %3
  store i32 1, ptr %4, align 8, !tbaa !205
  tail call fastcc void @unsubscribe(ptr noundef nonnull %0) #35
  %8 = load ptr, ptr @reactive_observer, align 8, !tbaa !9
  store ptr null, ptr @reactive_observer, align 8, !tbaa !9
  %9 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 4
  %10 = load ptr, ptr %9, align 8, !tbaa !194
  tail call void %10(ptr noundef nonnull %0) #34
  store ptr %8, ptr @reactive_observer, align 8, !tbaa !9
  br label %11

11:                                               ; preds = %1, %3, %7
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc void @unsubscribe(ptr noundef %0) unnamed_addr #10 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 8
  %3 = load i64, ptr %2, align 8, !tbaa !178
  %4 = icmp sgt i64 %3, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 6
  br label %8

7:                                                ; preds = %33, %1
  store i64 0, ptr %2, align 8, !tbaa !178
  ret void

8:                                                ; preds = %5, %33
  %9 = phi i64 [ 0, %5 ], [ %34, %33 ]
  %10 = load ptr, ptr %6, align 8, !tbaa !180
  %11 = getelementptr inbounds ptr, ptr %10, i64 %9
  %12 = load ptr, ptr %11, align 8, !tbaa !9
  %13 = getelementptr inbounds %struct.FaSource, ptr %12, i64 0, i32 3
  %14 = load i64, ptr %13, align 8, !tbaa !185
  %15 = icmp sgt i64 %14, 0
  br i1 %15, label %16, label %33

16:                                               ; preds = %8
  %17 = getelementptr inbounds %struct.FaSource, ptr %12, i64 0, i32 2
  %18 = load ptr, ptr %17, align 8, !tbaa !186
  br label %19

19:                                               ; preds = %16, %30
  %20 = phi i64 [ 0, %16 ], [ %31, %30 ]
  %21 = getelementptr inbounds ptr, ptr %18, i64 %20
  %22 = load ptr, ptr %21, align 8, !tbaa !9
  %23 = icmp eq ptr %22, %0
  br i1 %23, label %24, label %30

24:                                               ; preds = %19
  %25 = getelementptr inbounds ptr, ptr %18, i64 %20
  %26 = getelementptr ptr, ptr %18, i64 %14
  %27 = getelementptr ptr, ptr %26, i64 -1
  %28 = load ptr, ptr %27, align 8, !tbaa !9
  store ptr %28, ptr %25, align 8, !tbaa !9
  %29 = add nsw i64 %14, -1
  store i64 %29, ptr %13, align 8, !tbaa !185
  br label %33

30:                                               ; preds = %19
  %31 = add nuw nsw i64 %20, 1
  %32 = icmp eq i64 %31, %14
  br i1 %32, label %33, label %19, !llvm.loop !206

33:                                               ; preds = %30, %8, %24
  %34 = add nuw nsw i64 %9, 1
  %35 = icmp eq i64 %34, %3
  br i1 %35, label %7, label %8, !llvm.loop !207
}

; Function Attrs: nounwind optsize
define void @fa_reactive_flush() local_unnamed_addr #0 {
  %1 = load i1, ptr @reactive_flushing, align 4
  %2 = load i64, ptr @reactive_pending_count, align 8
  %3 = icmp eq i64 %2, 0
  %4 = select i1 %1, i1 true, i1 %3
  br i1 %4, label %65, label %5

5:                                                ; preds = %0
  store i1 true, ptr @reactive_flushing, align 4
  %6 = icmp sgt i64 %2, 0
  br i1 %6, label %7, label %64

7:                                                ; preds = %5, %41
  %8 = phi i32 [ %10, %41 ], [ 0, %5 ]
  %9 = phi i64 [ %42, %41 ], [ %2, %5 ]
  %10 = add nuw nsw i32 %8, 1
  %11 = icmp eq i32 %8, 100
  br i1 %11, label %12, label %13

12:                                               ; preds = %7
  tail call fastcc void @reactive_stop(ptr noundef nonnull @.str.15) #37
  unreachable

13:                                               ; preds = %7
  %14 = shl i64 %9, 3
  %15 = tail call ptr @fa_alloc(i64 noundef %14, i64 noundef 8) #34
  %16 = load ptr, ptr @reactive_pending, align 8, !tbaa !9
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %15, ptr align 1 %16, i64 %14, i1 false)
  store i64 0, ptr @reactive_pending_count, align 8, !tbaa !5
  %17 = icmp ugt i64 %9, 1
  br i1 %17, label %19, label %18

18:                                               ; preds = %36, %13
  br label %44

19:                                               ; preds = %13, %36
  %20 = phi i64 [ %39, %36 ], [ 1, %13 ]
  %21 = getelementptr inbounds ptr, ptr %15, i64 %20
  %22 = load ptr, ptr %21, align 8, !tbaa !9
  %23 = getelementptr inbounds %struct.FaObserver, ptr %22, i64 0, i32 10
  %24 = load i64, ptr %23, align 8, !tbaa !204
  br label %25

25:                                               ; preds = %19, %33
  %26 = phi i64 [ %20, %19 ], [ %27, %33 ]
  %27 = add nsw i64 %26, -1
  %28 = getelementptr inbounds ptr, ptr %15, i64 %27
  %29 = load ptr, ptr %28, align 8, !tbaa !9
  %30 = getelementptr inbounds %struct.FaObserver, ptr %29, i64 0, i32 10
  %31 = load i64, ptr %30, align 8, !tbaa !204
  %32 = icmp sgt i64 %31, %24
  br i1 %32, label %33, label %36

33:                                               ; preds = %25
  %34 = getelementptr inbounds ptr, ptr %15, i64 %26
  store ptr %29, ptr %34, align 8, !tbaa !9
  %35 = icmp sgt i64 %26, 1
  br i1 %35, label %25, label %36, !llvm.loop !208

36:                                               ; preds = %33, %25
  %37 = phi i64 [ 0, %33 ], [ %26, %25 ]
  %38 = getelementptr inbounds ptr, ptr %15, i64 %37
  store ptr %22, ptr %38, align 8, !tbaa !9
  %39 = add nuw nsw i64 %20, 1
  %40 = icmp eq i64 %39, %9
  br i1 %40, label %18, label %19, !llvm.loop !209

41:                                               ; preds = %61
  tail call void @fa_free(ptr noundef nonnull %15, i64 noundef %14, i64 noundef 8) #34
  %42 = load i64, ptr @reactive_pending_count, align 8, !tbaa !5
  %43 = icmp sgt i64 %42, 0
  br i1 %43, label %7, label %64, !llvm.loop !210

44:                                               ; preds = %18, %61
  %45 = phi i64 [ %62, %61 ], [ 0, %18 ]
  %46 = getelementptr inbounds ptr, ptr %15, i64 %45
  %47 = load ptr, ptr %46, align 8, !tbaa !9
  %48 = getelementptr inbounds %struct.FaObserver, ptr %47, i64 0, i32 14
  store i32 0, ptr %48, align 4, !tbaa !211
  %49 = getelementptr inbounds %struct.FaObserver, ptr %47, i64 0, i32 15
  %50 = load i32, ptr %49, align 8, !tbaa !205
  %51 = icmp eq i32 %50, 0
  br i1 %51, label %52, label %61

52:                                               ; preds = %44
  %53 = tail call fastcc i32 @outdated(ptr noundef nonnull %47) #35
  %54 = icmp eq i32 %53, 0
  br i1 %54, label %61, label %55

55:                                               ; preds = %52
  tail call fastcc void @unsubscribe(ptr noundef nonnull %47) #35
  %56 = load ptr, ptr @reactive_observer, align 8, !tbaa !9
  store ptr null, ptr @reactive_observer, align 8, !tbaa !9
  %57 = getelementptr inbounds %struct.FaObserver, ptr %47, i64 0, i32 4
  %58 = load ptr, ptr %57, align 8, !tbaa !194
  tail call void %58(ptr noundef nonnull %47) #34
  store ptr %47, ptr @reactive_observer, align 8, !tbaa !9
  %59 = getelementptr inbounds %struct.FaObserver, ptr %47, i64 0, i32 3
  %60 = load ptr, ptr %59, align 8, !tbaa !193
  tail call void %60(ptr noundef nonnull %47) #34
  store ptr %56, ptr @reactive_observer, align 8, !tbaa !9
  br label %61

61:                                               ; preds = %55, %52, %44
  %62 = add nuw i64 %45, 1
  %63 = icmp eq i64 %62, %9
  br i1 %63, label %41, label %44, !llvm.loop !212

64:                                               ; preds = %41, %5
  store i1 false, ptr @reactive_flushing, align 4
  br label %65

65:                                               ; preds = %0, %64
  ret void
}

; Function Attrs: noreturn nounwind optsize
define internal fastcc void @reactive_stop(ptr noundef %0) unnamed_addr #2 {
  tail call void @fa_flush() #35
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.42, i64 noundef 6) #34
  br label %2

2:                                                ; preds = %2, %1
  %3 = phi i64 [ 0, %1 ], [ %7, %2 ]
  %4 = getelementptr inbounds i8, ptr %0, i64 %3
  %5 = load i8, ptr %4, align 1, !tbaa !12
  %6 = icmp eq i8 %5, 0
  %7 = add i64 %3, 1
  br i1 %6, label %8, label %2, !llvm.loop !24

8:                                                ; preds = %2
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull %0, i64 noundef %3) #34
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.22, i64 noundef 1) #34
  tail call void @fa_exit(i32 noundef 101) #36
  unreachable
}

; Function Attrs: nounwind optsize
define internal fastcc noundef i32 @outdated(ptr nocapture noundef readonly %0) unnamed_addr #0 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 8
  %3 = load i64, ptr %2, align 8, !tbaa !178
  %4 = icmp sgt i64 %3, 0
  br i1 %4, label %5, label %29

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 6
  %7 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 7
  br label %12

8:                                                ; preds = %20
  %9 = add nuw nsw i64 %13, 1
  %10 = load i64, ptr %2, align 8, !tbaa !178
  %11 = icmp slt i64 %9, %10
  br i1 %11, label %12, label %27, !llvm.loop !213

12:                                               ; preds = %5, %8
  %13 = phi i64 [ 0, %5 ], [ %9, %8 ]
  %14 = load ptr, ptr %6, align 8, !tbaa !180
  %15 = getelementptr inbounds ptr, ptr %14, i64 %13
  %16 = load ptr, ptr %15, align 8, !tbaa !9
  %17 = load i64, ptr %16, align 8, !tbaa !214
  %18 = icmp eq i64 %17, 1
  br i1 %18, label %19, label %20

19:                                               ; preds = %12
  tail call fastcc void @refresh(ptr noundef nonnull %16) #35
  br label %20

20:                                               ; preds = %19, %12
  %21 = getelementptr inbounds %struct.FaSource, ptr %16, i64 0, i32 1
  %22 = load i64, ptr %21, align 8, !tbaa !184
  %23 = load ptr, ptr %7, align 8, !tbaa !183
  %24 = getelementptr inbounds i64, ptr %23, i64 %13
  %25 = load i64, ptr %24, align 8, !tbaa !5
  %26 = icmp ne i64 %22, %25
  br i1 %26, label %27, label %8

27:                                               ; preds = %8, %20
  %28 = zext i1 %26 to i32
  br label %29

29:                                               ; preds = %27, %1
  %30 = phi i32 [ 0, %1 ], [ %28, %27 ]
  ret i32 %30
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read)
define ptr @fa_observer_value(ptr nocapture noundef readonly %0) local_unnamed_addr #20 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 1
  %3 = load ptr, ptr %2, align 8, !tbaa !197
  ret ptr %3
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read)
define ptr @fa_observer_env(ptr nocapture noundef readonly %0) local_unnamed_addr #20 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 2
  %3 = load ptr, ptr %2, align 8, !tbaa !196
  ret ptr %3
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(none)
define nonnull ptr @fa_observer_cleanup(ptr noundef readnone %0) local_unnamed_addr #11 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 20
  ret ptr %2
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read)
define i32 @fa_observer_computed(ptr nocapture noundef readonly %0) local_unnamed_addr #20 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 12
  %3 = load i32, ptr %2, align 4, !tbaa !199
  ret i32 %3
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: write)
define void @fa_observer_changed(ptr nocapture noundef writeonly %0, i32 noundef %1) local_unnamed_addr #21 {
  %3 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 16
  store i32 %1, ptr %3, align 4, !tbaa !201
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_reactive_release() local_unnamed_addr #0 {
  %1 = load i64, ptr @reactive_made_count, align 8, !tbaa !5
  %2 = icmp sgt i64 %1, 0
  br i1 %2, label %22, label %3

3:                                                ; preds = %69, %0
  %4 = load i64, ptr @reactive_made_capacity, align 8, !tbaa !5
  %5 = icmp sgt i64 %4, 0
  br i1 %5, label %6, label %9

6:                                                ; preds = %3
  %7 = load ptr, ptr @reactive_made, align 8, !tbaa !9
  %8 = shl i64 %4, 3
  tail call void @fa_free(ptr noundef %7, i64 noundef %8, i64 noundef 8) #34
  br label %9

9:                                                ; preds = %3, %6
  %10 = load i64, ptr @reactive_pending_capacity, align 8, !tbaa !5
  %11 = icmp sgt i64 %10, 0
  br i1 %11, label %12, label %15

12:                                               ; preds = %9
  %13 = load ptr, ptr @reactive_pending, align 8, !tbaa !9
  %14 = shl i64 %10, 3
  tail call void @fa_free(ptr noundef %13, i64 noundef %14, i64 noundef 8) #34
  br label %15

15:                                               ; preds = %9, %12
  %16 = load i64, ptr @field_source_capacity, align 8, !tbaa !5
  %17 = icmp sgt i64 %16, 0
  br i1 %17, label %18, label %21

18:                                               ; preds = %15
  %19 = load ptr, ptr @field_sources, align 8, !tbaa !9
  %20 = mul i64 %16, 24
  tail call void @fa_free(ptr noundef %19, i64 noundef %20, i64 noundef 8) #34
  br label %21

21:                                               ; preds = %15, %18
  store i64 0, ptr @reactive_made_count, align 8, !tbaa !5
  ret void

22:                                               ; preds = %0, %69
  %23 = phi i64 [ %24, %69 ], [ %1, %0 ]
  %24 = add nsw i64 %23, -1
  %25 = load ptr, ptr @reactive_made, align 8, !tbaa !9
  %26 = getelementptr inbounds ptr, ptr %25, i64 %24
  %27 = load ptr, ptr %26, align 8, !tbaa !9
  %28 = getelementptr inbounds %struct.FaSource, ptr %27, i64 0, i32 4
  %29 = load i64, ptr %28, align 8, !tbaa !215
  %30 = icmp sgt i64 %29, 0
  br i1 %30, label %31, label %35

31:                                               ; preds = %22
  %32 = getelementptr inbounds %struct.FaSource, ptr %27, i64 0, i32 2
  %33 = load ptr, ptr %32, align 8, !tbaa !186
  %34 = shl i64 %29, 3
  tail call void @fa_free(ptr noundef %33, i64 noundef %34, i64 noundef 8) #34
  br label %35

35:                                               ; preds = %22, %31
  %36 = getelementptr inbounds %struct.FaSource, ptr %27, i64 0, i32 8
  %37 = load i64, ptr %36, align 8, !tbaa !216
  %38 = icmp sgt i64 %37, 0
  br i1 %38, label %39, label %43

39:                                               ; preds = %35
  %40 = getelementptr inbounds %struct.FaSource, ptr %27, i64 0, i32 6
  %41 = load ptr, ptr %40, align 8, !tbaa !167
  %42 = mul i64 %37, 24
  tail call void @fa_free(ptr noundef %41, i64 noundef %42, i64 noundef 8) #34
  br label %43

43:                                               ; preds = %35, %39
  %44 = load i64, ptr %27, align 8, !tbaa !214
  %45 = icmp eq i64 %44, 0
  br i1 %45, label %69, label %46

46:                                               ; preds = %43
  %47 = getelementptr inbounds %struct.FaObserver, ptr %27, i64 0, i32 5
  %48 = load ptr, ptr %47, align 8, !tbaa !195
  tail call void %48(ptr noundef nonnull %27) #34
  %49 = getelementptr inbounds %struct.FaObserver, ptr %27, i64 0, i32 9
  %50 = load i64, ptr %49, align 8, !tbaa !182
  %51 = icmp sgt i64 %50, 0
  br i1 %51, label %52, label %62

52:                                               ; preds = %46
  %53 = getelementptr inbounds %struct.FaObserver, ptr %27, i64 0, i32 6
  %54 = load ptr, ptr %53, align 8, !tbaa !180
  %55 = shl i64 %50, 3
  tail call void @fa_free(ptr noundef %54, i64 noundef %55, i64 noundef 8) #34
  %56 = load i64, ptr %49, align 8, !tbaa !182
  %57 = icmp sgt i64 %56, 0
  br i1 %57, label %58, label %62

58:                                               ; preds = %52
  %59 = getelementptr inbounds %struct.FaObserver, ptr %27, i64 0, i32 7
  %60 = load ptr, ptr %59, align 8, !tbaa !183
  %61 = shl i64 %56, 3
  tail call void @fa_free(ptr noundef %60, i64 noundef %61, i64 noundef 8) #34
  br label %62

62:                                               ; preds = %46, %52, %58
  %63 = getelementptr inbounds %struct.FaObserver, ptr %27, i64 0, i32 19
  %64 = load i64, ptr %63, align 8, !tbaa !198
  %65 = icmp sgt i64 %64, 0
  br i1 %65, label %66, label %69

66:                                               ; preds = %62
  %67 = getelementptr inbounds %struct.FaObserver, ptr %27, i64 0, i32 1
  %68 = load ptr, ptr %67, align 8, !tbaa !197
  tail call void @fa_free(ptr noundef %68, i64 noundef %64, i64 noundef 16) #34
  br label %69

69:                                               ; preds = %62, %66, %43
  %70 = phi i64 [ 72, %43 ], [ 208, %66 ], [ 208, %62 ]
  tail call void @fa_free(ptr noundef nonnull %27, i64 noundef %70, i64 noundef 8) #34
  %71 = icmp ugt i64 %23, 1
  br i1 %71, label %22, label %3, !llvm.loop !217
}

; Function Attrs: cold noinline noreturn nounwind optsize
define void @fa_overflow(i64 noundef %0, i64 noundef %1, i32 noundef %2, ptr noundef %3) local_unnamed_addr #22 {
  %5 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %5, align 8, !tbaa !5
  tail call fastcc void @append_c(ptr noundef nonnull @.str.16) #35
  %6 = icmp eq i32 %2, 0
  br i1 %6, label %20, label %7

7:                                                ; preds = %4
  %8 = zext i64 %1 to i128
  %9 = shl nuw i128 %8, 64
  %10 = zext i64 %0 to i128
  %11 = or disjoint i128 %9, %10
  %12 = icmp slt i64 %1, 0
  %13 = lshr i64 %1, 63
  %14 = trunc i64 %13 to i32
  %15 = sub i128 0, %11
  %16 = select i1 %12, i128 %15, i128 %11
  %17 = trunc i128 %16 to i64
  %18 = lshr i128 %16, 64
  %19 = trunc i128 %18 to i64
  tail call fastcc void @append_u128(i64 noundef %17, i64 noundef %19, i32 noundef %14) #35
  br label %21

20:                                               ; preds = %4
  tail call fastcc void @append_u128(i64 noundef %0, i64 noundef %1, i32 noundef 0) #35
  br label %21

21:                                               ; preds = %20, %7
  tail call fastcc void @append_c(ptr noundef nonnull @.str.17) #35
  tail call fastcc void @append_c(ptr noundef %3) #35
  br label %22

22:                                               ; preds = %22, %21
  %23 = phi i64 [ 0, %21 ], [ %27, %22 ]
  %24 = getelementptr inbounds i8, ptr %3, i64 %23
  %25 = load i8, ptr %24, align 1, !tbaa !12
  %26 = icmp eq i8 %25, 0
  %27 = add i64 %23, 1
  br i1 %26, label %28, label %22, !llvm.loop !24

28:                                               ; preds = %22
  %29 = getelementptr inbounds i8, ptr %3, i64 %23
  %30 = getelementptr inbounds i8, ptr %29, i64 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %30) #37
  unreachable
}

; Function Attrs: noreturn nounwind optsize
define internal fastcc void @stop_with_builder(ptr noundef %0) unnamed_addr #2 {
  tail call void @fa_flush() #35
  %2 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %3 = load ptr, ptr %2, align 8, !tbaa !9
  %4 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %5 = load i64, ptr %4, align 8, !tbaa !5
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.42, i64 noundef 6) #34
  tail call void @fa_write(i32 noundef 2, ptr noundef %3, i64 noundef %5) #34
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.43, i64 noundef 6) #34
  br label %6

6:                                                ; preds = %6, %1
  %7 = phi i64 [ 0, %1 ], [ %11, %6 ]
  %8 = getelementptr inbounds i8, ptr %0, i64 %7
  %9 = load i8, ptr %8, align 1, !tbaa !12
  %10 = icmp eq i8 %9, 0
  %11 = add i64 %7, 1
  br i1 %10, label %12, label %6, !llvm.loop !24

12:                                               ; preds = %6
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull %0, i64 noundef %7) #34
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.22, i64 noundef 1) #34
  tail call void @fa_exit(i32 noundef 101) #36
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define void @fa_overflow_unknown(ptr noundef %0) local_unnamed_addr #22 {
  %2 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %2, align 8, !tbaa !5
  tail call fastcc void @append_c(ptr noundef nonnull @.str.18) #35
  tail call fastcc void @append_c(ptr noundef %0) #35
  br label %3

3:                                                ; preds = %3, %1
  %4 = phi i64 [ 0, %1 ], [ %8, %3 ]
  %5 = getelementptr inbounds i8, ptr %0, i64 %4
  %6 = load i8, ptr %5, align 1, !tbaa !12
  %7 = icmp eq i8 %6, 0
  %8 = add i64 %4, 1
  br i1 %7, label %9, label %3, !llvm.loop !24

9:                                                ; preds = %3
  %10 = getelementptr inbounds i8, ptr %0, i64 %4
  %11 = getelementptr inbounds i8, ptr %10, i64 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %11) #37
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define void @fa_float_overflow(double noundef %0, ptr noundef %1) local_unnamed_addr #22 {
  %3 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %3, align 8, !tbaa !5
  tail call fastcc void @append_c(ptr noundef nonnull @.str.16) #35
  tail call void @fa_b_f64(double noundef %0) #35
  tail call fastcc void @append_c(ptr noundef nonnull @.str.17) #35
  tail call fastcc void @append_c(ptr noundef %1) #35
  br label %4

4:                                                ; preds = %4, %2
  %5 = phi i64 [ 0, %2 ], [ %9, %4 ]
  %6 = getelementptr inbounds i8, ptr %1, i64 %5
  %7 = load i8, ptr %6, align 1, !tbaa !12
  %8 = icmp eq i8 %7, 0
  %9 = add i64 %5, 1
  br i1 %8, label %10, label %4, !llvm.loop !24

10:                                               ; preds = %4
  %11 = getelementptr inbounds i8, ptr %1, i64 %5
  %12 = getelementptr inbounds i8, ptr %11, i64 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %12) #37
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define void @fa_assert_failed(ptr noundef %0) local_unnamed_addr #22 {
  %2 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %2, align 8, !tbaa !5
  tail call fastcc void @append_c(ptr noundef nonnull @.str.19) #35
  tail call fastcc void @append_c(ptr noundef %0) #35
  br label %3

3:                                                ; preds = %3, %1
  %4 = phi i64 [ 0, %1 ], [ %8, %3 ]
  %5 = getelementptr inbounds i8, ptr %0, i64 %4
  %6 = load i8, ptr %5, align 1, !tbaa !12
  %7 = icmp eq i8 %6, 0
  %8 = add i64 %4, 1
  br i1 %7, label %9, label %3, !llvm.loop !24

9:                                                ; preds = %3
  %10 = getelementptr inbounds i8, ptr %0, i64 %4
  %11 = getelementptr inbounds i8, ptr %10, i64 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %11) #37
  unreachable
}

; Function Attrs: nounwind optsize
define void @fa_guard_stack() local_unnamed_addr #0 {
  %1 = alloca %struct.AlternateStack, align 8
  %2 = alloca %struct.SignalAction, align 8
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %1) #31
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %1, ptr noundef nonnull align 8 dereferenceable(24) @__const.fa_guard_stack.stack, i64 24, i1 false)
  %3 = call i32 @sigaltstack(ptr noundef nonnull %1, ptr noundef null) #34
  call void @llvm.lifetime.start.p0(i64 152, ptr nonnull %2) #31
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(152) %2, ptr noundef nonnull align 8 dereferenceable(152) @__const.fa_guard_stack.action, i64 152, i1 false)
  %4 = call i32 @sigaction(i32 noundef 11, ptr noundef nonnull %2, ptr noundef null) #34
  call void @llvm.lifetime.end.p0(i64 152, ptr nonnull %2) #31
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %1) #31
  ret void
}

; Function Attrs: optsize
declare i32 @sigaltstack(ptr noundef, ptr noundef) local_unnamed_addr #1

; Function Attrs: noreturn nounwind optsize
define internal void @on_fault(i32 %0) #2 {
  tail call void @fa_flush() #35
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @on_fault.message, i64 noundef 21) #34
  tail call void @_exit(i32 noundef 101) #36
  unreachable
}

; Function Attrs: optsize
declare i32 @sigaction(i32 noundef, ptr noundef, ptr noundef) local_unnamed_addr #1

; Function Attrs: cold noinline noreturn nounwind optsize
define void @fa_assert_failed_operands(ptr noundef %0) local_unnamed_addr #22 {
  %2 = alloca %struct.FaString, align 8
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %2) #31
  call void @fa_b_take(ptr noundef nonnull %2) #35
  tail call fastcc void @append_c(ptr noundef nonnull @.str.19) #35
  tail call fastcc void @append_c(ptr noundef %0) #35
  tail call fastcc void @append_c(ptr noundef nonnull @.str.20) #35
  %3 = load ptr, ptr %2, align 8, !tbaa !36
  %4 = getelementptr inbounds %struct.FaString, ptr %2, i64 0, i32 1
  %5 = load i64, ptr %4, align 8, !tbaa !38
  tail call void @fa_b_bytes(ptr noundef %3, i64 noundef %5) #35
  br label %6

6:                                                ; preds = %6, %1
  %7 = phi i64 [ 0, %1 ], [ %11, %6 ]
  %8 = getelementptr inbounds i8, ptr %0, i64 %7
  %9 = load i8, ptr %8, align 1, !tbaa !12
  %10 = icmp eq i8 %9, 0
  %11 = add i64 %7, 1
  br i1 %10, label %12, label %6, !llvm.loop !24

12:                                               ; preds = %6
  %13 = getelementptr inbounds i8, ptr %0, i64 %7
  %14 = getelementptr inbounds i8, ptr %13, i64 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %14) #37
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define void @fa_uncaught(ptr noundef %0) local_unnamed_addr #22 {
  %2 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %2, align 8, !tbaa !5
  tail call void @fa_format_union(ptr noundef %0, i32 noundef 1) #34
  tail call void @fa_flush() #35
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.21, i64 noundef 7) #34
  %3 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %4 = load ptr, ptr %3, align 8, !tbaa !9
  %5 = load i64, ptr %2, align 8, !tbaa !5
  tail call void @fa_write(i32 noundef 2, ptr noundef %4, i64 noundef %5) #34
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.22, i64 noundef 1) #34
  tail call void @fa_exit(i32 noundef 1) #36
  unreachable
}

; Function Attrs: optsize
declare void @fa_format_union(ptr noundef, i32 noundef) local_unnamed_addr #1

; Function Attrs: cold mustprogress nofree noinline norecurse nosync nounwind optsize willreturn memory(none)
define noundef nonnull ptr @fa_error_division_by_zero() local_unnamed_addr #23 {
  ret ptr @division_by_zero
}

; Function Attrs: cold noinline nounwind optsize
define noundef ptr @fa_error_conversion(i64 noundef %0, i64 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3) local_unnamed_addr #12 {
  %5 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %5, align 8, !tbaa !5
  %6 = icmp eq i32 %2, 0
  br i1 %6, label %20, label %7

7:                                                ; preds = %4
  %8 = zext i64 %1 to i128
  %9 = shl nuw i128 %8, 64
  %10 = zext i64 %0 to i128
  %11 = or disjoint i128 %9, %10
  %12 = icmp slt i64 %1, 0
  %13 = lshr i64 %1, 63
  %14 = trunc i64 %13 to i32
  %15 = sub i128 0, %11
  %16 = select i1 %12, i128 %15, i128 %11
  %17 = trunc i128 %16 to i64
  %18 = lshr i128 %16, 64
  %19 = trunc i128 %18 to i64
  tail call fastcc void @append_u128(i64 noundef %17, i64 noundef %19, i32 noundef %14) #35
  br label %21

20:                                               ; preds = %4
  tail call fastcc void @append_u128(i64 noundef %0, i64 noundef %1, i32 noundef 0) #35
  br label %21

21:                                               ; preds = %20, %7
  tail call fastcc void @append_c(ptr noundef nonnull @.str.17) #35
  tail call fastcc void @append_c(ptr noundef %3) #35
  %22 = tail call ptr @fa_alloc(i64 noundef 32, i64 noundef 8) #34
  store i64 4, ptr %22, align 8, !tbaa !218
  %23 = getelementptr inbounds %struct.MessageError, ptr %22, i64 0, i32 1
  tail call void @fa_b_take(ptr noundef nonnull %23) #35
  ret ptr %22
}

; Function Attrs: cold noinline nounwind optsize
define noundef ptr @fa_error_negative_exponent(i64 noundef %0) local_unnamed_addr #12 {
  %2 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %2, align 8, !tbaa !5
  tail call fastcc void @append_c(ptr noundef nonnull @.str.23) #35
  %3 = tail call i64 @llvm.abs.i64(i64 %0, i1 false)
  %4 = lshr i64 %0, 63
  %5 = trunc i64 %4 to i32
  tail call fastcc void @append_u64(i64 noundef %3, i32 noundef %5) #35
  %6 = tail call ptr @fa_alloc(i64 noundef 32, i64 noundef 8) #34
  store i64 4, ptr %6, align 8, !tbaa !218
  %7 = getelementptr inbounds %struct.MessageError, ptr %6, i64 0, i32 1
  tail call void @fa_b_take(ptr noundef nonnull %7) #35
  ret ptr %6
}

; Function Attrs: cold noinline nounwind optsize
define noundef ptr @fa_error_float(double noundef %0, i32 noundef %1) local_unnamed_addr #12 {
  %3 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %3, align 8, !tbaa !5
  tail call fastcc void @append_double(double noundef %0) #35
  %4 = icmp eq i32 %1, 0
  %5 = select i1 %4, ptr @.str.25, ptr @.str.24
  tail call fastcc void @append_c(ptr noundef nonnull %5) #35
  %6 = tail call ptr @fa_alloc(i64 noundef 32, i64 noundef 8) #34
  store i64 4, ptr %6, align 8, !tbaa !218
  %7 = getelementptr inbounds %struct.MessageError, ptr %6, i64 0, i32 1
  tail call void @fa_b_take(ptr noundef nonnull %7) #35
  ret ptr %6
}

; Function Attrs: cold noinline nounwind optsize
define noundef ptr @fa_error_shift(i64 noundef %0, i64 noundef %1) local_unnamed_addr #12 {
  %3 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %3, align 8, !tbaa !5
  tail call fastcc void @append_c(ptr noundef nonnull @.str.26) #35
  %4 = tail call i64 @llvm.abs.i64(i64 %0, i1 false)
  %5 = lshr i64 %0, 63
  %6 = trunc i64 %5 to i32
  tail call fastcc void @append_u64(i64 noundef %4, i32 noundef %6) #35
  tail call fastcc void @append_c(ptr noundef nonnull @.str.27) #35
  %7 = tail call i64 @llvm.abs.i64(i64 %1, i1 false)
  %8 = lshr i64 %1, 63
  %9 = trunc i64 %8 to i32
  tail call fastcc void @append_u64(i64 noundef %7, i32 noundef %9) #35
  %10 = tail call ptr @fa_alloc(i64 noundef 32, i64 noundef 8) #34
  store i64 4, ptr %10, align 8, !tbaa !218
  %11 = getelementptr inbounds %struct.MessageError, ptr %10, i64 0, i32 1
  tail call void @fa_b_take(ptr noundef nonnull %11) #35
  ret ptr %10
}

; Function Attrs: cold noinline nounwind optsize
define noundef ptr @fa_error_checked(i64 noundef %0, i64 noundef %1, ptr nocapture noundef readonly %2) local_unnamed_addr #12 {
  %4 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %4, align 8, !tbaa !5
  tail call fastcc void @append_c(ptr noundef nonnull @.str.16) #35
  %5 = zext i64 %1 to i128
  %6 = shl nuw i128 %5, 64
  %7 = zext i64 %0 to i128
  %8 = or disjoint i128 %6, %7
  %9 = icmp slt i64 %1, 0
  %10 = lshr i64 %1, 63
  %11 = trunc i64 %10 to i32
  %12 = sub i128 0, %8
  %13 = select i1 %9, i128 %12, i128 %8
  %14 = trunc i128 %13 to i64
  %15 = lshr i128 %13, 64
  %16 = trunc i128 %15 to i64
  tail call fastcc void @append_u128(i64 noundef %14, i64 noundef %16, i32 noundef %11) #35
  tail call fastcc void @append_c(ptr noundef nonnull @.str.17) #35
  tail call fastcc void @append_c(ptr noundef %2) #35
  %17 = tail call ptr @fa_alloc(i64 noundef 32, i64 noundef 8) #34
  store i64 4, ptr %17, align 8, !tbaa !218
  %18 = getelementptr inbounds %struct.MessageError, ptr %17, i64 0, i32 1
  tail call void @fa_b_take(ptr noundef nonnull %18) #35
  ret ptr %17
}

; Function Attrs: nounwind optsize
define noundef ptr @fa_parse_integer(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef writeonly %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %13, label %5

5:                                                ; preds = %3
  %6 = load i8, ptr %0, align 1, !tbaa !12
  switch i8 %6, label %9 [
    i8 43, label %7
    i8 45, label %7
  ]

7:                                                ; preds = %5, %5
  %8 = icmp ne i8 %6, 45
  br label %9

9:                                                ; preds = %5, %7
  %10 = phi i1 [ %8, %7 ], [ true, %5 ]
  %11 = phi i64 [ 1, %7 ], [ 0, %5 ]
  %12 = icmp ult i64 %11, %1
  br i1 %12, label %24, label %34

13:                                               ; preds = %3
  %14 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %14, align 8, !tbaa !5
  br label %62

15:                                               ; preds = %24
  %16 = icmp eq i32 %31, 0
  br i1 %16, label %17, label %19

17:                                               ; preds = %15
  %18 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %18, align 8, !tbaa !5
  br label %36

19:                                               ; preds = %15
  br i1 %12, label %20, label %83

20:                                               ; preds = %19
  %21 = getelementptr inbounds i8, ptr %0, i64 %11
  %22 = load i8, ptr %21, align 1, !tbaa !12
  %23 = zext i8 %22 to i64
  br label %72

24:                                               ; preds = %9, %24
  %25 = phi i64 [ %32, %24 ], [ %11, %9 ]
  %26 = phi i32 [ %31, %24 ], [ 1, %9 ]
  %27 = getelementptr inbounds i8, ptr %0, i64 %25
  %28 = load i8, ptr %27, align 1, !tbaa !12
  %29 = add i8 %28, -48
  %30 = icmp ult i8 %29, 10
  %31 = select i1 %30, i32 %26, i32 0
  %32 = add nuw i64 %25, 1
  %33 = icmp eq i64 %32, %1
  br i1 %33, label %15, label %24, !llvm.loop !220

34:                                               ; preds = %9
  %35 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %35, align 8, !tbaa !5
  br i1 %4, label %62, label %36

36:                                               ; preds = %17, %34
  %37 = phi ptr [ %18, %17 ], [ %35, %34 ]
  tail call fastcc void @reserve(i64 noundef 1) #35
  %38 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %39 = load ptr, ptr %38, align 8, !tbaa !9
  %40 = load i64, ptr %37, align 8, !tbaa !5
  %41 = getelementptr inbounds i8, ptr %39, i64 %40
  store i8 34, ptr %41, align 1, !tbaa !12
  %42 = load i64, ptr %37, align 8, !tbaa !5
  %43 = add i64 %42, 1
  store i64 %43, ptr %37, align 8, !tbaa !5
  br label %44

44:                                               ; preds = %36, %54
  %45 = phi i64 [ 0, %36 ], [ %60, %54 ]
  %46 = getelementptr inbounds i8, ptr %0, i64 %45
  %47 = load i8, ptr %46, align 1, !tbaa !12
  switch i8 %47, label %54 [
    i8 92, label %48
    i8 34, label %48
  ]

48:                                               ; preds = %44, %44
  tail call fastcc void @reserve(i64 noundef 1) #35
  %49 = load ptr, ptr %38, align 8, !tbaa !9
  %50 = load i64, ptr %37, align 8, !tbaa !5
  %51 = getelementptr inbounds i8, ptr %49, i64 %50
  store i8 92, ptr %51, align 1, !tbaa !12
  %52 = load i64, ptr %37, align 8, !tbaa !5
  %53 = add i64 %52, 1
  store i64 %53, ptr %37, align 8, !tbaa !5
  br label %54

54:                                               ; preds = %44, %48
  tail call fastcc void @reserve(i64 noundef 1) #35
  %55 = load ptr, ptr %38, align 8, !tbaa !9
  %56 = load i64, ptr %37, align 8, !tbaa !5
  %57 = getelementptr inbounds i8, ptr %55, i64 %56
  store i8 %47, ptr %57, align 1, !tbaa !12
  %58 = load i64, ptr %37, align 8, !tbaa !5
  %59 = add i64 %58, 1
  store i64 %59, ptr %37, align 8, !tbaa !5
  %60 = add nuw i64 %45, 1
  %61 = icmp eq i64 %60, %1
  br i1 %61, label %62, label %44, !llvm.loop !221

62:                                               ; preds = %54, %34, %13
  %63 = phi ptr [ @.str.28, %13 ], [ @.str.28, %34 ], [ @.str.29, %54 ]
  tail call fastcc void @append_c(ptr noundef nonnull %63) #35
  %64 = tail call fastcc ptr @parse_error(ptr noundef %0, i64 noundef %1) #35
  br label %101

65:                                               ; preds = %72
  %66 = getelementptr inbounds i8, ptr %0, i64 %79
  %67 = load i8, ptr %66, align 1, !tbaa !12
  %68 = zext i8 %67 to i64
  %69 = sub nsw i64 47, %68
  %70 = udiv i64 %69, 10
  %71 = icmp ugt i64 %78, %70
  br i1 %71, label %81, label %72, !llvm.loop !222

72:                                               ; preds = %20, %65
  %73 = phi i64 [ %23, %20 ], [ %68, %65 ]
  %74 = phi i64 [ 0, %20 ], [ %78, %65 ]
  %75 = phi i64 [ %11, %20 ], [ %79, %65 ]
  %76 = mul nuw i64 %74, 10
  %77 = add i64 %76, -48
  %78 = add i64 %77, %73
  %79 = add i64 %75, 1
  %80 = icmp eq i64 %79, %1
  br i1 %80, label %81, label %65, !llvm.loop !222

81:                                               ; preds = %65, %72
  %82 = icmp ult i64 %79, %1
  br label %83

83:                                               ; preds = %81, %19
  %84 = phi i64 [ 0, %19 ], [ %78, %81 ]
  %85 = phi i1 [ false, %19 ], [ %82, %81 ]
  %86 = select i1 %10, i64 9223372036854775807, i64 -9223372036854775808
  %87 = icmp ugt i64 %84, %86
  %88 = select i1 %85, i1 true, i1 %87
  br i1 %88, label %89, label %98

89:                                               ; preds = %83
  %90 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %90, align 8, !tbaa !5
  tail call fastcc void @reserve(i64 noundef %1) #35
  %91 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %92 = load ptr, ptr %91, align 8, !tbaa !9
  %93 = load i64, ptr %90, align 8, !tbaa !5
  %94 = getelementptr inbounds i8, ptr %92, i64 %93
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %94, ptr nonnull align 1 %0, i64 %1, i1 false)
  %95 = load i64, ptr %90, align 8, !tbaa !5
  %96 = add i64 %95, %1
  store i64 %96, ptr %90, align 8, !tbaa !5
  tail call fastcc void @append_c(ptr noundef nonnull @.str.24) #35
  %97 = tail call fastcc ptr @parse_error(ptr noundef nonnull %0, i64 noundef %1) #35
  br label %101

98:                                               ; preds = %83
  %99 = sub i64 0, %84
  %100 = select i1 %10, i64 %84, i64 %99
  store i64 %100, ptr %2, align 8, !tbaa !5
  br label %101

101:                                              ; preds = %89, %98, %62
  %102 = phi ptr [ %64, %62 ], [ %97, %89 ], [ null, %98 ]
  ret ptr %102
}

; Function Attrs: nounwind optsize
define internal fastcc noundef ptr @parse_error(ptr nocapture noundef readonly %0, i64 noundef %1) unnamed_addr #0 {
  %3 = tail call ptr @fa_alloc(i64 noundef 56, i64 noundef 8) #34
  store i64 5, ptr %3, align 8, !tbaa !223
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %7, label %5

5:                                                ; preds = %2
  %6 = tail call ptr @fa_alloc(i64 noundef %1, i64 noundef 1) #34, !noalias !225
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %6, ptr align 1 %0, i64 %1, i1 false), !noalias !225
  br label %7

7:                                                ; preds = %2, %5
  %8 = phi ptr [ %6, %5 ], [ @.str.34, %2 ]
  %9 = getelementptr inbounds %struct.ParseError, ptr %3, i64 0, i32 1
  store ptr %8, ptr %9, align 8, !tbaa.struct !16
  %10 = getelementptr inbounds %struct.ParseError, ptr %3, i64 0, i32 1, i32 1
  store i64 %1, ptr %10, align 8, !tbaa.struct !17
  %11 = getelementptr inbounds %struct.ParseError, ptr %3, i64 0, i32 1, i32 2
  store i64 %1, ptr %11, align 8, !tbaa.struct !18
  %12 = getelementptr inbounds %struct.ParseError, ptr %3, i64 0, i32 2
  tail call void @fa_b_take(ptr noundef nonnull %12) #35
  ret ptr %3
}

; Function Attrs: nounwind optsize
define i64 @fa_pow_i64(i64 noundef %0, i64 noundef %1, i32 noundef %2, ptr noundef %3) local_unnamed_addr #0 {
  %5 = icmp ne i32 %2, 0
  br label %6

6:                                                ; preds = %22, %4
  %7 = phi i64 [ %1, %4 ], [ %23, %22 ]
  %8 = phi i64 [ %0, %4 ], [ %26, %22 ]
  %9 = phi i64 [ 1, %4 ], [ %19, %22 ]
  %10 = and i64 %7, 1
  %11 = icmp eq i64 %10, 0
  br i1 %11, label %18, label %12

12:                                               ; preds = %6
  %13 = tail call { i64, i1 } @llvm.smul.with.overflow.i64(i64 %9, i64 %8)
  %14 = extractvalue { i64, i1 } %13, 1
  %15 = extractvalue { i64, i1 } %13, 0
  %16 = and i1 %5, %14
  br i1 %16, label %17, label %18

17:                                               ; preds = %12
  tail call void @fa_overflow_unknown(ptr noundef %3) #39
  unreachable

18:                                               ; preds = %12, %6
  %19 = phi i64 [ %15, %12 ], [ %9, %6 ]
  %20 = icmp ult i64 %7, 2
  br i1 %20, label %21, label %22

21:                                               ; preds = %18
  ret i64 %19

22:                                               ; preds = %18
  %23 = ashr i64 %7, 1
  %24 = tail call { i64, i1 } @llvm.smul.with.overflow.i64(i64 %8, i64 %8)
  %25 = extractvalue { i64, i1 } %24, 1
  %26 = extractvalue { i64, i1 } %24, 0
  %27 = and i1 %5, %25
  br i1 %27, label %28, label %6

28:                                               ; preds = %22
  tail call void @fa_overflow_unknown(ptr noundef %3) #39
  unreachable
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare { i64, i1 } @llvm.smul.with.overflow.i64(i64, i64) #5

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(read, inaccessiblemem: none)
define ptr @fa_table_entry(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #24 {
  %4 = load ptr, ptr %0, align 8, !tbaa !228
  %5 = icmp ult i64 %1, 4
  br i1 %5, label %12, label %6

6:                                                ; preds = %3
  %7 = lshr i64 %1, 2
  %8 = tail call i64 @llvm.ctlz.i64(i64 %7, i1 true), !range !108
  %9 = sub nuw nsw i64 64, %8
  %10 = sub nuw nsw i64 65, %8
  %11 = shl nsw i64 -1, %10
  br label %12

12:                                               ; preds = %3, %6
  %13 = phi i64 [ %11, %6 ], [ 0, %3 ]
  %14 = phi i64 [ %9, %6 ], [ 0, %3 ]
  %15 = getelementptr inbounds %struct.FaSegmented, ptr %4, i64 0, i32 2, i64 %14
  %16 = load ptr, ptr %15, align 8, !tbaa !9
  %17 = add i64 %13, %1
  %18 = mul nsw i64 %17, %2
  %19 = getelementptr inbounds i8, ptr %16, i64 %18
  ret ptr %19
}

; Function Attrs: nounwind optsize
define ptr @fa_table_new(i64 noundef %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = tail call ptr @fa_alloc(i64 noundef 56, i64 noundef 8) #34
  %4 = tail call i64 @llvm.smax.i64(i64 %0, i64 4)
  %5 = mul nsw i64 %4, %1
  %6 = tail call ptr @fa_alloc(i64 noundef %5, i64 noundef 16) #34
  store ptr %6, ptr %3, align 8, !tbaa !228
  %7 = getelementptr inbounds %struct.FaTable, ptr %3, i64 0, i32 1
  store i64 0, ptr %7, align 8, !tbaa !230
  %8 = getelementptr inbounds %struct.FaTable, ptr %3, i64 0, i32 2
  store i64 0, ptr %8, align 8, !tbaa !231
  %9 = getelementptr inbounds %struct.FaTable, ptr %3, i64 0, i32 3
  store i64 %4, ptr %9, align 8, !tbaa !232
  %10 = getelementptr inbounds %struct.FaTable, ptr %3, i64 0, i32 4
  store ptr null, ptr %10, align 8, !tbaa !233
  %11 = getelementptr inbounds %struct.FaTable, ptr %3, i64 0, i32 6
  store i64 0, ptr %11, align 8, !tbaa !234
  %12 = shl nuw nsw i64 %4, 1
  br label %13

13:                                               ; preds = %13, %2
  %14 = phi i64 [ 8, %2 ], [ %16, %13 ]
  %15 = icmp slt i64 %14, %12
  %16 = shl nsw i64 %14, 1
  br i1 %15, label %13, label %17, !llvm.loop !235

17:                                               ; preds = %13
  tail call fastcc void @table_index(ptr noundef nonnull %3, i64 noundef %14, i64 noundef %1) #35
  ret ptr %3
}

; Function Attrs: nounwind optsize
define internal fastcc void @table_index(ptr nocapture noundef %0, i64 noundef %1, i64 noundef %2) unnamed_addr #0 {
  %4 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 4
  %5 = load ptr, ptr %4, align 8, !tbaa !233
  %6 = icmp eq ptr %5, null
  br i1 %6, label %12, label %7

7:                                                ; preds = %3
  %8 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 5
  %9 = load i64, ptr %8, align 8, !tbaa !236
  %10 = shl i64 %9, 3
  %11 = add i64 %10, 8
  tail call void @fa_free(ptr noundef nonnull %5, i64 noundef %11, i64 noundef 8) #34
  br label %12

12:                                               ; preds = %7, %3
  %13 = shl nsw i64 %1, 3
  %14 = tail call ptr @fa_alloc(i64 noundef %13, i64 noundef 8) #34
  store ptr %14, ptr %4, align 8, !tbaa !233
  tail call void @llvm.memset.p0.i64(ptr align 8 %14, i8 0, i64 %13, i1 false)
  %15 = add nsw i64 %1, -1
  %16 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 5
  store i64 %15, ptr %16, align 8, !tbaa !236
  %17 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 2
  %18 = load i64, ptr %17, align 8, !tbaa !231
  %19 = icmp sgt i64 %18, 0
  br i1 %19, label %20, label %23

20:                                               ; preds = %12
  %21 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 6
  %22 = load ptr, ptr %0, align 8, !tbaa !228
  br label %24

23:                                               ; preds = %68, %12
  ret void

24:                                               ; preds = %20, %68
  %25 = phi i64 [ %18, %20 ], [ %70, %68 ]
  %26 = phi i64 [ 0, %20 ], [ %69, %68 ]
  %27 = load i64, ptr %21, align 8, !tbaa !234
  %28 = icmp eq i64 %27, 0
  br i1 %28, label %43, label %29

29:                                               ; preds = %24
  %30 = icmp ult i64 %26, 4
  br i1 %30, label %37, label %31

31:                                               ; preds = %29
  %32 = lshr i64 %26, 2
  %33 = tail call i64 @llvm.ctlz.i64(i64 %32, i1 true), !range !108
  %34 = sub nuw nsw i64 64, %33
  %35 = sub nuw nsw i64 65, %33
  %36 = shl nsw i64 -1, %35
  br label %37

37:                                               ; preds = %31, %29
  %38 = phi i64 [ %36, %31 ], [ 0, %29 ]
  %39 = phi i64 [ %34, %31 ], [ 0, %29 ]
  %40 = getelementptr inbounds %struct.FaSegmented, ptr %22, i64 0, i32 2, i64 %39
  %41 = load ptr, ptr %40, align 8, !tbaa !9
  %42 = add i64 %38, %26
  br label %43

43:                                               ; preds = %24, %37
  %44 = phi i64 [ %42, %37 ], [ %26, %24 ]
  %45 = phi ptr [ %41, %37 ], [ %22, %24 ]
  %46 = mul nsw i64 %44, %2
  %47 = getelementptr inbounds i8, ptr %45, i64 %46
  %48 = getelementptr inbounds i8, ptr %47, i64 8
  %49 = load i64, ptr %48, align 8, !tbaa !5
  %50 = icmp eq i64 %49, 0
  br i1 %50, label %51, label %53

51:                                               ; preds = %43
  %52 = add nuw nsw i64 %26, 1
  br label %68

53:                                               ; preds = %43
  %54 = load i64, ptr %47, align 8, !tbaa !5
  %55 = load i64, ptr %16, align 8, !tbaa !236
  %56 = load ptr, ptr %4, align 8, !tbaa !233
  br label %57

57:                                               ; preds = %57, %53
  %58 = phi i64 [ %54, %53 ], [ %63, %57 ]
  %59 = and i64 %58, %55
  %60 = getelementptr inbounds i64, ptr %56, i64 %59
  %61 = load i64, ptr %60, align 8, !tbaa !5
  %62 = icmp eq i64 %61, 0
  %63 = add nsw i64 %59, 1
  br i1 %62, label %64, label %57, !llvm.loop !237

64:                                               ; preds = %57
  %65 = getelementptr inbounds i64, ptr %56, i64 %59
  %66 = add nuw nsw i64 %26, 1
  store i64 %66, ptr %65, align 8, !tbaa !5
  %67 = load i64, ptr %17, align 8, !tbaa !231
  br label %68

68:                                               ; preds = %51, %64
  %69 = phi i64 [ %52, %51 ], [ %66, %64 ]
  %70 = phi i64 [ %25, %51 ], [ %67, %64 ]
  %71 = icmp slt i64 %69, %70
  br i1 %71, label %24, label %23, !llvm.loop !238
}

; Function Attrs: nounwind optsize
define ptr @fa_table_new_growing(i64 noundef %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = tail call ptr @fa_alloc(i64 noundef 56, i64 noundef 8) #34
  %4 = tail call i64 @llvm.smax.i64(i64 %0, i64 4)
  %5 = tail call noundef ptr @fa_alloc(i64 noundef 400, i64 noundef 8) #34
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(400) %5, i8 0, i64 400, i1 false)
  store ptr %5, ptr %3, align 8, !tbaa !228
  %6 = getelementptr inbounds %struct.FaTable, ptr %3, i64 0, i32 1
  store i64 0, ptr %6, align 8, !tbaa !230
  %7 = getelementptr inbounds %struct.FaTable, ptr %3, i64 0, i32 2
  store i64 0, ptr %7, align 8, !tbaa !231
  %8 = getelementptr inbounds %struct.FaTable, ptr %3, i64 0, i32 3
  store i64 %4, ptr %8, align 8, !tbaa !232
  %9 = getelementptr inbounds %struct.FaTable, ptr %3, i64 0, i32 4
  store ptr null, ptr %9, align 8, !tbaa !233
  %10 = getelementptr inbounds %struct.FaTable, ptr %3, i64 0, i32 6
  store i64 1, ptr %10, align 8, !tbaa !234
  %11 = shl nuw nsw i64 %4, 1
  br label %12

12:                                               ; preds = %12, %2
  %13 = phi i64 [ 8, %2 ], [ %15, %12 ]
  %14 = icmp slt i64 %13, %11
  %15 = shl nsw i64 %13, 1
  br i1 %14, label %12, label %16, !llvm.loop !235

16:                                               ; preds = %12
  tail call fastcc void @table_index(ptr noundef nonnull %3, i64 noundef %13, i64 noundef %1) #35
  ret ptr %3
}

; Function Attrs: alwaysinline nounwind optsize
define i64 @fa_table_find(ptr nocapture noundef readonly %0, i64 noundef %1, ptr noundef %2, ptr nocapture noundef readonly %3, i64 noundef %4) local_unnamed_addr #25 {
  %6 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 5
  %7 = load i64, ptr %6, align 8, !tbaa !236
  %8 = and i64 %7, %1
  %9 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 4
  %10 = load ptr, ptr %9, align 8, !tbaa !233
  %11 = getelementptr inbounds i64, ptr %10, i64 %8
  %12 = load i64, ptr %11, align 8, !tbaa !5
  %13 = icmp eq i64 %12, 0
  br i1 %13, label %42, label %14

14:                                               ; preds = %5, %34
  %15 = phi ptr [ %35, %34 ], [ %10, %5 ]
  %16 = phi i64 [ %36, %34 ], [ %7, %5 ]
  %17 = phi i64 [ %40, %34 ], [ %12, %5 ]
  %18 = phi i64 [ %38, %34 ], [ %8, %5 ]
  %19 = icmp sgt i64 %17, 0
  br i1 %19, label %20, label %34

20:                                               ; preds = %14
  %21 = add nsw i64 %17, -1
  %22 = load ptr, ptr %0, align 8, !tbaa !228
  %23 = mul nsw i64 %21, %4
  %24 = getelementptr inbounds i8, ptr %22, i64 %23
  %25 = load i64, ptr %24, align 8, !tbaa !5
  %26 = icmp eq i64 %25, %1
  br i1 %26, label %27, label %34

27:                                               ; preds = %20
  %28 = getelementptr inbounds i8, ptr %24, i64 16
  %29 = tail call i32 %3(ptr noundef nonnull %28, ptr noundef %2) #34
  %30 = icmp eq i32 %29, 0
  br i1 %30, label %31, label %42

31:                                               ; preds = %27
  %32 = load i64, ptr %6, align 8, !tbaa !236
  %33 = load ptr, ptr %9, align 8, !tbaa !233
  br label %34

34:                                               ; preds = %31, %20, %14
  %35 = phi ptr [ %33, %31 ], [ %15, %20 ], [ %15, %14 ]
  %36 = phi i64 [ %32, %31 ], [ %16, %20 ], [ %16, %14 ]
  %37 = add nsw i64 %18, 1
  %38 = and i64 %36, %37
  %39 = getelementptr inbounds i64, ptr %35, i64 %38
  %40 = load i64, ptr %39, align 8, !tbaa !5
  %41 = icmp eq i64 %40, 0
  br i1 %41, label %42, label %14

42:                                               ; preds = %27, %34, %5
  %43 = phi i64 [ -1, %5 ], [ -1, %34 ], [ %21, %27 ]
  ret i64 %43
}

; Function Attrs: alwaysinline nounwind optsize
define i64 @fa_table_find_growing(ptr nocapture noundef readonly %0, i64 noundef %1, ptr noundef %2, ptr nocapture noundef readonly %3, i64 noundef %4) local_unnamed_addr #25 {
  %6 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 5
  %7 = load i64, ptr %6, align 8, !tbaa !236
  %8 = and i64 %7, %1
  %9 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 4
  %10 = load ptr, ptr %9, align 8, !tbaa !233
  %11 = getelementptr inbounds i64, ptr %10, i64 %8
  %12 = load i64, ptr %11, align 8, !tbaa !5
  %13 = icmp eq i64 %12, 0
  br i1 %13, label %55, label %14

14:                                               ; preds = %5, %47
  %15 = phi ptr [ %48, %47 ], [ %10, %5 ]
  %16 = phi i64 [ %49, %47 ], [ %7, %5 ]
  %17 = phi i64 [ %53, %47 ], [ %12, %5 ]
  %18 = phi i64 [ %51, %47 ], [ %8, %5 ]
  %19 = icmp sgt i64 %17, 0
  br i1 %19, label %20, label %47

20:                                               ; preds = %14
  %21 = add nsw i64 %17, -1
  %22 = load ptr, ptr %0, align 8, !tbaa !228
  %23 = icmp ult i64 %17, 5
  br i1 %23, label %30, label %24

24:                                               ; preds = %20
  %25 = lshr i64 %21, 2
  %26 = tail call i64 @llvm.ctlz.i64(i64 %25, i1 true), !range !108
  %27 = sub nuw nsw i64 64, %26
  %28 = sub nuw nsw i64 65, %26
  %29 = shl nsw i64 -1, %28
  br label %30

30:                                               ; preds = %24, %20
  %31 = phi i64 [ %29, %24 ], [ 0, %20 ]
  %32 = phi i64 [ %27, %24 ], [ 0, %20 ]
  %33 = getelementptr inbounds %struct.FaSegmented, ptr %22, i64 0, i32 2, i64 %32
  %34 = load ptr, ptr %33, align 8, !tbaa !9
  %35 = add nsw i64 %31, %21
  %36 = mul nsw i64 %35, %4
  %37 = getelementptr inbounds i8, ptr %34, i64 %36
  %38 = load i64, ptr %37, align 8, !tbaa !5
  %39 = icmp eq i64 %38, %1
  br i1 %39, label %40, label %47

40:                                               ; preds = %30
  %41 = getelementptr inbounds i8, ptr %37, i64 16
  %42 = tail call i32 %3(ptr noundef nonnull %41, ptr noundef %2) #34
  %43 = icmp eq i32 %42, 0
  br i1 %43, label %44, label %55

44:                                               ; preds = %40
  %45 = load i64, ptr %6, align 8, !tbaa !236
  %46 = load ptr, ptr %9, align 8, !tbaa !233
  br label %47

47:                                               ; preds = %44, %30, %14
  %48 = phi ptr [ %46, %44 ], [ %15, %30 ], [ %15, %14 ]
  %49 = phi i64 [ %45, %44 ], [ %16, %30 ], [ %16, %14 ]
  %50 = add nsw i64 %18, 1
  %51 = and i64 %49, %50
  %52 = getelementptr inbounds i64, ptr %48, i64 %51
  %53 = load i64, ptr %52, align 8, !tbaa !5
  %54 = icmp eq i64 %53, 0
  br i1 %54, label %55, label %14

55:                                               ; preds = %40, %47, %5
  %56 = phi i64 [ -1, %5 ], [ -1, %47 ], [ %21, %40 ]
  ret i64 %56
}

; Function Attrs: nounwind optsize
define noundef ptr @fa_table_append(ptr nocapture noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #0 {
  %4 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 2
  %5 = load i64, ptr %4, align 8, !tbaa !231
  %6 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 3
  %7 = load i64, ptr %6, align 8, !tbaa !232
  %8 = icmp slt i64 %5, %7
  br i1 %8, label %66, label %9

9:                                                ; preds = %3
  %10 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 6
  %11 = load i64, ptr %10, align 8, !tbaa !234
  %12 = icmp eq i64 %11, 0
  br i1 %12, label %21, label %13

13:                                               ; preds = %9
  %14 = shl nsw i64 %7, 1
  store i64 %14, ptr %6, align 8, !tbaa !232
  %15 = shl nsw i64 %7, 2
  br label %16

16:                                               ; preds = %16, %13
  %17 = phi i64 [ 8, %13 ], [ %19, %16 ]
  %18 = icmp slt i64 %17, %15
  %19 = shl nsw i64 %17, 1
  br i1 %18, label %16, label %20, !llvm.loop !235

20:                                               ; preds = %16
  tail call fastcc void @table_index(ptr noundef %0, i64 noundef %17, i64 noundef %2) #35
  br label %66

21:                                               ; preds = %9
  %22 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 1
  %23 = load i64, ptr %22, align 8, !tbaa !230
  %24 = shl nsw i64 %23, 1
  %25 = icmp sgt i64 %24, %5
  %26 = zext i1 %25 to i64
  %27 = shl nsw i64 %7, %26
  %28 = mul nsw i64 %27, %2
  %29 = tail call ptr @fa_alloc(i64 noundef %28, i64 noundef 16) #34
  %30 = load i64, ptr %4, align 8, !tbaa !231
  %31 = icmp sgt i64 %30, 0
  br i1 %31, label %41, label %32

32:                                               ; preds = %56, %21
  %33 = phi i64 [ 0, %21 ], [ %58, %56 ]
  %34 = load ptr, ptr %0, align 8, !tbaa !228
  %35 = load i64, ptr %6, align 8, !tbaa !232
  %36 = mul nsw i64 %35, %2
  tail call void @fa_free(ptr noundef %34, i64 noundef %36, i64 noundef 16) #34
  store ptr %29, ptr %0, align 8, !tbaa !228
  store i64 %33, ptr %4, align 8, !tbaa !231
  store i64 %27, ptr %6, align 8, !tbaa !232
  %37 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 5
  %38 = load i64, ptr %37, align 8, !tbaa !236
  %39 = add nsw i64 %38, 1
  %40 = shl nsw i64 %27, 1
  br label %61

41:                                               ; preds = %21, %56
  %42 = phi i64 [ %57, %56 ], [ %30, %21 ]
  %43 = phi i64 [ %59, %56 ], [ 0, %21 ]
  %44 = phi i64 [ %58, %56 ], [ 0, %21 ]
  %45 = load ptr, ptr %0, align 8, !tbaa !228
  %46 = mul nsw i64 %43, %2
  %47 = getelementptr inbounds i8, ptr %45, i64 %46
  %48 = getelementptr inbounds i8, ptr %47, i64 8
  %49 = load i64, ptr %48, align 8, !tbaa !5
  %50 = icmp eq i64 %49, 0
  br i1 %50, label %56, label %51

51:                                               ; preds = %41
  %52 = mul nsw i64 %44, %2
  %53 = getelementptr inbounds i8, ptr %29, i64 %52
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %53, ptr nonnull align 1 %47, i64 %2, i1 false)
  %54 = add nsw i64 %44, 1
  %55 = load i64, ptr %4, align 8, !tbaa !231
  br label %56

56:                                               ; preds = %51, %41
  %57 = phi i64 [ %55, %51 ], [ %42, %41 ]
  %58 = phi i64 [ %54, %51 ], [ %44, %41 ]
  %59 = add nuw nsw i64 %43, 1
  %60 = icmp slt i64 %59, %57
  br i1 %60, label %41, label %32, !llvm.loop !239

61:                                               ; preds = %61, %32
  %62 = phi i64 [ %39, %32 ], [ %64, %61 ]
  %63 = icmp slt i64 %62, %40
  %64 = shl nsw i64 %62, 1
  br i1 %63, label %61, label %65, !llvm.loop !240

65:                                               ; preds = %61
  tail call fastcc void @table_index(ptr noundef nonnull %0, i64 noundef %62, i64 noundef %2) #35
  br label %66

66:                                               ; preds = %3, %20, %65
  %67 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 6
  %68 = load i64, ptr %67, align 8, !tbaa !234
  %69 = icmp eq i64 %68, 0
  %70 = load ptr, ptr %0, align 8, !tbaa !228
  br i1 %69, label %73, label %71

71:                                               ; preds = %66
  %72 = tail call ptr @fa_segmented_push_slot(ptr noundef %70, i64 noundef %2, i32 noundef 2) #35
  br label %77

73:                                               ; preds = %66
  %74 = load i64, ptr %4, align 8, !tbaa !231
  %75 = mul nsw i64 %74, %2
  %76 = getelementptr inbounds i8, ptr %70, i64 %75
  br label %77

77:                                               ; preds = %73, %71
  %78 = phi ptr [ %72, %71 ], [ %76, %73 ]
  store i64 %1, ptr %78, align 8, !tbaa !5
  %79 = getelementptr inbounds i8, ptr %78, i64 8
  store i64 1, ptr %79, align 8, !tbaa !5
  %80 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 5
  %81 = load i64, ptr %80, align 8, !tbaa !236
  %82 = and i64 %81, %1
  %83 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 4
  %84 = load ptr, ptr %83, align 8, !tbaa !233
  %85 = getelementptr inbounds i64, ptr %84, i64 %82
  %86 = load i64, ptr %85, align 8, !tbaa !5
  %87 = icmp sgt i64 %86, 0
  br i1 %87, label %88, label %95

88:                                               ; preds = %77, %88
  %89 = phi i64 [ %91, %88 ], [ %82, %77 ]
  %90 = add nsw i64 %89, 1
  %91 = and i64 %90, %81
  %92 = getelementptr inbounds i64, ptr %84, i64 %91
  %93 = load i64, ptr %92, align 8, !tbaa !5
  %94 = icmp sgt i64 %93, 0
  br i1 %94, label %88, label %95, !llvm.loop !241

95:                                               ; preds = %88, %77
  %96 = phi i64 [ %82, %77 ], [ %91, %88 ]
  %97 = getelementptr inbounds i64, ptr %84, i64 %96
  %98 = load i64, ptr %4, align 8, !tbaa !231
  %99 = add nsw i64 %98, 1
  store i64 %99, ptr %97, align 8, !tbaa !5
  %100 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 1
  %101 = load <2 x i64>, ptr %100, align 8, !tbaa !5
  %102 = add nsw <2 x i64> %101, <i64 1, i64 1>
  store <2 x i64> %102, ptr %100, align 8, !tbaa !5
  ret ptr %78
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define void @fa_table_kill(ptr nocapture noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #10 {
  %4 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 6
  %5 = load i64, ptr %4, align 8, !tbaa !234
  %6 = icmp eq i64 %5, 0
  %7 = load ptr, ptr %0, align 8, !tbaa !228
  br i1 %6, label %24, label %8

8:                                                ; preds = %3
  %9 = icmp ult i64 %1, 4
  br i1 %9, label %16, label %10

10:                                               ; preds = %8
  %11 = lshr i64 %1, 2
  %12 = tail call i64 @llvm.ctlz.i64(i64 %11, i1 true), !range !108
  %13 = sub nuw nsw i64 64, %12
  %14 = sub nuw nsw i64 65, %12
  %15 = shl nsw i64 -1, %14
  br label %16

16:                                               ; preds = %10, %8
  %17 = phi i64 [ %15, %10 ], [ 0, %8 ]
  %18 = phi i64 [ %13, %10 ], [ 0, %8 ]
  %19 = getelementptr inbounds %struct.FaSegmented, ptr %7, i64 0, i32 2, i64 %18
  %20 = load ptr, ptr %19, align 8, !tbaa !9
  %21 = add i64 %17, %1
  %22 = mul nsw i64 %21, %2
  %23 = getelementptr inbounds i8, ptr %20, i64 %22
  br label %27

24:                                               ; preds = %3
  %25 = mul nsw i64 %2, %1
  %26 = getelementptr inbounds i8, ptr %7, i64 %25
  br label %27

27:                                               ; preds = %16, %24
  %28 = phi ptr [ %23, %16 ], [ %26, %24 ]
  %29 = load i64, ptr %28, align 8, !tbaa !5
  %30 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 5
  %31 = load i64, ptr %30, align 8, !tbaa !236
  %32 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 4
  %33 = load ptr, ptr %32, align 8, !tbaa !233
  %34 = add nsw i64 %1, 1
  br label %35

35:                                               ; preds = %35, %27
  %36 = phi i64 [ %29, %27 ], [ %41, %35 ]
  %37 = and i64 %36, %31
  %38 = getelementptr inbounds i64, ptr %33, i64 %37
  %39 = load i64, ptr %38, align 8, !tbaa !5
  %40 = icmp eq i64 %39, %34
  %41 = add nsw i64 %37, 1
  br i1 %40, label %42, label %35, !llvm.loop !242

42:                                               ; preds = %35
  %43 = getelementptr inbounds i64, ptr %33, i64 %37
  store i64 -1, ptr %43, align 8, !tbaa !5
  %44 = getelementptr inbounds i8, ptr %28, i64 8
  store i64 0, ptr %44, align 8, !tbaa !5
  %45 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 1
  %46 = load i64, ptr %45, align 8, !tbaa !230
  %47 = add nsw i64 %46, -1
  store i64 %47, ptr %45, align 8, !tbaa !230
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, argmem: readwrite, inaccessiblemem: none)
define void @fa_table_clear(ptr nocapture noundef %0) local_unnamed_addr #26 {
  %2 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 1
  store i64 0, ptr %2, align 8, !tbaa !230
  %3 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 2
  store i64 0, ptr %3, align 8, !tbaa !231
  %4 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 6
  %5 = load i64, ptr %4, align 8, !tbaa !234
  %6 = icmp eq i64 %5, 0
  br i1 %6, label %9, label %7

7:                                                ; preds = %1
  %8 = load ptr, ptr %0, align 8, !tbaa !228
  store i64 0, ptr %8, align 8, !tbaa !109
  br label %9

9:                                                ; preds = %7, %1
  %10 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 4
  %11 = load ptr, ptr %10, align 8, !tbaa !233
  %12 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 5
  %13 = load i64, ptr %12, align 8, !tbaa !236
  %14 = shl i64 %13, 3
  %15 = add i64 %14, 8
  tail call void @llvm.memset.p0.i64(ptr align 8 %11, i8 0, i64 %15, i1 false)
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_table_free(ptr noundef %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 6
  %4 = load i64, ptr %3, align 8, !tbaa !234
  %5 = icmp eq i64 %4, 0
  %6 = load ptr, ptr %0, align 8, !tbaa !228
  br i1 %5, label %8, label %7

7:                                                ; preds = %2
  tail call void @fa_segmented_free(ptr noundef %6, i64 noundef %1, i32 noundef 2) #35
  br label %12

8:                                                ; preds = %2
  %9 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 3
  %10 = load i64, ptr %9, align 8, !tbaa !232
  %11 = mul nsw i64 %10, %1
  tail call void @fa_free(ptr noundef %6, i64 noundef %11, i64 noundef 16) #34
  br label %12

12:                                               ; preds = %8, %7
  %13 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 4
  %14 = load ptr, ptr %13, align 8, !tbaa !233
  %15 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 5
  %16 = load i64, ptr %15, align 8, !tbaa !236
  %17 = shl i64 %16, 3
  %18 = add i64 %17, 8
  tail call void @fa_free(ptr noundef %14, i64 noundef %18, i64 noundef 8) #34
  tail call void @fa_free(ptr noundef nonnull %0, i64 noundef 56, i64 noundef 8) #34
  ret void
}

; Function Attrs: nounwind optsize
define ptr @fa_table_copy(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 6
  %4 = load i64, ptr %3, align 8, !tbaa !234
  %5 = icmp eq i64 %4, 0
  %6 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 1
  %7 = load i64, ptr %6, align 8, !tbaa !230
  %8 = tail call i64 @llvm.smax.i64(i64 %7, i64 4)
  br i1 %5, label %11, label %9

9:                                                ; preds = %2
  %10 = tail call ptr @fa_table_new_growing(i64 noundef %8, i64 noundef %1) #35
  br label %13

11:                                               ; preds = %2
  %12 = tail call ptr @fa_table_new(i64 noundef %8, i64 noundef %1) #35
  br label %13

13:                                               ; preds = %11, %9
  %14 = phi ptr [ %10, %9 ], [ %12, %11 ]
  %15 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 2
  %16 = load i64, ptr %15, align 8, !tbaa !231
  %17 = icmp sgt i64 %16, 0
  br i1 %17, label %25, label %18

18:                                               ; preds = %67, %13
  %19 = phi i64 [ 0, %13 ], [ %69, %67 ]
  %20 = getelementptr inbounds %struct.FaTable, ptr %14, i64 0, i32 2
  store i64 %19, ptr %20, align 8, !tbaa !231
  %21 = getelementptr inbounds %struct.FaTable, ptr %14, i64 0, i32 1
  store i64 %19, ptr %21, align 8, !tbaa !230
  %22 = getelementptr inbounds %struct.FaTable, ptr %14, i64 0, i32 5
  %23 = load i64, ptr %22, align 8, !tbaa !236
  %24 = add nsw i64 %23, 1
  tail call fastcc void @table_index(ptr noundef %14, i64 noundef %24, i64 noundef %1) #35
  ret ptr %14

25:                                               ; preds = %13, %67
  %26 = phi i64 [ %68, %67 ], [ %16, %13 ]
  %27 = phi i64 [ %69, %67 ], [ 0, %13 ]
  %28 = phi i64 [ %70, %67 ], [ 0, %13 ]
  %29 = load ptr, ptr %0, align 8, !tbaa !228
  br i1 %5, label %38, label %30

30:                                               ; preds = %25
  %31 = icmp ult i64 %28, 4
  br i1 %31, label %44, label %32

32:                                               ; preds = %30
  %33 = lshr i64 %28, 2
  %34 = tail call i64 @llvm.ctlz.i64(i64 %33, i1 true), !range !108
  %35 = sub nuw nsw i64 64, %34
  %36 = sub nuw nsw i64 65, %34
  %37 = shl nsw i64 -1, %36
  br label %44

38:                                               ; preds = %25
  %39 = mul nsw i64 %28, %1
  %40 = getelementptr inbounds i8, ptr %29, i64 %39
  %41 = getelementptr inbounds i8, ptr %40, i64 8
  %42 = load i64, ptr %41, align 8, !tbaa !5
  %43 = icmp eq i64 %42, 0
  br i1 %43, label %67, label %58

44:                                               ; preds = %30, %32
  %45 = phi i64 [ %37, %32 ], [ 0, %30 ]
  %46 = phi i64 [ %35, %32 ], [ 0, %30 ]
  %47 = getelementptr inbounds %struct.FaSegmented, ptr %29, i64 0, i32 2, i64 %46
  %48 = load ptr, ptr %47, align 8, !tbaa !9
  %49 = add i64 %45, %28
  %50 = mul nsw i64 %49, %1
  %51 = getelementptr inbounds i8, ptr %48, i64 %50
  %52 = getelementptr inbounds i8, ptr %51, i64 8
  %53 = load i64, ptr %52, align 8, !tbaa !5
  %54 = icmp eq i64 %53, 0
  br i1 %54, label %67, label %55

55:                                               ; preds = %44
  %56 = load ptr, ptr %14, align 8, !tbaa !228
  %57 = tail call ptr @fa_segmented_push_slot(ptr noundef %56, i64 noundef %1, i32 noundef 2) #35
  br label %62

58:                                               ; preds = %38
  %59 = load ptr, ptr %14, align 8, !tbaa !228
  %60 = mul nsw i64 %27, %1
  %61 = getelementptr inbounds i8, ptr %59, i64 %60
  br label %62

62:                                               ; preds = %58, %55
  %63 = phi ptr [ %51, %55 ], [ %40, %58 ]
  %64 = phi ptr [ %57, %55 ], [ %61, %58 ]
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %64, ptr nonnull align 1 %63, i64 %1, i1 false)
  %65 = add nsw i64 %27, 1
  %66 = load i64, ptr %15, align 8, !tbaa !231
  br label %67

67:                                               ; preds = %44, %62, %38
  %68 = phi i64 [ %66, %62 ], [ %26, %38 ], [ %26, %44 ]
  %69 = phi i64 [ %65, %62 ], [ %27, %38 ], [ %27, %44 ]
  %70 = add nuw nsw i64 %28, 1
  %71 = icmp slt i64 %70, %68
  br i1 %71, label %25, label %18, !llvm.loop !243
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: read)
define i64 @fa_hash_bytes(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #8 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %11

4:                                                ; preds = %11, %2
  %5 = phi i64 [ -3750763034362895579, %2 ], [ %18, %11 ]
  %6 = lshr i64 %5, 33
  %7 = xor i64 %6, %5
  %8 = mul i64 %7, -49064778989728563
  %9 = lshr i64 %8, 33
  %10 = xor i64 %9, %8
  ret i64 %10

11:                                               ; preds = %2, %11
  %12 = phi i64 [ %19, %11 ], [ 0, %2 ]
  %13 = phi i64 [ %18, %11 ], [ -3750763034362895579, %2 ]
  %14 = getelementptr inbounds i8, ptr %0, i64 %12
  %15 = load i8, ptr %14, align 1, !tbaa !12
  %16 = zext i8 %15 to i64
  %17 = xor i64 %13, %16
  %18 = mul i64 %17, 1099511628211
  %19 = add nuw i64 %12, 1
  %20 = icmp eq i64 %19, %1
  br i1 %20, label %4, label %11, !llvm.loop !244
}

; Function Attrs: cold noinline nounwind optsize
define noundef ptr @fa_error_key_not_found() local_unnamed_addr #12 {
  %1 = tail call ptr @fa_alloc(i64 noundef 32, i64 noundef 8) #34
  store i64 2, ptr %1, align 8, !tbaa !218
  %2 = getelementptr inbounds %struct.MessageError, ptr %1, i64 0, i32 1
  tail call void @fa_b_take(ptr noundef nonnull %2) #35
  ret ptr %1
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(argmem: readwrite)
define noundef ptr @fa_shared_retain(ptr noundef returned %0) local_unnamed_addr #27 {
  %2 = icmp eq ptr %0, null
  br i1 %2, label %5, label %3

3:                                                ; preds = %1
  %4 = atomicrmw add ptr %0, i64 1 monotonic, align 8
  br label %5

5:                                                ; preds = %3, %1
  ret ptr %0
}

; Function Attrs: nounwind optsize
define void @fa_shared_release(ptr noundef %0) local_unnamed_addr #0 {
  %2 = icmp eq ptr %0, null
  br i1 %2, label %14, label %3

3:                                                ; preds = %1
  %4 = atomicrmw sub ptr %0, i64 1 acq_rel, align 8
  %5 = icmp eq i64 %4, 1
  br i1 %5, label %6, label %14

6:                                                ; preds = %3
  %7 = getelementptr inbounds %struct.FaShared, ptr %0, i64 0, i32 2
  %8 = load ptr, ptr %7, align 8, !tbaa !245
  %9 = icmp eq ptr %8, null
  br i1 %9, label %11, label %10

10:                                               ; preds = %6
  tail call void %8(ptr noundef nonnull %0) #34
  br label %11

11:                                               ; preds = %10, %6
  %12 = getelementptr inbounds %struct.FaShared, ptr %0, i64 0, i32 1
  %13 = load i64, ptr %12, align 8, !tbaa !247
  tail call void @fa_free(ptr noundef nonnull %0, i64 noundef %13, i64 noundef 16) #34
  br label %14

14:                                               ; preds = %11, %3, %1
  ret void
}

; Function Attrs: nounwind optsize
define ptr @fa_atomic_new(i64 noundef %0) local_unnamed_addr #0 {
  %2 = tail call fastcc ptr @shared_new(i64 noundef 40, ptr noundef null) #35
  %3 = getelementptr inbounds %struct.FaAtomicBlock, ptr %2, i64 0, i32 1
  store i64 %0, ptr %3, align 8, !tbaa !248
  ret ptr %2
}

; Function Attrs: nounwind optsize
define internal fastcc ptr @shared_new(i64 noundef %0, ptr noundef %1) unnamed_addr #0 {
  %3 = tail call ptr @fa_alloc(i64 noundef %0, i64 noundef 16) #34
  %4 = icmp eq ptr %3, null
  br i1 %4, label %5, label %6

5:                                                ; preds = %2
  tail call fastcc void @out_of_memory() #37
  unreachable

6:                                                ; preds = %2
  tail call void @llvm.memset.p0.i64(ptr nonnull align 8 %3, i8 0, i64 %0, i1 false)
  store i64 1, ptr %3, align 8, !tbaa !250
  %7 = getelementptr inbounds %struct.FaShared, ptr %3, i64 0, i32 1
  store i64 %0, ptr %7, align 8, !tbaa !247
  %8 = getelementptr inbounds %struct.FaShared, ptr %3, i64 0, i32 2
  store ptr %1, ptr %8, align 8, !tbaa !245
  ret ptr %3
}

; Function Attrs: nounwind optsize
define void @fa_task_spawn(ptr noundef %0, i32 noundef %1) local_unnamed_addr #0 {
  %3 = tail call fastcc ptr @executor() #35
  %4 = icmp eq i32 %1, 0
  br i1 %4, label %12, label %5

5:                                                ; preds = %2
  %6 = getelementptr inbounds i8, ptr %0, i64 44
  %7 = load i32, ptr %6, align 4, !tbaa !251
  %8 = or i32 %7, 1
  store i32 %8, ptr %6, align 4, !tbaa !251
  %9 = getelementptr inbounds %struct.FaExecutor, ptr %3, i64 0, i32 8
  %10 = load i64, ptr %9, align 8, !tbaa !253
  %11 = add nsw i64 %10, 1
  store i64 %11, ptr %9, align 8, !tbaa !253
  br label %12

12:                                               ; preds = %5, %2
  %13 = getelementptr inbounds i8, ptr %0, i64 16
  %14 = getelementptr inbounds i8, ptr %0, i64 24
  store ptr null, ptr %14, align 8, !tbaa !255
  %15 = getelementptr inbounds %struct.FaExecutor, ptr %3, i64 0, i32 1
  %16 = load ptr, ptr %15, align 8, !tbaa !256
  %17 = getelementptr inbounds i8, ptr %0, i64 32
  store ptr %16, ptr %17, align 8, !tbaa !257
  %18 = icmp eq ptr %16, null
  %19 = getelementptr inbounds %struct.FaTask, ptr %16, i64 0, i32 1
  %20 = select i1 %18, ptr %3, ptr %19
  store ptr %13, ptr %20, align 8, !tbaa !9
  store ptr %13, ptr %15, align 8, !tbaa !256
  %21 = getelementptr inbounds %struct.FaExecutor, ptr %3, i64 0, i32 2
  %22 = load i64, ptr %21, align 8, !tbaa !258
  %23 = add nsw i64 %22, 1
  store i64 %23, ptr %21, align 8, !tbaa !258
  %24 = getelementptr inbounds i8, ptr %0, i64 40
  store atomic i32 1, ptr %24 release, align 8
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc ptr @executor() unnamed_addr #0 {
  %1 = alloca %struct.FaEpollEvent, align 1
  %2 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @current_executor)
  %3 = load ptr, ptr %2, align 8, !tbaa !9
  %4 = icmp eq ptr %3, null
  br i1 %4, label %5, label %25

5:                                                ; preds = %0
  %6 = tail call ptr @fa_alloc(i64 noundef 424, i64 noundef 16) #34
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(424) %6, i8 0, i64 424, i1 false)
  %7 = getelementptr inbounds %struct.FaExecutor, ptr %6, i64 0, i32 11
  %8 = getelementptr inbounds %struct.FaExecutor, ptr %6, i64 0, i32 9
  store ptr %7, ptr %8, align 8, !tbaa !259
  %9 = getelementptr inbounds %struct.FaExecutor, ptr %6, i64 0, i32 10
  store ptr %7, ptr %9, align 8, !tbaa !260
  %10 = getelementptr inbounds %struct.FaExecutor, ptr %6, i64 0, i32 13
  store i32 -1, ptr %10, align 4, !tbaa !261
  %11 = getelementptr inbounds %struct.FaExecutor, ptr %6, i64 0, i32 14
  store i32 -1, ptr %11, align 8, !tbaa !262
  %12 = or i1 icmp eq (ptr @epoll_create1, ptr null), icmp eq (ptr @eventfd, ptr null)
  %13 = or i1 %12, icmp eq (ptr @pthread_create, ptr null)
  br i1 %13, label %23, label %14

14:                                               ; preds = %5
  %15 = tail call i32 @epoll_create1(i32 noundef 524288) #34
  store i32 %15, ptr %10, align 4, !tbaa !261
  %16 = tail call i32 @eventfd(i32 noundef 0, i32 noundef 526336) #34
  store i32 %16, ptr %11, align 8, !tbaa !262
  %17 = load i32, ptr %10, align 4, !tbaa !261
  %18 = icmp sgt i32 %17, -1
  %19 = icmp sgt i32 %16, -1
  %20 = select i1 %18, i1 %19, i1 false
  br i1 %20, label %21, label %23

21:                                               ; preds = %14
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %1) #31
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(12) %1, ptr noundef nonnull align 1 dereferenceable(12) @__const.executor.event, i64 12, i1 false)
  %22 = call i32 @epoll_ctl(i32 noundef %17, i32 noundef 1, i32 noundef %16, ptr noundef nonnull %1) #34
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %1) #31
  br label %23

23:                                               ; preds = %5, %14, %21
  %24 = call align 4 ptr @llvm.threadlocal.address.p0(ptr align 4 @fa_coop_budget)
  store i32 128, ptr %24, align 4, !tbaa !20
  store ptr %6, ptr %2, align 8, !tbaa !9
  br label %25

25:                                               ; preds = %0, %23
  %26 = phi ptr [ %6, %23 ], [ %3, %0 ]
  ret ptr %26
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define void @fa_task_await(ptr nocapture noundef writeonly %0, ptr noundef %1) local_unnamed_addr #28 {
  %3 = getelementptr inbounds i8, ptr %0, i64 16
  store ptr %1, ptr %3, align 8, !tbaa !263
  %4 = getelementptr inbounds i8, ptr %1, i64 40
  store atomic i32 2, ptr %4 release, align 8
  %5 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @current_executor)
  %6 = load ptr, ptr %5, align 8, !tbaa !9
  %7 = icmp eq ptr %6, null
  br i1 %7, label %14, label %8

8:                                                ; preds = %2
  %9 = getelementptr inbounds %struct.FaExecutor, ptr %6, i64 0, i32 7
  %10 = load ptr, ptr %9, align 8, !tbaa !264
  %11 = icmp eq ptr %10, %1
  br i1 %11, label %12, label %14

12:                                               ; preds = %8
  %13 = getelementptr inbounds %struct.FaExecutor, ptr %6, i64 0, i32 17
  store ptr null, ptr %13, align 8, !tbaa !265
  br label %14

14:                                               ; preds = %12, %8, %2
  ret void
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define void @fa_task_finish(ptr noundef %0) local_unnamed_addr #28 {
  %2 = getelementptr inbounds i8, ptr %0, i64 16
  %3 = getelementptr inbounds i8, ptr %0, i64 44
  %4 = load i32, ptr %3, align 4, !tbaa !251
  %5 = or i32 %4, 2
  store i32 %5, ptr %3, align 4, !tbaa !251
  %6 = getelementptr inbounds i8, ptr %0, i64 40
  store atomic i32 3, ptr %6 release, align 8
  %7 = and i32 %4, 1
  %8 = icmp eq i32 %7, 0
  br i1 %8, label %16, label %9

9:                                                ; preds = %1
  %10 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @current_executor)
  %11 = load ptr, ptr %10, align 8, !tbaa !9
  %12 = getelementptr inbounds %struct.FaExecutor, ptr %11, i64 0, i32 8
  %13 = load i64, ptr %12, align 8, !tbaa !253
  %14 = add nsw i64 %13, -1
  store i64 %14, ptr %12, align 8, !tbaa !253
  %15 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @frame_to_free)
  store ptr %0, ptr %15, align 8, !tbaa !9
  br label %42

16:                                               ; preds = %1
  %17 = load ptr, ptr %2, align 8, !tbaa !263
  %18 = icmp eq ptr %17, null
  br i1 %18, label %42, label %19

19:                                               ; preds = %16
  store ptr null, ptr %2, align 8, !tbaa !263
  %20 = ptrtoint ptr %17 to i64
  %21 = and i64 %20, 1
  %22 = icmp eq i64 %21, 0
  br i1 %22, label %40, label %23

23:                                               ; preds = %19
  %24 = and i64 %20, -2
  %25 = inttoptr i64 %24 to ptr
  %26 = getelementptr inbounds i8, ptr %0, i64 24
  store ptr null, ptr %26, align 8, !tbaa !255
  %27 = getelementptr inbounds %struct.FaJoin, ptr %25, i64 0, i32 3
  %28 = load ptr, ptr %27, align 8, !tbaa !266
  %29 = icmp eq ptr %28, null
  %30 = getelementptr inbounds %struct.FaJoin, ptr %25, i64 0, i32 2
  %31 = getelementptr inbounds %struct.FaTask, ptr %28, i64 0, i32 1
  %32 = select i1 %29, ptr %30, ptr %31
  store ptr %2, ptr %32, align 8, !tbaa !9
  store ptr %2, ptr %27, align 8, !tbaa !266
  %33 = getelementptr inbounds %struct.FaJoin, ptr %25, i64 0, i32 1
  %34 = load i64, ptr %33, align 8, !tbaa !268
  %35 = add nsw i64 %34, -1
  store i64 %35, ptr %33, align 8, !tbaa !268
  %36 = load ptr, ptr %25, align 8, !tbaa !269
  %37 = icmp eq ptr %36, null
  br i1 %37, label %42, label %38

38:                                               ; preds = %23
  %39 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @next_frame)
  store ptr %36, ptr %39, align 8, !tbaa !9
  store ptr null, ptr %25, align 8, !tbaa !269
  br label %42

40:                                               ; preds = %19
  %41 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @next_frame)
  store ptr %17, ptr %41, align 8, !tbaa !9
  br label %42

42:                                               ; preds = %38, %23, %40, %16, %9
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_task_drop(ptr noundef %0) local_unnamed_addr #0 {
  %2 = icmp ult ptr %0, inttoptr (i64 2 to ptr)
  br i1 %2, label %31, label %3

3:                                                ; preds = %1
  %4 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @current_executor)
  %5 = load ptr, ptr %4, align 8, !tbaa !9
  %6 = icmp eq ptr %5, null
  br i1 %6, label %30, label %7

7:                                                ; preds = %3
  %8 = getelementptr inbounds i8, ptr %0, i64 40
  %9 = load atomic i32, ptr %8 acquire, align 8
  %10 = icmp eq i32 %9, 5
  br i1 %10, label %11, label %12

11:                                               ; preds = %7
  tail call fastcc void @drain_remote(ptr noundef nonnull %5) #35
  br label %12

12:                                               ; preds = %11, %7
  %13 = load atomic i32, ptr %8 acquire, align 8
  %14 = icmp eq i32 %13, 1
  br i1 %14, label %15, label %30

15:                                               ; preds = %12
  %16 = getelementptr inbounds i8, ptr %0, i64 32
  %17 = load ptr, ptr %16, align 8, !tbaa !257
  %18 = icmp eq ptr %17, null
  %19 = getelementptr inbounds i8, ptr %0, i64 24
  %20 = load ptr, ptr %19, align 8, !tbaa !255
  %21 = getelementptr inbounds %struct.FaTask, ptr %17, i64 0, i32 1
  %22 = select i1 %18, ptr %5, ptr %21
  store ptr %20, ptr %22, align 8, !tbaa !9
  %23 = icmp eq ptr %20, null
  %24 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 1
  %25 = getelementptr inbounds %struct.FaTask, ptr %20, i64 0, i32 2
  %26 = select i1 %23, ptr %24, ptr %25
  store ptr %17, ptr %26, align 8, !tbaa !9
  store ptr null, ptr %19, align 8, !tbaa !255
  store ptr null, ptr %16, align 8, !tbaa !257
  %27 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 2
  %28 = load i64, ptr %27, align 8, !tbaa !258
  %29 = add nsw i64 %28, -1
  store i64 %29, ptr %27, align 8, !tbaa !258
  br label %30

30:                                               ; preds = %12, %15, %3
  tail call void @fa_destroy(ptr noundef %0) #34
  br label %31

31:                                               ; preds = %1, %30
  ret void
}

; Function Attrs: nofree norecurse nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc void @drain_remote(ptr noundef %0) unnamed_addr #29 {
  %2 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 10
  %3 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 11, i32 1
  %4 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 9
  %5 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 11
  %6 = ptrtoint ptr %5 to i64
  %7 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 1
  %8 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 2
  br label %9

9:                                                ; preds = %38, %1
  %10 = load ptr, ptr %4, align 8, !tbaa !259
  %11 = getelementptr inbounds %struct.FaTask, ptr %10, i64 0, i32 1
  %12 = load atomic i64, ptr %11 acquire, align 8
  %13 = inttoptr i64 %12 to ptr
  %14 = icmp eq ptr %10, %5
  br i1 %14, label %15, label %21

15:                                               ; preds = %9
  %16 = icmp eq i64 %12, 0
  br i1 %16, label %49, label %17

17:                                               ; preds = %15
  store ptr %13, ptr %4, align 8, !tbaa !259
  %18 = getelementptr inbounds %struct.FaTask, ptr %13, i64 0, i32 1
  %19 = load atomic i64, ptr %18 acquire, align 8
  %20 = inttoptr i64 %19 to ptr
  br label %21

21:                                               ; preds = %17, %9
  %22 = phi ptr [ %20, %17 ], [ %13, %9 ]
  %23 = phi ptr [ %13, %17 ], [ %10, %9 ]
  %24 = icmp eq ptr %22, null
  br i1 %24, label %25, label %38

25:                                               ; preds = %21
  %26 = load atomic i64, ptr %2 acquire, align 8
  %27 = inttoptr i64 %26 to ptr
  %28 = icmp eq ptr %23, %27
  br i1 %28, label %29, label %49

29:                                               ; preds = %25
  store atomic i64 0, ptr %3 monotonic, align 8
  %30 = atomicrmw xchg ptr %2, i64 %6 acq_rel, align 8
  %31 = inttoptr i64 %30 to ptr
  %32 = getelementptr inbounds %struct.FaTask, ptr %31, i64 0, i32 1
  store atomic i64 %6, ptr %32 release, align 8
  %33 = getelementptr inbounds %struct.FaTask, ptr %23, i64 0, i32 1
  %34 = load atomic i64, ptr %33 acquire, align 8
  %35 = icmp eq i64 %34, 0
  br i1 %35, label %49, label %36

36:                                               ; preds = %29
  %37 = inttoptr i64 %34 to ptr
  br label %38

38:                                               ; preds = %36, %21
  %39 = phi ptr [ %37, %36 ], [ %22, %21 ]
  store ptr %39, ptr %4, align 8, !tbaa !259
  %40 = getelementptr inbounds %struct.FaTask, ptr %23, i64 0, i32 1
  store ptr null, ptr %40, align 8, !tbaa !255
  %41 = load ptr, ptr %7, align 8, !tbaa !256
  %42 = getelementptr inbounds %struct.FaTask, ptr %23, i64 0, i32 2
  store ptr %41, ptr %42, align 8, !tbaa !257
  %43 = icmp eq ptr %41, null
  %44 = getelementptr inbounds %struct.FaTask, ptr %41, i64 0, i32 1
  %45 = select i1 %43, ptr %0, ptr %44
  store ptr %23, ptr %45, align 8, !tbaa !9
  store ptr %23, ptr %7, align 8, !tbaa !256
  %46 = load i64, ptr %8, align 8, !tbaa !258
  %47 = add nsw i64 %46, 1
  store i64 %47, ptr %8, align 8, !tbaa !258
  %48 = getelementptr inbounds %struct.FaTask, ptr %23, i64 0, i32 3
  store atomic i32 1, ptr %48 release, align 8
  br label %9, !llvm.loop !270

49:                                               ; preds = %15, %25, %29
  ret void
}

; Function Attrs: optsize
declare void @fa_destroy(ptr noundef) local_unnamed_addr #1

; Function Attrs: nounwind optsize
define void @fa_task_yield(ptr noundef %0) local_unnamed_addr #0 {
  %2 = tail call fastcc ptr @executor() #35
  %3 = getelementptr inbounds i8, ptr %0, i64 16
  %4 = getelementptr inbounds i8, ptr %0, i64 24
  store ptr null, ptr %4, align 8, !tbaa !255
  %5 = getelementptr inbounds %struct.FaExecutor, ptr %2, i64 0, i32 1
  %6 = load ptr, ptr %5, align 8, !tbaa !256
  %7 = getelementptr inbounds i8, ptr %0, i64 32
  store ptr %6, ptr %7, align 8, !tbaa !257
  %8 = icmp eq ptr %6, null
  %9 = getelementptr inbounds %struct.FaTask, ptr %6, i64 0, i32 1
  %10 = select i1 %8, ptr %2, ptr %9
  store ptr %3, ptr %10, align 8, !tbaa !9
  store ptr %3, ptr %5, align 8, !tbaa !256
  %11 = getelementptr inbounds %struct.FaExecutor, ptr %2, i64 0, i32 2
  %12 = load i64, ptr %11, align 8, !tbaa !258
  %13 = add nsw i64 %12, 1
  store i64 %13, ptr %11, align 8, !tbaa !258
  %14 = getelementptr inbounds i8, ptr %0, i64 40
  store atomic i32 1, ptr %14 release, align 8
  ret void
}

; Function Attrs: nounwind optsize
define i32 @fa_coop_back_edge() local_unnamed_addr #0 {
  %1 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @current_executor)
  %2 = load ptr, ptr %1, align 8, !tbaa !9
  %3 = tail call align 4 ptr @llvm.threadlocal.address.p0(ptr align 4 @fa_coop_budget)
  store i32 128, ptr %3, align 4, !tbaa !20
  %4 = icmp eq ptr %2, null
  br i1 %4, label %15, label %5

5:                                                ; preds = %0
  %6 = tail call i64 @fa_clock() #34
  %7 = getelementptr inbounds %struct.FaExecutor, ptr %2, i64 0, i32 16
  %8 = load i64, ptr %7, align 8, !tbaa !271
  %9 = icmp eq i64 %8, 0
  br i1 %9, label %10, label %11

10:                                               ; preds = %5
  store i64 %6, ptr %7, align 8, !tbaa !271
  br label %15

11:                                               ; preds = %5
  %12 = sub nsw i64 %6, %8
  %13 = icmp sgt i64 %12, 2000000
  %14 = zext i1 %13 to i32
  br label %15

15:                                               ; preds = %10, %11, %0
  %16 = phi i32 [ 0, %0 ], [ 0, %10 ], [ %14, %11 ]
  ret i32 %16
}

; Function Attrs: optsize
declare i64 @fa_clock() local_unnamed_addr #1

; Function Attrs: nounwind optsize
define void @fa_sleep_async(ptr noundef %0, ptr noundef %1, i64 noundef %2) local_unnamed_addr #0 {
  %4 = tail call fastcc ptr @executor() #35
  store ptr %1, ptr %0, align 8, !tbaa !272
  %5 = getelementptr inbounds %struct.FaWait, ptr %0, i64 0, i32 1
  store ptr %4, ptr %5, align 8, !tbaa !274
  %6 = getelementptr inbounds %struct.FaWait, ptr %0, i64 0, i32 8
  store i64 -1, ptr %6, align 8, !tbaa !275
  %7 = icmp slt i64 %2, 1
  br i1 %7, label %8, label %21

8:                                                ; preds = %3
  %9 = getelementptr inbounds i8, ptr %1, i64 16
  %10 = getelementptr inbounds i8, ptr %1, i64 24
  store ptr null, ptr %10, align 8, !tbaa !255
  %11 = getelementptr inbounds %struct.FaExecutor, ptr %4, i64 0, i32 1
  %12 = load ptr, ptr %11, align 8, !tbaa !256
  %13 = getelementptr inbounds i8, ptr %1, i64 32
  store ptr %12, ptr %13, align 8, !tbaa !257
  %14 = icmp eq ptr %12, null
  %15 = getelementptr inbounds %struct.FaTask, ptr %12, i64 0, i32 1
  %16 = select i1 %14, ptr %4, ptr %15
  store ptr %9, ptr %16, align 8, !tbaa !9
  store ptr %9, ptr %11, align 8, !tbaa !256
  %17 = getelementptr inbounds %struct.FaExecutor, ptr %4, i64 0, i32 2
  %18 = load i64, ptr %17, align 8, !tbaa !258
  %19 = add nsw i64 %18, 1
  store i64 %19, ptr %17, align 8, !tbaa !258
  %20 = getelementptr inbounds i8, ptr %1, i64 40
  store atomic i32 1, ptr %20 release, align 8
  br label %74

21:                                               ; preds = %3
  %22 = getelementptr inbounds %struct.FaExecutor, ptr %4, i64 0, i32 4
  %23 = load i64, ptr %22, align 8, !tbaa !276
  %24 = getelementptr inbounds %struct.FaExecutor, ptr %4, i64 0, i32 5
  %25 = load i64, ptr %24, align 8, !tbaa !277
  %26 = icmp eq i64 %23, %25
  br i1 %26, label %27, label %54

27:                                               ; preds = %21
  %28 = icmp slt i64 %23, 16
  %29 = shl nuw nsw i64 %23, 1
  %30 = select i1 %28, i64 16, i64 %29
  %31 = shl i64 %30, 3
  %32 = tail call ptr @fa_alloc(i64 noundef %31, i64 noundef 8) #34
  %33 = load i64, ptr %22, align 8, !tbaa !276
  %34 = icmp sgt i64 %33, 0
  br i1 %34, label %35, label %37

35:                                               ; preds = %27
  %36 = getelementptr inbounds %struct.FaExecutor, ptr %4, i64 0, i32 3
  br label %40

37:                                               ; preds = %40, %27
  %38 = load i64, ptr %24, align 8, !tbaa !277
  %39 = icmp sgt i64 %38, 0
  br i1 %39, label %48, label %52

40:                                               ; preds = %35, %40
  %41 = phi i64 [ 0, %35 ], [ %46, %40 ]
  %42 = load ptr, ptr %36, align 8, !tbaa !278
  %43 = getelementptr inbounds ptr, ptr %42, i64 %41
  %44 = load ptr, ptr %43, align 8, !tbaa !9
  %45 = getelementptr inbounds ptr, ptr %32, i64 %41
  store ptr %44, ptr %45, align 8, !tbaa !9
  %46 = add nuw nsw i64 %41, 1
  %47 = icmp eq i64 %46, %33
  br i1 %47, label %37, label %40, !llvm.loop !279

48:                                               ; preds = %37
  %49 = getelementptr inbounds %struct.FaExecutor, ptr %4, i64 0, i32 3
  %50 = load ptr, ptr %49, align 8, !tbaa !278
  %51 = shl i64 %38, 3
  tail call void @fa_free(ptr noundef %50, i64 noundef %51, i64 noundef 8) #34
  br label %52

52:                                               ; preds = %48, %37
  %53 = getelementptr inbounds %struct.FaExecutor, ptr %4, i64 0, i32 3
  store ptr %32, ptr %53, align 8, !tbaa !278
  store i64 %30, ptr %24, align 8, !tbaa !277
  br label %54

54:                                               ; preds = %52, %21
  %55 = tail call i64 @fa_clock() #34
  %56 = mul nsw i64 %2, 1000000
  %57 = add nsw i64 %55, %56
  %58 = getelementptr inbounds %struct.FaWait, ptr %0, i64 0, i32 6
  store i64 %57, ptr %58, align 8, !tbaa !280
  %59 = getelementptr inbounds %struct.FaExecutor, ptr %4, i64 0, i32 6
  %60 = load i64, ptr %59, align 8, !tbaa !281
  %61 = getelementptr inbounds %struct.FaWait, ptr %0, i64 0, i32 7
  store i64 %60, ptr %61, align 8, !tbaa !282
  %62 = add nsw i64 %60, 1
  store i64 %62, ptr %59, align 8, !tbaa !281
  %63 = load i64, ptr %22, align 8, !tbaa !276
  %64 = add nsw i64 %63, 1
  store i64 %64, ptr %22, align 8, !tbaa !276
  %65 = getelementptr inbounds %struct.FaExecutor, ptr %4, i64 0, i32 3
  %66 = load ptr, ptr %65, align 8, !tbaa !278
  %67 = getelementptr inbounds ptr, ptr %66, i64 %63
  store ptr %0, ptr %67, align 8, !tbaa !9
  store i64 %63, ptr %6, align 8, !tbaa !275
  tail call fastcc void @timer_up(ptr noundef nonnull %4, i64 noundef %63) #35
  %68 = getelementptr inbounds i8, ptr %1, i64 40
  store atomic i32 2, ptr %68 release, align 8
  %69 = getelementptr inbounds %struct.FaExecutor, ptr %4, i64 0, i32 7
  %70 = load ptr, ptr %69, align 8, !tbaa !264
  %71 = icmp eq ptr %70, %1
  br i1 %71, label %72, label %74

72:                                               ; preds = %54
  %73 = getelementptr inbounds %struct.FaExecutor, ptr %4, i64 0, i32 17
  store ptr null, ptr %73, align 8, !tbaa !265
  br label %74

74:                                               ; preds = %54, %72, %8
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc void @timer_up(ptr nocapture noundef readonly %0, i64 noundef %1) unnamed_addr #10 {
  %3 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 3
  %4 = load ptr, ptr %3, align 8, !tbaa !278
  %5 = getelementptr inbounds ptr, ptr %4, i64 %1
  %6 = load ptr, ptr %5, align 8, !tbaa !9
  %7 = getelementptr inbounds %struct.FaWait, ptr %6, i64 0, i32 7
  %8 = icmp sgt i64 %1, 0
  br i1 %8, label %9, label %35

9:                                                ; preds = %2
  %10 = getelementptr inbounds %struct.FaWait, ptr %6, i64 0, i32 6
  %11 = load i64, ptr %10, align 8, !tbaa !280
  br label %12

12:                                               ; preds = %9, %29
  %13 = phi i64 [ %1, %9 ], [ %15, %29 ]
  %14 = add nsw i64 %13, -1
  %15 = lshr i64 %14, 2
  %16 = load ptr, ptr %3, align 8, !tbaa !278
  %17 = getelementptr inbounds ptr, ptr %16, i64 %15
  %18 = load ptr, ptr %17, align 8, !tbaa !9
  %19 = getelementptr inbounds %struct.FaWait, ptr %18, i64 0, i32 6
  %20 = load i64, ptr %19, align 8, !tbaa !280
  %21 = icmp slt i64 %11, %20
  br i1 %21, label %29, label %22

22:                                               ; preds = %12
  %23 = icmp eq i64 %11, %20
  br i1 %23, label %24, label %35

24:                                               ; preds = %22
  %25 = load i64, ptr %7, align 8, !tbaa !282
  %26 = getelementptr inbounds %struct.FaWait, ptr %18, i64 0, i32 7
  %27 = load i64, ptr %26, align 8, !tbaa !282
  %28 = icmp slt i64 %25, %27
  br i1 %28, label %29, label %35

29:                                               ; preds = %12, %24
  %30 = getelementptr inbounds ptr, ptr %16, i64 %13
  store ptr %18, ptr %30, align 8, !tbaa !9
  %31 = getelementptr inbounds %struct.FaWait, ptr %18, i64 0, i32 8
  store i64 %13, ptr %31, align 8, !tbaa !275
  %32 = icmp ult i64 %14, 4
  br i1 %32, label %33, label %12

33:                                               ; preds = %29
  %34 = load ptr, ptr %3, align 8, !tbaa !278
  br label %35

35:                                               ; preds = %24, %22, %33, %2
  %36 = phi ptr [ %4, %2 ], [ %34, %33 ], [ %16, %22 ], [ %16, %24 ]
  %37 = phi i64 [ %1, %2 ], [ %15, %33 ], [ %13, %22 ], [ %13, %24 ]
  %38 = getelementptr inbounds ptr, ptr %36, i64 %37
  store ptr %6, ptr %38, align 8, !tbaa !9
  %39 = getelementptr inbounds %struct.FaWait, ptr %6, i64 0, i32 8
  store i64 %37, ptr %39, align 8, !tbaa !275
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define void @fa_timer_cancel(ptr nocapture noundef readonly %0) local_unnamed_addr #10 {
  %2 = getelementptr inbounds %struct.FaWait, ptr %0, i64 0, i32 8
  %3 = load i64, ptr %2, align 8, !tbaa !275
  %4 = icmp sgt i64 %3, -1
  br i1 %4, label %5, label %8

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaWait, ptr %0, i64 0, i32 1
  %7 = load ptr, ptr %6, align 8, !tbaa !274
  tail call fastcc void @timer_remove(ptr noundef %7, i64 noundef %3) #35
  br label %8

8:                                                ; preds = %5, %1
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc void @timer_remove(ptr nocapture noundef %0, i64 noundef %1) unnamed_addr #10 {
  %3 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 3
  %4 = load ptr, ptr %3, align 8, !tbaa !278
  %5 = getelementptr inbounds ptr, ptr %4, i64 %1
  %6 = load ptr, ptr %5, align 8, !tbaa !9
  %7 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 4
  %8 = load i64, ptr %7, align 8, !tbaa !276
  %9 = add nsw i64 %8, -1
  store i64 %9, ptr %7, align 8, !tbaa !276
  %10 = icmp sgt i64 %9, %1
  br i1 %10, label %11, label %108

11:                                               ; preds = %2
  %12 = getelementptr inbounds ptr, ptr %4, i64 %9
  %13 = load ptr, ptr %12, align 8, !tbaa !9
  store ptr %13, ptr %5, align 8, !tbaa !9
  %14 = getelementptr inbounds %struct.FaWait, ptr %13, i64 0, i32 8
  store i64 %1, ptr %14, align 8, !tbaa !275
  %15 = icmp sgt i64 %1, 0
  %16 = load ptr, ptr %3, align 8, !tbaa !278
  br i1 %15, label %17, label %36

17:                                               ; preds = %11
  %18 = add nsw i64 %1, -1
  %19 = lshr i64 %18, 2
  %20 = getelementptr inbounds ptr, ptr %16, i64 %19
  %21 = load ptr, ptr %20, align 8, !tbaa !9
  %22 = getelementptr inbounds %struct.FaWait, ptr %13, i64 0, i32 6
  %23 = load i64, ptr %22, align 8, !tbaa !280
  %24 = getelementptr inbounds %struct.FaWait, ptr %21, i64 0, i32 6
  %25 = load i64, ptr %24, align 8, !tbaa !280
  %26 = icmp slt i64 %23, %25
  br i1 %26, label %35, label %27

27:                                               ; preds = %17
  %28 = icmp eq i64 %23, %25
  br i1 %28, label %29, label %36

29:                                               ; preds = %27
  %30 = getelementptr inbounds %struct.FaWait, ptr %13, i64 0, i32 7
  %31 = load i64, ptr %30, align 8, !tbaa !282
  %32 = getelementptr inbounds %struct.FaWait, ptr %21, i64 0, i32 7
  %33 = load i64, ptr %32, align 8, !tbaa !282
  %34 = icmp slt i64 %31, %33
  br i1 %34, label %35, label %36

35:                                               ; preds = %17, %29
  tail call fastcc void @timer_up(ptr noundef nonnull %0, i64 noundef %1) #35
  br label %108

36:                                               ; preds = %27, %29, %11
  %37 = getelementptr inbounds ptr, ptr %16, i64 %1
  %38 = load ptr, ptr %37, align 8, !tbaa !9
  %39 = shl nsw i64 %1, 2
  %40 = or disjoint i64 %39, 1
  %41 = icmp slt i64 %40, %9
  br i1 %41, label %42, label %103

42:                                               ; preds = %36
  %43 = getelementptr inbounds %struct.FaWait, ptr %38, i64 0, i32 6
  %44 = load i64, ptr %43, align 8, !tbaa !280
  %45 = getelementptr inbounds %struct.FaWait, ptr %38, i64 0, i32 7
  br label %46

46:                                               ; preds = %95, %42
  %47 = phi i64 [ %40, %42 ], [ %99, %95 ]
  %48 = phi i64 [ %39, %42 ], [ %98, %95 ]
  %49 = phi i64 [ %1, %42 ], [ %56, %95 ]
  %50 = add i64 %48, 5
  %51 = tail call i64 @llvm.smin.i64(i64 %50, i64 %9)
  %52 = or disjoint i64 %48, 2
  %53 = icmp slt i64 %52, %51
  %54 = load ptr, ptr %3, align 8, !tbaa !278
  br i1 %53, label %69, label %55

55:                                               ; preds = %91, %46
  %56 = phi i64 [ %47, %46 ], [ %92, %91 ]
  %57 = getelementptr inbounds ptr, ptr %54, i64 %56
  %58 = load ptr, ptr %57, align 8, !tbaa !9
  %59 = getelementptr inbounds %struct.FaWait, ptr %58, i64 0, i32 6
  %60 = load i64, ptr %59, align 8, !tbaa !280
  %61 = icmp slt i64 %60, %44
  br i1 %61, label %95, label %62

62:                                               ; preds = %55
  %63 = icmp eq i64 %60, %44
  br i1 %63, label %64, label %103

64:                                               ; preds = %62
  %65 = getelementptr inbounds %struct.FaWait, ptr %58, i64 0, i32 7
  %66 = load i64, ptr %65, align 8, !tbaa !282
  %67 = load i64, ptr %45, align 8, !tbaa !282
  %68 = icmp slt i64 %66, %67
  br i1 %68, label %95, label %103

69:                                               ; preds = %46, %91
  %70 = phi i64 [ %93, %91 ], [ %52, %46 ]
  %71 = phi i64 [ %92, %91 ], [ %47, %46 ]
  %72 = getelementptr inbounds ptr, ptr %54, i64 %70
  %73 = load ptr, ptr %72, align 8, !tbaa !9
  %74 = getelementptr inbounds ptr, ptr %54, i64 %71
  %75 = load ptr, ptr %74, align 8, !tbaa !9
  %76 = getelementptr inbounds %struct.FaWait, ptr %73, i64 0, i32 6
  %77 = load i64, ptr %76, align 8, !tbaa !280
  %78 = getelementptr inbounds %struct.FaWait, ptr %75, i64 0, i32 6
  %79 = load i64, ptr %78, align 8, !tbaa !280
  %80 = icmp slt i64 %77, %79
  br i1 %80, label %91, label %81

81:                                               ; preds = %69
  %82 = icmp eq i64 %77, %79
  br i1 %82, label %83, label %90

83:                                               ; preds = %81
  %84 = getelementptr inbounds %struct.FaWait, ptr %73, i64 0, i32 7
  %85 = load i64, ptr %84, align 8, !tbaa !282
  %86 = getelementptr inbounds %struct.FaWait, ptr %75, i64 0, i32 7
  %87 = load i64, ptr %86, align 8, !tbaa !282
  %88 = icmp sge i64 %85, %87
  %89 = freeze i1 %88
  br i1 %89, label %90, label %91

90:                                               ; preds = %83, %81
  br label %91

91:                                               ; preds = %90, %83, %69
  %92 = phi i64 [ %71, %90 ], [ %70, %83 ], [ %70, %69 ]
  %93 = add nsw i64 %70, 1
  %94 = icmp eq i64 %93, %51
  br i1 %94, label %55, label %69, !llvm.loop !283

95:                                               ; preds = %64, %55
  %96 = getelementptr inbounds ptr, ptr %54, i64 %49
  store ptr %58, ptr %96, align 8, !tbaa !9
  %97 = getelementptr inbounds %struct.FaWait, ptr %58, i64 0, i32 8
  store i64 %49, ptr %97, align 8, !tbaa !275
  %98 = shl nsw i64 %56, 2
  %99 = or disjoint i64 %98, 1
  %100 = icmp slt i64 %99, %9
  br i1 %100, label %46, label %101

101:                                              ; preds = %95
  %102 = load ptr, ptr %3, align 8, !tbaa !278
  br label %103

103:                                              ; preds = %62, %64, %36, %101
  %104 = phi ptr [ %16, %36 ], [ %102, %101 ], [ %54, %64 ], [ %54, %62 ]
  %105 = phi i64 [ %1, %36 ], [ %56, %101 ], [ %49, %64 ], [ %49, %62 ]
  %106 = getelementptr inbounds ptr, ptr %104, i64 %105
  store ptr %38, ptr %106, align 8, !tbaa !9
  %107 = getelementptr inbounds %struct.FaWait, ptr %38, i64 0, i32 8
  store i64 %105, ptr %107, align 8, !tbaa !275
  br label %108

108:                                              ; preds = %35, %103, %2
  %109 = getelementptr inbounds %struct.FaWait, ptr %6, i64 0, i32 8
  store i64 -1, ptr %109, align 8, !tbaa !275
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_sleep(i64 noundef %0) local_unnamed_addr #0 {
  %2 = icmp sgt i64 %0, 0
  br i1 %2, label %3, label %5

3:                                                ; preds = %1
  %4 = mul nsw i64 %0, 1000000
  tail call void @fa_nanosleep(i64 noundef %4) #34
  br label %5

5:                                                ; preds = %3, %1
  ret void
}

; Function Attrs: optsize
declare void @fa_nanosleep(i64 noundef) local_unnamed_addr #1

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: write)
define void @fa_join_init(ptr nocapture noundef writeonly %0) local_unnamed_addr #21 {
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %0, i8 0, i64 32, i1 false)
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, argmem: readwrite, inaccessiblemem: none)
define void @fa_join_add(ptr noundef %0, ptr noundef %1) local_unnamed_addr #26 {
  %3 = icmp ult ptr %1, inttoptr (i64 2 to ptr)
  br i1 %3, label %23, label %4

4:                                                ; preds = %2
  %5 = getelementptr inbounds i8, ptr %1, i64 16
  %6 = load ptr, ptr %1, align 8, !tbaa !9
  %7 = icmp eq ptr %6, null
  br i1 %7, label %8, label %16

8:                                                ; preds = %4
  %9 = getelementptr inbounds i8, ptr %1, i64 24
  store ptr null, ptr %9, align 8, !tbaa !255
  %10 = getelementptr inbounds %struct.FaJoin, ptr %0, i64 0, i32 3
  %11 = load ptr, ptr %10, align 8, !tbaa !266
  %12 = icmp eq ptr %11, null
  %13 = getelementptr inbounds %struct.FaJoin, ptr %0, i64 0, i32 2
  %14 = getelementptr inbounds %struct.FaTask, ptr %11, i64 0, i32 1
  %15 = select i1 %12, ptr %13, ptr %14
  store ptr %5, ptr %15, align 8, !tbaa !9
  store ptr %5, ptr %10, align 8, !tbaa !266
  br label %23

16:                                               ; preds = %4
  %17 = ptrtoint ptr %0 to i64
  %18 = or i64 %17, 1
  %19 = inttoptr i64 %18 to ptr
  store ptr %19, ptr %5, align 8, !tbaa !263
  %20 = getelementptr inbounds %struct.FaJoin, ptr %0, i64 0, i32 1
  %21 = load i64, ptr %20, align 8, !tbaa !268
  %22 = add nsw i64 %21, 1
  store i64 %22, ptr %20, align 8, !tbaa !268
  br label %23

23:                                               ; preds = %8, %16, %2
  ret void
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define ptr @fa_join_next(ptr nocapture noundef %0, ptr noundef %1) local_unnamed_addr #28 {
  %3 = getelementptr inbounds %struct.FaJoin, ptr %0, i64 0, i32 2
  %4 = load ptr, ptr %3, align 8, !tbaa !284
  %5 = icmp eq ptr %4, null
  br i1 %5, label %14, label %6

6:                                                ; preds = %2
  %7 = getelementptr inbounds %struct.FaTask, ptr %4, i64 0, i32 1
  %8 = load ptr, ptr %7, align 8, !tbaa !255
  store ptr %8, ptr %3, align 8, !tbaa !284
  %9 = icmp eq ptr %8, null
  br i1 %9, label %10, label %12

10:                                               ; preds = %6
  %11 = getelementptr inbounds %struct.FaJoin, ptr %0, i64 0, i32 3
  store ptr null, ptr %11, align 8, !tbaa !266
  br label %12

12:                                               ; preds = %10, %6
  store ptr null, ptr %7, align 8, !tbaa !255
  %13 = getelementptr inbounds i8, ptr %4, i64 -16
  br label %22

14:                                               ; preds = %2
  %15 = getelementptr inbounds %struct.FaJoin, ptr %0, i64 0, i32 1
  %16 = load i64, ptr %15, align 8, !tbaa !268
  %17 = icmp eq i64 %16, 0
  br i1 %17, label %22, label %18

18:                                               ; preds = %14
  %19 = icmp eq ptr %1, null
  br i1 %19, label %22, label %20

20:                                               ; preds = %18
  store ptr %1, ptr %0, align 8, !tbaa !269
  %21 = getelementptr inbounds i8, ptr %1, i64 40
  store atomic i32 2, ptr %21 release, align 8
  br label %22

22:                                               ; preds = %18, %14, %20, %12
  %23 = phi ptr [ %13, %12 ], [ null, %20 ], [ inttoptr (i64 1 to ptr), %14 ], [ null, %18 ]
  ret ptr %23
}

; Function Attrs: cold noinline noreturn nounwind optsize
define void @fa_task_stuck() local_unnamed_addr #22 {
  tail call fastcc void @deadlock(ptr noundef nonnull @.str.30) #39
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define internal fastcc void @deadlock(ptr noundef %0) unnamed_addr #22 {
  tail call void @fa_flush() #35
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.44, i64 noundef 16) #34
  br label %2

2:                                                ; preds = %2, %1
  %3 = phi i64 [ 0, %1 ], [ %7, %2 ]
  %4 = getelementptr inbounds i8, ptr %0, i64 %3
  %5 = load i8, ptr %4, align 1, !tbaa !12
  %6 = icmp eq i8 %5, 0
  %7 = add i64 %3, 1
  br i1 %6, label %8, label %2, !llvm.loop !24

8:                                                ; preds = %2
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull %0, i64 noundef %3) #34
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.22, i64 noundef 1) #34
  tail call void @fa_exit(i32 noundef 101) #36
  unreachable
}

; Function Attrs: nounwind optsize
define void @fa_run_main(ptr noundef %0, i32 noundef %1) local_unnamed_addr #0 {
  %3 = alloca [16 x %struct.FaEpollEvent], align 16
  %4 = alloca i64, align 8
  %5 = tail call fastcc ptr @executor() #35
  %6 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 7
  store ptr %0, ptr %6, align 8, !tbaa !264
  %7 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 15
  store i32 %1, ptr %7, align 4, !tbaa !285
  %8 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 8
  %9 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 2
  %10 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 1
  %11 = tail call align 4 ptr @llvm.threadlocal.address.p0(ptr align 4 @fa_coop_budget)
  %12 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 16
  %13 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @frame_to_free)
  %14 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @next_frame)
  %15 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @fa_turn_site)
  %16 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 20
  %17 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %18 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 4
  %19 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 3
  %20 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 13
  %21 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 12
  %22 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 9
  %23 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 10
  %24 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 14
  br label %25

25:                                               ; preds = %184, %2
  %26 = load ptr, ptr %0, align 8, !tbaa !9
  %27 = icmp eq ptr %26, null
  br i1 %27, label %28, label %31

28:                                               ; preds = %25
  %29 = load i64, ptr %8, align 8, !tbaa !253
  %30 = icmp sgt i64 %29, 0
  br i1 %30, label %31, label %185

31:                                               ; preds = %25, %28
  %32 = load i64, ptr %9, align 8, !tbaa !258
  %33 = icmp sgt i64 %32, 0
  br i1 %33, label %34, label %99

34:                                               ; preds = %31
  %35 = call i64 @llvm.smin.i64(i64 %32, i64 61)
  br label %36

36:                                               ; preds = %96, %34
  %37 = phi i64 [ %35, %34 ], [ %97, %96 ]
  %38 = load ptr, ptr %5, align 8, !tbaa !286
  %39 = icmp eq ptr %38, null
  br i1 %39, label %99, label %40

40:                                               ; preds = %36
  %41 = getelementptr inbounds %struct.FaTask, ptr %38, i64 0, i32 1
  %42 = load ptr, ptr %41, align 8, !tbaa !255
  store ptr %42, ptr %5, align 8, !tbaa !286
  %43 = icmp eq ptr %42, null
  %44 = getelementptr inbounds %struct.FaTask, ptr %42, i64 0, i32 2
  %45 = select i1 %43, ptr %10, ptr %44
  store ptr null, ptr %45, align 8, !tbaa !9
  %46 = load i64, ptr %9, align 8, !tbaa !258
  %47 = add nsw i64 %46, -1
  store i64 %47, ptr %9, align 8, !tbaa !258
  store ptr null, ptr %41, align 8, !tbaa !255
  %48 = getelementptr inbounds %struct.FaTask, ptr %38, i64 0, i32 2
  store ptr null, ptr %48, align 8, !tbaa !257
  %49 = getelementptr inbounds i8, ptr %38, i64 -16
  store i32 128, ptr %11, align 4, !tbaa !20
  store i64 0, ptr %12, align 8, !tbaa !271
  %50 = load i32, ptr %7, align 4, !tbaa !285
  %51 = icmp eq i32 %50, 0
  br i1 %51, label %54, label %52

52:                                               ; preds = %40
  %53 = call i64 @fa_clock() #34
  br label %54

54:                                               ; preds = %52, %40
  %55 = phi i64 [ %53, %52 ], [ 0, %40 ]
  br label %56

56:                                               ; preds = %62, %54
  %57 = phi ptr [ %49, %54 ], [ %63, %62 ]
  %58 = getelementptr inbounds i8, ptr %57, i64 40
  store atomic i32 4, ptr %58 release, align 8
  call void @fa_resume(ptr noundef nonnull %57) #34
  %59 = load ptr, ptr %13, align 8, !tbaa !9
  %60 = icmp eq ptr %59, null
  br i1 %60, label %62, label %61

61:                                               ; preds = %56
  store ptr null, ptr %13, align 8, !tbaa !9
  call void @fa_destroy(ptr noundef nonnull %59) #34
  br label %62

62:                                               ; preds = %61, %56
  %63 = load ptr, ptr %14, align 8, !tbaa !9
  store ptr null, ptr %14, align 8, !tbaa !9
  %64 = icmp eq ptr %63, null
  br i1 %64, label %65, label %56, !llvm.loop !287

65:                                               ; preds = %62
  %66 = load i32, ptr %7, align 4, !tbaa !285
  %67 = icmp eq i32 %66, 0
  br i1 %67, label %96, label %68

68:                                               ; preds = %65
  %69 = call i64 @fa_clock() #34
  %70 = sub nsw i64 %69, %55
  %71 = icmp sgt i64 %70, 50000000
  br i1 %71, label %72, label %96

72:                                               ; preds = %68
  %73 = load ptr, ptr %15, align 8, !tbaa !9
  %74 = load i32, ptr %16, align 8, !tbaa !288
  %75 = icmp sgt i32 %74, 0
  br i1 %75, label %76, label %88

76:                                               ; preds = %72
  %77 = zext nneg i32 %74 to i64
  br label %81

78:                                               ; preds = %81
  %79 = add nuw nsw i64 %82, 1
  %80 = icmp eq i64 %79, %77
  br i1 %80, label %86, label %81, !llvm.loop !289

81:                                               ; preds = %78, %76
  %82 = phi i64 [ 0, %76 ], [ %79, %78 ]
  %83 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 19, i64 %82
  %84 = load ptr, ptr %83, align 8, !tbaa !9
  %85 = icmp eq ptr %84, %73
  br i1 %85, label %96, label %78

86:                                               ; preds = %78
  %87 = icmp slt i32 %74, 32
  br i1 %87, label %88, label %92

88:                                               ; preds = %86, %72
  %89 = sext i32 %74 to i64
  %90 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 19, i64 %89
  store ptr %73, ptr %90, align 8, !tbaa !9
  %91 = add nsw i32 %74, 1
  store i32 %91, ptr %16, align 8, !tbaa !288
  br label %92

92:                                               ; preds = %88, %86
  call void @fa_flush() #35
  store i64 0, ptr %17, align 8, !tbaa !5
  call fastcc void @append_c(ptr noundef nonnull @.str.45) #35
  %93 = udiv i64 %70, 1000000
  call fastcc void @append_u64(i64 noundef %93, i32 noundef 0) #35
  call fastcc void @append_c(ptr noundef nonnull @.str.46) #35
  %94 = icmp eq ptr %73, null
  %95 = select i1 %94, ptr @.str.47, ptr %73
  call fastcc void @append_c(ptr noundef nonnull %95) #35
  call fastcc void @append_c(ptr noundef nonnull @.str.48) #35
  call void @fa_b_line(i32 noundef 2) #35
  br label %96

96:                                               ; preds = %81, %92, %68, %65
  %97 = add nsw i64 %37, -1
  %98 = icmp sgt i64 %37, 1
  br i1 %98, label %36, label %99, !llvm.loop !290

99:                                               ; preds = %96, %36, %31
  %100 = load i64, ptr %18, align 8, !tbaa !276
  %101 = icmp sgt i64 %100, 0
  br i1 %101, label %102, label %126

102:                                              ; preds = %99
  %103 = call i64 @fa_clock() #34
  %104 = load i64, ptr %18, align 8, !tbaa !276
  %105 = icmp sgt i64 %104, 0
  br i1 %105, label %106, label %126

106:                                              ; preds = %102, %112
  %107 = load ptr, ptr %19, align 8, !tbaa !278
  %108 = load ptr, ptr %107, align 8, !tbaa !9
  %109 = getelementptr inbounds %struct.FaWait, ptr %108, i64 0, i32 6
  %110 = load i64, ptr %109, align 8, !tbaa !280
  %111 = icmp sgt i64 %110, %103
  br i1 %111, label %126, label %112

112:                                              ; preds = %106
  call fastcc void @timer_remove(ptr noundef nonnull %5, i64 noundef 0) #35
  %113 = load ptr, ptr %108, align 8, !tbaa !272
  %114 = getelementptr inbounds i8, ptr %113, i64 16
  %115 = getelementptr inbounds i8, ptr %113, i64 24
  store ptr null, ptr %115, align 8, !tbaa !255
  %116 = load ptr, ptr %10, align 8, !tbaa !256
  %117 = getelementptr inbounds i8, ptr %113, i64 32
  store ptr %116, ptr %117, align 8, !tbaa !257
  %118 = icmp eq ptr %116, null
  %119 = getelementptr inbounds %struct.FaTask, ptr %116, i64 0, i32 1
  %120 = select i1 %118, ptr %5, ptr %119
  store ptr %114, ptr %120, align 8, !tbaa !9
  store ptr %114, ptr %10, align 8, !tbaa !256
  %121 = load i64, ptr %9, align 8, !tbaa !258
  %122 = add nsw i64 %121, 1
  store i64 %122, ptr %9, align 8, !tbaa !258
  %123 = getelementptr inbounds i8, ptr %113, i64 40
  store atomic i32 1, ptr %123 release, align 8
  %124 = load i64, ptr %18, align 8, !tbaa !276
  %125 = icmp sgt i64 %124, 0
  br i1 %125, label %106, label %126, !llvm.loop !291

126:                                              ; preds = %112, %106, %102, %99
  %127 = phi i64 [ %103, %102 ], [ 0, %99 ], [ %103, %106 ], [ %103, %112 ]
  call fastcc void @drain_remote(ptr noundef nonnull %5) #35
  %128 = load i64, ptr %9, align 8, !tbaa !258
  %129 = icmp sgt i64 %128, 0
  br i1 %129, label %130, label %133

130:                                              ; preds = %126
  %131 = load i32, ptr %20, align 4, !tbaa !261
  %132 = icmp sgt i32 %131, -1
  br i1 %132, label %169, label %184

133:                                              ; preds = %126
  %134 = load ptr, ptr %6, align 8, !tbaa !264
  %135 = load ptr, ptr %134, align 8, !tbaa !9
  %136 = icmp eq ptr %135, null
  br i1 %136, label %137, label %140

137:                                              ; preds = %133
  %138 = load i64, ptr %8, align 8, !tbaa !253
  %139 = icmp eq i64 %138, 0
  br i1 %139, label %184, label %140

140:                                              ; preds = %137, %133
  %141 = load i64, ptr %18, align 8, !tbaa !276
  %142 = icmp sgt i64 %141, 0
  br i1 %142, label %144, label %143

143:                                              ; preds = %140
  call fastcc void @deadlock_stop(ptr noundef nonnull %5) #39
  unreachable

144:                                              ; preds = %140
  %145 = load ptr, ptr %19, align 8, !tbaa !278
  %146 = load ptr, ptr %145, align 8, !tbaa !9
  %147 = getelementptr inbounds %struct.FaWait, ptr %146, i64 0, i32 6
  %148 = load i64, ptr %147, align 8, !tbaa !280
  %149 = sub nsw i64 %148, %127
  %150 = call i64 @llvm.smax.i64(i64 %149, i64 0)
  %151 = load i32, ptr %20, align 4, !tbaa !261
  %152 = icmp sgt i32 %151, -1
  br i1 %152, label %153, label %181

153:                                              ; preds = %144
  %154 = add nuw nsw i64 %150, 999999
  %155 = udiv i64 %154, 1000000
  %156 = trunc i64 %155 to i32
  %157 = icmp slt i64 %149, 1
  br i1 %157, label %169, label %158

158:                                              ; preds = %153
  store atomic i32 1, ptr %21 seq_cst, align 8
  %159 = load ptr, ptr %22, align 8, !tbaa !259
  %160 = getelementptr inbounds %struct.FaTask, ptr %159, i64 0, i32 1
  %161 = load atomic i64, ptr %160 acquire, align 8
  %162 = icmp eq i64 %161, 0
  br i1 %162, label %163, label %168

163:                                              ; preds = %158
  %164 = load atomic i64, ptr %23 acquire, align 8
  %165 = inttoptr i64 %164 to ptr
  %166 = load ptr, ptr %22, align 8, !tbaa !259
  %167 = icmp eq ptr %166, %165
  br i1 %167, label %169, label %168

168:                                              ; preds = %163, %158
  br label %169

169:                                              ; preds = %130, %168, %163, %153
  %170 = phi i32 [ 0, %168 ], [ %156, %163 ], [ %156, %153 ], [ 0, %130 ]
  call void @llvm.lifetime.start.p0(i64 192, ptr nonnull %3) #31
  %171 = load i32, ptr %20, align 4, !tbaa !261
  %172 = call i32 @epoll_wait(i32 noundef %171, ptr noundef nonnull %3, i32 noundef 16, i32 noundef %170) #34
  store atomic i32 0, ptr %21 seq_cst, align 8
  %173 = icmp sgt i32 %172, 0
  br i1 %173, label %175, label %174

174:                                              ; preds = %175, %169
  call fastcc void @drain_remote(ptr noundef nonnull %5) #35
  call void @llvm.lifetime.end.p0(i64 192, ptr nonnull %3) #31
  br label %184

175:                                              ; preds = %169, %175
  %176 = phi i32 [ %179, %175 ], [ 0, %169 ]
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %4) #31
  %177 = load i32, ptr %24, align 8, !tbaa !262
  %178 = call i64 @read(i32 noundef %177, ptr noundef nonnull %4, i64 noundef 8) #34
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %4) #31
  %179 = add nuw nsw i32 %176, 1
  %180 = icmp eq i32 %179, %172
  br i1 %180, label %174, label %175, !llvm.loop !292

181:                                              ; preds = %144
  %182 = icmp slt i64 %149, 1
  br i1 %182, label %184, label %183

183:                                              ; preds = %181
  call void @fa_nanosleep(i64 noundef %150) #34
  br label %184

184:                                              ; preds = %183, %181, %174, %130, %137
  br label %25, !llvm.loop !293

185:                                              ; preds = %28
  call void @fa_destroy(ptr noundef nonnull %0) #34
  %186 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 5
  %187 = load i64, ptr %186, align 8, !tbaa !277
  %188 = icmp sgt i64 %187, 0
  br i1 %188, label %189, label %192

189:                                              ; preds = %185
  %190 = load ptr, ptr %19, align 8, !tbaa !278
  %191 = shl i64 %187, 3
  call void @fa_free(ptr noundef %190, i64 noundef %191, i64 noundef 8) #34
  br label %192

192:                                              ; preds = %189, %185
  %193 = load i32, ptr %20, align 4, !tbaa !261
  %194 = icmp sgt i32 %193, -1
  br i1 %194, label %195, label %197

195:                                              ; preds = %192
  %196 = call i32 @close(i32 noundef %193) #34
  br label %197

197:                                              ; preds = %195, %192
  %198 = load i32, ptr %24, align 8, !tbaa !262
  %199 = icmp sgt i32 %198, -1
  br i1 %199, label %200, label %202

200:                                              ; preds = %197
  %201 = call i32 @close(i32 noundef %198) #34
  br label %202

202:                                              ; preds = %200, %197
  %203 = call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @current_executor)
  store ptr null, ptr %203, align 8, !tbaa !9
  call void @fa_free(ptr noundef nonnull %5, i64 noundef 424, i64 noundef 16) #34
  ret void
}

; Function Attrs: optsize
declare extern_weak i32 @close(i32 noundef) local_unnamed_addr #1

; Function Attrs: nounwind optsize
define ptr @fa_mutex_new(i64 noundef %0, ptr noundef %1) local_unnamed_addr #0 {
  %3 = add i64 %0, 176
  %4 = tail call fastcc ptr @shared_new(i64 noundef %3, ptr noundef %1) #35
  ret ptr %4
}

; Function Attrs: nounwind optsize
define noundef ptr @fa_mutex_lock(ptr noundef returned %0) local_unnamed_addr #0 {
  %2 = icmp eq ptr %0, null
  br i1 %2, label %5, label %3

3:                                                ; preds = %1
  %4 = atomicrmw add ptr %0, i64 1 monotonic, align 8
  br label %5

5:                                                ; preds = %1, %3
  %6 = getelementptr inbounds %struct.FaMutex, ptr %0, i64 0, i32 1
  %7 = atomicrmw xchg ptr %6, i32 1 acquire, align 16
  %8 = icmp eq i32 %7, 0
  br i1 %8, label %32, label %9

9:                                                ; preds = %5
  br i1 icmp eq (ptr @pthread_create, ptr null), label %10, label %11

10:                                               ; preds = %9
  tail call fastcc void @deadlock(ptr noundef nonnull @.str.31) #39
  unreachable

11:                                               ; preds = %9
  %12 = getelementptr inbounds %struct.FaMutex, ptr %0, i64 0, i32 4
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %13, label %15

13:                                               ; preds = %11
  %14 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %12) #34
  br label %15

15:                                               ; preds = %11, %13
  %16 = getelementptr inbounds %struct.FaMutex, ptr %0, i64 0, i32 2
  %17 = load i32, ptr %16, align 4, !tbaa !20
  %18 = add nsw i32 %17, 1
  store i32 %18, ptr %16, align 4, !tbaa !20
  %19 = atomicrmw xchg ptr %6, i32 1 acquire, align 16
  %20 = icmp eq i32 %19, 0
  br i1 %20, label %27, label %21

21:                                               ; preds = %15
  %22 = getelementptr inbounds %struct.FaMutex, ptr %0, i64 0, i32 5
  br label %23

23:                                               ; preds = %21, %23
  %24 = tail call i32 @pthread_cond_wait(ptr noundef nonnull %22, ptr noundef nonnull %12) #34
  %25 = atomicrmw xchg ptr %6, i32 1 acquire, align 16
  %26 = icmp eq i32 %25, 0
  br i1 %26, label %27, label %23, !llvm.loop !294

27:                                               ; preds = %23, %15
  %28 = load i32, ptr %16, align 4, !tbaa !20
  %29 = add nsw i32 %28, -1
  store i32 %29, ptr %16, align 4, !tbaa !20
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %30, label %32

30:                                               ; preds = %27
  %31 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %12) #34
  br label %32

32:                                               ; preds = %30, %27, %5
  ret ptr %0
}

; Function Attrs: optsize
declare extern_weak i32 @pthread_cond_wait(ptr noundef, ptr noundef) local_unnamed_addr #1

; Function Attrs: nounwind optsize
define noundef i32 @fa_mutex_lock_async(ptr noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef %3) local_unnamed_addr #0 {
  %5 = icmp eq ptr %0, null
  br i1 %5, label %8, label %6

6:                                                ; preds = %4
  %7 = atomicrmw add ptr %0, i64 1 monotonic, align 8
  br label %8

8:                                                ; preds = %4, %6
  %9 = getelementptr inbounds %struct.FaMutex, ptr %0, i64 0, i32 4
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %10, label %12

10:                                               ; preds = %8
  %11 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %9) #34
  br label %12

12:                                               ; preds = %8, %10
  %13 = getelementptr inbounds %struct.FaMutex, ptr %0, i64 0, i32 3
  %14 = load ptr, ptr %13, align 8, !tbaa !9
  %15 = icmp eq ptr %14, null
  br i1 %15, label %16, label %44

16:                                               ; preds = %12
  %17 = getelementptr inbounds %struct.FaMutex, ptr %0, i64 0, i32 1
  %18 = atomicrmw xchg ptr %17, i32 1 acquire, align 16
  %19 = icmp eq i32 %18, 0
  br i1 %19, label %20, label %44

20:                                               ; preds = %16
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %21, label %23

21:                                               ; preds = %20
  %22 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %9) #34
  br label %23

23:                                               ; preds = %20, %21
  %24 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 9
  store i32 1, ptr %24, align 8, !tbaa !295
  %25 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 10
  store i32 0, ptr %25, align 4, !tbaa !296
  %26 = tail call align 4 ptr @llvm.threadlocal.address.p0(ptr align 4 @fa_coop_budget)
  %27 = load i32, ptr %26, align 4, !tbaa !20
  %28 = add nsw i32 %27, -1
  store i32 %28, ptr %26, align 4, !tbaa !20
  %29 = icmp sgt i32 %27, 1
  br i1 %29, label %47, label %30

30:                                               ; preds = %23
  %31 = tail call fastcc ptr @executor() #35
  %32 = getelementptr inbounds i8, ptr %2, i64 16
  %33 = getelementptr inbounds i8, ptr %2, i64 24
  store ptr null, ptr %33, align 8, !tbaa !255
  %34 = getelementptr inbounds %struct.FaExecutor, ptr %31, i64 0, i32 1
  %35 = load ptr, ptr %34, align 8, !tbaa !256
  %36 = getelementptr inbounds i8, ptr %2, i64 32
  store ptr %35, ptr %36, align 8, !tbaa !257
  %37 = icmp eq ptr %35, null
  %38 = getelementptr inbounds %struct.FaTask, ptr %35, i64 0, i32 1
  %39 = select i1 %37, ptr %31, ptr %38
  store ptr %32, ptr %39, align 8, !tbaa !9
  store ptr %32, ptr %34, align 8, !tbaa !256
  %40 = getelementptr inbounds %struct.FaExecutor, ptr %31, i64 0, i32 2
  %41 = load i64, ptr %40, align 8, !tbaa !258
  %42 = add nsw i64 %41, 1
  store i64 %42, ptr %40, align 8, !tbaa !258
  %43 = getelementptr inbounds i8, ptr %2, i64 40
  store atomic i32 1, ptr %43 release, align 8
  br label %47

44:                                               ; preds = %16, %12
  tail call fastcc void @line_push(ptr noundef nonnull %13, ptr noundef %1, ptr noundef %2, ptr noundef %3) #35
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %45, label %47

45:                                               ; preds = %44
  %46 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %9) #34
  br label %47

47:                                               ; preds = %45, %44, %23, %30
  %48 = phi i32 [ 2, %30 ], [ 0, %23 ], [ 1, %44 ], [ 1, %45 ]
  ret i32 %48
}

; Function Attrs: nounwind optsize
define internal fastcc void @line_push(ptr nocapture noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef %3) unnamed_addr #0 {
  %5 = tail call fastcc ptr @executor() #35
  store ptr %2, ptr %1, align 8, !tbaa !272
  %6 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 1
  store ptr %5, ptr %6, align 8, !tbaa !274
  %7 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 5
  store ptr %3, ptr %7, align 8, !tbaa !297
  %8 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 9
  store i32 0, ptr %8, align 8, !tbaa !295
  %9 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 10
  store i32 0, ptr %9, align 4, !tbaa !296
  %10 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 8
  store i64 -1, ptr %10, align 8, !tbaa !275
  %11 = load ptr, ptr %0, align 8, !tbaa !9
  %12 = icmp eq ptr %11, null
  %13 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 2
  br i1 %12, label %14, label %16

14:                                               ; preds = %4
  store ptr %1, ptr %13, align 8, !tbaa !298
  %15 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 3
  br label %21

16:                                               ; preds = %4
  store ptr %11, ptr %13, align 8, !tbaa !298
  %17 = getelementptr inbounds %struct.FaWait, ptr %11, i64 0, i32 3
  %18 = load ptr, ptr %17, align 8, !tbaa !299
  %19 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 3
  store ptr %18, ptr %19, align 8, !tbaa !299
  %20 = getelementptr inbounds %struct.FaWait, ptr %18, i64 0, i32 2
  br label %21

21:                                               ; preds = %16, %14
  %22 = phi ptr [ %20, %16 ], [ %15, %14 ]
  %23 = phi ptr [ %17, %16 ], [ %0, %14 ]
  store ptr %1, ptr %22, align 8, !tbaa !9
  store ptr %1, ptr %23, align 8, !tbaa !9
  %24 = getelementptr inbounds i8, ptr %2, i64 40
  store atomic i32 2, ptr %24 release, align 8
  %25 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 18
  store ptr %3, ptr %25, align 8, !tbaa !300
  %26 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 7
  %27 = load ptr, ptr %26, align 8, !tbaa !264
  %28 = icmp eq ptr %27, %2
  br i1 %28, label %29, label %31

29:                                               ; preds = %21
  %30 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 17
  store ptr %3, ptr %30, align 8, !tbaa !265
  br label %31

31:                                               ; preds = %29, %21
  ret void
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(argmem: readwrite)
define noundef ptr @fa_mutex_try_lock(ptr noundef %0) local_unnamed_addr #27 {
  %2 = getelementptr inbounds %struct.FaMutex, ptr %0, i64 0, i32 1
  %3 = atomicrmw xchg ptr %2, i32 1 acquire, align 16
  %4 = icmp ne i32 %3, 0
  %5 = icmp eq ptr %0, null
  %6 = or i1 %5, %4
  br i1 %6, label %9, label %7

7:                                                ; preds = %1
  %8 = atomicrmw add ptr %0, i64 1 monotonic, align 8
  br label %9

9:                                                ; preds = %7, %1
  %10 = phi ptr [ null, %1 ], [ %0, %7 ]
  ret ptr %10
}

; Function Attrs: nounwind optsize
define void @fa_mutex_unlock(ptr noundef %0) local_unnamed_addr #0 {
  %2 = icmp eq ptr %0, null
  br i1 %2, label %40, label %3

3:                                                ; preds = %1
  %4 = getelementptr inbounds %struct.FaMutex, ptr %0, i64 0, i32 4
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %5, label %7

5:                                                ; preds = %3
  %6 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %4) #34
  br label %7

7:                                                ; preds = %3, %5
  %8 = getelementptr inbounds %struct.FaMutex, ptr %0, i64 0, i32 3
  %9 = load ptr, ptr %8, align 8, !tbaa !9
  %10 = icmp eq ptr %9, null
  br i1 %10, label %28, label %11

11:                                               ; preds = %7
  %12 = getelementptr inbounds %struct.FaWait, ptr %9, i64 0, i32 2
  %13 = load ptr, ptr %12, align 8, !tbaa !298
  %14 = icmp eq ptr %13, %9
  br i1 %14, label %23, label %15

15:                                               ; preds = %11
  %16 = getelementptr inbounds %struct.FaWait, ptr %9, i64 0, i32 3
  %17 = load ptr, ptr %16, align 8, !tbaa !299
  %18 = getelementptr inbounds %struct.FaWait, ptr %17, i64 0, i32 2
  store ptr %13, ptr %18, align 8, !tbaa !298
  %19 = load ptr, ptr %12, align 8, !tbaa !298
  %20 = getelementptr inbounds %struct.FaWait, ptr %19, i64 0, i32 3
  store ptr %17, ptr %20, align 8, !tbaa !299
  %21 = load ptr, ptr %8, align 8, !tbaa !9
  %22 = icmp eq ptr %21, %9
  br i1 %22, label %23, label %25

23:                                               ; preds = %15, %11
  %24 = phi ptr [ null, %11 ], [ %19, %15 ]
  store ptr %24, ptr %8, align 8, !tbaa !9
  br label %25

25:                                               ; preds = %23, %15
  store ptr null, ptr %12, align 8, !tbaa !298
  %26 = getelementptr inbounds %struct.FaWait, ptr %9, i64 0, i32 3
  store ptr null, ptr %26, align 8, !tbaa !299
  %27 = getelementptr inbounds %struct.FaWait, ptr %9, i64 0, i32 9
  store i32 1, ptr %27, align 8, !tbaa !295
  tail call fastcc void @wake(ptr noundef nonnull %9) #35
  br label %36

28:                                               ; preds = %7
  %29 = getelementptr inbounds %struct.FaMutex, ptr %0, i64 0, i32 1
  store atomic i32 0, ptr %29 release, align 16
  %30 = getelementptr inbounds %struct.FaMutex, ptr %0, i64 0, i32 2
  %31 = load i32, ptr %30, align 4, !tbaa !20
  %32 = icmp sgt i32 %31, 0
  br i1 %32, label %33, label %36

33:                                               ; preds = %28
  %34 = getelementptr inbounds %struct.FaMutex, ptr %0, i64 0, i32 5
  %35 = tail call i32 @pthread_cond_signal(ptr noundef nonnull %34) #34
  br label %36

36:                                               ; preds = %28, %33, %25
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %37, label %39

37:                                               ; preds = %36
  %38 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %4) #34
  br label %39

39:                                               ; preds = %36, %37
  tail call void @fa_shared_release(ptr noundef nonnull %0) #35
  br label %40

40:                                               ; preds = %1, %39
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @wake(ptr nocapture noundef %0) unnamed_addr #0 {
  %2 = alloca i64, align 8
  %3 = load ptr, ptr %0, align 8, !tbaa !272
  %4 = getelementptr inbounds i8, ptr %3, i64 16
  %5 = getelementptr inbounds %struct.FaWait, ptr %0, i64 0, i32 1
  %6 = load ptr, ptr %5, align 8, !tbaa !274
  %7 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @current_executor)
  %8 = load ptr, ptr %7, align 8, !tbaa !9
  %9 = icmp eq ptr %6, %8
  br i1 %9, label %10, label %22

10:                                               ; preds = %1
  %11 = getelementptr inbounds i8, ptr %3, i64 24
  store ptr null, ptr %11, align 8, !tbaa !255
  %12 = getelementptr inbounds %struct.FaExecutor, ptr %6, i64 0, i32 1
  %13 = load ptr, ptr %12, align 8, !tbaa !256
  %14 = getelementptr inbounds i8, ptr %3, i64 32
  store ptr %13, ptr %14, align 8, !tbaa !257
  %15 = icmp eq ptr %13, null
  %16 = getelementptr inbounds %struct.FaTask, ptr %13, i64 0, i32 1
  %17 = select i1 %15, ptr %6, ptr %16
  store ptr %4, ptr %17, align 8, !tbaa !9
  store ptr %4, ptr %12, align 8, !tbaa !256
  %18 = getelementptr inbounds %struct.FaExecutor, ptr %6, i64 0, i32 2
  %19 = load i64, ptr %18, align 8, !tbaa !258
  %20 = add nsw i64 %19, 1
  store i64 %20, ptr %18, align 8, !tbaa !258
  %21 = getelementptr inbounds i8, ptr %3, i64 40
  store atomic i32 1, ptr %21 release, align 8
  br label %40

22:                                               ; preds = %1
  %23 = getelementptr inbounds %struct.FaWait, ptr %0, i64 0, i32 10
  store i32 1, ptr %23, align 4, !tbaa !296
  %24 = getelementptr inbounds i8, ptr %3, i64 40
  store atomic i32 5, ptr %24 release, align 8
  %25 = getelementptr inbounds i8, ptr %3, i64 24
  store atomic i64 0, ptr %25 monotonic, align 8
  %26 = getelementptr inbounds %struct.FaExecutor, ptr %6, i64 0, i32 10
  %27 = ptrtoint ptr %4 to i64
  %28 = atomicrmw xchg ptr %26, i64 %27 acq_rel, align 8
  %29 = inttoptr i64 %28 to ptr
  %30 = getelementptr inbounds %struct.FaTask, ptr %29, i64 0, i32 1
  store atomic i64 %27, ptr %30 release, align 8
  %31 = getelementptr inbounds %struct.FaExecutor, ptr %6, i64 0, i32 12
  %32 = atomicrmw xchg ptr %31, i32 0 seq_cst, align 8
  %33 = icmp eq i32 %32, 0
  br i1 %33, label %40, label %34

34:                                               ; preds = %22
  %35 = getelementptr inbounds %struct.FaExecutor, ptr %6, i64 0, i32 14
  %36 = load i32, ptr %35, align 8, !tbaa !262
  %37 = icmp sgt i32 %36, -1
  br i1 %37, label %38, label %40

38:                                               ; preds = %34
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %2) #31
  store i64 1, ptr %2, align 8, !tbaa !5
  %39 = call i64 @write(i32 noundef %36, ptr noundef nonnull %2, i64 noundef 8) #34
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %2) #31
  br label %40

40:                                               ; preds = %38, %34, %22, %10
  ret void
}

; Function Attrs: optsize
declare extern_weak i32 @pthread_cond_signal(ptr noundef) local_unnamed_addr #1

; Function Attrs: nounwind optsize
define void @fa_mutex_cancel(ptr noundef %0, ptr noundef %1) local_unnamed_addr #0 {
  %3 = getelementptr inbounds %struct.FaMutex, ptr %0, i64 0, i32 4
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %4, label %6

4:                                                ; preds = %2
  %5 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %3) #34
  br label %6

6:                                                ; preds = %2, %4
  %7 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 9
  %8 = load i32, ptr %7, align 8, !tbaa !295
  %9 = icmp eq i32 %8, 0
  br i1 %9, label %10, label %30

10:                                               ; preds = %6
  %11 = getelementptr inbounds %struct.FaMutex, ptr %0, i64 0, i32 3
  %12 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 2
  %13 = load ptr, ptr %12, align 8, !tbaa !298
  %14 = icmp eq ptr %13, %1
  br i1 %14, label %23, label %15

15:                                               ; preds = %10
  %16 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 3
  %17 = load ptr, ptr %16, align 8, !tbaa !299
  %18 = getelementptr inbounds %struct.FaWait, ptr %17, i64 0, i32 2
  store ptr %13, ptr %18, align 8, !tbaa !298
  %19 = load ptr, ptr %12, align 8, !tbaa !298
  %20 = getelementptr inbounds %struct.FaWait, ptr %19, i64 0, i32 3
  store ptr %17, ptr %20, align 8, !tbaa !299
  %21 = load ptr, ptr %11, align 8, !tbaa !9
  %22 = icmp eq ptr %21, %1
  br i1 %22, label %23, label %25

23:                                               ; preds = %15, %10
  %24 = phi ptr [ null, %10 ], [ %19, %15 ]
  store ptr %24, ptr %11, align 8, !tbaa !9
  br label %25

25:                                               ; preds = %15, %23
  store ptr null, ptr %12, align 8, !tbaa !298
  %26 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 3
  store ptr null, ptr %26, align 8, !tbaa !299
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %27, label %29

27:                                               ; preds = %25
  %28 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %3) #34
  br label %29

29:                                               ; preds = %25, %27
  tail call void @fa_shared_release(ptr noundef nonnull %0) #35
  br label %34

30:                                               ; preds = %6
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %31, label %33

31:                                               ; preds = %30
  %32 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %3) #34
  br label %33

33:                                               ; preds = %30, %31
  tail call fastcc void @forget_remote_wake(ptr noundef nonnull %1) #35
  tail call void @fa_mutex_unlock(ptr noundef %0) #35
  br label %34

34:                                               ; preds = %33, %29
  ret void
}

; Function Attrs: nofree norecurse nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc void @forget_remote_wake(ptr nocapture noundef readonly %0) unnamed_addr #29 {
  %2 = getelementptr inbounds %struct.FaWait, ptr %0, i64 0, i32 10
  %3 = load i32, ptr %2, align 4, !tbaa !296
  %4 = icmp eq i32 %3, 0
  br i1 %4, label %27, label %5

5:                                                ; preds = %1
  %6 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @current_executor)
  %7 = load ptr, ptr %6, align 8, !tbaa !9
  tail call fastcc void @drain_remote(ptr noundef %7) #35
  %8 = load ptr, ptr %0, align 8, !tbaa !272
  %9 = getelementptr inbounds i8, ptr %8, i64 40
  %10 = load atomic i32, ptr %9 acquire, align 8
  %11 = icmp eq i32 %10, 1
  br i1 %11, label %12, label %27

12:                                               ; preds = %5
  %13 = getelementptr inbounds i8, ptr %8, i64 32
  %14 = load ptr, ptr %13, align 8, !tbaa !257
  %15 = icmp eq ptr %14, null
  %16 = getelementptr inbounds i8, ptr %8, i64 24
  %17 = load ptr, ptr %16, align 8, !tbaa !255
  %18 = getelementptr inbounds %struct.FaTask, ptr %14, i64 0, i32 1
  %19 = select i1 %15, ptr %7, ptr %18
  store ptr %17, ptr %19, align 8, !tbaa !9
  %20 = icmp eq ptr %17, null
  %21 = getelementptr inbounds %struct.FaExecutor, ptr %7, i64 0, i32 1
  %22 = getelementptr inbounds %struct.FaTask, ptr %17, i64 0, i32 2
  %23 = select i1 %20, ptr %21, ptr %22
  store ptr %14, ptr %23, align 8, !tbaa !9
  store ptr null, ptr %16, align 8, !tbaa !255
  store ptr null, ptr %13, align 8, !tbaa !257
  %24 = getelementptr inbounds %struct.FaExecutor, ptr %7, i64 0, i32 2
  %25 = load i64, ptr %24, align 8, !tbaa !258
  %26 = add nsw i64 %25, -1
  store i64 %26, ptr %24, align 8, !tbaa !258
  br label %27

27:                                               ; preds = %5, %12, %1
  ret void
}

; Function Attrs: nounwind optsize
define ptr @fa_channel_new(i64 noundef %0, i64 noundef %1, ptr noundef %2) local_unnamed_addr #0 {
  %4 = tail call fastcc ptr @shared_new(i64 noundef 240, ptr noundef nonnull @channel_drop) #35
  %5 = tail call i64 @llvm.umin.i64(i64 %0, i64 9223372036854775807)
  %6 = getelementptr inbounds %struct.FaChannel, ptr %4, i64 0, i32 3
  store i64 %5, ptr %6, align 16, !tbaa !301
  %7 = tail call i64 @llvm.smax.i64(i64 %1, i64 1)
  %8 = getelementptr inbounds %struct.FaChannel, ptr %4, i64 0, i32 4
  store i64 %7, ptr %8, align 8, !tbaa !304
  %9 = getelementptr inbounds %struct.FaChannel, ptr %4, i64 0, i32 11
  store ptr %2, ptr %9, align 8, !tbaa !305
  ret ptr %4
}

; Function Attrs: nounwind optsize
define internal void @channel_drop(ptr nocapture noundef readonly %0) #0 {
  %2 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 7
  %3 = load i64, ptr %2, align 16, !tbaa !306
  %4 = icmp sgt i64 %3, 0
  br i1 %4, label %5, label %11

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 11
  %7 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 6
  %8 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 8
  %9 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 5
  %10 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 4
  br label %15

11:                                               ; preds = %30, %1
  %12 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 8
  %13 = load i64, ptr %12, align 8, !tbaa !307
  %14 = icmp sgt i64 %13, 0
  br i1 %14, label %34, label %40

15:                                               ; preds = %5, %30
  %16 = phi i64 [ %3, %5 ], [ %31, %30 ]
  %17 = phi i64 [ 0, %5 ], [ %32, %30 ]
  %18 = load ptr, ptr %6, align 8, !tbaa !305
  %19 = icmp eq ptr %18, null
  br i1 %19, label %30, label %20

20:                                               ; preds = %15
  %21 = load i64, ptr %7, align 8, !tbaa !308
  %22 = add nsw i64 %21, %17
  %23 = load i64, ptr %8, align 8, !tbaa !307
  %24 = srem i64 %22, %23
  %25 = load ptr, ptr %9, align 16, !tbaa !309
  %26 = load i64, ptr %10, align 8, !tbaa !304
  %27 = mul nsw i64 %26, %24
  %28 = getelementptr inbounds i8, ptr %25, i64 %27
  tail call void %18(ptr noundef %28) #34
  %29 = load i64, ptr %2, align 16, !tbaa !306
  br label %30

30:                                               ; preds = %20, %15
  %31 = phi i64 [ %29, %20 ], [ %16, %15 ]
  %32 = add nuw nsw i64 %17, 1
  %33 = icmp slt i64 %32, %31
  br i1 %33, label %15, label %11, !llvm.loop !310

34:                                               ; preds = %11
  %35 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 5
  %36 = load ptr, ptr %35, align 16, !tbaa !309
  %37 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 4
  %38 = load i64, ptr %37, align 8, !tbaa !304
  %39 = mul nsw i64 %38, %13
  tail call void @fa_free(ptr noundef %36, i64 noundef %39, i64 noundef 16) #34
  br label %40

40:                                               ; preds = %34, %11
  ret void
}

; Function Attrs: nounwind optsize
define noundef i32 @fa_channel_send(ptr noundef %0, ptr nocapture noundef readonly %1, i32 noundef %2) local_unnamed_addr #0 {
  %4 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 1
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %5, label %7

5:                                                ; preds = %3
  %6 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %4) #34
  br label %7

7:                                                ; preds = %3, %5
  %8 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 9
  %9 = load i32, ptr %8, align 16, !tbaa !311
  %10 = icmp eq i32 %9, 0
  br i1 %10, label %11, label %15

11:                                               ; preds = %7
  %12 = icmp eq i32 %2, 0
  %13 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 10
  %14 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 2
  br label %16

15:                                               ; preds = %24, %7
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %32, label %35

16:                                               ; preds = %11, %24
  %17 = tail call fastcc i32 @channel_send_now(ptr noundef nonnull %0, ptr noundef %1) #35, !range !46
  %18 = icmp eq i32 %17, 0
  br i1 %18, label %20, label %19

19:                                               ; preds = %16
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %32, label %35

20:                                               ; preds = %16
  br i1 %12, label %21, label %22

21:                                               ; preds = %20
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %32, label %35

22:                                               ; preds = %20
  br i1 icmp eq (ptr @pthread_create, ptr null), label %23, label %24

23:                                               ; preds = %22
  tail call fastcc void @deadlock(ptr noundef nonnull @.str.32) #39
  unreachable

24:                                               ; preds = %22
  %25 = load i32, ptr %13, align 4, !tbaa !312
  %26 = add nsw i32 %25, 1
  store i32 %26, ptr %13, align 4, !tbaa !312
  %27 = tail call i32 @pthread_cond_wait(ptr noundef nonnull %14, ptr noundef nonnull %4) #34
  %28 = load i32, ptr %13, align 4, !tbaa !312
  %29 = add nsw i32 %28, -1
  store i32 %29, ptr %13, align 4, !tbaa !312
  %30 = load i32, ptr %8, align 16, !tbaa !311
  %31 = icmp eq i32 %30, 0
  br i1 %31, label %16, label %15

32:                                               ; preds = %21, %19, %15
  %33 = phi i32 [ 0, %15 ], [ 1, %19 ], [ 0, %21 ]
  %34 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %4) #34
  br label %35

35:                                               ; preds = %32, %21, %19, %15
  %36 = phi i32 [ 0, %15 ], [ 1, %19 ], [ 0, %21 ], [ %33, %32 ]
  ret i32 %36
}

; Function Attrs: nounwind optsize
define internal fastcc noundef i32 @channel_send_now(ptr noundef %0, ptr nocapture noundef readonly %1) unnamed_addr #0 {
  %3 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 13
  %4 = load ptr, ptr %3, align 8, !tbaa !9
  %5 = icmp eq ptr %4, null
  br i1 %5, label %27, label %6

6:                                                ; preds = %2
  %7 = getelementptr inbounds %struct.FaWait, ptr %4, i64 0, i32 2
  %8 = load ptr, ptr %7, align 8, !tbaa !298
  %9 = icmp eq ptr %8, %4
  br i1 %9, label %18, label %10

10:                                               ; preds = %6
  %11 = getelementptr inbounds %struct.FaWait, ptr %4, i64 0, i32 3
  %12 = load ptr, ptr %11, align 8, !tbaa !299
  %13 = getelementptr inbounds %struct.FaWait, ptr %12, i64 0, i32 2
  store ptr %8, ptr %13, align 8, !tbaa !298
  %14 = load ptr, ptr %7, align 8, !tbaa !298
  %15 = getelementptr inbounds %struct.FaWait, ptr %14, i64 0, i32 3
  store ptr %12, ptr %15, align 8, !tbaa !299
  %16 = load ptr, ptr %3, align 8, !tbaa !9
  %17 = icmp eq ptr %16, %4
  br i1 %17, label %18, label %20

18:                                               ; preds = %10, %6
  %19 = phi ptr [ null, %6 ], [ %14, %10 ]
  store ptr %19, ptr %3, align 8, !tbaa !9
  br label %20

20:                                               ; preds = %18, %10
  store ptr null, ptr %7, align 8, !tbaa !298
  %21 = getelementptr inbounds %struct.FaWait, ptr %4, i64 0, i32 3
  store ptr null, ptr %21, align 8, !tbaa !299
  %22 = getelementptr inbounds %struct.FaWait, ptr %4, i64 0, i32 4
  %23 = load ptr, ptr %22, align 8, !tbaa !313
  %24 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 4
  %25 = load i64, ptr %24, align 8, !tbaa !304
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %23, ptr align 1 %1, i64 %25, i1 false)
  %26 = getelementptr inbounds %struct.FaWait, ptr %4, i64 0, i32 9
  store i32 1, ptr %26, align 8, !tbaa !295
  tail call fastcc void @wake(ptr noundef nonnull %4) #35
  br label %40

27:                                               ; preds = %2
  %28 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 7
  %29 = load i64, ptr %28, align 16, !tbaa !306
  %30 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 3
  %31 = load i64, ptr %30, align 16, !tbaa !301
  %32 = icmp slt i64 %29, %31
  br i1 %32, label %33, label %40

33:                                               ; preds = %27
  tail call fastcc void @channel_push(ptr noundef nonnull %0, ptr noundef %1) #35
  %34 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 10
  %35 = load i32, ptr %34, align 4, !tbaa !312
  %36 = icmp sgt i32 %35, 0
  br i1 %36, label %37, label %40

37:                                               ; preds = %33
  %38 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 2
  %39 = tail call i32 @pthread_cond_broadcast(ptr noundef nonnull %38) #34
  br label %40

40:                                               ; preds = %37, %33, %27, %20
  %41 = phi i32 [ 1, %20 ], [ 0, %27 ], [ 1, %33 ], [ 1, %37 ]
  ret i32 %41
}

; Function Attrs: nounwind optsize
define noundef i32 @fa_channel_send_async(ptr noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef %3, ptr noundef %4) local_unnamed_addr #0 {
  %6 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 1
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %7, label %9

7:                                                ; preds = %5
  %8 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %6) #34
  br label %9

9:                                                ; preds = %5, %7
  %10 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 9
  %11 = load i32, ptr %10, align 16, !tbaa !311
  %12 = icmp eq i32 %11, 0
  br i1 %12, label %16, label %13

13:                                               ; preds = %9
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %14, label %48

14:                                               ; preds = %13
  %15 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %6) #34
  br label %48

16:                                               ; preds = %9
  %17 = tail call fastcc i32 @channel_send_now(ptr noundef nonnull %0, ptr noundef %1) #35, !range !46
  %18 = icmp eq i32 %17, 0
  br i1 %18, label %43, label %19

19:                                               ; preds = %16
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %20, label %22

20:                                               ; preds = %19
  %21 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %6) #34
  br label %22

22:                                               ; preds = %19, %20
  %23 = getelementptr inbounds %struct.FaWait, ptr %2, i64 0, i32 9
  store i32 1, ptr %23, align 8, !tbaa !295
  %24 = getelementptr inbounds %struct.FaWait, ptr %2, i64 0, i32 10
  store i32 0, ptr %24, align 4, !tbaa !296
  %25 = tail call align 4 ptr @llvm.threadlocal.address.p0(ptr align 4 @fa_coop_budget)
  %26 = load i32, ptr %25, align 4, !tbaa !20
  %27 = add nsw i32 %26, -1
  store i32 %27, ptr %25, align 4, !tbaa !20
  %28 = icmp sgt i32 %26, 1
  br i1 %28, label %48, label %29

29:                                               ; preds = %22
  %30 = tail call fastcc ptr @executor() #35
  %31 = getelementptr inbounds i8, ptr %3, i64 16
  %32 = getelementptr inbounds i8, ptr %3, i64 24
  store ptr null, ptr %32, align 8, !tbaa !255
  %33 = getelementptr inbounds %struct.FaExecutor, ptr %30, i64 0, i32 1
  %34 = load ptr, ptr %33, align 8, !tbaa !256
  %35 = getelementptr inbounds i8, ptr %3, i64 32
  store ptr %34, ptr %35, align 8, !tbaa !257
  %36 = icmp eq ptr %34, null
  %37 = getelementptr inbounds %struct.FaTask, ptr %34, i64 0, i32 1
  %38 = select i1 %36, ptr %30, ptr %37
  store ptr %31, ptr %38, align 8, !tbaa !9
  store ptr %31, ptr %33, align 8, !tbaa !256
  %39 = getelementptr inbounds %struct.FaExecutor, ptr %30, i64 0, i32 2
  %40 = load i64, ptr %39, align 8, !tbaa !258
  %41 = add nsw i64 %40, 1
  store i64 %41, ptr %39, align 8, !tbaa !258
  %42 = getelementptr inbounds i8, ptr %3, i64 40
  store atomic i32 1, ptr %42 release, align 8
  br label %48

43:                                               ; preds = %16
  %44 = getelementptr inbounds %struct.FaWait, ptr %2, i64 0, i32 4
  store ptr %1, ptr %44, align 8, !tbaa !313
  %45 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 12
  tail call fastcc void @line_push(ptr noundef nonnull %45, ptr noundef %2, ptr noundef %3, ptr noundef %4) #35
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %46, label %48

46:                                               ; preds = %43
  %47 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %6) #34
  br label %48

48:                                               ; preds = %46, %43, %14, %13, %22, %29
  %49 = phi i32 [ 2, %29 ], [ 0, %22 ], [ 3, %13 ], [ 3, %14 ], [ 1, %43 ], [ 1, %46 ]
  ret i32 %49
}

; Function Attrs: nounwind optsize
define noundef i32 @fa_channel_receive(ptr noundef %0, ptr nocapture noundef writeonly %1, i32 noundef %2) local_unnamed_addr #0 {
  %4 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 1
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %5, label %7

5:                                                ; preds = %3
  %6 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %4) #34
  br label %7

7:                                                ; preds = %3, %5
  %8 = tail call fastcc i32 @channel_receive_now(ptr noundef %0, ptr noundef %1) #35, !range !46
  %9 = icmp eq i32 %8, 0
  br i1 %9, label %10, label %15

10:                                               ; preds = %7
  %11 = icmp eq i32 %2, 0
  %12 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 9
  %13 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 10
  %14 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 2
  br i1 %11, label %19, label %16

15:                                               ; preds = %22, %7
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %30, label %33

16:                                               ; preds = %10, %22
  %17 = load i32, ptr %12, align 16, !tbaa !311
  %18 = icmp eq i32 %17, 0
  br i1 %18, label %20, label %19

19:                                               ; preds = %16, %10
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %30, label %33

20:                                               ; preds = %16
  br i1 icmp eq (ptr @pthread_create, ptr null), label %21, label %22

21:                                               ; preds = %20
  tail call fastcc void @deadlock(ptr noundef nonnull @.str.33) #39
  unreachable

22:                                               ; preds = %20
  %23 = load i32, ptr %13, align 4, !tbaa !312
  %24 = add nsw i32 %23, 1
  store i32 %24, ptr %13, align 4, !tbaa !312
  %25 = tail call i32 @pthread_cond_wait(ptr noundef nonnull %14, ptr noundef nonnull %4) #34
  %26 = load i32, ptr %13, align 4, !tbaa !312
  %27 = add nsw i32 %26, -1
  store i32 %27, ptr %13, align 4, !tbaa !312
  %28 = tail call fastcc i32 @channel_receive_now(ptr noundef nonnull %0, ptr noundef %1) #35, !range !46
  %29 = icmp eq i32 %28, 0
  br i1 %29, label %16, label %15

30:                                               ; preds = %19, %15
  %31 = phi i32 [ 1, %15 ], [ 0, %19 ]
  %32 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %4) #34
  br label %33

33:                                               ; preds = %30, %19, %15
  %34 = phi i32 [ 1, %15 ], [ 0, %19 ], [ %31, %30 ]
  ret i32 %34
}

; Function Attrs: nounwind optsize
define internal fastcc noundef i32 @channel_receive_now(ptr noundef %0, ptr nocapture noundef writeonly %1) unnamed_addr #0 {
  %3 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 7
  %4 = load i64, ptr %3, align 16, !tbaa !306
  %5 = icmp sgt i64 %4, 0
  br i1 %5, label %6, label %48

6:                                                ; preds = %2
  %7 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 5
  %8 = load ptr, ptr %7, align 16, !tbaa !309
  %9 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 6
  %10 = load i64, ptr %9, align 8, !tbaa !308
  %11 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 4
  %12 = load i64, ptr %11, align 8, !tbaa !304
  %13 = mul nsw i64 %12, %10
  %14 = getelementptr inbounds i8, ptr %8, i64 %13
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %1, ptr align 1 %14, i64 %12, i1 false)
  %15 = load i64, ptr %9, align 8, !tbaa !308
  %16 = add nsw i64 %15, 1
  %17 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 8
  %18 = load i64, ptr %17, align 8, !tbaa !307
  %19 = srem i64 %16, %18
  store i64 %19, ptr %9, align 8, !tbaa !308
  %20 = load i64, ptr %3, align 16, !tbaa !306
  %21 = add nsw i64 %20, -1
  store i64 %21, ptr %3, align 16, !tbaa !306
  %22 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 12
  %23 = load ptr, ptr %22, align 8, !tbaa !9
  %24 = icmp eq ptr %23, null
  br i1 %24, label %44, label %25

25:                                               ; preds = %6
  %26 = getelementptr inbounds %struct.FaWait, ptr %23, i64 0, i32 2
  %27 = load ptr, ptr %26, align 8, !tbaa !298
  %28 = icmp eq ptr %27, %23
  br i1 %28, label %37, label %29

29:                                               ; preds = %25
  %30 = getelementptr inbounds %struct.FaWait, ptr %23, i64 0, i32 3
  %31 = load ptr, ptr %30, align 8, !tbaa !299
  %32 = getelementptr inbounds %struct.FaWait, ptr %31, i64 0, i32 2
  store ptr %27, ptr %32, align 8, !tbaa !298
  %33 = load ptr, ptr %26, align 8, !tbaa !298
  %34 = getelementptr inbounds %struct.FaWait, ptr %33, i64 0, i32 3
  store ptr %31, ptr %34, align 8, !tbaa !299
  %35 = load ptr, ptr %22, align 8, !tbaa !9
  %36 = icmp eq ptr %35, %23
  br i1 %36, label %37, label %39

37:                                               ; preds = %29, %25
  %38 = phi ptr [ null, %25 ], [ %33, %29 ]
  store ptr %38, ptr %22, align 8, !tbaa !9
  br label %39

39:                                               ; preds = %37, %29
  store ptr null, ptr %26, align 8, !tbaa !298
  %40 = getelementptr inbounds %struct.FaWait, ptr %23, i64 0, i32 3
  store ptr null, ptr %40, align 8, !tbaa !299
  %41 = getelementptr inbounds %struct.FaWait, ptr %23, i64 0, i32 4
  %42 = load ptr, ptr %41, align 8, !tbaa !313
  tail call fastcc void @channel_push(ptr noundef nonnull %0, ptr noundef %42) #35
  %43 = getelementptr inbounds %struct.FaWait, ptr %23, i64 0, i32 9
  store i32 1, ptr %43, align 8, !tbaa !295
  tail call fastcc void @wake(ptr noundef nonnull %23) #35
  br label %44

44:                                               ; preds = %6, %39
  %45 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 10
  %46 = load i32, ptr %45, align 4, !tbaa !312
  %47 = icmp sgt i32 %46, 0
  br i1 %47, label %76, label %79

48:                                               ; preds = %2
  %49 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 12
  %50 = load ptr, ptr %49, align 8, !tbaa !9
  %51 = icmp eq ptr %50, null
  br i1 %51, label %79, label %52

52:                                               ; preds = %48
  %53 = getelementptr inbounds %struct.FaWait, ptr %50, i64 0, i32 2
  %54 = load ptr, ptr %53, align 8, !tbaa !298
  %55 = icmp eq ptr %54, %50
  br i1 %55, label %64, label %56

56:                                               ; preds = %52
  %57 = getelementptr inbounds %struct.FaWait, ptr %50, i64 0, i32 3
  %58 = load ptr, ptr %57, align 8, !tbaa !299
  %59 = getelementptr inbounds %struct.FaWait, ptr %58, i64 0, i32 2
  store ptr %54, ptr %59, align 8, !tbaa !298
  %60 = load ptr, ptr %53, align 8, !tbaa !298
  %61 = getelementptr inbounds %struct.FaWait, ptr %60, i64 0, i32 3
  store ptr %58, ptr %61, align 8, !tbaa !299
  %62 = load ptr, ptr %49, align 8, !tbaa !9
  %63 = icmp eq ptr %62, %50
  br i1 %63, label %64, label %66

64:                                               ; preds = %56, %52
  %65 = phi ptr [ null, %52 ], [ %60, %56 ]
  store ptr %65, ptr %49, align 8, !tbaa !9
  br label %66

66:                                               ; preds = %64, %56
  store ptr null, ptr %53, align 8, !tbaa !298
  %67 = getelementptr inbounds %struct.FaWait, ptr %50, i64 0, i32 3
  store ptr null, ptr %67, align 8, !tbaa !299
  %68 = getelementptr inbounds %struct.FaWait, ptr %50, i64 0, i32 4
  %69 = load ptr, ptr %68, align 8, !tbaa !313
  %70 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 4
  %71 = load i64, ptr %70, align 8, !tbaa !304
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %1, ptr align 1 %69, i64 %71, i1 false)
  %72 = getelementptr inbounds %struct.FaWait, ptr %50, i64 0, i32 9
  store i32 1, ptr %72, align 8, !tbaa !295
  tail call fastcc void @wake(ptr noundef nonnull %50) #35
  %73 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 10
  %74 = load i32, ptr %73, align 4, !tbaa !312
  %75 = icmp sgt i32 %74, 0
  br i1 %75, label %76, label %79

76:                                               ; preds = %66, %44
  %77 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 2
  %78 = tail call i32 @pthread_cond_broadcast(ptr noundef nonnull %77) #34
  br label %79

79:                                               ; preds = %76, %48, %66, %44
  %80 = phi i32 [ 1, %44 ], [ 1, %66 ], [ 0, %48 ], [ 1, %76 ]
  ret i32 %80
}

; Function Attrs: nounwind optsize
define noundef i32 @fa_channel_receive_async(ptr noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef %3, ptr noundef %4) local_unnamed_addr #0 {
  %6 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 1
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %7, label %9

7:                                                ; preds = %5
  %8 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %6) #34
  br label %9

9:                                                ; preds = %5, %7
  %10 = tail call fastcc i32 @channel_receive_now(ptr noundef %0, ptr noundef %1) #35, !range !46
  %11 = icmp eq i32 %10, 0
  br i1 %11, label %36, label %12

12:                                               ; preds = %9
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %13, label %15

13:                                               ; preds = %12
  %14 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %6) #34
  br label %15

15:                                               ; preds = %12, %13
  %16 = getelementptr inbounds %struct.FaWait, ptr %2, i64 0, i32 9
  store i32 1, ptr %16, align 8, !tbaa !295
  %17 = getelementptr inbounds %struct.FaWait, ptr %2, i64 0, i32 10
  store i32 0, ptr %17, align 4, !tbaa !296
  %18 = tail call align 4 ptr @llvm.threadlocal.address.p0(ptr align 4 @fa_coop_budget)
  %19 = load i32, ptr %18, align 4, !tbaa !20
  %20 = add nsw i32 %19, -1
  store i32 %20, ptr %18, align 4, !tbaa !20
  %21 = icmp sgt i32 %19, 1
  br i1 %21, label %48, label %22

22:                                               ; preds = %15
  %23 = tail call fastcc ptr @executor() #35
  %24 = getelementptr inbounds i8, ptr %3, i64 16
  %25 = getelementptr inbounds i8, ptr %3, i64 24
  store ptr null, ptr %25, align 8, !tbaa !255
  %26 = getelementptr inbounds %struct.FaExecutor, ptr %23, i64 0, i32 1
  %27 = load ptr, ptr %26, align 8, !tbaa !256
  %28 = getelementptr inbounds i8, ptr %3, i64 32
  store ptr %27, ptr %28, align 8, !tbaa !257
  %29 = icmp eq ptr %27, null
  %30 = getelementptr inbounds %struct.FaTask, ptr %27, i64 0, i32 1
  %31 = select i1 %29, ptr %23, ptr %30
  store ptr %24, ptr %31, align 8, !tbaa !9
  store ptr %24, ptr %26, align 8, !tbaa !256
  %32 = getelementptr inbounds %struct.FaExecutor, ptr %23, i64 0, i32 2
  %33 = load i64, ptr %32, align 8, !tbaa !258
  %34 = add nsw i64 %33, 1
  store i64 %34, ptr %32, align 8, !tbaa !258
  %35 = getelementptr inbounds i8, ptr %3, i64 40
  store atomic i32 1, ptr %35 release, align 8
  br label %48

36:                                               ; preds = %9
  %37 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 9
  %38 = load i32, ptr %37, align 16, !tbaa !311
  %39 = icmp eq i32 %38, 0
  br i1 %39, label %43, label %40

40:                                               ; preds = %36
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %41, label %48

41:                                               ; preds = %40
  %42 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %6) #34
  br label %48

43:                                               ; preds = %36
  %44 = getelementptr inbounds %struct.FaWait, ptr %2, i64 0, i32 4
  store ptr %1, ptr %44, align 8, !tbaa !313
  %45 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 13
  tail call fastcc void @line_push(ptr noundef nonnull %45, ptr noundef %2, ptr noundef %3, ptr noundef %4) #35
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %46, label %48

46:                                               ; preds = %43
  %47 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %6) #34
  br label %48

48:                                               ; preds = %46, %43, %41, %40, %15, %22
  %49 = phi i32 [ 2, %22 ], [ 0, %15 ], [ 3, %40 ], [ 3, %41 ], [ 1, %43 ], [ 1, %46 ]
  ret i32 %49
}

; Function Attrs: nounwind optsize
define i32 @fa_channel_cancel(ptr noundef %0, ptr noundef %1) local_unnamed_addr #0 {
  %3 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 1
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %4, label %6

4:                                                ; preds = %2
  %5 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %3) #34
  br label %6

6:                                                ; preds = %2, %4
  %7 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 9
  %8 = load i32, ptr %7, align 8, !tbaa !295
  %9 = icmp eq i32 %8, 0
  br i1 %9, label %10, label %40

10:                                               ; preds = %6
  %11 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 12
  %12 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 13
  %13 = load ptr, ptr %12, align 8, !tbaa !314
  br label %14

14:                                               ; preds = %19, %10
  %15 = phi ptr [ %13, %10 ], [ %21, %19 ]
  %16 = icmp eq ptr %15, null
  br i1 %16, label %23, label %17

17:                                               ; preds = %14
  %18 = icmp eq ptr %15, %1
  br i1 %18, label %23, label %19

19:                                               ; preds = %17
  %20 = getelementptr inbounds %struct.FaWait, ptr %15, i64 0, i32 2
  %21 = load ptr, ptr %20, align 8, !tbaa !298
  %22 = icmp eq ptr %21, %13
  br i1 %22, label %23, label %14, !llvm.loop !315

23:                                               ; preds = %19, %17, %14
  %24 = phi ptr [ %11, %14 ], [ %12, %17 ], [ %11, %19 ]
  %25 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 2
  %26 = load ptr, ptr %25, align 8, !tbaa !298
  %27 = icmp eq ptr %26, %1
  br i1 %27, label %36, label %28

28:                                               ; preds = %23
  %29 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 3
  %30 = load ptr, ptr %29, align 8, !tbaa !299
  %31 = getelementptr inbounds %struct.FaWait, ptr %30, i64 0, i32 2
  store ptr %26, ptr %31, align 8, !tbaa !298
  %32 = load ptr, ptr %25, align 8, !tbaa !298
  %33 = getelementptr inbounds %struct.FaWait, ptr %32, i64 0, i32 3
  store ptr %30, ptr %33, align 8, !tbaa !299
  %34 = load ptr, ptr %24, align 8, !tbaa !9
  %35 = icmp eq ptr %34, %1
  br i1 %35, label %36, label %38

36:                                               ; preds = %28, %23
  %37 = phi ptr [ null, %23 ], [ %32, %28 ]
  store ptr %37, ptr %24, align 8, !tbaa !9
  br label %38

38:                                               ; preds = %28, %36
  store ptr null, ptr %25, align 8, !tbaa !298
  %39 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 3
  store ptr null, ptr %39, align 8, !tbaa !299
  br label %40

40:                                               ; preds = %38, %6
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %41, label %43

41:                                               ; preds = %40
  %42 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %3) #34
  br label %43

43:                                               ; preds = %40, %41
  br i1 %9, label %45, label %44

44:                                               ; preds = %43
  tail call fastcc void @forget_remote_wake(ptr noundef nonnull %1) #35
  br label %45

45:                                               ; preds = %44, %43
  ret i32 %8
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(argmem: readwrite)
define i32 @fa_wait_status(ptr nocapture noundef readonly %0) local_unnamed_addr #27 {
  %2 = getelementptr inbounds %struct.FaWait, ptr %0, i64 0, i32 9
  %3 = load atomic i32, ptr %2 acquire, align 8
  ret i32 %3
}

; Function Attrs: nounwind optsize
define void @fa_channel_close(ptr noundef %0) local_unnamed_addr #0 {
  %2 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 1
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %3, label %5

3:                                                ; preds = %1
  %4 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %2) #34
  br label %5

5:                                                ; preds = %1, %3
  %6 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 9
  store i32 1, ptr %6, align 16, !tbaa !311
  %7 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 13
  %8 = load ptr, ptr %7, align 8, !tbaa !9
  %9 = icmp eq ptr %8, null
  br i1 %9, label %10, label %14

10:                                               ; preds = %29, %5
  %11 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 12
  %12 = load ptr, ptr %11, align 8, !tbaa !9
  %13 = icmp eq ptr %12, null
  br i1 %13, label %54, label %34

14:                                               ; preds = %5, %29
  %15 = phi ptr [ %32, %29 ], [ %8, %5 ]
  %16 = getelementptr inbounds %struct.FaWait, ptr %15, i64 0, i32 2
  %17 = load ptr, ptr %16, align 8, !tbaa !298
  %18 = icmp eq ptr %17, %15
  br i1 %18, label %27, label %19

19:                                               ; preds = %14
  %20 = getelementptr inbounds %struct.FaWait, ptr %15, i64 0, i32 3
  %21 = load ptr, ptr %20, align 8, !tbaa !299
  %22 = getelementptr inbounds %struct.FaWait, ptr %21, i64 0, i32 2
  store ptr %17, ptr %22, align 8, !tbaa !298
  %23 = load ptr, ptr %16, align 8, !tbaa !298
  %24 = getelementptr inbounds %struct.FaWait, ptr %23, i64 0, i32 3
  store ptr %21, ptr %24, align 8, !tbaa !299
  %25 = load ptr, ptr %7, align 8, !tbaa !9
  %26 = icmp eq ptr %25, %15
  br i1 %26, label %27, label %29

27:                                               ; preds = %19, %14
  %28 = phi ptr [ null, %14 ], [ %23, %19 ]
  store ptr %28, ptr %7, align 8, !tbaa !9
  br label %29

29:                                               ; preds = %19, %27
  store ptr null, ptr %16, align 8, !tbaa !298
  %30 = getelementptr inbounds %struct.FaWait, ptr %15, i64 0, i32 3
  store ptr null, ptr %30, align 8, !tbaa !299
  %31 = getelementptr inbounds %struct.FaWait, ptr %15, i64 0, i32 9
  store i32 2, ptr %31, align 8, !tbaa !295
  tail call fastcc void @wake(ptr noundef nonnull %15) #35
  %32 = load ptr, ptr %7, align 8, !tbaa !9
  %33 = icmp eq ptr %32, null
  br i1 %33, label %10, label %14, !llvm.loop !316

34:                                               ; preds = %10, %49
  %35 = phi ptr [ %52, %49 ], [ %12, %10 ]
  %36 = getelementptr inbounds %struct.FaWait, ptr %35, i64 0, i32 2
  %37 = load ptr, ptr %36, align 8, !tbaa !298
  %38 = icmp eq ptr %37, %35
  br i1 %38, label %47, label %39

39:                                               ; preds = %34
  %40 = getelementptr inbounds %struct.FaWait, ptr %35, i64 0, i32 3
  %41 = load ptr, ptr %40, align 8, !tbaa !299
  %42 = getelementptr inbounds %struct.FaWait, ptr %41, i64 0, i32 2
  store ptr %37, ptr %42, align 8, !tbaa !298
  %43 = load ptr, ptr %36, align 8, !tbaa !298
  %44 = getelementptr inbounds %struct.FaWait, ptr %43, i64 0, i32 3
  store ptr %41, ptr %44, align 8, !tbaa !299
  %45 = load ptr, ptr %11, align 8, !tbaa !9
  %46 = icmp eq ptr %45, %35
  br i1 %46, label %47, label %49

47:                                               ; preds = %39, %34
  %48 = phi ptr [ null, %34 ], [ %43, %39 ]
  store ptr %48, ptr %11, align 8, !tbaa !9
  br label %49

49:                                               ; preds = %47, %39
  store ptr null, ptr %36, align 8, !tbaa !298
  %50 = getelementptr inbounds %struct.FaWait, ptr %35, i64 0, i32 3
  store ptr null, ptr %50, align 8, !tbaa !299
  %51 = getelementptr inbounds %struct.FaWait, ptr %35, i64 0, i32 9
  store i32 2, ptr %51, align 8, !tbaa !295
  tail call fastcc void @wake(ptr noundef nonnull %35) #35
  %52 = load ptr, ptr %11, align 8, !tbaa !9
  %53 = icmp eq ptr %52, null
  br i1 %53, label %54, label %34, !llvm.loop !317

54:                                               ; preds = %49, %10
  %55 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 10
  %56 = load i32, ptr %55, align 4, !tbaa !312
  %57 = icmp sgt i32 %56, 0
  br i1 %57, label %58, label %61

58:                                               ; preds = %54
  %59 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 2
  %60 = tail call i32 @pthread_cond_broadcast(ptr noundef nonnull %59) #34
  br label %61

61:                                               ; preds = %54, %58
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %62, label %64

62:                                               ; preds = %61
  %63 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %2) #34
  br label %64

64:                                               ; preds = %61, %62
  ret void
}

; Function Attrs: nounwind optsize
define ptr @fa_parallel_for(i64 noundef %0, ptr noundef %1, ptr noundef %2, i64 noundef %3, ptr noundef %4) local_unnamed_addr #0 {
  %6 = alloca i64, align 8
  %7 = alloca i64, align 8
  %8 = alloca %struct.ParallelJob, align 8
  %9 = icmp slt i64 %0, 1
  br i1 %9, label %75, label %10

10:                                               ; preds = %5
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %6) #31
  store i64 0, ptr %6, align 8, !tbaa !5
  %11 = icmp eq i64 %0, 1
  %12 = or i1 %11, icmp eq (ptr @pthread_create, ptr null)
  br i1 %12, label %17, label %13

13:                                               ; preds = %10
  %14 = tail call align 4 ptr @llvm.threadlocal.address.p0(ptr align 4 @in_parallel)
  %15 = load i32, ptr %14, align 4, !tbaa !20
  %16 = icmp eq i32 %15, 0
  br i1 %16, label %19, label %17

17:                                               ; preds = %13, %10
  %18 = call ptr %1(ptr noundef %2, i64 noundef 0, i64 noundef %0, ptr noundef nonnull %6) #34
  br label %73

19:                                               ; preds = %13
  br i1 icmp ne (ptr @sysconf, ptr null), label %20, label %22

20:                                               ; preds = %19
  %21 = tail call i64 @sysconf(i32 noundef 84) #34
  br label %22

22:                                               ; preds = %19, %20
  %23 = phi i64 [ %21, %20 ], [ 1, %19 ]
  %24 = tail call i64 @llvm.smax.i64(i64 %23, i64 1)
  %25 = icmp slt i64 %3, 1
  %26 = tail call i64 @llvm.umin.i64(i64 %24, i64 %3)
  %27 = select i1 %25, i64 %24, i64 %26
  %28 = icmp ult i64 %27, 2
  br i1 %28, label %32, label %29

29:                                               ; preds = %22
  %30 = atomicrmw xchg ptr getelementptr inbounds (%struct.anon, ptr @pool, i64 0, i32 5), i32 1 acquire, align 4
  %31 = icmp eq i32 %30, 0
  br i1 %31, label %34, label %32

32:                                               ; preds = %29, %22
  %33 = call ptr %1(ptr noundef %2, i64 noundef 0, i64 noundef %0, ptr noundef nonnull %6) #34
  br label %73

34:                                               ; preds = %29
  %35 = load i32, ptr getelementptr inbounds (%struct.anon, ptr @pool, i64 0, i32 6), align 8, !tbaa !318
  %36 = icmp eq i32 %35, 0
  br i1 %36, label %37, label %44

37:                                               ; preds = %34
  store i32 1, ptr getelementptr inbounds (%struct.anon, ptr @pool, i64 0, i32 6), align 8, !tbaa !318
  %38 = icmp sgt i64 %23, 1
  br i1 %38, label %39, label %44

39:                                               ; preds = %37, %39
  %40 = phi i64 [ %42, %39 ], [ 1, %37 ]
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %7) #31
  %41 = call i32 @pthread_create(ptr noundef nonnull %7, ptr noundef null, ptr noundef nonnull @pool_thread, ptr noundef null) #34
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %7) #31
  %42 = add nuw nsw i64 %40, 1
  %43 = icmp eq i64 %42, %23
  br i1 %43, label %44, label %39, !llvm.loop !320

44:                                               ; preds = %39, %37, %34
  call void @llvm.lifetime.start.p0(i64 80, ptr nonnull %8) #31
  %45 = getelementptr inbounds i8, ptr %8, i64 32
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(80) %45, i8 0, i64 40, i1 false)
  store ptr %1, ptr %8, align 8, !tbaa !321
  %46 = getelementptr inbounds %struct.ParallelJob, ptr %8, i64 0, i32 1
  store ptr %2, ptr %46, align 8, !tbaa !323
  %47 = getelementptr inbounds %struct.ParallelJob, ptr %8, i64 0, i32 2
  store i64 %0, ptr %47, align 8, !tbaa !324
  %48 = trunc i64 %27 to i32
  %49 = add i32 %48, -1
  %50 = getelementptr inbounds %struct.ParallelJob, ptr %8, i64 0, i32 6
  store i32 %49, ptr %50, align 4, !tbaa !325
  %51 = shl nsw i64 %27, 4
  %52 = udiv i64 %0, %51
  %53 = getelementptr inbounds %struct.ParallelJob, ptr %8, i64 0, i32 3
  %54 = call i64 @llvm.smax.i64(i64 %52, i64 1)
  store i64 %54, ptr %53, align 8, !tbaa !326
  %55 = getelementptr inbounds %struct.ParallelJob, ptr %8, i64 0, i32 11
  store ptr %4, ptr %55, align 8, !tbaa !327
  store i32 1, ptr %14, align 4, !tbaa !20
  %56 = ptrtoint ptr %8 to i64
  store atomic i64 %56, ptr getelementptr inbounds (%struct.anon, ptr @pool, i64 0, i32 2) seq_cst, align 16
  %57 = call i32 @pthread_mutex_lock(ptr noundef nonnull @pool) #34
  %58 = load i64, ptr getelementptr inbounds (%struct.anon, ptr @pool, i64 0, i32 3), align 8, !tbaa !328
  %59 = add nsw i64 %58, 1
  store i64 %59, ptr getelementptr inbounds (%struct.anon, ptr @pool, i64 0, i32 3), align 8, !tbaa !328
  %60 = call i32 @pthread_cond_broadcast(ptr noundef nonnull getelementptr inbounds (%struct.anon, ptr @pool, i64 0, i32 1)) #34
  %61 = call i32 @pthread_mutex_unlock(ptr noundef nonnull @pool) #34
  call fastcc void @run_chunks(ptr noundef nonnull %8) #35
  store atomic i64 0, ptr getelementptr inbounds (%struct.anon, ptr @pool, i64 0, i32 2) seq_cst, align 16
  %62 = load atomic i32, ptr getelementptr inbounds (%struct.anon, ptr @pool, i64 0, i32 4) seq_cst, align 16
  %63 = icmp eq i32 %62, 0
  br i1 %63, label %70, label %64

64:                                               ; preds = %44, %67
  br i1 icmp ne (ptr @sched_yield, ptr null), label %65, label %67

65:                                               ; preds = %64
  %66 = call i32 @sched_yield() #34
  br label %67

67:                                               ; preds = %65, %64
  %68 = load atomic i32, ptr getelementptr inbounds (%struct.anon, ptr @pool, i64 0, i32 4) seq_cst, align 16
  %69 = icmp eq i32 %68, 0
  br i1 %69, label %70, label %64, !llvm.loop !329

70:                                               ; preds = %67, %44
  store i32 0, ptr %14, align 4, !tbaa !20
  store atomic i32 0, ptr getelementptr inbounds (%struct.anon, ptr @pool, i64 0, i32 5) release, align 4
  %71 = getelementptr inbounds %struct.ParallelJob, ptr %8, i64 0, i32 9
  %72 = load ptr, ptr %71, align 8, !tbaa !330
  call void @llvm.lifetime.end.p0(i64 80, ptr nonnull %8) #31
  br label %73

73:                                               ; preds = %32, %70, %17
  %74 = phi ptr [ %18, %17 ], [ %33, %32 ], [ %72, %70 ]
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %6) #31
  br label %75

75:                                               ; preds = %5, %73
  %76 = phi ptr [ %74, %73 ], [ null, %5 ]
  ret ptr %76
}

; Function Attrs: optsize
declare extern_weak i32 @pthread_create(ptr noundef, ptr noundef, ptr noundef, ptr noundef) #1

; Function Attrs: noreturn nounwind optsize
define internal noalias noundef nonnull ptr @pool_thread(ptr nocapture readnone %0) #2 {
  %2 = tail call align 4 ptr @llvm.threadlocal.address.p0(ptr align 4 @in_parallel)
  store i32 1, ptr %2, align 4, !tbaa !20
  br label %3

3:                                                ; preds = %26, %1
  %4 = phi i64 [ 0, %1 ], [ %13, %26 ]
  %5 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull @pool) #34
  %6 = load i64, ptr getelementptr inbounds (%struct.anon, ptr @pool, i64 0, i32 3), align 8, !tbaa !328
  %7 = icmp eq i64 %6, %4
  br i1 %7, label %8, label %12

8:                                                ; preds = %3, %8
  %9 = tail call i32 @pthread_cond_wait(ptr noundef nonnull getelementptr inbounds (%struct.anon, ptr @pool, i64 0, i32 1), ptr noundef nonnull @pool) #34
  %10 = load i64, ptr getelementptr inbounds (%struct.anon, ptr @pool, i64 0, i32 3), align 8, !tbaa !328
  %11 = icmp eq i64 %10, %4
  br i1 %11, label %8, label %12, !llvm.loop !331

12:                                               ; preds = %8, %3
  %13 = phi i64 [ %6, %3 ], [ %10, %8 ]
  %14 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull @pool) #34
  %15 = atomicrmw add ptr getelementptr inbounds (%struct.anon, ptr @pool, i64 0, i32 4), i32 1 seq_cst, align 16
  %16 = load atomic i64, ptr getelementptr inbounds (%struct.anon, ptr @pool, i64 0, i32 2) seq_cst, align 16
  %17 = inttoptr i64 %16 to ptr
  %18 = icmp eq i64 %16, 0
  br i1 %18, label %26, label %19

19:                                               ; preds = %12
  %20 = getelementptr inbounds %struct.ParallelJob, ptr %17, i64 0, i32 7
  %21 = atomicrmw add ptr %20, i32 1 monotonic, align 8
  %22 = getelementptr inbounds %struct.ParallelJob, ptr %17, i64 0, i32 6
  %23 = load i32, ptr %22, align 4, !tbaa !325
  %24 = icmp slt i32 %21, %23
  br i1 %24, label %25, label %26

25:                                               ; preds = %19
  tail call fastcc void @run_chunks(ptr noundef nonnull %17) #35
  tail call void @fa_release_builder() #35
  br label %26

26:                                               ; preds = %25, %19, %12
  %27 = atomicrmw sub ptr getelementptr inbounds (%struct.anon, ptr @pool, i64 0, i32 4), i32 1 seq_cst, align 16
  br label %3
}

; Function Attrs: optsize
declare extern_weak i32 @pthread_mutex_lock(ptr noundef) #1

; Function Attrs: optsize
declare extern_weak i32 @pthread_cond_broadcast(ptr noundef) local_unnamed_addr #1

; Function Attrs: optsize
declare extern_weak i32 @pthread_mutex_unlock(ptr noundef) #1

; Function Attrs: nounwind optsize
define internal fastcc void @run_chunks(ptr nocapture noundef %0) unnamed_addr #0 {
  %2 = alloca i64, align 8
  %3 = getelementptr inbounds %struct.ParallelJob, ptr %0, i64 0, i32 4
  %4 = getelementptr inbounds %struct.ParallelJob, ptr %0, i64 0, i32 3
  %5 = getelementptr inbounds %struct.ParallelJob, ptr %0, i64 0, i32 5
  %6 = load atomic i32, ptr %5 monotonic, align 8
  %7 = icmp eq i32 %6, 0
  br i1 %7, label %8, label %59

8:                                                ; preds = %1
  %9 = getelementptr inbounds %struct.ParallelJob, ptr %0, i64 0, i32 2
  %10 = getelementptr inbounds %struct.ParallelJob, ptr %0, i64 0, i32 1
  %11 = getelementptr inbounds %struct.ParallelJob, ptr %0, i64 0, i32 8
  %12 = getelementptr inbounds %struct.ParallelJob, ptr %0, i64 0, i32 9
  %13 = getelementptr inbounds %struct.ParallelJob, ptr %0, i64 0, i32 10
  %14 = getelementptr inbounds %struct.ParallelJob, ptr %0, i64 0, i32 11
  br label %15

15:                                               ; preds = %8, %56
  %16 = load i64, ptr %4, align 8, !tbaa !326
  %17 = atomicrmw add ptr %3, i64 %16 monotonic, align 8
  %18 = load i64, ptr %9, align 8, !tbaa !324
  %19 = icmp slt i64 %17, %18
  br i1 %19, label %20, label %59

20:                                               ; preds = %15
  %21 = sub nsw i64 %18, %17
  %22 = icmp slt i64 %21, %16
  %23 = add nsw i64 %17, %16
  %24 = select i1 %22, i64 %18, i64 %23
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %2) #31
  store i64 %17, ptr %2, align 8, !tbaa !5
  %25 = load ptr, ptr %0, align 8, !tbaa !321
  %26 = load ptr, ptr %10, align 8, !tbaa !323
  %27 = call ptr %25(ptr noundef %26, i64 noundef %17, i64 noundef %24, ptr noundef nonnull %2) #34
  %28 = icmp eq ptr %27, null
  br i1 %28, label %56, label %29

29:                                               ; preds = %20
  %30 = load i64, ptr %2, align 8, !tbaa !5
  store atomic i32 1, ptr %5 monotonic, align 8
  %31 = atomicrmw xchg ptr %11, i32 1 acquire, align 4
  %32 = icmp eq i32 %31, 0
  br i1 %32, label %42, label %36

33:                                               ; preds = %39, %36
  %34 = atomicrmw xchg ptr %11, i32 1 acquire, align 4
  %35 = icmp eq i32 %34, 0
  br i1 %35, label %42, label %36, !llvm.loop !2

36:                                               ; preds = %29, %33
  %37 = load atomic i32, ptr %11 monotonic, align 4
  %38 = icmp eq i32 %37, 0
  br i1 %38, label %33, label %39

39:                                               ; preds = %36, %39
  call void @llvm.x86.sse2.pause()
  %40 = load atomic i32, ptr %11 monotonic, align 4
  %41 = icmp eq i32 %40, 0
  br i1 %41, label %33, label %39, !llvm.loop !4

42:                                               ; preds = %33, %29
  %43 = load ptr, ptr %12, align 8, !tbaa !330
  %44 = icmp eq ptr %43, null
  br i1 %44, label %48, label %45

45:                                               ; preds = %42
  %46 = load i64, ptr %13, align 8, !tbaa !332
  %47 = icmp sgt i64 %46, %30
  br i1 %47, label %48, label %49

48:                                               ; preds = %45, %42
  store ptr %27, ptr %12, align 8, !tbaa !330
  store i64 %30, ptr %13, align 8, !tbaa !332
  br label %49

49:                                               ; preds = %48, %45
  %50 = phi ptr [ %43, %48 ], [ %27, %45 ]
  store atomic i32 0, ptr %11 release, align 4
  %51 = icmp eq ptr %50, null
  br i1 %51, label %56, label %52

52:                                               ; preds = %49
  %53 = load ptr, ptr %14, align 8, !tbaa !327
  %54 = icmp eq ptr %53, null
  br i1 %54, label %56, label %55

55:                                               ; preds = %52
  call void %53(ptr noundef nonnull %50) #34
  br label %56

56:                                               ; preds = %20, %49, %52, %55
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %2) #31
  %57 = load atomic i32, ptr %5 monotonic, align 8
  %58 = icmp eq i32 %57, 0
  br i1 %58, label %15, label %59

59:                                               ; preds = %56, %15, %1
  ret void
}

; Function Attrs: optsize
declare extern_weak i32 @sched_yield() #1

; Function Attrs: nounwind optsize
define weak { i64, i64 } @__multi3(i64 noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3) local_unnamed_addr #0 {
  %5 = and i64 %0, 4294967295
  %6 = lshr i64 %0, 32
  %7 = and i64 %2, 4294967295
  %8 = lshr i64 %2, 32
  %9 = mul nuw i64 %7, %5
  %10 = mul nuw i64 %7, %6
  %11 = mul nuw i64 %8, %5
  %12 = mul nuw i64 %8, %6
  %13 = lshr i64 %9, 32
  %14 = and i64 %10, 4294967295
  %15 = add nuw nsw i64 %13, %14
  %16 = and i64 %11, 4294967295
  %17 = add nuw nsw i64 %15, %16
  %18 = shl i64 %17, 32
  %19 = and i64 %9, 4294967295
  %20 = or disjoint i64 %18, %19
  %21 = lshr i64 %10, 32
  %22 = lshr i64 %11, 32
  %23 = lshr i64 %17, 32
  %24 = insertvalue { i64, i64 } poison, i64 %20, 0
  %25 = mul i64 %3, %0
  %26 = mul i64 %2, %1
  %27 = add i64 %12, %26
  %28 = add i64 %27, %25
  %29 = add i64 %28, %21
  %30 = add i64 %29, %22
  %31 = add i64 %30, %23
  %32 = insertvalue { i64, i64 } %24, i64 %31, 1
  ret { i64, i64 } %32
}

; Function Attrs: nounwind optsize
define weak { i64, i64 } @__udivti3(i64 noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3) local_unnamed_addr #0 {
  %5 = alloca i128, align 16
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %5) #31
  %6 = call fastcc { i64, i64 } @divide_u128(i64 noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3, ptr noundef nonnull %5) #35
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %5) #31
  ret { i64, i64 } %6
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: write)
define internal fastcc { i64, i64 } @divide_u128(i64 noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3, ptr nocapture noundef writeonly %4) unnamed_addr #30 {
  br label %13

6:                                                ; preds = %45
  %7 = zext i64 %46 to i128
  %8 = shl nuw i128 %7, 64
  %9 = zext i64 %47 to i128
  %10 = or disjoint i128 %8, %9
  store i128 %10, ptr %4, align 16, !tbaa !333
  %11 = insertvalue { i64, i64 } poison, i64 %49, 0
  %12 = insertvalue { i64, i64 } %11, i64 %48, 1
  ret { i64, i64 } %12

13:                                               ; preds = %5, %45
  %14 = phi i64 [ 127, %5 ], [ %50, %45 ]
  %15 = phi i64 [ 0, %5 ], [ %49, %45 ]
  %16 = phi i64 [ 0, %5 ], [ %48, %45 ]
  %17 = phi i64 [ 0, %5 ], [ %47, %45 ]
  %18 = phi i64 [ 0, %5 ], [ %46, %45 ]
  %19 = tail call i64 @llvm.fshl.i64(i64 %18, i64 %17, i64 1)
  %20 = icmp ugt i64 %14, 63
  %21 = add nsw i64 %14, -64
  %22 = lshr i64 %1, %21
  %23 = lshr i64 %0, %14
  %24 = select i1 %20, i64 %22, i64 %23
  %25 = and i64 %24, 1
  %26 = shl i64 %17, 1
  %27 = or disjoint i64 %25, %26
  %28 = icmp ugt i64 %19, %3
  br i1 %28, label %33, label %29

29:                                               ; preds = %13
  %30 = icmp ne i64 %19, %3
  %31 = icmp ult i64 %27, %2
  %32 = select i1 %30, i1 true, i1 %31
  br i1 %32, label %45, label %33

33:                                               ; preds = %29, %13
  %34 = icmp ult i64 %27, %2
  %35 = sext i1 %34 to i64
  %36 = sub i64 %27, %2
  %37 = sub i64 %19, %3
  %38 = add i64 %37, %35
  br i1 %20, label %39, label %42

39:                                               ; preds = %33
  %40 = shl nuw i64 1, %21
  %41 = or i64 %16, %40
  br label %45

42:                                               ; preds = %33
  %43 = shl nuw i64 1, %14
  %44 = or i64 %15, %43
  br label %45

45:                                               ; preds = %39, %42, %29
  %46 = phi i64 [ %19, %29 ], [ %38, %42 ], [ %38, %39 ]
  %47 = phi i64 [ %27, %29 ], [ %36, %42 ], [ %36, %39 ]
  %48 = phi i64 [ %16, %29 ], [ %16, %42 ], [ %41, %39 ]
  %49 = phi i64 [ %15, %29 ], [ %44, %42 ], [ %15, %39 ]
  %50 = add nsw i64 %14, -1
  %51 = icmp eq i64 %14, 0
  br i1 %51, label %6, label %13, !llvm.loop !335
}

; Function Attrs: nounwind optsize
define weak { i64, i64 } @__umodti3(i64 noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3) local_unnamed_addr #0 {
  %5 = alloca i128, align 16
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %5) #31
  %6 = call fastcc { i64, i64 } @divide_u128(i64 noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3, ptr noundef nonnull %5) #35
  %7 = load i128, ptr %5, align 16, !tbaa !333
  %8 = trunc i128 %7 to i64
  %9 = lshr i128 %7, 64
  %10 = trunc i128 %9 to i64
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %5) #31
  %11 = insertvalue { i64, i64 } poison, i64 %8, 0
  %12 = insertvalue { i64, i64 } %11, i64 %10, 1
  ret { i64, i64 } %12
}

; Function Attrs: nounwind optsize
define weak { i64, i64 } @__divti3(i64 noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3) local_unnamed_addr #0 {
  %5 = alloca i128, align 16
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %5) #31
  %6 = zext i64 %1 to i128
  %7 = shl nuw i128 %6, 64
  %8 = zext i64 %0 to i128
  %9 = or disjoint i128 %7, %8
  %10 = tail call i128 @llvm.abs.i128(i128 %9, i1 false)
  %11 = trunc i128 %10 to i64
  %12 = lshr i128 %10, 64
  %13 = trunc i128 %12 to i64
  %14 = zext i64 %3 to i128
  %15 = shl nuw i128 %14, 64
  %16 = zext i64 %2 to i128
  %17 = or disjoint i128 %15, %16
  %18 = tail call i128 @llvm.abs.i128(i128 %17, i1 false)
  %19 = trunc i128 %18 to i64
  %20 = lshr i128 %18, 64
  %21 = trunc i128 %20 to i64
  %22 = call fastcc { i64, i64 } @divide_u128(i64 noundef %11, i64 noundef %13, i64 noundef %19, i64 noundef %21, ptr noundef nonnull %5) #35
  %23 = extractvalue { i64, i64 } %22, 0
  %24 = extractvalue { i64, i64 } %22, 1
  %25 = zext i64 %24 to i128
  %26 = shl nuw i128 %25, 64
  %27 = zext i64 %23 to i128
  %28 = or disjoint i128 %26, %27
  %29 = xor i64 %3, %1
  %30 = sub i128 0, %28
  %31 = icmp slt i64 %29, 0
  %32 = select i1 %31, i128 %30, i128 %28
  %33 = trunc i128 %32 to i64
  %34 = lshr i128 %32, 64
  %35 = trunc i128 %34 to i64
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %5) #31
  %36 = insertvalue { i64, i64 } poison, i64 %33, 0
  %37 = insertvalue { i64, i64 } %36, i64 %35, 1
  ret { i64, i64 } %37
}

; Function Attrs: nounwind optsize
define weak { i64, i64 } @__modti3(i64 noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3) local_unnamed_addr #0 {
  %5 = alloca i128, align 16
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %5) #31
  %6 = zext i64 %1 to i128
  %7 = shl nuw i128 %6, 64
  %8 = zext i64 %0 to i128
  %9 = or disjoint i128 %7, %8
  %10 = tail call i128 @llvm.abs.i128(i128 %9, i1 false)
  %11 = trunc i128 %10 to i64
  %12 = lshr i128 %10, 64
  %13 = trunc i128 %12 to i64
  %14 = zext i64 %3 to i128
  %15 = shl nuw i128 %14, 64
  %16 = zext i64 %2 to i128
  %17 = or disjoint i128 %15, %16
  %18 = tail call i128 @llvm.abs.i128(i128 %17, i1 false)
  %19 = trunc i128 %18 to i64
  %20 = lshr i128 %18, 64
  %21 = trunc i128 %20 to i64
  %22 = call fastcc { i64, i64 } @divide_u128(i64 noundef %11, i64 noundef %13, i64 noundef %19, i64 noundef %21, ptr noundef nonnull %5) #35
  %23 = load i128, ptr %5, align 16
  %24 = sub i128 0, %23
  %25 = icmp slt i64 %1, 0
  %26 = select i1 %25, i128 %24, i128 %23
  %27 = trunc i128 %26 to i64
  %28 = lshr i128 %26, 64
  %29 = trunc i128 %28 to i64
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %5) #31
  %30 = insertvalue { i64, i64 } poison, i64 %27, 0
  %31 = insertvalue { i64, i64 } %30, i64 %29, 1
  ret { i64, i64 } %31
}

; Function Attrs: nounwind
declare void @llvm.x86.sse2.pause() #31

; Function Attrs: optsize
declare i32 @snprintf(ptr noundef, i64 noundef, ptr noundef, ...) local_unnamed_addr #1

; Function Attrs: optsize
declare double @strtod(ptr noundef, ptr noundef) local_unnamed_addr #1

; Function Attrs: nofree norecurse nosync nounwind optsize memory(none)
define internal fastcc i32 @break_kind(i32 noundef %0) unnamed_addr #32 {
  switch i32 %0, label %3 [
    i32 13, label %83
    i32 10, label %2
  ]

2:                                                ; preds = %1
  br label %83

3:                                                ; preds = %1
  %4 = icmp ult i32 %0, 32
  br i1 %4, label %83, label %5

5:                                                ; preds = %3
  %6 = add i32 %0, -127
  %7 = icmp ult i32 %6, 33
  %8 = and i32 %0, -2
  %9 = icmp eq i32 %8, 8232
  %10 = or i1 %7, %9
  br i1 %10, label %83, label %11

11:                                               ; preds = %5
  %12 = icmp ult i32 %0, 768
  br i1 %12, label %13, label %18

13:                                               ; preds = %11
  %14 = icmp eq i32 %0, 169
  %15 = icmp eq i32 %0, 174
  %16 = or i1 %14, %15
  %17 = select i1 %16, i32 7, i32 0
  br label %83

18:                                               ; preds = %11
  %19 = icmp eq i32 %0, 8205
  br i1 %19, label %83, label %20

20:                                               ; preds = %18
  %21 = add i32 %0, -127462
  %22 = icmp ult i32 %21, 26
  br i1 %22, label %83, label %23

23:                                               ; preds = %20
  %24 = add i32 %0, -4352
  %25 = icmp ult i32 %24, 96
  %26 = add i32 %0, -43360
  %27 = icmp ult i32 %26, 29
  %28 = or i1 %25, %27
  br i1 %28, label %83, label %29

29:                                               ; preds = %23
  %30 = add i32 %0, -4448
  %31 = icmp ult i32 %30, 72
  %32 = add i32 %0, -55216
  %33 = icmp ult i32 %32, 23
  %34 = or i1 %31, %33
  br i1 %34, label %83, label %35

35:                                               ; preds = %29
  %36 = add i32 %0, -4520
  %37 = icmp ult i32 %36, 88
  %38 = add i32 %0, -55243
  %39 = icmp ult i32 %38, 49
  %40 = or i1 %37, %39
  br i1 %40, label %83, label %41

41:                                               ; preds = %35
  %42 = add i32 %0, -44032
  %43 = icmp ult i32 %42, 11172
  br i1 %43, label %44, label %49

44:                                               ; preds = %41
  %45 = trunc i32 %42 to i16
  %46 = urem i16 %45, 28
  %47 = icmp eq i16 %46, 0
  %48 = select i1 %47, i32 11, i32 12
  br label %83

49:                                               ; preds = %41, %61
  %50 = phi i64 [ %65, %61 ], [ 0, %41 ]
  %51 = phi i64 [ %53, %61 ], [ 153, %41 ]
  br label %52

52:                                               ; preds = %55, %49
  %53 = phi i64 [ %57, %55 ], [ %51, %49 ]
  %54 = icmp ult i64 %50, %53
  br i1 %54, label %55, label %66

55:                                               ; preds = %52
  %56 = add i64 %53, %50
  %57 = lshr i64 %56, 1
  %58 = getelementptr inbounds %struct.Range, ptr @EXTEND, i64 %57
  %59 = load i32, ptr %58, align 8, !tbaa !115
  %60 = icmp ugt i32 %59, %0
  br i1 %60, label %52, label %61, !llvm.loop !336

61:                                               ; preds = %55
  %62 = getelementptr inbounds %struct.Range, ptr @EXTEND, i64 %57, i32 1
  %63 = load i32, ptr %62, align 4, !tbaa !117
  %64 = icmp ult i32 %63, %0
  %65 = add nuw i64 %57, 1
  br i1 %64, label %49, label %83, !llvm.loop !336

66:                                               ; preds = %52, %78
  %67 = phi i64 [ %82, %78 ], [ 0, %52 ]
  %68 = phi i64 [ %70, %78 ], [ 22, %52 ]
  br label %69

69:                                               ; preds = %72, %66
  %70 = phi i64 [ %74, %72 ], [ %68, %66 ]
  %71 = icmp ult i64 %67, %70
  br i1 %71, label %72, label %83

72:                                               ; preds = %69
  %73 = add i64 %70, %67
  %74 = lshr i64 %73, 1
  %75 = getelementptr inbounds %struct.Range, ptr @PICTOGRAPHIC, i64 %74
  %76 = load i32, ptr %75, align 8, !tbaa !115
  %77 = icmp ugt i32 %76, %0
  br i1 %77, label %69, label %78, !llvm.loop !336

78:                                               ; preds = %72
  %79 = getelementptr inbounds %struct.Range, ptr @PICTOGRAPHIC, i64 %74, i32 1
  %80 = load i32, ptr %79, align 4, !tbaa !117
  %81 = icmp ult i32 %80, %0
  %82 = add nuw i64 %74, 1
  br i1 %81, label %66, label %83, !llvm.loop !336

83:                                               ; preds = %61, %78, %69, %35, %29, %23, %20, %18, %3, %5, %1, %44, %13, %2
  %84 = phi i32 [ 2, %2 ], [ %17, %13 ], [ %48, %44 ], [ 1, %1 ], [ 3, %5 ], [ 3, %3 ], [ 5, %18 ], [ 6, %20 ], [ 8, %23 ], [ 9, %29 ], [ 10, %35 ], [ 0, %69 ], [ 7, %78 ], [ 4, %61 ]
  ret i32 %84
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc i32 @at_boundary(ptr nocapture noundef %0, i64 noundef %1) unnamed_addr #10 {
  %3 = load ptr, ptr %0, align 8, !tbaa !36
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %33, label %5

5:                                                ; preds = %2
  %6 = getelementptr inbounds %struct.Boundaries, ptr %0, i64 0, i32 1
  %7 = load i64, ptr %6, align 8, !tbaa !38
  %8 = icmp ugt i64 %7, %1
  br i1 %8, label %9, label %33

9:                                                ; preds = %5
  %10 = getelementptr inbounds i8, ptr %3, i64 %1
  %11 = load i8, ptr %10, align 1, !tbaa !12
  %12 = icmp sgt i8 %11, -1
  br i1 %12, label %13, label %21

13:                                               ; preds = %9
  %14 = getelementptr i8, ptr %10, i64 -1
  %15 = load i8, ptr %14, align 1, !tbaa !12
  %16 = icmp sgt i8 %15, -1
  br i1 %16, label %17, label %21

17:                                               ; preds = %13
  %18 = icmp eq i8 %15, 13
  %19 = icmp eq i8 %11, 10
  %20 = and i1 %19, %18
  br i1 %20, label %21, label %33

21:                                               ; preds = %17, %13, %9
  %22 = getelementptr inbounds %struct.Boundaries, ptr %0, i64 0, i32 2
  %23 = load i64, ptr %22, align 8, !tbaa !40
  %24 = icmp ult i64 %23, %1
  br i1 %24, label %25, label %29

25:                                               ; preds = %21, %25
  %26 = phi i64 [ %27, %25 ], [ %23, %21 ]
  %27 = tail call fastcc i64 @next_boundary(ptr noundef %3, i64 noundef %7, i64 noundef %26) #35
  store i64 %27, ptr %22, align 8, !tbaa !40
  %28 = icmp ult i64 %27, %1
  br i1 %28, label %25, label %29, !llvm.loop !337

29:                                               ; preds = %25, %21
  %30 = phi i64 [ %23, %21 ], [ %27, %25 ]
  %31 = icmp eq i64 %30, %1
  %32 = zext i1 %31 to i32
  br label %33

33:                                               ; preds = %2, %5, %17, %29
  %34 = phi i32 [ %32, %29 ], [ 1, %17 ], [ 1, %5 ], [ 1, %2 ]
  ret i32 %34
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.ctlz.i64(i64, i1 immarg) #5

; Function Attrs: optsize
declare ptr @fa_pages(i64 noundef) local_unnamed_addr #1

; Function Attrs: nounwind optsize
define internal fastcc void @mark(ptr noundef %0) unnamed_addr #0 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 17
  %3 = load i32, ptr %2, align 8, !tbaa !203
  %4 = icmp eq i32 %3, 0
  br i1 %4, label %20, label %5

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 14
  %7 = load i32, ptr %6, align 4, !tbaa !211
  %8 = icmp eq i32 %7, 0
  br i1 %8, label %9, label %38

9:                                                ; preds = %5
  %10 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 15
  %11 = load i32, ptr %10, align 8, !tbaa !205
  %12 = icmp eq i32 %11, 0
  br i1 %12, label %13, label %38

13:                                               ; preds = %9
  store i32 1, ptr %6, align 4, !tbaa !211
  %14 = load ptr, ptr @reactive_pending, align 8, !tbaa !9
  %15 = load i64, ptr @reactive_pending_count, align 8, !tbaa !5
  %16 = tail call fastcc ptr @grow_items(ptr noundef %14, i64 noundef %15, ptr noundef nonnull @reactive_pending_capacity, i64 noundef 8) #35
  store ptr %16, ptr @reactive_pending, align 8, !tbaa !9
  %17 = load i64, ptr @reactive_pending_count, align 8, !tbaa !5
  %18 = getelementptr inbounds ptr, ptr %16, i64 %17
  store ptr %0, ptr %18, align 8, !tbaa !9
  %19 = add nsw i64 %17, 1
  store i64 %19, ptr @reactive_pending_count, align 8, !tbaa !5
  br label %38

20:                                               ; preds = %1
  %21 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 11
  %22 = load i32, ptr %21, align 8, !tbaa !192
  %23 = icmp eq i32 %22, 0
  br i1 %23, label %24, label %38

24:                                               ; preds = %20
  store i32 1, ptr %21, align 8, !tbaa !192
  %25 = getelementptr inbounds %struct.FaSource, ptr %0, i64 0, i32 3
  %26 = load i64, ptr %25, align 8, !tbaa !338
  %27 = icmp sgt i64 %26, 0
  br i1 %27, label %28, label %38

28:                                               ; preds = %24
  %29 = getelementptr inbounds %struct.FaSource, ptr %0, i64 0, i32 2
  br label %30

30:                                               ; preds = %28, %30
  %31 = phi i64 [ 0, %28 ], [ %35, %30 ]
  %32 = load ptr, ptr %29, align 8, !tbaa !339
  %33 = getelementptr inbounds ptr, ptr %32, i64 %31
  %34 = load ptr, ptr %33, align 8, !tbaa !9
  tail call fastcc void @mark(ptr noundef %34) #35
  %35 = add nuw nsw i64 %31, 1
  %36 = load i64, ptr %25, align 8, !tbaa !338
  %37 = icmp slt i64 %35, %36
  br i1 %37, label %30, label %38, !llvm.loop !340

38:                                               ; preds = %30, %24, %20, %5, %9, %13
  ret void
}

; Function Attrs: noreturn optsize
declare void @_exit(i32 noundef) local_unnamed_addr #3

; Function Attrs: optsize
declare extern_weak i32 @epoll_create1(i32 noundef) #1

; Function Attrs: optsize
declare extern_weak i32 @eventfd(i32 noundef, i32 noundef) #1

; Function Attrs: optsize
declare extern_weak i32 @epoll_ctl(i32 noundef, i32 noundef, i32 noundef, ptr noundef) local_unnamed_addr #1

; Function Attrs: cold noinline noreturn nounwind optsize
define internal fastcc void @deadlock_stop(ptr nocapture noundef readonly %0) unnamed_addr #22 {
  tail call void @fa_flush() #35
  %2 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %2, align 8, !tbaa !5
  tail call fastcc void @append_c(ptr noundef nonnull @.str.49) #35
  %3 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 17
  %4 = load ptr, ptr %3, align 8, !tbaa !265
  %5 = icmp eq ptr %4, null
  br i1 %5, label %6, label %10

6:                                                ; preds = %1
  %7 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 18
  %8 = load ptr, ptr %7, align 8, !tbaa !300
  %9 = icmp eq ptr %8, null
  br i1 %9, label %14, label %10

10:                                               ; preds = %6, %1
  %11 = phi ptr [ @.str.50, %1 ], [ @.str.52, %6 ]
  %12 = phi ptr [ %3, %1 ], [ %7, %6 ]
  tail call fastcc void @append_c(ptr noundef nonnull %11) #35
  %13 = load ptr, ptr %12, align 8, !tbaa !9
  tail call fastcc void @append_c(ptr noundef %13) #35
  tail call fastcc void @append_c(ptr noundef nonnull @.str.51) #35
  br label %14

14:                                               ; preds = %10, %6
  tail call void @fa_b_line(i32 noundef 2) #35
  tail call void @fa_exit(i32 noundef 101) #36
  unreachable
}

; Function Attrs: optsize
declare void @fa_resume(ptr noundef) local_unnamed_addr #1

; Function Attrs: optsize
declare extern_weak i32 @epoll_wait(i32 noundef, ptr noundef, i32 noundef, i32 noundef) local_unnamed_addr #1

; Function Attrs: optsize
declare extern_weak i64 @read(i32 noundef, ptr noundef, i64 noundef) local_unnamed_addr #1

; Function Attrs: optsize
declare extern_weak i64 @write(i32 noundef, ptr noundef, i64 noundef) local_unnamed_addr #1

; Function Attrs: nounwind optsize
define internal fastcc void @channel_push(ptr nocapture noundef %0, ptr nocapture noundef readonly %1) unnamed_addr #0 {
  %3 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 7
  %4 = load i64, ptr %3, align 16, !tbaa !306
  %5 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 8
  %6 = load i64, ptr %5, align 8, !tbaa !307
  %7 = icmp eq i64 %4, %6
  br i1 %7, label %13, label %8

8:                                                ; preds = %2
  %9 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 6
  %10 = load i64, ptr %9, align 8, !tbaa !308
  %11 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 5
  %12 = load ptr, ptr %11, align 16, !tbaa !309
  br label %58

13:                                               ; preds = %2
  %14 = icmp slt i64 %4, 4
  %15 = shl nuw nsw i64 %4, 1
  %16 = select i1 %14, i64 4, i64 %15
  %17 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 4
  %18 = load i64, ptr %17, align 8, !tbaa !304
  %19 = mul nsw i64 %18, %16
  %20 = tail call ptr @fa_alloc(i64 noundef %19, i64 noundef 16) #34
  %21 = icmp eq ptr %20, null
  br i1 %21, label %28, label %22

22:                                               ; preds = %13
  %23 = load i64, ptr %3, align 16, !tbaa !306
  %24 = icmp sgt i64 %23, 0
  br i1 %24, label %25, label %29

25:                                               ; preds = %22
  %26 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 6
  %27 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 5
  br label %33

28:                                               ; preds = %13
  tail call fastcc void @out_of_memory() #37
  unreachable

29:                                               ; preds = %33, %22
  %30 = phi i64 [ %23, %22 ], [ %46, %33 ]
  %31 = load i64, ptr %5, align 8, !tbaa !307
  %32 = icmp sgt i64 %31, 0
  br i1 %32, label %48, label %54

33:                                               ; preds = %25, %33
  %34 = phi i64 [ 0, %25 ], [ %45, %33 ]
  %35 = load i64, ptr %26, align 8, !tbaa !308
  %36 = add nsw i64 %35, %34
  %37 = load i64, ptr %5, align 8, !tbaa !307
  %38 = srem i64 %36, %37
  %39 = load i64, ptr %17, align 8, !tbaa !304
  %40 = mul nsw i64 %39, %34
  %41 = getelementptr inbounds i8, ptr %20, i64 %40
  %42 = load ptr, ptr %27, align 16, !tbaa !309
  %43 = mul nsw i64 %39, %38
  %44 = getelementptr inbounds i8, ptr %42, i64 %43
  tail call void @llvm.memcpy.p0.p0.i64(ptr nonnull align 1 %41, ptr align 1 %44, i64 %39, i1 false)
  %45 = add nuw nsw i64 %34, 1
  %46 = load i64, ptr %3, align 16, !tbaa !306
  %47 = icmp slt i64 %45, %46
  br i1 %47, label %33, label %29, !llvm.loop !341

48:                                               ; preds = %29
  %49 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 5
  %50 = load ptr, ptr %49, align 16, !tbaa !309
  %51 = load i64, ptr %17, align 8, !tbaa !304
  %52 = mul nsw i64 %51, %31
  tail call void @fa_free(ptr noundef %50, i64 noundef %52, i64 noundef 16) #34
  %53 = load i64, ptr %3, align 16, !tbaa !306
  br label %54

54:                                               ; preds = %48, %29
  %55 = phi i64 [ %53, %48 ], [ %30, %29 ]
  %56 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 5
  store ptr %20, ptr %56, align 16, !tbaa !309
  %57 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 6
  store i64 0, ptr %57, align 8, !tbaa !308
  store i64 %16, ptr %5, align 8, !tbaa !307
  br label %58

58:                                               ; preds = %8, %54
  %59 = phi ptr [ %12, %8 ], [ %20, %54 ]
  %60 = phi i64 [ %6, %8 ], [ %16, %54 ]
  %61 = phi i64 [ %4, %8 ], [ %55, %54 ]
  %62 = phi i64 [ %10, %8 ], [ 0, %54 ]
  %63 = add nsw i64 %61, %62
  %64 = srem i64 %63, %60
  %65 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 4
  %66 = load i64, ptr %65, align 8, !tbaa !304
  %67 = mul nsw i64 %66, %64
  %68 = getelementptr inbounds i8, ptr %59, i64 %67
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %68, ptr align 1 %1, i64 %66, i1 false)
  %69 = load i64, ptr %3, align 16, !tbaa !306
  %70 = add nsw i64 %69, 1
  store i64 %70, ptr %3, align 16, !tbaa !306
  ret void
}

; Function Attrs: optsize
declare extern_weak i64 @sysconf(i32 noundef) #1

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.abs.i64(i64, i1 immarg) #33

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.umin.i64(i64, i64) #33

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.umax.i64(i64, i64) #33

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.smin.i64(i64, i64) #33

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.smax.i64(i64, i64) #33

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.fshl.i64(i64, i64, i64) #33

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i128 @llvm.abs.i128(i128, i1 immarg) #33

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.smin.i32(i32, i32) #33

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.abs.i32(i32, i1 immarg) #33

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.usub.sat.i64(i64, i64) #33

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.vector.reduce.or.v4i32(<4 x i32>) #33

attributes #0 = { nounwind optsize "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #1 = { optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #2 = { noreturn nounwind optsize "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #3 = { noreturn optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #4 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, inaccessiblemem: none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #5 = { mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #6 = { mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite) }
attributes #7 = { mustprogress nocallback nofree nounwind willreturn memory(argmem: readwrite) }
attributes #8 = { nofree norecurse nosync nounwind optsize memory(argmem: read) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #9 = { nofree norecurse nosync nounwind optsize memory(argmem: readwrite) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #10 = { nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #11 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #12 = { cold noinline nounwind optsize "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #13 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(readwrite, inaccessiblemem: none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #14 = { alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #15 = { mustprogress nocallback nofree nounwind willreturn memory(argmem: write) }
attributes #16 = { nofree norecurse nosync nounwind optsize memory(read, inaccessiblemem: none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #17 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(read, inaccessiblemem: none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #18 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, argmem: none, inaccessiblemem: none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #19 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(read, argmem: none, inaccessiblemem: none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #20 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #21 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: write) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #22 = { cold noinline noreturn nounwind optsize "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #23 = { cold mustprogress nofree noinline norecurse nosync nounwind optsize willreturn memory(none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #24 = { alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(read, inaccessiblemem: none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #25 = { alwaysinline nounwind optsize "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #26 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, argmem: readwrite, inaccessiblemem: none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #27 = { mustprogress nofree norecurse nounwind optsize willreturn memory(argmem: readwrite) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #28 = { mustprogress nofree norecurse nounwind optsize willreturn memory(readwrite, inaccessiblemem: none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #29 = { nofree norecurse nounwind optsize memory(readwrite, inaccessiblemem: none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #30 = { nofree norecurse nosync nounwind optsize memory(argmem: write) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #31 = { nounwind }
attributes #32 = { nofree norecurse nosync nounwind optsize memory(none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #33 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #34 = { nounwind optsize }
attributes #35 = { optsize }
attributes #36 = { noreturn nounwind optsize }
attributes #37 = { noreturn optsize }
attributes #38 = { cold optsize }
attributes #39 = { cold noreturn optsize }

!llvm.module.flags = !{!0, !1}

!0 = !{i32 1, !"wchar_size", i32 4}
!1 = !{i32 8, !"PIC Level", i32 2}
!2 = distinct !{!2, !3}
!3 = !{!"llvm.loop.mustprogress"}
!4 = distinct !{!4, !3}
!5 = !{!6, !6, i64 0}
!6 = !{!"long", !7, i64 0}
!7 = !{!"omnipotent char", !8, i64 0}
!8 = !{!"Simple C/C++ TBAA"}
!9 = !{!10, !10, i64 0}
!10 = !{!"any pointer", !7, i64 0}
!11 = distinct !{!11, !3}
!12 = !{!7, !7, i64 0}
!13 = !{!14}
!14 = distinct !{!14, !15, !"owned_copy: argument 0"}
!15 = distinct !{!15, !"owned_copy"}
!16 = !{i64 0, i64 8, !9, i64 8, i64 8, !5, i64 16, i64 8, !5}
!17 = !{i64 0, i64 8, !5, i64 8, i64 8, !5}
!18 = !{i64 0, i64 8, !5}
!19 = distinct !{!19, !3}
!20 = !{!21, !21, i64 0}
!21 = !{!"int", !7, i64 0}
!22 = distinct !{!22, !3}
!23 = distinct !{!23, !3}
!24 = distinct !{!24, !3}
!25 = distinct !{!25, !3}
!26 = distinct !{!26, !3}
!27 = distinct !{!27, !3}
!28 = distinct !{!28, !3}
!29 = distinct !{!29, !3}
!30 = distinct !{!30, !3}
!31 = distinct !{!31, !3}
!32 = distinct !{!32, !3}
!33 = distinct !{!33, !3}
!34 = distinct !{!34, !3}
!35 = distinct !{!35, !3}
!36 = !{!37, !10, i64 0}
!37 = !{!"", !10, i64 0, !6, i64 8, !6, i64 16}
!38 = !{!37, !6, i64 8}
!39 = distinct !{!39, !3}
!40 = !{!37, !6, i64 16}
!41 = distinct !{!41, !3}
!42 = distinct !{!42, !3}
!43 = !{i32 0, i32 13}
!44 = distinct !{!44, !3}
!45 = distinct !{!45, !3}
!46 = !{i32 0, i32 2}
!47 = distinct !{!47, !3}
!48 = distinct !{!48, !3}
!49 = distinct !{!49, !3}
!50 = !{!51, !10, i64 0}
!51 = !{!"", !10, i64 0, !6, i64 8, !6, i64 16, !10, i64 24, !6, i64 32, !21, i64 40, !21, i64 44}
!52 = !{!51, !6, i64 8}
!53 = !{!51, !6, i64 16}
!54 = !{!51, !10, i64 24}
!55 = !{!51, !6, i64 32}
!56 = !{!51, !21, i64 40}
!57 = !{!51, !21, i64 44}
!58 = !{ptr @replace_one, ptr @split_one}
!59 = distinct !{!59, !3}
!60 = !{!61, !63}
!61 = distinct !{!61, !62, !"owned_copy: argument 0"}
!62 = distinct !{!62, !"owned_copy"}
!63 = distinct !{!63, !64, !"slice: argument 0"}
!64 = distinct !{!64, !"slice"}
!65 = distinct !{!65, !3}
!66 = !{!67, !10, i64 0}
!67 = !{!"", !10, i64 0, !6, i64 8, !6, i64 16, !10, i64 24}
!68 = !{!67, !6, i64 8}
!69 = !{!67, !6, i64 16}
!70 = !{!67, !10, i64 24}
!71 = !{!72, !74}
!72 = distinct !{!72, !73, !"owned_copy: argument 0"}
!73 = distinct !{!73, !"owned_copy"}
!74 = distinct !{!74, !75, !"slice: argument 0"}
!75 = distinct !{!75, !"slice"}
!76 = !{!77, !79}
!77 = distinct !{!77, !78, !"owned_copy: argument 0"}
!78 = distinct !{!78, !"owned_copy"}
!79 = distinct !{!79, !80, !"slice: argument 0"}
!80 = distinct !{!80, !"slice"}
!81 = distinct !{!81, !3}
!82 = distinct !{!82, !3}
!83 = distinct !{!83, !3}
!84 = !{!85, !87}
!85 = distinct !{!85, !86, !"owned_copy: argument 0"}
!86 = distinct !{!86, !"owned_copy"}
!87 = distinct !{!87, !88, !"slice: argument 0"}
!88 = distinct !{!88, !"slice"}
!89 = distinct !{!89, !3}
!90 = distinct !{!90, !3}
!91 = !{!92}
!92 = distinct !{!92, !93, !"owned_copy: argument 0"}
!93 = distinct !{!93, !"owned_copy"}
!94 = distinct !{!94, !3}
!95 = distinct !{!95, !3}
!96 = distinct !{!96, !3}
!97 = distinct !{!97, !3}
!98 = distinct !{!98, !3}
!99 = !{!100, !6, i64 0}
!100 = !{!"", !6, i64 0, !6, i64 8, !6, i64 16}
!101 = !{!100, !6, i64 8}
!102 = !{!100, !6, i64 16}
!103 = distinct !{!103, !3}
!104 = distinct !{!104, !3}
!105 = distinct !{!105, !3}
!106 = distinct !{!106, !3}
!107 = distinct !{!107, !3}
!108 = !{i64 0, i64 65}
!109 = !{!110, !6, i64 0}
!110 = !{!"", !6, i64 0, !6, i64 8, !7, i64 16}
!111 = !{!110, !6, i64 8}
!112 = distinct !{!112, !3}
!113 = distinct !{!113, !3}
!114 = distinct !{!114, !3}
!115 = !{!116, !21, i64 0}
!116 = !{!"", !21, i64 0, !21, i64 4}
!117 = !{!116, !21, i64 4}
!118 = distinct !{!118, !3}
!119 = !{!120, !10, i64 0}
!120 = !{!"FreeBlock", !10, i64 0}
!121 = !{!122, !6, i64 8}
!122 = !{!"FreeSpan", !10, i64 0, !6, i64 8}
!123 = !{!122, !10, i64 0}
!124 = distinct !{!124, !3}
!125 = distinct !{!125, !3}
!126 = !{!127, !21, i64 40}
!127 = !{!"", !6, i64 0, !10, i64 8, !10, i64 16, !10, i64 24, !21, i64 32, !21, i64 36, !21, i64 40, !21, i64 44, !21, i64 48, !21, i64 52}
!128 = distinct !{!128, !3}
!129 = !{!127, !21, i64 44}
!130 = !{!127, !21, i64 48}
!131 = !{!127, !21, i64 52}
!132 = !{!127, !21, i64 32}
!133 = !{!127, !10, i64 16}
!134 = distinct !{!134, !3}
!135 = !{!127, !10, i64 8}
!136 = !{!127, !6, i64 0}
!137 = !{!127, !10, i64 24}
!138 = distinct !{!138, !3}
!139 = !{!127, !21, i64 36}
!140 = distinct !{!140, !3}
!141 = distinct !{!141, !3}
!142 = distinct !{!142, !3}
!143 = !{!144, !10, i64 32}
!144 = !{!"", !6, i64 0, !10, i64 8, !6, i64 16, !6, i64 24, !10, i64 32, !6, i64 40}
!145 = !{!144, !6, i64 16}
!146 = !{!144, !10, i64 8}
!147 = distinct !{!147, !3}
!148 = !{!144, !6, i64 40}
!149 = !{!144, !6, i64 24}
!150 = !{!144, !6, i64 0}
!151 = distinct !{!151, !3}
!152 = distinct !{!152, !3}
!153 = distinct !{!153, !3}
!154 = distinct !{!154, !3}
!155 = distinct !{!155, !3}
!156 = distinct !{!156, !3}
!157 = distinct !{!157, !3}
!158 = distinct !{!158, !3}
!159 = !{!160, !6, i64 16}
!160 = !{!"", !10, i64 0, !10, i64 8, !6, i64 16}
!161 = !{!160, !10, i64 0}
!162 = !{!160, !10, i64 8}
!163 = distinct !{!163, !3}
!164 = distinct !{!164, !3}
!165 = !{!166, !6, i64 56}
!166 = !{!"FaSource", !6, i64 0, !6, i64 8, !10, i64 16, !6, i64 24, !6, i64 32, !10, i64 40, !10, i64 48, !6, i64 56, !6, i64 64}
!167 = !{!166, !10, i64 48}
!168 = !{!169, !6, i64 0}
!169 = !{!"", !6, i64 0, !6, i64 8, !10, i64 16}
!170 = !{!169, !6, i64 8}
!171 = distinct !{!171, !3}
!172 = !{!169, !10, i64 16}
!173 = !{!166, !10, i64 40}
!174 = !{i64 0, i64 8, !5, i64 8, i64 8, !5, i64 16, i64 8, !9}
!175 = !{i64 0, i64 8, !5, i64 8, i64 8, !9}
!176 = !{i64 0, i64 8, !9}
!177 = distinct !{!177, !3}
!178 = !{!179, !6, i64 128}
!179 = !{!"FaObserver", !166, i64 0, !10, i64 72, !10, i64 80, !10, i64 88, !10, i64 96, !10, i64 104, !10, i64 112, !10, i64 120, !6, i64 128, !6, i64 136, !6, i64 144, !21, i64 152, !21, i64 156, !21, i64 160, !21, i64 164, !21, i64 168, !21, i64 172, !21, i64 176, !21, i64 180, !6, i64 184, !7, i64 192}
!180 = !{!179, !10, i64 112}
!181 = distinct !{!181, !3}
!182 = !{!179, !6, i64 136}
!183 = !{!179, !10, i64 120}
!184 = !{!166, !6, i64 8}
!185 = !{!166, !6, i64 24}
!186 = !{!166, !10, i64 16}
!187 = distinct !{!187, !3}
!188 = distinct !{!188, !3}
!189 = distinct !{!189, !3}
!190 = distinct !{!190, !3}
!191 = !{!179, !6, i64 0}
!192 = !{!179, !21, i64 152}
!193 = !{!179, !10, i64 88}
!194 = !{!179, !10, i64 96}
!195 = !{!179, !10, i64 104}
!196 = !{!179, !10, i64 80}
!197 = !{!179, !10, i64 72}
!198 = !{!179, !6, i64 184}
!199 = !{!179, !21, i64 156}
!200 = !{!179, !21, i64 160}
!201 = !{!179, !21, i64 172}
!202 = !{!179, !6, i64 8}
!203 = !{!179, !21, i64 176}
!204 = !{!179, !6, i64 144}
!205 = !{!179, !21, i64 168}
!206 = distinct !{!206, !3}
!207 = distinct !{!207, !3}
!208 = distinct !{!208, !3}
!209 = distinct !{!209, !3}
!210 = distinct !{!210, !3}
!211 = !{!179, !21, i64 164}
!212 = distinct !{!212, !3}
!213 = distinct !{!213, !3}
!214 = !{!166, !6, i64 0}
!215 = !{!166, !6, i64 32}
!216 = !{!166, !6, i64 64}
!217 = distinct !{!217, !3}
!218 = !{!219, !6, i64 0}
!219 = !{!"", !6, i64 0, !37, i64 8}
!220 = distinct !{!220, !3}
!221 = distinct !{!221, !3}
!222 = distinct !{!222, !3}
!223 = !{!224, !6, i64 0}
!224 = !{!"", !6, i64 0, !37, i64 8, !37, i64 32}
!225 = !{!226}
!226 = distinct !{!226, !227, !"owned_copy: argument 0"}
!227 = distinct !{!227, !"owned_copy"}
!228 = !{!229, !10, i64 0}
!229 = !{!"", !10, i64 0, !6, i64 8, !6, i64 16, !6, i64 24, !10, i64 32, !6, i64 40, !6, i64 48}
!230 = !{!229, !6, i64 8}
!231 = !{!229, !6, i64 16}
!232 = !{!229, !6, i64 24}
!233 = !{!229, !10, i64 32}
!234 = !{!229, !6, i64 48}
!235 = distinct !{!235, !3}
!236 = !{!229, !6, i64 40}
!237 = distinct !{!237, !3}
!238 = distinct !{!238, !3}
!239 = distinct !{!239, !3}
!240 = distinct !{!240, !3}
!241 = distinct !{!241, !3}
!242 = distinct !{!242, !3}
!243 = distinct !{!243, !3}
!244 = distinct !{!244, !3}
!245 = !{!246, !10, i64 16}
!246 = !{!"", !6, i64 0, !6, i64 8, !10, i64 16, !6, i64 24}
!247 = !{!246, !6, i64 8}
!248 = !{!249, !6, i64 32}
!249 = !{!"", !246, i64 0, !6, i64 32}
!250 = !{!246, !6, i64 0}
!251 = !{!252, !21, i64 28}
!252 = !{!"FaTask", !10, i64 0, !10, i64 8, !10, i64 16, !21, i64 24, !21, i64 28}
!253 = !{!254, !6, i64 64}
!254 = !{!"FaExecutor", !10, i64 0, !10, i64 8, !6, i64 16, !10, i64 24, !6, i64 32, !6, i64 40, !6, i64 48, !10, i64 56, !6, i64 64, !10, i64 72, !10, i64 80, !252, i64 88, !21, i64 120, !21, i64 124, !21, i64 128, !21, i64 132, !6, i64 136, !10, i64 144, !10, i64 152, !7, i64 160, !21, i64 416}
!255 = !{!252, !10, i64 8}
!256 = !{!254, !10, i64 8}
!257 = !{!252, !10, i64 16}
!258 = !{!254, !6, i64 16}
!259 = !{!254, !10, i64 72}
!260 = !{!254, !10, i64 80}
!261 = !{!254, !21, i64 124}
!262 = !{!254, !21, i64 128}
!263 = !{!252, !10, i64 0}
!264 = !{!254, !10, i64 56}
!265 = !{!254, !10, i64 144}
!266 = !{!267, !10, i64 24}
!267 = !{!"", !10, i64 0, !6, i64 8, !10, i64 16, !10, i64 24}
!268 = !{!267, !6, i64 8}
!269 = !{!267, !10, i64 0}
!270 = distinct !{!270, !3}
!271 = !{!254, !6, i64 136}
!272 = !{!273, !10, i64 0}
!273 = !{!"FaWait", !10, i64 0, !10, i64 8, !10, i64 16, !10, i64 24, !10, i64 32, !10, i64 40, !6, i64 48, !6, i64 56, !6, i64 64, !21, i64 72, !21, i64 76}
!274 = !{!273, !10, i64 8}
!275 = !{!273, !6, i64 64}
!276 = !{!254, !6, i64 32}
!277 = !{!254, !6, i64 40}
!278 = !{!254, !10, i64 24}
!279 = distinct !{!279, !3}
!280 = !{!273, !6, i64 48}
!281 = !{!254, !6, i64 48}
!282 = !{!273, !6, i64 56}
!283 = distinct !{!283, !3}
!284 = !{!267, !10, i64 16}
!285 = !{!254, !21, i64 132}
!286 = !{!254, !10, i64 0}
!287 = distinct !{!287, !3}
!288 = !{!254, !21, i64 416}
!289 = distinct !{!289, !3}
!290 = distinct !{!290, !3}
!291 = distinct !{!291, !3}
!292 = distinct !{!292, !3}
!293 = distinct !{!293, !3}
!294 = distinct !{!294, !3}
!295 = !{!273, !21, i64 72}
!296 = !{!273, !21, i64 76}
!297 = !{!273, !10, i64 40}
!298 = !{!273, !10, i64 16}
!299 = !{!273, !10, i64 24}
!300 = !{!254, !10, i64 152}
!301 = !{!302, !6, i64 160}
!302 = !{!"", !246, i64 0, !303, i64 32, !303, i64 96, !6, i64 160, !6, i64 168, !10, i64 176, !6, i64 184, !6, i64 192, !6, i64 200, !21, i64 208, !21, i64 212, !10, i64 216, !10, i64 224, !10, i64 232}
!303 = !{!"", !7, i64 0}
!304 = !{!302, !6, i64 168}
!305 = !{!302, !10, i64 216}
!306 = !{!302, !6, i64 192}
!307 = !{!302, !6, i64 200}
!308 = !{!302, !6, i64 184}
!309 = !{!302, !10, i64 176}
!310 = distinct !{!310, !3}
!311 = !{!302, !21, i64 208}
!312 = !{!302, !21, i64 212}
!313 = !{!273, !10, i64 32}
!314 = !{!302, !10, i64 232}
!315 = distinct !{!315, !3}
!316 = distinct !{!316, !3}
!317 = distinct !{!317, !3}
!318 = !{!319, !21, i64 152}
!319 = !{!"", !303, i64 0, !303, i64 64, !10, i64 128, !6, i64 136, !21, i64 144, !21, i64 148, !21, i64 152}
!320 = distinct !{!320, !3}
!321 = !{!322, !10, i64 0}
!322 = !{!"", !10, i64 0, !10, i64 8, !6, i64 16, !6, i64 24, !6, i64 32, !21, i64 40, !21, i64 44, !21, i64 48, !21, i64 52, !10, i64 56, !6, i64 64, !10, i64 72}
!323 = !{!322, !10, i64 8}
!324 = !{!322, !6, i64 16}
!325 = !{!322, !21, i64 44}
!326 = !{!322, !6, i64 24}
!327 = !{!322, !10, i64 72}
!328 = !{!319, !6, i64 136}
!329 = distinct !{!329, !3}
!330 = !{!322, !10, i64 56}
!331 = distinct !{!331, !3}
!332 = !{!322, !6, i64 64}
!333 = !{!334, !334, i64 0}
!334 = !{!"__int128", !7, i64 0}
!335 = distinct !{!335, !3}
!336 = distinct !{!336, !3}
!337 = distinct !{!337, !3}
!338 = !{!179, !6, i64 24}
!339 = !{!179, !10, i64 16}
!340 = distinct !{!340, !3}
!341 = distinct !{!341, !3}
