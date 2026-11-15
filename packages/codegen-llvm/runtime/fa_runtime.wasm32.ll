source_filename = "fa_runtime.c"

%struct.BareError = type { i64 }
%struct.Range = type { i32, i32 }
%struct.FaString = type { ptr, i32, i32 }
%struct.Boundaries = type { ptr, i32, i32, i32 }
%struct.Replacing = type { ptr, i32, i32, ptr, i32, i32, i32 }
%struct.FaArrayHeader = type { ptr, i64, i64 }
%struct.Splitting = type { ptr, i32, i32, ptr, i32 }
%struct.IndexError = type { i64, i64, i64 }
%struct.FaSegmented = type { i64, i64, ptr, ptr, [48 x ptr], i64 }
%struct.PageHeader = type { i32, i32 }
%struct.FreeSpan = type { ptr, i32 }
%struct.FaArena = type { ptr, ptr, ptr, ptr, ptr }
%struct.FaArenaCleanup = type { ptr, ptr, ptr }
%struct.FaPool = type { i64, ptr, ptr, ptr, i32, i32, i32, i32, i32, i32, i64 }
%struct.PoolChunk = type { ptr, i32, i32, i32, i32, [0 x i64] }
%struct.FaLinks = type { i64, ptr, i64, i64, ptr, i64 }
%struct.FaAddressMap = type { ptr, ptr, i64 }
%struct.FaSource = type { i64, i64, ptr, i64, i64, ptr, ptr, i64, i64 }
%struct.FaChild = type { i64, i64, ptr }
%struct.FaObserver = type { %struct.FaSource, ptr, ptr, ptr, ptr, ptr, ptr, ptr, i64, i64, i64, i32, i32, i32, i32, i32, i32, i32, i32, i64, [2 x ptr] }
%struct.MessageError = type { i64, %struct.FaString }
%struct.ParseError = type { i64, %struct.FaString, %struct.FaString }
%struct.FaTable = type { ptr, i64, i64, i64, ptr, i64, i64, i64, i64, i64 }
%struct.FaTableEntries = type { ptr, i64, i64 }
%struct.FaShared = type { i64, i64, ptr, i64 }
%struct.FaAtomicBlock = type { %struct.FaShared, i64 }
%struct.FaExecutor = type { ptr, ptr, i64, ptr, i64, i64, i64, ptr, i64, i64, i64, ptr, ptr, %struct.FaTask, i32, i32, i32, i32, i64, i64, ptr, ptr, [32 x ptr], i32 }
%struct.FaTask = type { ptr, ptr, ptr, i32, i32 }
%struct.FaJoin = type { ptr, i64, ptr, ptr }
%struct.FaWait = type { ptr, ptr, ptr, ptr, ptr, ptr, i64, i64, i64, i32, i32 }
%struct.FaIo = type { i32, i32, ptr, ptr, ptr }
%struct.FaJob = type { ptr, ptr, ptr, ptr, ptr, i32 }
%struct.FaMutex = type { %struct.FaShared, i32, i32, ptr, [4 x i8], %struct.FaSystemLock, %struct.FaSystemLock, [0 x i8] }
%struct.FaSystemLock = type { [64 x i8] }
%struct.FaChannel = type { %struct.FaShared, %struct.FaSystemLock, %struct.FaSystemLock, i64, i64, ptr, i64, i64, i64, i32, i32, ptr, ptr, ptr, [12 x i8] }

@output_length = internal unnamed_addr global i32 0, align 4
@output = internal global [8192 x i8] zeroinitializer, align 16
@builder_length = internal unnamed_addr global i32 0, align 4
@builder = internal unnamed_addr global ptr null, align 4
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
@fa_compact_characters = hidden local_unnamed_addr global i32 1, align 4
@.str.12 = private unnamed_addr constant [3 x i8] c"SS\00", align 1
@argument_bytes = internal unnamed_addr global ptr null, align 4
@argument_count = internal unnamed_addr global i32 0, align 4
@argument_items = internal unnamed_addr global ptr null, align 4
@free_blocks = internal unnamed_addr global [40 x ptr] zeroinitializer, align 16
@class_next = internal unnamed_addr global [40 x ptr] zeroinitializer, align 16
@class_end = internal unnamed_addr global [40 x ptr] zeroinitializer, align 16
@tree_arena_depth = internal unnamed_addr global i32 0, align 4
@tree_arena = internal unnamed_addr global ptr null, align 4
@builder_capacity = internal unnamed_addr global i32 0, align 4
@fa_live_blocks = hidden global i32 0, align 4
@live_tree_arenas = internal global i32 0, align 4
@.str.13 = private unnamed_addr constant [7 x i8] c"leak: \00", align 1
@.str.14 = private unnamed_addr constant [8 x i8] c" blocks\00", align 1
@argument_sources = internal unnamed_addr global [16 x ptr] zeroinitializer, align 16
@field_source_count = internal unnamed_addr global i64 0, align 8
@field_sources = internal unnamed_addr global ptr null, align 4
@field_source_capacity = internal global i64 0, align 8
@reactive_observer = internal unnamed_addr global ptr null, align 4
@reactive_sequence = internal unnamed_addr global i64 0, align 8
@reactive_flushing = internal unnamed_addr global i1 false, align 4
@reactive_pending_count = internal unnamed_addr global i64 0, align 8
@.str.15 = private unnamed_addr constant [74 x i8] c"the `watch` blocks kept changing what they read: stopped after 100 rounds\00", align 1
@reactive_pending = internal unnamed_addr global ptr null, align 4
@reactive_made_count = internal unnamed_addr global i64 0, align 8
@reactive_made = internal unnamed_addr global ptr null, align 4
@reactive_made_capacity = internal global i64 0, align 8
@reactive_pending_capacity = internal global i64 0, align 8
@.str.16 = private unnamed_addr constant [19 x i8] c"integer overflow: \00", align 1
@.str.17 = private unnamed_addr constant [17 x i8] c" doesn't fit in \00", align 1
@.str.18 = private unnamed_addr constant [44 x i8] c"integer overflow: the value doesn't fit in \00", align 1
@.str.19 = private unnamed_addr constant [19 x i8] c"assertion failed: \00", align 1
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
@current_executor = internal unnamed_addr global ptr null, align 4
@frame_to_free = internal unnamed_addr global ptr null, align 4
@next_frame = internal unnamed_addr global ptr null, align 4
@fa_coop_budget = hidden local_unnamed_addr global i32 0, align 4
@.str.30 = private unnamed_addr constant [49 x i8] c"a blocking call waits for a task that isn't done\00", align 1
@wasm_finish = internal unnamed_addr global ptr null, align 4
@wasm_timer_token = internal unnamed_addr global i32 0, align 4
@.str.31 = private unnamed_addr constant [38 x i8] c"`lock()` on a mutex this thread holds\00", align 1
@.str.32 = private unnamed_addr constant [60 x i8] c"`send` on a full channel that no other thread receives from\00", align 1
@.str.33 = private unnamed_addr constant [60 x i8] c"`receive` on an empty channel that no other thread sends to\00", align 1
@fa_blocking_depth = hidden local_unnamed_addr global i32 0, align 4
@fa_turn_site = hidden local_unnamed_addr global ptr null, align 4
@.str.34 = private unnamed_addr constant [1 x i8] zeroinitializer, align 1
@decimal_u64.pairs = internal unnamed_addr constant [201 x i8] c"00010203040506070809101112131415161718192021222324252627282930313233343536373839404142434445464748495051525354555657585960616263646566676869707172737475767778798081828384858687888990919293949596979899\00", align 16
@out_of_memory.message = internal constant [21 x i8] c"stop: out of memory\0A\00", align 16
@EXTEND = internal unnamed_addr constant [153 x %struct.Range] [%struct.Range { i32 768, i32 879 }, %struct.Range { i32 1155, i32 1161 }, %struct.Range { i32 1425, i32 1469 }, %struct.Range { i32 1471, i32 1471 }, %struct.Range { i32 1473, i32 1474 }, %struct.Range { i32 1476, i32 1477 }, %struct.Range { i32 1479, i32 1479 }, %struct.Range { i32 1552, i32 1562 }, %struct.Range { i32 1611, i32 1631 }, %struct.Range { i32 1648, i32 1648 }, %struct.Range { i32 1750, i32 1756 }, %struct.Range { i32 1759, i32 1764 }, %struct.Range { i32 1767, i32 1768 }, %struct.Range { i32 1770, i32 1773 }, %struct.Range { i32 1809, i32 1809 }, %struct.Range { i32 1840, i32 1866 }, %struct.Range { i32 1958, i32 1968 }, %struct.Range { i32 2027, i32 2035 }, %struct.Range { i32 2070, i32 2093 }, %struct.Range { i32 2137, i32 2139 }, %struct.Range { i32 2200, i32 2207 }, %struct.Range { i32 2250, i32 2273 }, %struct.Range { i32 2275, i32 2307 }, %struct.Range { i32 2362, i32 2364 }, %struct.Range { i32 2366, i32 2383 }, %struct.Range { i32 2385, i32 2391 }, %struct.Range { i32 2402, i32 2403 }, %struct.Range { i32 2433, i32 2435 }, %struct.Range { i32 2492, i32 2492 }, %struct.Range { i32 2494, i32 2509 }, %struct.Range { i32 2519, i32 2519 }, %struct.Range { i32 2530, i32 2531 }, %struct.Range { i32 2561, i32 2563 }, %struct.Range { i32 2620, i32 2641 }, %struct.Range { i32 2672, i32 2673 }, %struct.Range { i32 2677, i32 2677 }, %struct.Range { i32 2689, i32 2691 }, %struct.Range { i32 2748, i32 2748 }, %struct.Range { i32 2750, i32 2765 }, %struct.Range { i32 2786, i32 2787 }, %struct.Range { i32 2817, i32 2819 }, %struct.Range { i32 2876, i32 2876 }, %struct.Range { i32 2878, i32 2903 }, %struct.Range { i32 2914, i32 2915 }, %struct.Range { i32 2946, i32 2946 }, %struct.Range { i32 3006, i32 3021 }, %struct.Range { i32 3031, i32 3031 }, %struct.Range { i32 3072, i32 3076 }, %struct.Range { i32 3132, i32 3132 }, %struct.Range { i32 3134, i32 3158 }, %struct.Range { i32 3170, i32 3171 }, %struct.Range { i32 3201, i32 3203 }, %struct.Range { i32 3260, i32 3260 }, %struct.Range { i32 3262, i32 3286 }, %struct.Range { i32 3298, i32 3299 }, %struct.Range { i32 3328, i32 3331 }, %struct.Range { i32 3387, i32 3388 }, %struct.Range { i32 3390, i32 3405 }, %struct.Range { i32 3415, i32 3415 }, %struct.Range { i32 3426, i32 3427 }, %struct.Range { i32 3457, i32 3459 }, %struct.Range { i32 3530, i32 3551 }, %struct.Range { i32 3570, i32 3571 }, %struct.Range { i32 3633, i32 3633 }, %struct.Range { i32 3635, i32 3642 }, %struct.Range { i32 3655, i32 3662 }, %struct.Range { i32 3761, i32 3761 }, %struct.Range { i32 3763, i32 3772 }, %struct.Range { i32 3784, i32 3790 }, %struct.Range { i32 3864, i32 3865 }, %struct.Range { i32 3893, i32 3893 }, %struct.Range { i32 3895, i32 3895 }, %struct.Range { i32 3897, i32 3897 }, %struct.Range { i32 3902, i32 3903 }, %struct.Range { i32 3953, i32 3972 }, %struct.Range { i32 3974, i32 3975 }, %struct.Range { i32 3981, i32 4028 }, %struct.Range { i32 4038, i32 4038 }, %struct.Range { i32 4139, i32 4158 }, %struct.Range { i32 4182, i32 4185 }, %struct.Range { i32 4190, i32 4192 }, %struct.Range { i32 4194, i32 4196 }, %struct.Range { i32 4199, i32 4205 }, %struct.Range { i32 4209, i32 4212 }, %struct.Range { i32 4226, i32 4237 }, %struct.Range { i32 4239, i32 4239 }, %struct.Range { i32 4250, i32 4253 }, %struct.Range { i32 4957, i32 4959 }, %struct.Range { i32 5906, i32 5909 }, %struct.Range { i32 5938, i32 5940 }, %struct.Range { i32 5970, i32 5971 }, %struct.Range { i32 6002, i32 6003 }, %struct.Range { i32 6068, i32 6099 }, %struct.Range { i32 6109, i32 6109 }, %struct.Range { i32 6155, i32 6157 }, %struct.Range { i32 6159, i32 6159 }, %struct.Range { i32 6277, i32 6278 }, %struct.Range { i32 6313, i32 6313 }, %struct.Range { i32 6432, i32 6459 }, %struct.Range { i32 6679, i32 6683 }, %struct.Range { i32 6741, i32 6783 }, %struct.Range { i32 6832, i32 6911 }, %struct.Range { i32 6912, i32 6916 }, %struct.Range { i32 6964, i32 6980 }, %struct.Range { i32 7019, i32 7027 }, %struct.Range { i32 7040, i32 7042 }, %struct.Range { i32 7073, i32 7085 }, %struct.Range { i32 7142, i32 7155 }, %struct.Range { i32 7204, i32 7223 }, %struct.Range { i32 7376, i32 7378 }, %struct.Range { i32 7380, i32 7400 }, %struct.Range { i32 7405, i32 7405 }, %struct.Range { i32 7412, i32 7412 }, %struct.Range { i32 7415, i32 7417 }, %struct.Range { i32 7616, i32 7679 }, %struct.Range { i32 8204, i32 8204 }, %struct.Range { i32 8400, i32 8432 }, %struct.Range { i32 11503, i32 11505 }, %struct.Range { i32 11647, i32 11647 }, %struct.Range { i32 11744, i32 11775 }, %struct.Range { i32 12330, i32 12335 }, %struct.Range { i32 12441, i32 12442 }, %struct.Range { i32 42607, i32 42610 }, %struct.Range { i32 42612, i32 42621 }, %struct.Range { i32 42654, i32 42655 }, %struct.Range { i32 42736, i32 42737 }, %struct.Range { i32 43010, i32 43010 }, %struct.Range { i32 43014, i32 43014 }, %struct.Range { i32 43019, i32 43019 }, %struct.Range { i32 43043, i32 43047 }, %struct.Range { i32 43052, i32 43052 }, %struct.Range { i32 43136, i32 43137 }, %struct.Range { i32 43188, i32 43205 }, %struct.Range { i32 43232, i32 43249 }, %struct.Range { i32 43263, i32 43263 }, %struct.Range { i32 43302, i32 43309 }, %struct.Range { i32 43335, i32 43347 }, %struct.Range { i32 43392, i32 43395 }, %struct.Range { i32 43443, i32 43456 }, %struct.Range { i32 43561, i32 43574 }, %struct.Range { i32 43587, i32 43587 }, %struct.Range { i32 43596, i32 43597 }, %struct.Range { i32 43755, i32 43759 }, %struct.Range { i32 43765, i32 43766 }, %struct.Range { i32 44003, i32 44010 }, %struct.Range { i32 44012, i32 44013 }, %struct.Range { i32 64286, i32 64286 }, %struct.Range { i32 65024, i32 65039 }, %struct.Range { i32 65056, i32 65071 }, %struct.Range { i32 65438, i32 65439 }, %struct.Range { i32 127995, i32 127999 }, %struct.Range { i32 917536, i32 917631 }, %struct.Range { i32 917760, i32 917999 }], align 16
@PICTOGRAPHIC = internal unnamed_addr constant [22 x %struct.Range] [%struct.Range { i32 169, i32 169 }, %struct.Range { i32 174, i32 174 }, %struct.Range { i32 8252, i32 8252 }, %struct.Range { i32 8265, i32 8265 }, %struct.Range { i32 8482, i32 8482 }, %struct.Range { i32 8505, i32 8505 }, %struct.Range { i32 8596, i32 8618 }, %struct.Range { i32 8986, i32 9215 }, %struct.Range { i32 9410, i32 9410 }, %struct.Range { i32 9642, i32 9726 }, %struct.Range { i32 9728, i32 10175 }, %struct.Range { i32 10548, i32 10549 }, %struct.Range { i32 11013, i32 11093 }, %struct.Range { i32 12336, i32 12336 }, %struct.Range { i32 12349, i32 12349 }, %struct.Range { i32 12951, i32 12951 }, %struct.Range { i32 12953, i32 12953 }, %struct.Range { i32 126976, i32 127231 }, %struct.Range { i32 127245, i32 127405 }, %struct.Range { i32 127489, i32 127994 }, %struct.Range { i32 128000, i32 129791 }, %struct.Range { i32 130048, i32 131069 }], align 16
@ascii_table = internal constant [128 x i8] c"\00\01\02\03\04\05\06\07\08\09\0A\0B\0C\0D\0E\0F\10\11\12\13\14\15\16\17\18\19\1A\1B\1C\1D\1E\1F !\22#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\7F", align 16
@free_spans = internal unnamed_addr global ptr null, align 4
@arena_refill.message = internal constant [21 x i8] c"stop: out of memory\0A\00", align 16
@.str.35 = private unnamed_addr constant [34 x i8] c"a derived value depends on itself\00", align 1
@.str.36 = private unnamed_addr constant [7 x i8] c"stop: \00", align 1
@.str.37 = private unnamed_addr constant [7 x i8] c"\0A  at \00", align 1
@.str.38 = private unnamed_addr constant [17 x i8] c"stop: deadlock: \00", align 1
@.str.39 = private unnamed_addr constant [37 x i8] c"warning: a task kept the thread for \00", align 1
@.str.40 = private unnamed_addr constant [6 x i8] c" ms (\00", align 1
@.str.41 = private unnamed_addr constant [7 x i8] c"a task\00", align 1
@.str.42 = private unnamed_addr constant [51 x i8] c"): await Task.yield() or move the work to a worker\00", align 1
@.str.43 = private unnamed_addr constant [33 x i8] c"stop: deadlock: every task waits\00", align 1
@.str.44 = private unnamed_addr constant [17 x i8] c" (main waits on \00", align 1
@.str.45 = private unnamed_addr constant [2 x i8] c")\00", align 1
@.str.46 = private unnamed_addr constant [19 x i8] c" (a task waits on \00", align 1
@llvm.used = appending global [1 x ptr] [ptr @fa_flush], section "llvm.metadata"

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: readwrite)
define hidden void @fa_copy_bytes(ptr nocapture noundef writeonly %0, ptr nocapture noundef readonly %1, i32 noundef %2) local_unnamed_addr #0 {
  tail call fastcc void @copy_bytes(ptr noundef %0, ptr noundef %1, i32 noundef %2) #53
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: readwrite)
define internal fastcc void @copy_bytes(ptr nocapture noundef writeonly %0, ptr nocapture noundef readonly %1, i32 noundef %2) unnamed_addr #1 {
  %4 = alloca [16 x i8], align 16
  %5 = icmp ult i32 %2, 33
  br i1 %5, label %6, label %43

6:                                                ; preds = %3
  %7 = icmp ugt i32 %2, 15
  br i1 %7, label %8, label %13

8:                                                ; preds = %6
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %4)
  %9 = getelementptr inbounds i8, ptr %1, i32 %2
  %10 = getelementptr inbounds i8, ptr %9, i32 -16
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 16 dereferenceable(16) %4, ptr noundef nonnull align 1 dereferenceable(16) %10, i32 16, i1 false)
  tail call void @llvm.memmove.p0.p0.i32(ptr noundef nonnull align 1 dereferenceable(16) %0, ptr noundef nonnull align 1 dereferenceable(16) %1, i32 16, i1 false)
  %11 = getelementptr inbounds i8, ptr %0, i32 %2
  %12 = getelementptr inbounds i8, ptr %11, i32 -16
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 1 dereferenceable(16) %12, ptr noundef nonnull align 16 dereferenceable(16) %4, i32 16, i1 false)
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %4)
  br label %44

13:                                               ; preds = %6
  %14 = icmp ugt i32 %2, 7
  br i1 %14, label %15, label %22

15:                                               ; preds = %13
  %16 = load i64, ptr %1, align 1
  %17 = getelementptr inbounds i8, ptr %1, i32 %2
  %18 = getelementptr inbounds i8, ptr %17, i32 -8
  %19 = load i64, ptr %18, align 1
  store i64 %16, ptr %0, align 1
  %20 = getelementptr inbounds i8, ptr %0, i32 %2
  %21 = getelementptr inbounds i8, ptr %20, i32 -8
  store i64 %19, ptr %21, align 1
  br label %44

22:                                               ; preds = %13
  %23 = icmp ugt i32 %2, 3
  br i1 %23, label %24, label %31

24:                                               ; preds = %22
  %25 = load i32, ptr %1, align 1
  %26 = getelementptr inbounds i8, ptr %1, i32 %2
  %27 = getelementptr inbounds i8, ptr %26, i32 -4
  %28 = load i32, ptr %27, align 1
  store i32 %25, ptr %0, align 1
  %29 = getelementptr inbounds i8, ptr %0, i32 %2
  %30 = getelementptr inbounds i8, ptr %29, i32 -4
  store i32 %28, ptr %30, align 1
  br label %44

31:                                               ; preds = %22
  %32 = icmp eq i32 %2, 0
  br i1 %32, label %44, label %33

33:                                               ; preds = %31
  %34 = load i8, ptr %1, align 1, !tbaa !1
  %35 = lshr i32 %2, 1
  %36 = getelementptr inbounds i8, ptr %1, i32 %35
  %37 = load i8, ptr %36, align 1, !tbaa !1
  %38 = add nsw i32 %2, -1
  %39 = getelementptr inbounds i8, ptr %1, i32 %38
  %40 = load i8, ptr %39, align 1, !tbaa !1
  store i8 %34, ptr %0, align 1, !tbaa !1
  %41 = getelementptr inbounds i8, ptr %0, i32 %35
  store i8 %37, ptr %41, align 1, !tbaa !1
  %42 = getelementptr inbounds i8, ptr %0, i32 %38
  store i8 %40, ptr %42, align 1, !tbaa !1
  br label %44

43:                                               ; preds = %3
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %0, ptr align 1 %1, i32 %2, i1 false)
  br label %44

44:                                               ; preds = %8, %24, %33, %31, %15, %43
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_flush() #2 {
  %1 = load i32, ptr @output_length, align 4, !tbaa !4
  %2 = icmp eq i32 %1, 0
  br i1 %2, label %4, label %3

3:                                                ; preds = %0
  tail call void @fa_write(i32 noundef 1, ptr noundef nonnull @output, i32 noundef %1) #54
  store i32 0, ptr @output_length, align 4, !tbaa !4
  br label %4

4:                                                ; preds = %0, %3
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_out(i32 noundef %0, ptr noundef %1, i32 noundef %2) local_unnamed_addr #3 {
  %4 = icmp eq i32 %0, 1
  %5 = load i32, ptr @output_length, align 4, !tbaa !4
  br i1 %4, label %10, label %6

6:                                                ; preds = %3
  %7 = icmp eq i32 %5, 0
  br i1 %7, label %9, label %8

8:                                                ; preds = %6
  tail call void @fa_write(i32 noundef 1, ptr noundef nonnull @output, i32 noundef %5) #54
  store i32 0, ptr @output_length, align 4, !tbaa !4
  br label %9

9:                                                ; preds = %6, %8
  tail call void @fa_write(i32 noundef %0, ptr noundef %1, i32 noundef %2) #54
  br label %23

10:                                               ; preds = %3
  %11 = add i32 %5, %2
  %12 = icmp ugt i32 %11, 8192
  br i1 %12, label %13, label %19

13:                                               ; preds = %10
  %14 = icmp eq i32 %5, 0
  br i1 %14, label %16, label %15

15:                                               ; preds = %13
  tail call void @fa_write(i32 noundef 1, ptr noundef nonnull @output, i32 noundef %5) #54
  store i32 0, ptr @output_length, align 4, !tbaa !4
  br label %16

16:                                               ; preds = %13, %15
  %17 = icmp ugt i32 %2, 8192
  br i1 %17, label %18, label %19

18:                                               ; preds = %16
  tail call void @fa_write(i32 noundef 1, ptr noundef %1, i32 noundef %2) #54
  br label %23

19:                                               ; preds = %16, %10
  %20 = phi i32 [ %5, %10 ], [ 0, %16 ]
  %21 = getelementptr inbounds i8, ptr @output, i32 %20
  tail call fastcc void @copy_bytes(ptr noundef nonnull %21, ptr noundef %1, i32 noundef %2) #53
  %22 = add i32 %20, %2
  store i32 %22, ptr @output_length, align 4, !tbaa !4
  br label %23

23:                                               ; preds = %19, %18, %9
  ret void
}

; Function Attrs: optsize
declare void @fa_write(i32 noundef, ptr noundef, i32 noundef) local_unnamed_addr #4

; Function Attrs: noreturn nounwind optsize
define hidden void @fa_terminate(i32 noundef %0) local_unnamed_addr #5 {
  tail call void @fa_flush() #53
  tail call void @fa_exit(i32 noundef %0) #55
  unreachable
}

; Function Attrs: noreturn optsize
declare void @fa_exit(i32 noundef) local_unnamed_addr #6

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, argmem: none, inaccessiblemem: none)
define hidden void @fa_b_reset() local_unnamed_addr #7 {
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_bytes(ptr nocapture noundef readonly %0, i32 noundef %1) local_unnamed_addr #3 {
  tail call fastcc void @reserve(i32 noundef %1) #53
  %3 = load ptr, ptr @builder, align 4, !tbaa !6
  %4 = load i32, ptr @builder_length, align 4, !tbaa !4
  %5 = getelementptr inbounds i8, ptr %3, i32 %4
  tail call fastcc void @copy_bytes(ptr noundef %5, ptr noundef %0, i32 noundef %1) #53
  %6 = add i32 %4, %1
  store i32 %6, ptr @builder_length, align 4, !tbaa !4
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @reserve(i32 noundef %0) unnamed_addr #3 {
  %2 = load i32, ptr @builder_length, align 4, !tbaa !4
  %3 = add i32 %2, %0
  %4 = load i32, ptr @builder_capacity, align 4
  %5 = icmp ugt i32 %3, %4
  br i1 %5, label %6, label %25

6:                                                ; preds = %1
  %7 = icmp ult i32 %4, 256
  %8 = shl i32 %4, 1
  %9 = select i1 %7, i32 256, i32 %8
  br label %10

10:                                               ; preds = %10, %6
  %11 = phi i32 [ %9, %6 ], [ %13, %10 ]
  %12 = icmp ult i32 %11, %3
  %13 = shl i32 %11, 1
  br i1 %12, label %10, label %14, !llvm.loop !8

14:                                               ; preds = %10
  %15 = tail call ptr @fa_alloc(i32 noundef %11, i32 noundef 1) #54
  %16 = load i32, ptr @builder_length, align 4, !tbaa !4
  %17 = icmp eq i32 %16, 0
  %18 = load ptr, ptr @builder, align 4, !tbaa !6
  br i1 %17, label %20, label %19

19:                                               ; preds = %14
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %15, ptr align 1 %18, i32 %16, i1 false)
  br label %20

20:                                               ; preds = %14, %19
  %21 = icmp eq ptr %18, null
  br i1 %21, label %24, label %22

22:                                               ; preds = %20
  %23 = load i32, ptr @builder_capacity, align 4, !tbaa !4
  tail call void @fa_free(ptr noundef nonnull %18, i32 noundef %23, i32 noundef 1) #54
  br label %24

24:                                               ; preds = %22, %20
  store ptr %15, ptr @builder, align 4, !tbaa !6
  store i32 %11, ptr @builder_capacity, align 4, !tbaa !4
  br label %25

25:                                               ; preds = %1, %24
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_line(i32 noundef %0) local_unnamed_addr #3 {
  tail call fastcc void @reserve(i32 noundef 1) #53
  %2 = load ptr, ptr @builder, align 4, !tbaa !6
  %3 = load i32, ptr @builder_length, align 4, !tbaa !4
  %4 = getelementptr inbounds i8, ptr %2, i32 %3
  store i8 10, ptr %4, align 1, !tbaa !1
  %5 = add i32 %3, 1
  store i32 %5, ptr @builder_length, align 4, !tbaa !4
  tail call void @fa_out(i32 noundef %0, ptr noundef %2, i32 noundef %5) #53
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_string_own(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef writeonly %3) local_unnamed_addr #3 {
  %5 = alloca %struct.FaString, align 4
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %5) #56
  %6 = and i32 %2, -2147483648
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %5, ptr noundef %0, i32 noundef %1, i32 noundef %6) #53
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %3, ptr noundef nonnull align 4 dereferenceable(12) %5, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %5) #56
  ret void
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.start.p0(i64 immarg, ptr nocapture) #8

; Function Attrs: nounwind optsize
define internal fastcc void @owned_copy_with(ptr dead_on_unwind noalias nocapture writable writeonly sret(%struct.FaString) align 4 %0, ptr nocapture noundef readonly %1, i32 noundef %2, i32 noundef %3) unnamed_addr #3 {
  %5 = icmp ugt i32 %2, 1073741822
  br i1 %5, label %6, label %7

6:                                                ; preds = %4
  tail call fastcc void @out_of_memory() #57
  unreachable

7:                                                ; preds = %4
  switch i32 %2, label %21 [
    i32 0, label %8
    i32 1, label %11
  ]

8:                                                ; preds = %7
  store ptr @.str.34, ptr %0, align 4, !tbaa.struct !10
  %9 = getelementptr inbounds i8, ptr %0, i32 4
  store i32 0, ptr %9, align 4, !tbaa.struct !11
  %10 = getelementptr inbounds i8, ptr %0, i32 8
  store i32 -2147483648, ptr %10, align 4, !tbaa.struct !12
  br label %44

11:                                               ; preds = %7
  %12 = load i8, ptr %1, align 1, !tbaa !1
  %13 = icmp sgt i8 %12, -1
  br i1 %13, label %14, label %23

14:                                               ; preds = %11
  %15 = zext nneg i8 %12 to i32
  %16 = getelementptr inbounds i8, ptr @ascii_table, i32 %15
  store ptr %16, ptr %0, align 4, !tbaa !13, !alias.scope !15
  %17 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 1
  store i32 1, ptr %17, align 4, !tbaa !18, !alias.scope !15
  %18 = icmp eq i8 %12, 13
  %19 = select i1 %18, i32 0, i32 -2147483648
  %20 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 2
  store i32 %19, ptr %20, align 4, !tbaa !19, !alias.scope !15
  br label %44

21:                                               ; preds = %7
  %22 = icmp ult i32 %2, 12
  br i1 %22, label %23, label %34

23:                                               ; preds = %11, %21
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %0, i8 0, i32 12, i1 false)
  tail call fastcc void @copy_bytes(ptr noundef %0, ptr noundef %1, i32 noundef %2) #53
  %24 = icmp eq i32 %3, 0
  br i1 %24, label %25, label %29

25:                                               ; preds = %23
  %26 = tail call fastcc i32 @simple_flag(ptr noundef %1, i32 noundef %2) #53, !range !20
  %27 = icmp eq i32 %26, 0
  %28 = select i1 %27, i32 64, i32 192
  br label %29

29:                                               ; preds = %23, %25
  %30 = phi i32 [ %28, %25 ], [ 192, %23 ]
  %31 = or i32 %30, %2
  %32 = trunc i32 %31 to i8
  %33 = getelementptr inbounds i8, ptr %0, i32 11
  store i8 %32, ptr %33, align 1, !tbaa !1
  br label %44

34:                                               ; preds = %21
  %35 = tail call ptr @fa_alloc(i32 noundef %2, i32 noundef 1) #54
  tail call fastcc void @copy_bytes(ptr noundef %35, ptr noundef %1, i32 noundef %2) #53
  %36 = icmp eq i32 %3, 0
  br i1 %36, label %37, label %39

37:                                               ; preds = %34
  %38 = tail call fastcc i32 @simple_flag(ptr noundef %35, i32 noundef %2) #53, !range !20
  br label %39

39:                                               ; preds = %34, %37
  %40 = phi i32 [ %38, %37 ], [ %3, %34 ]
  %41 = or i32 %40, %2
  store ptr %35, ptr %0, align 4, !tbaa.struct !10
  %42 = getelementptr inbounds i8, ptr %0, i32 4
  store i32 %2, ptr %42, align 4, !tbaa.struct !11
  %43 = getelementptr inbounds i8, ptr %0, i32 8
  store i32 %41, ptr %43, align 4, !tbaa.struct !12
  br label %44

44:                                               ; preds = %39, %29, %14, %8
  ret void
}

; Function Attrs: mustprogress nocallback nofree nounwind willreturn memory(argmem: readwrite)
declare void @llvm.memcpy.p0.p0.i32(ptr noalias nocapture writeonly, ptr noalias nocapture readonly, i32, i1 immarg) #9

; Function Attrs: mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.end.p0(i64 immarg, ptr nocapture) #8

; Function Attrs: nounwind optsize
define hidden void @fa_b_take(ptr nocapture noundef writeonly %0) local_unnamed_addr #3 {
  %2 = alloca %struct.FaString, align 4
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %2) #56
  %3 = load ptr, ptr @builder, align 4, !tbaa !6
  %4 = load i32, ptr @builder_length, align 4, !tbaa !4
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %2, ptr noundef %3, i32 noundef %4, i32 noundef 0) #53
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %0, ptr noundef nonnull align 4 dereferenceable(12) %2, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %2) #56
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: write)
define hidden i32 @fa_decimal_i64(i64 noundef %0, ptr nocapture noundef writeonly %1) local_unnamed_addr #10 {
  %3 = tail call i64 @llvm.abs.i64(i64 %0, i1 false)
  %4 = icmp ugt i64 %3, 4294967295
  br i1 %4, label %5, label %19

5:                                                ; preds = %2, %5
  %6 = phi i64 [ %9, %5 ], [ %3, %2 ]
  %7 = phi i32 [ %12, %5 ], [ 24, %2 ]
  %8 = freeze i64 %6
  %9 = udiv i64 %8, 100
  %10 = mul i64 %9, 100
  %11 = sub i64 %8, %10
  %12 = add i32 %7, -2
  %13 = getelementptr inbounds i8, ptr %1, i32 %12
  %14 = trunc i64 %11 to i32
  %15 = shl nuw nsw i32 %14, 1
  %16 = getelementptr inbounds i8, ptr @decimal_u64.pairs, i32 %15
  %17 = load i16, ptr %16, align 2
  store i16 %17, ptr %13, align 1
  %18 = icmp ugt i64 %6, 429496729599
  br i1 %18, label %5, label %19, !llvm.loop !21

19:                                               ; preds = %5, %2
  %20 = phi i32 [ 24, %2 ], [ %12, %5 ]
  %21 = phi i64 [ %3, %2 ], [ %9, %5 ]
  %22 = trunc i64 %21 to i32
  %23 = icmp ugt i32 %22, 9999
  br i1 %23, label %24, label %47

24:                                               ; preds = %19, %24
  %25 = phi i32 [ %31, %24 ], [ %20, %19 ]
  %26 = phi i32 [ %28, %24 ], [ %22, %19 ]
  %27 = freeze i32 %26
  %28 = udiv i32 %27, 10000
  %29 = mul i32 %28, 10000
  %30 = sub i32 %27, %29
  %31 = add i32 %25, -4
  %32 = getelementptr inbounds i8, ptr %1, i32 %31
  %33 = trunc i32 %30 to i16
  %34 = udiv i16 %33, 100
  %35 = shl nuw nsw i16 %34, 1
  %36 = zext nneg i16 %35 to i32
  %37 = getelementptr inbounds i8, ptr @decimal_u64.pairs, i32 %36
  %38 = load i16, ptr %37, align 2
  store i16 %38, ptr %32, align 1
  %39 = getelementptr inbounds i8, ptr %32, i32 2
  %40 = mul i16 %34, 100
  %41 = sub i16 %33, %40
  %42 = shl nuw nsw i16 %41, 1
  %43 = zext nneg i16 %42 to i32
  %44 = getelementptr inbounds i8, ptr @decimal_u64.pairs, i32 %43
  %45 = load i16, ptr %44, align 2
  store i16 %45, ptr %39, align 1
  %46 = icmp ugt i32 %26, 99999999
  br i1 %46, label %24, label %47, !llvm.loop !22

47:                                               ; preds = %24, %19
  %48 = phi i32 [ %22, %19 ], [ %28, %24 ]
  %49 = phi i32 [ %20, %19 ], [ %31, %24 ]
  %50 = icmp ugt i32 %48, 99
  br i1 %50, label %51, label %63

51:                                               ; preds = %47
  %52 = trunc i32 %48 to i16
  %53 = udiv i16 %52, 100
  %54 = mul i16 %53, 100
  %55 = sub i16 %52, %54
  %56 = zext nneg i16 %53 to i32
  %57 = add i32 %49, -2
  %58 = getelementptr inbounds i8, ptr %1, i32 %57
  %59 = shl nuw nsw i16 %55, 1
  %60 = zext nneg i16 %59 to i32
  %61 = getelementptr inbounds i8, ptr @decimal_u64.pairs, i32 %60
  %62 = load i16, ptr %61, align 2
  store i16 %62, ptr %58, align 1
  br label %63

63:                                               ; preds = %51, %47
  %64 = phi i32 [ %56, %51 ], [ %48, %47 ]
  %65 = phi i32 [ %57, %51 ], [ %49, %47 ]
  %66 = icmp ugt i32 %64, 9
  br i1 %66, label %67, label %73

67:                                               ; preds = %63
  %68 = add i32 %65, -2
  %69 = getelementptr inbounds i8, ptr %1, i32 %68
  %70 = shl nuw nsw i32 %64, 1
  %71 = getelementptr inbounds i8, ptr @decimal_u64.pairs, i32 %70
  %72 = load i16, ptr %71, align 2
  store i16 %72, ptr %69, align 1
  br label %78

73:                                               ; preds = %63
  %74 = trunc i32 %64 to i8
  %75 = or disjoint i8 %74, 48
  %76 = add i32 %65, -1
  %77 = getelementptr inbounds i8, ptr %1, i32 %76
  store i8 %75, ptr %77, align 1, !tbaa !1
  br label %78

78:                                               ; preds = %73, %67
  %79 = phi i32 [ %68, %67 ], [ %76, %73 ]
  %80 = icmp sgt i64 %0, -1
  br i1 %80, label %84, label %81

81:                                               ; preds = %78
  %82 = add i32 %79, -1
  %83 = getelementptr inbounds i8, ptr %1, i32 %82
  store i8 45, ptr %83, align 1, !tbaa !1
  br label %84

84:                                               ; preds = %78, %81
  %85 = phi i32 [ %82, %81 ], [ %79, %78 ]
  ret i32 %85
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: write)
define hidden i32 @fa_decimal_u64(i64 noundef %0, ptr nocapture noundef writeonly %1) local_unnamed_addr #10 {
  %3 = icmp ugt i64 %0, 4294967295
  br i1 %3, label %4, label %18

4:                                                ; preds = %2, %4
  %5 = phi i64 [ %8, %4 ], [ %0, %2 ]
  %6 = phi i32 [ %11, %4 ], [ 24, %2 ]
  %7 = freeze i64 %5
  %8 = udiv i64 %7, 100
  %9 = mul i64 %8, 100
  %10 = sub i64 %7, %9
  %11 = add i32 %6, -2
  %12 = getelementptr inbounds i8, ptr %1, i32 %11
  %13 = trunc i64 %10 to i32
  %14 = shl nuw nsw i32 %13, 1
  %15 = getelementptr inbounds i8, ptr @decimal_u64.pairs, i32 %14
  %16 = load i16, ptr %15, align 2
  store i16 %16, ptr %12, align 1
  %17 = icmp ugt i64 %5, 429496729599
  br i1 %17, label %4, label %18, !llvm.loop !21

18:                                               ; preds = %4, %2
  %19 = phi i32 [ 24, %2 ], [ %11, %4 ]
  %20 = phi i64 [ %0, %2 ], [ %8, %4 ]
  %21 = trunc i64 %20 to i32
  %22 = icmp ugt i32 %21, 9999
  br i1 %22, label %23, label %46

23:                                               ; preds = %18, %23
  %24 = phi i32 [ %30, %23 ], [ %19, %18 ]
  %25 = phi i32 [ %27, %23 ], [ %21, %18 ]
  %26 = freeze i32 %25
  %27 = udiv i32 %26, 10000
  %28 = mul i32 %27, 10000
  %29 = sub i32 %26, %28
  %30 = add i32 %24, -4
  %31 = getelementptr inbounds i8, ptr %1, i32 %30
  %32 = trunc i32 %29 to i16
  %33 = udiv i16 %32, 100
  %34 = shl nuw nsw i16 %33, 1
  %35 = zext nneg i16 %34 to i32
  %36 = getelementptr inbounds i8, ptr @decimal_u64.pairs, i32 %35
  %37 = load i16, ptr %36, align 2
  store i16 %37, ptr %31, align 1
  %38 = getelementptr inbounds i8, ptr %31, i32 2
  %39 = mul i16 %33, 100
  %40 = sub i16 %32, %39
  %41 = shl nuw nsw i16 %40, 1
  %42 = zext nneg i16 %41 to i32
  %43 = getelementptr inbounds i8, ptr @decimal_u64.pairs, i32 %42
  %44 = load i16, ptr %43, align 2
  store i16 %44, ptr %38, align 1
  %45 = icmp ugt i32 %25, 99999999
  br i1 %45, label %23, label %46, !llvm.loop !22

46:                                               ; preds = %23, %18
  %47 = phi i32 [ %21, %18 ], [ %27, %23 ]
  %48 = phi i32 [ %19, %18 ], [ %30, %23 ]
  %49 = icmp ugt i32 %47, 99
  br i1 %49, label %50, label %62

50:                                               ; preds = %46
  %51 = trunc i32 %47 to i16
  %52 = udiv i16 %51, 100
  %53 = mul i16 %52, 100
  %54 = sub i16 %51, %53
  %55 = zext nneg i16 %52 to i32
  %56 = add i32 %48, -2
  %57 = getelementptr inbounds i8, ptr %1, i32 %56
  %58 = shl nuw nsw i16 %54, 1
  %59 = zext nneg i16 %58 to i32
  %60 = getelementptr inbounds i8, ptr @decimal_u64.pairs, i32 %59
  %61 = load i16, ptr %60, align 2
  store i16 %61, ptr %57, align 1
  br label %62

62:                                               ; preds = %50, %46
  %63 = phi i32 [ %55, %50 ], [ %47, %46 ]
  %64 = phi i32 [ %56, %50 ], [ %48, %46 ]
  %65 = icmp ugt i32 %63, 9
  br i1 %65, label %66, label %72

66:                                               ; preds = %62
  %67 = add i32 %64, -2
  %68 = getelementptr inbounds i8, ptr %1, i32 %67
  %69 = shl nuw nsw i32 %63, 1
  %70 = getelementptr inbounds i8, ptr @decimal_u64.pairs, i32 %69
  %71 = load i16, ptr %70, align 2
  store i16 %71, ptr %68, align 1
  br label %77

72:                                               ; preds = %62
  %73 = trunc i32 %63 to i8
  %74 = or disjoint i8 %73, 48
  %75 = add i32 %64, -1
  %76 = getelementptr inbounds i8, ptr %1, i32 %75
  store i8 %74, ptr %76, align 1, !tbaa !1
  br label %77

77:                                               ; preds = %66, %72
  %78 = phi i32 [ %67, %66 ], [ %75, %72 ]
  ret i32 %78
}

; Function Attrs: nounwind optsize
define hidden void @fa_string_i64(i64 noundef %0, ptr nocapture noundef writeonly %1) local_unnamed_addr #3 {
  %3 = tail call i64 @llvm.abs.i64(i64 %0, i1 false)
  %4 = lshr i64 %0, 63
  %5 = trunc i64 %4 to i32
  tail call fastcc void @string_u64(i64 noundef %3, i32 noundef %5, ptr noundef %1) #53
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @string_u64(i64 noundef %0, i32 noundef %1, ptr nocapture noundef writeonly %2) unnamed_addr #3 {
  %4 = alloca [24 x i8], align 16
  %5 = alloca %struct.FaString, align 4
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %4) #56
  %6 = icmp ugt i64 %0, 4294967295
  br i1 %6, label %7, label %21

7:                                                ; preds = %3, %7
  %8 = phi i64 [ %11, %7 ], [ %0, %3 ]
  %9 = phi i32 [ %14, %7 ], [ 24, %3 ]
  %10 = freeze i64 %8
  %11 = udiv i64 %10, 100
  %12 = mul i64 %11, 100
  %13 = sub i64 %10, %12
  %14 = add i32 %9, -2
  %15 = getelementptr inbounds i8, ptr %4, i32 %14
  %16 = trunc i64 %13 to i32
  %17 = shl nuw nsw i32 %16, 1
  %18 = getelementptr inbounds i8, ptr @decimal_u64.pairs, i32 %17
  %19 = load i16, ptr %18, align 2
  store i16 %19, ptr %15, align 2
  %20 = icmp ugt i64 %8, 429496729599
  br i1 %20, label %7, label %21, !llvm.loop !21

21:                                               ; preds = %7, %3
  %22 = phi i32 [ 24, %3 ], [ %14, %7 ]
  %23 = phi i64 [ %0, %3 ], [ %11, %7 ]
  %24 = trunc i64 %23 to i32
  %25 = icmp ugt i32 %24, 9999
  br i1 %25, label %26, label %49

26:                                               ; preds = %21, %26
  %27 = phi i32 [ %33, %26 ], [ %22, %21 ]
  %28 = phi i32 [ %30, %26 ], [ %24, %21 ]
  %29 = freeze i32 %28
  %30 = udiv i32 %29, 10000
  %31 = mul i32 %30, 10000
  %32 = sub i32 %29, %31
  %33 = add i32 %27, -4
  %34 = getelementptr inbounds i8, ptr %4, i32 %33
  %35 = trunc i32 %32 to i16
  %36 = udiv i16 %35, 100
  %37 = shl nuw nsw i16 %36, 1
  %38 = zext nneg i16 %37 to i32
  %39 = getelementptr inbounds i8, ptr @decimal_u64.pairs, i32 %38
  %40 = load i16, ptr %39, align 2
  store i16 %40, ptr %34, align 1
  %41 = getelementptr inbounds i8, ptr %34, i32 2
  %42 = mul i16 %36, 100
  %43 = sub i16 %35, %42
  %44 = shl nuw nsw i16 %43, 1
  %45 = zext nneg i16 %44 to i32
  %46 = getelementptr inbounds i8, ptr @decimal_u64.pairs, i32 %45
  %47 = load i16, ptr %46, align 2
  store i16 %47, ptr %41, align 1
  %48 = icmp ugt i32 %28, 99999999
  br i1 %48, label %26, label %49, !llvm.loop !22

49:                                               ; preds = %26, %21
  %50 = phi i32 [ %24, %21 ], [ %30, %26 ]
  %51 = phi i32 [ %22, %21 ], [ %33, %26 ]
  %52 = icmp ugt i32 %50, 99
  br i1 %52, label %53, label %65

53:                                               ; preds = %49
  %54 = trunc i32 %50 to i16
  %55 = udiv i16 %54, 100
  %56 = mul i16 %55, 100
  %57 = sub i16 %54, %56
  %58 = zext nneg i16 %55 to i32
  %59 = add i32 %51, -2
  %60 = getelementptr inbounds i8, ptr %4, i32 %59
  %61 = shl nuw nsw i16 %57, 1
  %62 = zext nneg i16 %61 to i32
  %63 = getelementptr inbounds i8, ptr @decimal_u64.pairs, i32 %62
  %64 = load i16, ptr %63, align 2
  store i16 %64, ptr %60, align 1
  br label %65

65:                                               ; preds = %53, %49
  %66 = phi i32 [ %58, %53 ], [ %50, %49 ]
  %67 = phi i32 [ %59, %53 ], [ %51, %49 ]
  %68 = icmp ugt i32 %66, 9
  br i1 %68, label %69, label %75

69:                                               ; preds = %65
  %70 = add i32 %67, -2
  %71 = getelementptr inbounds i8, ptr %4, i32 %70
  %72 = shl nuw nsw i32 %66, 1
  %73 = getelementptr inbounds i8, ptr @decimal_u64.pairs, i32 %72
  %74 = load i16, ptr %73, align 2
  store i16 %74, ptr %71, align 1
  br label %80

75:                                               ; preds = %65
  %76 = trunc i32 %66 to i8
  %77 = or disjoint i8 %76, 48
  %78 = add i32 %67, -1
  %79 = getelementptr inbounds i8, ptr %4, i32 %78
  store i8 %77, ptr %79, align 1, !tbaa !1
  br label %80

80:                                               ; preds = %75, %69
  %81 = phi i32 [ %70, %69 ], [ %78, %75 ]
  %82 = icmp eq i32 %1, 0
  br i1 %82, label %86, label %83

83:                                               ; preds = %80
  %84 = add i32 %81, -1
  %85 = getelementptr inbounds i8, ptr %4, i32 %84
  store i8 45, ptr %85, align 1, !tbaa !1
  br label %86

86:                                               ; preds = %80, %83
  %87 = phi i32 [ %84, %83 ], [ %81, %80 ]
  %88 = sub i32 24, %87
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %5) #56
  %89 = getelementptr inbounds i8, ptr %4, i32 %87
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %5, ptr noundef nonnull %89, i32 noundef %88, i32 noundef -2147483648) #53
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %2, ptr noundef nonnull align 4 dereferenceable(12) %5, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %5) #56
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %4) #56
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_string_u64(i64 noundef %0, ptr nocapture noundef writeonly %1) local_unnamed_addr #3 {
  tail call fastcc void @string_u64(i64 noundef %0, i32 noundef 0, ptr noundef %1) #53
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_i64(i64 noundef %0) local_unnamed_addr #3 {
  %2 = tail call i64 @llvm.abs.i64(i64 %0, i1 false)
  %3 = lshr i64 %0, 63
  %4 = trunc i64 %3 to i32
  tail call fastcc void @append_u64(i64 noundef %2, i32 noundef %4) #53
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @append_u64(i64 noundef %0, i32 noundef %1) unnamed_addr #3 {
  %3 = alloca [24 x i8], align 16
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %3) #56
  %4 = icmp ugt i64 %0, 4294967295
  br i1 %4, label %5, label %19

5:                                                ; preds = %2, %5
  %6 = phi i64 [ %9, %5 ], [ %0, %2 ]
  %7 = phi i32 [ %12, %5 ], [ 24, %2 ]
  %8 = freeze i64 %6
  %9 = udiv i64 %8, 100
  %10 = mul i64 %9, 100
  %11 = sub i64 %8, %10
  %12 = add i32 %7, -2
  %13 = getelementptr inbounds i8, ptr %3, i32 %12
  %14 = trunc i64 %11 to i32
  %15 = shl nuw nsw i32 %14, 1
  %16 = getelementptr inbounds i8, ptr @decimal_u64.pairs, i32 %15
  %17 = load i16, ptr %16, align 2
  store i16 %17, ptr %13, align 2
  %18 = icmp ugt i64 %6, 429496729599
  br i1 %18, label %5, label %19, !llvm.loop !21

19:                                               ; preds = %5, %2
  %20 = phi i32 [ 24, %2 ], [ %12, %5 ]
  %21 = phi i64 [ %0, %2 ], [ %9, %5 ]
  %22 = trunc i64 %21 to i32
  %23 = icmp ugt i32 %22, 9999
  br i1 %23, label %24, label %47

24:                                               ; preds = %19, %24
  %25 = phi i32 [ %31, %24 ], [ %20, %19 ]
  %26 = phi i32 [ %28, %24 ], [ %22, %19 ]
  %27 = freeze i32 %26
  %28 = udiv i32 %27, 10000
  %29 = mul i32 %28, 10000
  %30 = sub i32 %27, %29
  %31 = add i32 %25, -4
  %32 = getelementptr inbounds i8, ptr %3, i32 %31
  %33 = trunc i32 %30 to i16
  %34 = udiv i16 %33, 100
  %35 = shl nuw nsw i16 %34, 1
  %36 = zext nneg i16 %35 to i32
  %37 = getelementptr inbounds i8, ptr @decimal_u64.pairs, i32 %36
  %38 = load i16, ptr %37, align 2
  store i16 %38, ptr %32, align 1
  %39 = getelementptr inbounds i8, ptr %32, i32 2
  %40 = mul i16 %34, 100
  %41 = sub i16 %33, %40
  %42 = shl nuw nsw i16 %41, 1
  %43 = zext nneg i16 %42 to i32
  %44 = getelementptr inbounds i8, ptr @decimal_u64.pairs, i32 %43
  %45 = load i16, ptr %44, align 2
  store i16 %45, ptr %39, align 1
  %46 = icmp ugt i32 %26, 99999999
  br i1 %46, label %24, label %47, !llvm.loop !22

47:                                               ; preds = %24, %19
  %48 = phi i32 [ %22, %19 ], [ %28, %24 ]
  %49 = phi i32 [ %20, %19 ], [ %31, %24 ]
  %50 = icmp ugt i32 %48, 99
  br i1 %50, label %51, label %63

51:                                               ; preds = %47
  %52 = trunc i32 %48 to i16
  %53 = udiv i16 %52, 100
  %54 = mul i16 %53, 100
  %55 = sub i16 %52, %54
  %56 = zext nneg i16 %53 to i32
  %57 = add i32 %49, -2
  %58 = getelementptr inbounds i8, ptr %3, i32 %57
  %59 = shl nuw nsw i16 %55, 1
  %60 = zext nneg i16 %59 to i32
  %61 = getelementptr inbounds i8, ptr @decimal_u64.pairs, i32 %60
  %62 = load i16, ptr %61, align 2
  store i16 %62, ptr %58, align 1
  br label %63

63:                                               ; preds = %51, %47
  %64 = phi i32 [ %56, %51 ], [ %48, %47 ]
  %65 = phi i32 [ %57, %51 ], [ %49, %47 ]
  %66 = icmp ugt i32 %64, 9
  br i1 %66, label %67, label %73

67:                                               ; preds = %63
  %68 = add i32 %65, -2
  %69 = getelementptr inbounds i8, ptr %3, i32 %68
  %70 = shl nuw nsw i32 %64, 1
  %71 = getelementptr inbounds i8, ptr @decimal_u64.pairs, i32 %70
  %72 = load i16, ptr %71, align 2
  store i16 %72, ptr %69, align 1
  br label %78

73:                                               ; preds = %63
  %74 = trunc i32 %64 to i8
  %75 = or disjoint i8 %74, 48
  %76 = add i32 %65, -1
  %77 = getelementptr inbounds i8, ptr %3, i32 %76
  store i8 %75, ptr %77, align 1, !tbaa !1
  br label %78

78:                                               ; preds = %73, %67
  %79 = phi i32 [ %68, %67 ], [ %76, %73 ]
  %80 = icmp eq i32 %1, 0
  br i1 %80, label %84, label %81

81:                                               ; preds = %78
  %82 = add i32 %79, -1
  %83 = getelementptr inbounds i8, ptr %3, i32 %82
  store i8 45, ptr %83, align 1, !tbaa !1
  br label %84

84:                                               ; preds = %78, %81
  %85 = phi i32 [ %82, %81 ], [ %79, %78 ]
  %86 = getelementptr inbounds i8, ptr %3, i32 %85
  %87 = sub i32 24, %85
  tail call fastcc void @reserve(i32 noundef %87) #53
  %88 = load ptr, ptr @builder, align 4, !tbaa !6
  %89 = load i32, ptr @builder_length, align 4, !tbaa !4
  %90 = getelementptr inbounds i8, ptr %88, i32 %89
  call fastcc void @copy_bytes(ptr noundef %90, ptr noundef nonnull %86, i32 noundef %87) #53
  %91 = add i32 %89, %87
  store i32 %91, ptr @builder_length, align 4, !tbaa !4
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %3) #56
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_u64(i64 noundef %0) local_unnamed_addr #3 {
  tail call fastcc void @append_u64(i64 noundef %0, i32 noundef 0) #53
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_i128(i64 noundef %0, i64 noundef %1) local_unnamed_addr #3 {
  %3 = zext i64 %1 to i128
  %4 = shl nuw i128 %3, 64
  %5 = zext i64 %0 to i128
  %6 = or disjoint i128 %4, %5
  %7 = icmp slt i64 %1, 0
  %8 = sub i128 0, %6
  %9 = select i1 %7, i128 %8, i128 %6
  %10 = lshr i64 %1, 63
  %11 = trunc i64 %10 to i32
  tail call fastcc void @append_u128(i128 noundef %9, i32 noundef %11) #53
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @append_u128(i128 noundef %0, i32 noundef %1) unnamed_addr #3 {
  %3 = alloca [4 x i32], align 16
  %4 = alloca [48 x i8], align 16
  %5 = icmp ult i128 %0, 18446744073709551616
  br i1 %5, label %6, label %8

6:                                                ; preds = %2
  %7 = trunc i128 %0 to i64
  tail call fastcc void @append_u64(i64 noundef %7, i32 noundef %1) #53
  br label %80

8:                                                ; preds = %2
  %9 = lshr i128 %0, 64
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %3) #56
  %10 = lshr i128 %0, 96
  %11 = trunc i128 %10 to i32
  store i32 %11, ptr %3, align 16, !tbaa !23
  %12 = getelementptr inbounds i32, ptr %3, i32 1
  %13 = trunc i128 %9 to i32
  store i32 %13, ptr %12, align 4, !tbaa !23
  %14 = getelementptr inbounds i32, ptr %3, i32 2
  %15 = lshr i128 %0, 32
  %16 = trunc i128 %15 to i32
  store i32 %16, ptr %14, align 8, !tbaa !23
  %17 = getelementptr inbounds i32, ptr %3, i32 3
  %18 = trunc i128 %0 to i32
  store i32 %18, ptr %17, align 4, !tbaa !23
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %4) #56
  br label %19

19:                                               ; preds = %48, %8
  %20 = phi i32 [ 48, %8 ], [ %53, %48 ]
  br label %21

21:                                               ; preds = %21, %19
  %22 = phi i64 [ 0, %19 ], [ %33, %21 ]
  %23 = phi i32 [ 0, %19 ], [ %34, %21 ]
  %24 = shl nuw nsw i64 %22, 32
  %25 = getelementptr inbounds i32, ptr %3, i32 %23
  %26 = load i32, ptr %25, align 4, !tbaa !23
  %27 = zext i32 %26 to i64
  %28 = or disjoint i64 %24, %27
  %29 = freeze i64 %28
  %30 = udiv i64 %29, 1000000000
  %31 = trunc i64 %30 to i32
  store i32 %31, ptr %25, align 4, !tbaa !23
  %32 = mul i64 %30, 1000000000
  %33 = sub i64 %29, %32
  %34 = add nuw nsw i32 %23, 1
  %35 = icmp eq i32 %34, 4
  br i1 %35, label %36, label %21, !llvm.loop !25

36:                                               ; preds = %21
  %37 = trunc i64 %33 to i32
  %38 = load i32, ptr %3, align 16, !tbaa !23
  %39 = load i32, ptr %12, align 4, !tbaa !23
  %40 = or i32 %39, %38
  %41 = load i32, ptr %14, align 8, !tbaa !23
  %42 = or i32 %40, %41
  %43 = load i32, ptr %17, align 4, !tbaa !23
  %44 = or i32 %42, %43
  %45 = icmp ne i32 %44, 0
  %46 = icmp ne i32 %37, 0
  %47 = select i1 %45, i1 true, i1 %46
  br i1 %47, label %49, label %66

48:                                               ; preds = %49
  br i1 %45, label %19, label %66

49:                                               ; preds = %36, %49
  %50 = phi i32 [ %61, %49 ], [ 0, %36 ]
  %51 = phi i32 [ %55, %49 ], [ %37, %36 ]
  %52 = phi i32 [ %53, %49 ], [ %20, %36 ]
  %53 = add i32 %52, -1
  %54 = freeze i32 %51
  %55 = udiv i32 %54, 10
  %56 = mul i32 %55, 10
  %57 = sub i32 %54, %56
  %58 = trunc i32 %57 to i8
  %59 = or disjoint i8 %58, 48
  %60 = getelementptr inbounds [48 x i8], ptr %4, i32 0, i32 %53
  store i8 %59, ptr %60, align 1, !tbaa !1
  %61 = add nuw nsw i32 %50, 1
  %62 = icmp ult i32 %50, 8
  %63 = icmp ugt i32 %51, 9
  %64 = select i1 %45, i1 true, i1 %63
  %65 = select i1 %62, i1 %64, i1 false
  br i1 %65, label %49, label %48, !llvm.loop !26

66:                                               ; preds = %36, %48
  %67 = phi i32 [ %53, %48 ], [ %20, %36 ]
  %68 = icmp eq i32 %1, 0
  br i1 %68, label %72, label %69

69:                                               ; preds = %66
  %70 = add i32 %67, -1
  %71 = getelementptr inbounds [48 x i8], ptr %4, i32 0, i32 %70
  store i8 45, ptr %71, align 1, !tbaa !1
  br label %72

72:                                               ; preds = %69, %66
  %73 = phi i32 [ %70, %69 ], [ %67, %66 ]
  %74 = getelementptr inbounds i8, ptr %4, i32 %73
  %75 = sub i32 48, %73
  tail call fastcc void @reserve(i32 noundef %75) #53
  %76 = load ptr, ptr @builder, align 4, !tbaa !6
  %77 = load i32, ptr @builder_length, align 4, !tbaa !4
  %78 = getelementptr inbounds i8, ptr %76, i32 %77
  call fastcc void @copy_bytes(ptr noundef %78, ptr noundef nonnull %74, i32 noundef %75) #53
  %79 = add i32 %77, %75
  store i32 %79, ptr @builder_length, align 4, !tbaa !4
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %4) #56
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %3) #56
  br label %80

80:                                               ; preds = %72, %6
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_u128(i64 noundef %0, i64 noundef %1) local_unnamed_addr #3 {
  %3 = zext i64 %1 to i128
  %4 = shl nuw i128 %3, 64
  %5 = zext i64 %0 to i128
  %6 = or disjoint i128 %4, %5
  tail call fastcc void @append_u128(i128 noundef %6, i32 noundef 0) #53
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_bool(i32 noundef %0) local_unnamed_addr #3 {
  %2 = icmp eq i32 %0, 0
  %3 = select i1 %2, ptr @.str.1, ptr @.str
  tail call fastcc void @append_c(ptr noundef nonnull %3) #53
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @append_c(ptr nocapture noundef readonly %0) unnamed_addr #3 {
  br label %2

2:                                                ; preds = %2, %1
  %3 = phi i32 [ 0, %1 ], [ %7, %2 ]
  %4 = getelementptr inbounds i8, ptr %0, i32 %3
  %5 = load i8, ptr %4, align 1, !tbaa !1
  %6 = icmp eq i8 %5, 0
  %7 = add i32 %3, 1
  br i1 %6, label %8, label %2, !llvm.loop !27

8:                                                ; preds = %2
  tail call fastcc void @reserve(i32 noundef %3) #53
  %9 = load ptr, ptr @builder, align 4, !tbaa !6
  %10 = load i32, ptr @builder_length, align 4, !tbaa !4
  %11 = getelementptr inbounds i8, ptr %9, i32 %10
  tail call fastcc void @copy_bytes(ptr noundef %11, ptr noundef nonnull %0, i32 noundef %3) #53
  %12 = add i32 %10, %3
  store i32 %12, ptr @builder_length, align 4, !tbaa !4
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_f64(double noundef %0) local_unnamed_addr #3 {
  tail call fastcc void @reserve(i32 noundef 40) #53
  %2 = load ptr, ptr @builder, align 4, !tbaa !6
  %3 = load i32, ptr @builder_length, align 4, !tbaa !4
  %4 = getelementptr inbounds i8, ptr %2, i32 %3
  %5 = tail call i32 @fa_host_format_float(double noundef %0, i32 noundef 0, ptr noundef %4) #54
  %6 = load i32, ptr @builder_length, align 4, !tbaa !4
  %7 = add i32 %6, %5
  store i32 %7, ptr @builder_length, align 4, !tbaa !4
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_f32(float noundef %0) local_unnamed_addr #3 {
  %2 = fpext float %0 to double
  tail call fastcc void @reserve(i32 noundef 40) #53
  %3 = load ptr, ptr @builder, align 4, !tbaa !6
  %4 = load i32, ptr @builder_length, align 4, !tbaa !4
  %5 = getelementptr inbounds i8, ptr %3, i32 %4
  %6 = tail call i32 @fa_host_format_float(double noundef %2, i32 noundef 1, ptr noundef %5) #54
  %7 = load i32, ptr @builder_length, align 4, !tbaa !4
  %8 = add i32 %7, %6
  store i32 %8, ptr @builder_length, align 4, !tbaa !4
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_percentage(double noundef %0) local_unnamed_addr #3 {
  tail call fastcc void @reserve(i32 noundef 40) #53
  %2 = load ptr, ptr @builder, align 4, !tbaa !6
  %3 = load i32, ptr @builder_length, align 4, !tbaa !4
  %4 = getelementptr inbounds i8, ptr %2, i32 %3
  %5 = tail call i32 @fa_host_format_float(double noundef %0, i32 noundef 2, ptr noundef %4) #54
  %6 = load i32, ptr @builder_length, align 4, !tbaa !4
  %7 = add i32 %6, %5
  store i32 %7, ptr @builder_length, align 4, !tbaa !4
  tail call fastcc void @reserve(i32 noundef 1) #53
  %8 = load ptr, ptr @builder, align 4, !tbaa !6
  %9 = load i32, ptr @builder_length, align 4, !tbaa !4
  %10 = getelementptr inbounds i8, ptr %8, i32 %9
  store i8 37, ptr %10, align 1, !tbaa !1
  %11 = add i32 %9, 1
  store i32 %11, ptr @builder_length, align 4, !tbaa !4
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_quoted(ptr nocapture noundef readonly %0, i32 noundef %1) local_unnamed_addr #3 {
  tail call fastcc void @reserve(i32 noundef 1) #53
  %3 = load ptr, ptr @builder, align 4, !tbaa !6
  %4 = load i32, ptr @builder_length, align 4, !tbaa !4
  %5 = getelementptr inbounds i8, ptr %3, i32 %4
  store i8 34, ptr %5, align 1, !tbaa !1
  %6 = add i32 %4, 1
  store i32 %6, ptr @builder_length, align 4, !tbaa !4
  %7 = icmp eq i32 %1, 0
  br i1 %7, label %8, label %13

8:                                                ; preds = %50, %2
  tail call fastcc void @reserve(i32 noundef 1) #53
  %9 = load ptr, ptr @builder, align 4, !tbaa !6
  %10 = load i32, ptr @builder_length, align 4, !tbaa !4
  %11 = getelementptr inbounds i8, ptr %9, i32 %10
  store i8 34, ptr %11, align 1, !tbaa !1
  %12 = add i32 %10, 1
  store i32 %12, ptr @builder_length, align 4, !tbaa !4
  ret void

13:                                               ; preds = %2, %50
  %14 = phi i32 [ %51, %50 ], [ 0, %2 ]
  %15 = getelementptr inbounds i8, ptr %0, i32 %14
  %16 = load i8, ptr %15, align 1, !tbaa !1
  %17 = zext i8 %16 to i32
  switch i8 %16, label %26 [
    i8 92, label %18
    i8 34, label %19
    i8 10, label %20
    i8 13, label %21
    i8 9, label %22
    i8 0, label %23
    i8 123, label %24
    i8 125, label %25
  ]

18:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.2) #53
  br label %50

19:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.3) #53
  br label %50

20:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.4) #53
  br label %50

21:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.5) #53
  br label %50

22:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.6) #53
  br label %50

23:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.7) #53
  br label %50

24:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.8) #53
  br label %50

25:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.9) #53
  br label %50

26:                                               ; preds = %13
  %27 = icmp ult i8 %16, 32
  %28 = icmp eq i8 %16, 127
  %29 = or i1 %27, %28
  br i1 %29, label %30, label %45

30:                                               ; preds = %26
  tail call fastcc void @append_c(ptr noundef nonnull @.str.10) #53
  %31 = lshr i32 %17, 4
  %32 = getelementptr inbounds [17 x i8], ptr @fa_b_quoted.hex, i32 0, i32 %31
  %33 = load i8, ptr %32, align 1, !tbaa !1
  tail call fastcc void @reserve(i32 noundef 1) #53
  %34 = load ptr, ptr @builder, align 4, !tbaa !6
  %35 = load i32, ptr @builder_length, align 4, !tbaa !4
  %36 = getelementptr inbounds i8, ptr %34, i32 %35
  store i8 %33, ptr %36, align 1, !tbaa !1
  %37 = add i32 %35, 1
  store i32 %37, ptr @builder_length, align 4, !tbaa !4
  %38 = and i32 %17, 15
  %39 = getelementptr inbounds [17 x i8], ptr @fa_b_quoted.hex, i32 0, i32 %38
  %40 = load i8, ptr %39, align 1, !tbaa !1
  tail call fastcc void @reserve(i32 noundef 1) #53
  %41 = load ptr, ptr @builder, align 4, !tbaa !6
  %42 = load i32, ptr @builder_length, align 4, !tbaa !4
  %43 = getelementptr inbounds i8, ptr %41, i32 %42
  store i8 %40, ptr %43, align 1, !tbaa !1
  %44 = add i32 %42, 1
  store i32 %44, ptr @builder_length, align 4, !tbaa !4
  br label %50

45:                                               ; preds = %26
  tail call fastcc void @reserve(i32 noundef 1) #53
  %46 = load ptr, ptr @builder, align 4, !tbaa !6
  %47 = load i32, ptr @builder_length, align 4, !tbaa !4
  %48 = getelementptr inbounds i8, ptr %46, i32 %47
  store i8 %16, ptr %48, align 1, !tbaa !1
  %49 = add i32 %47, 1
  store i32 %49, ptr @builder_length, align 4, !tbaa !4
  br label %50

50:                                               ; preds = %30, %45, %25, %24, %23, %22, %21, %20, %19, %18
  %51 = add nuw i32 %14, 1
  %52 = icmp eq i32 %51, %1
  br i1 %52, label %8, label %13, !llvm.loop !28
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_string_array(ptr nocapture noundef readonly %0, i32 noundef %1) local_unnamed_addr #3 {
  tail call fastcc void @reserve(i32 noundef 1) #53
  %3 = load ptr, ptr @builder, align 4, !tbaa !6
  %4 = load i32, ptr @builder_length, align 4, !tbaa !4
  %5 = getelementptr inbounds i8, ptr %3, i32 %4
  store i8 91, ptr %5, align 1, !tbaa !1
  %6 = add i32 %4, 1
  store i32 %6, ptr @builder_length, align 4, !tbaa !4
  %7 = icmp eq i32 %1, 0
  br i1 %7, label %8, label %13

8:                                                ; preds = %30, %2
  tail call fastcc void @reserve(i32 noundef 1) #53
  %9 = load ptr, ptr @builder, align 4, !tbaa !6
  %10 = load i32, ptr @builder_length, align 4, !tbaa !4
  %11 = getelementptr inbounds i8, ptr %9, i32 %10
  store i8 93, ptr %11, align 1, !tbaa !1
  %12 = add i32 %10, 1
  store i32 %12, ptr @builder_length, align 4, !tbaa !4
  ret void

13:                                               ; preds = %2, %30
  %14 = phi i32 [ %33, %30 ], [ 0, %2 ]
  %15 = icmp eq i32 %14, 0
  br i1 %15, label %17, label %16

16:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.11) #53
  br label %17

17:                                               ; preds = %16, %13
  %18 = getelementptr inbounds %struct.FaString, ptr %0, i32 %14
  %19 = getelementptr inbounds %struct.FaString, ptr %0, i32 %14, i32 2
  %20 = load i32, ptr %19, align 4, !tbaa !19
  %21 = and i32 %20, 1073741824
  %22 = icmp eq i32 %21, 0
  br i1 %22, label %26, label %23

23:                                               ; preds = %17
  %24 = lshr i32 %20, 24
  %25 = and i32 %24, 31
  br label %30

26:                                               ; preds = %17
  %27 = load ptr, ptr %18, align 4, !tbaa !13
  %28 = getelementptr inbounds %struct.FaString, ptr %0, i32 %14, i32 1
  %29 = load i32, ptr %28, align 4, !tbaa !18
  br label %30

30:                                               ; preds = %23, %26
  %31 = phi ptr [ %18, %23 ], [ %27, %26 ]
  %32 = phi i32 [ %25, %23 ], [ %29, %26 ]
  tail call void @fa_b_quoted(ptr noundef %31, i32 noundef %32) #53
  %33 = add nuw i32 %14, 1
  %34 = icmp eq i32 %33, %1
  br i1 %34, label %8, label %13, !llvm.loop !29
}

; Function Attrs: nounwind optsize
define hidden void @fa_concat(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef readonly %2, i32 noundef %3, ptr nocapture noundef writeonly %4) local_unnamed_addr #3 {
  %6 = alloca %struct.FaString, align 4
  %7 = alloca %struct.FaString, align 4
  %8 = add i32 %3, %1
  %9 = icmp ult i32 %8, %1
  %10 = icmp ugt i32 %8, 1073741822
  %11 = or i1 %9, %10
  br i1 %11, label %12, label %13

12:                                               ; preds = %5
  tail call fastcc void @out_of_memory() #57
  unreachable

13:                                               ; preds = %5
  %14 = icmp ult i32 %8, 2
  br i1 %14, label %15, label %18

15:                                               ; preds = %13
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %6) #56
  %16 = icmp eq i32 %1, 0
  %17 = select i1 %16, ptr %2, ptr %0
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %6, ptr noundef %17, i32 noundef %8, i32 noundef 0) #53
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %4, ptr noundef nonnull align 4 dereferenceable(12) %6, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %6) #56
  br label %35

18:                                               ; preds = %13
  %19 = icmp ult i32 %8, 12
  br i1 %19, label %20, label %28

20:                                               ; preds = %18
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %7) #56
  call void @llvm.memset.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %7, i8 0, i32 12, i1 false)
  call fastcc void @copy_bytes(ptr noundef nonnull %7, ptr noundef %0, i32 noundef %1) #53
  %21 = getelementptr inbounds i8, ptr %7, i32 %1
  call fastcc void @copy_bytes(ptr noundef nonnull %21, ptr noundef %2, i32 noundef %3) #53
  %22 = call fastcc i32 @simple_flag(ptr noundef nonnull %7, i32 noundef %8) #53, !range !20
  %23 = icmp eq i32 %22, 0
  %24 = select i1 %23, i32 64, i32 192
  %25 = or disjoint i32 %24, %8
  %26 = trunc i32 %25 to i8
  %27 = getelementptr inbounds i8, ptr %7, i32 11
  store i8 %26, ptr %27, align 1, !tbaa !1
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %4, ptr noundef nonnull align 4 dereferenceable(12) %7, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %7) #56
  br label %35

28:                                               ; preds = %18
  %29 = tail call ptr @fa_alloc(i32 noundef %8, i32 noundef 1) #54
  tail call fastcc void @copy_bytes(ptr noundef %29, ptr noundef %0, i32 noundef %1) #53
  %30 = getelementptr inbounds i8, ptr %29, i32 %1
  tail call fastcc void @copy_bytes(ptr noundef %30, ptr noundef %2, i32 noundef %3) #53
  store ptr %29, ptr %4, align 4, !tbaa !13
  %31 = getelementptr inbounds %struct.FaString, ptr %4, i32 0, i32 1
  store i32 %8, ptr %31, align 4, !tbaa !18
  %32 = tail call fastcc i32 @simple_flag(ptr noundef %29, i32 noundef %8) #53, !range !20
  %33 = or i32 %32, %8
  %34 = getelementptr inbounds %struct.FaString, ptr %4, i32 0, i32 2
  store i32 %33, ptr %34, align 4, !tbaa !19
  br label %35

35:                                               ; preds = %28, %20, %15
  ret void
}

; Function Attrs: noreturn nounwind optsize
define internal fastcc void @out_of_memory() unnamed_addr #5 {
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @out_of_memory.message, i32 noundef 20) #54
  tail call void @fa_exit(i32 noundef 101) #55
  unreachable
}

; Function Attrs: mustprogress nocallback nofree nounwind willreturn memory(argmem: write)
declare void @llvm.memset.p0.i32(ptr nocapture writeonly, i8, i32, i1 immarg) #11

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: read)
define internal fastcc i32 @simple_flag(ptr nocapture noundef readonly %0, i32 noundef %1) unnamed_addr #12 {
  %3 = icmp ult i32 %1, 8
  br i1 %3, label %4, label %8

4:                                                ; preds = %8, %2
  %5 = phi i32 [ 0, %2 ], [ %9, %8 ]
  %6 = phi i64 [ 0, %2 ], [ %19, %8 ]
  %7 = icmp ult i32 %5, %1
  br i1 %7, label %22, label %33

8:                                                ; preds = %2, %8
  %9 = phi i32 [ %20, %8 ], [ 8, %2 ]
  %10 = phi i64 [ %19, %8 ], [ 0, %2 ]
  %11 = phi i32 [ %9, %8 ], [ 0, %2 ]
  %12 = getelementptr inbounds i8, ptr %0, i32 %11
  %13 = load i64, ptr %12, align 1
  %14 = xor i64 %13, 940422246894996749
  %15 = add i64 %14, -72340172838076673
  %16 = xor i64 %13, -940422246894996750
  %17 = and i64 %15, %16
  %18 = or i64 %10, %17
  %19 = or i64 %18, %13
  %20 = add nuw i32 %9, 8
  %21 = icmp ugt i32 %20, %1
  br i1 %21, label %4, label %8, !llvm.loop !30

22:                                               ; preds = %4, %22
  %23 = phi i64 [ %30, %22 ], [ %6, %4 ]
  %24 = phi i32 [ %31, %22 ], [ %5, %4 ]
  %25 = getelementptr inbounds i8, ptr %0, i32 %24
  %26 = load i8, ptr %25, align 1, !tbaa !1
  %27 = icmp eq i8 %26, 13
  %28 = select i1 %27, i8 -128, i8 %26
  %29 = zext i8 %28 to i64
  %30 = or i64 %23, %29
  %31 = add nuw i32 %24, 1
  %32 = icmp eq i32 %31, %1
  br i1 %32, label %33, label %22, !llvm.loop !31

33:                                               ; preds = %22, %4
  %34 = phi i64 [ %6, %4 ], [ %30, %22 ]
  %35 = and i64 %34, -9187201950435737472
  %36 = icmp eq i64 %35, 0
  %37 = select i1 %36, i32 -2147483648, i32 0
  ret i32 %37
}

; Function Attrs: optsize
declare ptr @fa_alloc(i32 noundef, i32 noundef) local_unnamed_addr #4

; Function Attrs: nounwind optsize
define hidden noundef i32 @fa_string_size(i32 noundef %0, i32 noundef %1) local_unnamed_addr #3 {
  %3 = sub i32 1073741823, %0
  %4 = icmp ugt i32 %3, %1
  br i1 %4, label %6, label %5

5:                                                ; preds = %2
  tail call fastcc void @out_of_memory() #57
  unreachable

6:                                                ; preds = %2
  %7 = add i32 %1, %0
  ret i32 %7
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_string_allocate(i32 noundef %0, i32 noundef %1, ptr noundef writeonly %2) local_unnamed_addr #3 {
  store ptr null, ptr %2, align 4, !tbaa.struct !10
  %4 = getelementptr inbounds i8, ptr %2, i32 4
  store i32 0, ptr %4, align 4, !tbaa.struct !11
  %5 = getelementptr inbounds i8, ptr %2, i32 8
  store i32 0, ptr %5, align 4, !tbaa.struct !12
  %6 = icmp ult i32 %0, 12
  br i1 %6, label %7, label %13

7:                                                ; preds = %3
  %8 = icmp eq i32 %1, 0
  %9 = select i1 %8, i32 64, i32 192
  %10 = or disjoint i32 %9, %0
  %11 = trunc i32 %10 to i8
  %12 = getelementptr inbounds i8, ptr %2, i32 11
  store i8 %11, ptr %12, align 1, !tbaa !1
  br label %16

13:                                               ; preds = %3
  %14 = tail call ptr @fa_alloc(i32 noundef %0, i32 noundef 1) #54
  store ptr %14, ptr %2, align 4, !tbaa !13
  store i32 %0, ptr %4, align 4, !tbaa !18
  %15 = or i32 %1, %0
  store i32 %15, ptr %5, align 4, !tbaa !19
  br label %16

16:                                               ; preds = %13, %7
  %17 = phi ptr [ %2, %7 ], [ %14, %13 ]
  ret ptr %17
}

; Function Attrs: alwaysinline nounwind optsize
define hidden ptr @fa_string_replace_buffer(i32 noundef %0, i32 noundef %1, ptr noundef %2) local_unnamed_addr #13 {
  %4 = getelementptr inbounds %struct.FaString, ptr %2, i32 0, i32 2
  %5 = load i32, ptr %4, align 4, !tbaa !19
  %6 = freeze i32 %5
  %7 = and i32 %6, 1073741823
  %8 = and i32 %6, 1073741824
  %9 = icmp eq i32 %8, 0
  %10 = icmp ne i32 %7, 0
  %11 = icmp ne i32 %7, 1073741823
  %12 = and i1 %10, %11
  %13 = load ptr, ptr %2, align 4, !tbaa !13
  %14 = icmp uge i32 %7, %0
  %15 = and i1 %12, %14
  %16 = and i1 %15, %9
  br i1 %16, label %17, label %20

17:                                               ; preds = %3
  %18 = getelementptr inbounds %struct.FaString, ptr %2, i32 0, i32 1
  store i32 %0, ptr %18, align 4, !tbaa !18
  %19 = or i32 %7, %1
  store i32 %19, ptr %4, align 4, !tbaa !19
  br label %24

20:                                               ; preds = %3
  %21 = tail call ptr @fa_string_allocate(i32 noundef %0, i32 noundef %1, ptr noundef nonnull %2) #53
  br i1 %9, label %22, label %24

22:                                               ; preds = %20
  switch i32 %7, label %23 [
    i32 1073741823, label %24
    i32 0, label %24
  ]

23:                                               ; preds = %22
  tail call void @fa_free(ptr noundef %13, i32 noundef %7, i32 noundef 1) #54
  br label %24

24:                                               ; preds = %22, %22, %20, %23, %17
  %25 = phi ptr [ %13, %17 ], [ %21, %23 ], [ %21, %22 ], [ %21, %20 ], [ %21, %22 ]
  ret ptr %25
}

; Function Attrs: optsize
declare void @fa_free(ptr noundef, i32 noundef, i32 noundef) local_unnamed_addr #4

; Function Attrs: nofree norecurse nosync nounwind optsize memory(read, argmem: readwrite, inaccessiblemem: none)
define hidden void @fa_string_finish(ptr nocapture noundef %0) local_unnamed_addr #14 {
  %2 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 2
  %3 = load i32, ptr %2, align 4, !tbaa !19
  %4 = icmp sgt i32 %3, -1
  br i1 %4, label %5, label %19

5:                                                ; preds = %1
  %6 = icmp ult i32 %3, 1073741824
  br i1 %6, label %10, label %7

7:                                                ; preds = %5
  %8 = lshr i32 %3, 24
  %9 = and i32 %8, 31
  br label %14

10:                                               ; preds = %5
  %11 = load ptr, ptr %0, align 4, !tbaa !13
  %12 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 1
  %13 = load i32, ptr %12, align 4, !tbaa !18
  br label %14

14:                                               ; preds = %7, %10
  %15 = phi ptr [ %0, %7 ], [ %11, %10 ]
  %16 = phi i32 [ %9, %7 ], [ %13, %10 ]
  %17 = tail call fastcc i32 @simple_flag(ptr noundef %15, i32 noundef %16) #53, !range !20
  %18 = or i32 %17, %3
  store i32 %18, ptr %2, align 4, !tbaa !19
  br label %19

19:                                               ; preds = %14, %1
  ret void
}

; Function Attrs: alwaysinline nounwind optsize
define hidden void @fa_strings_join(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef writeonly %2) local_unnamed_addr #13 {
  %4 = alloca %struct.FaString, align 4
  %5 = icmp eq i32 %1, 0
  br i1 %5, label %36, label %8

6:                                                ; preds = %27
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %4) #56
  call void @llvm.memset.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %4, i8 0, i32 12, i1 false)
  %7 = icmp ult i32 %28, 12
  br i1 %7, label %37, label %34

8:                                                ; preds = %3, %27
  %9 = phi i32 [ %28, %27 ], [ 0, %3 ]
  %10 = phi i32 [ %31, %27 ], [ -2147483648, %3 ]
  %11 = phi i32 [ %32, %27 ], [ 0, %3 ]
  %12 = getelementptr inbounds %struct.FaString, ptr %0, i32 %11, i32 2
  %13 = load i32, ptr %12, align 4, !tbaa !19
  %14 = and i32 %13, 1073741824
  %15 = icmp eq i32 %14, 0
  br i1 %15, label %19, label %16

16:                                               ; preds = %8
  %17 = lshr i32 %13, 24
  %18 = and i32 %17, 31
  br label %22

19:                                               ; preds = %8
  %20 = getelementptr inbounds %struct.FaString, ptr %0, i32 %11, i32 1
  %21 = load i32, ptr %20, align 4, !tbaa !18
  br label %22

22:                                               ; preds = %16, %19
  %23 = phi i32 [ %18, %16 ], [ %21, %19 ]
  %24 = sub i32 1073741823, %9
  %25 = icmp ult i32 %23, %24
  br i1 %25, label %27, label %26

26:                                               ; preds = %22
  tail call fastcc void @out_of_memory() #57
  unreachable

27:                                               ; preds = %22
  %28 = add i32 %23, %9
  %29 = icmp eq i32 %23, 0
  %30 = select i1 %29, i32 -1, i32 %13
  %31 = and i32 %10, %30
  %32 = add nuw i32 %11, 1
  %33 = icmp eq i32 %32, %1
  br i1 %33, label %6, label %8, !llvm.loop !32

34:                                               ; preds = %6
  %35 = tail call ptr @fa_alloc(i32 noundef %28, i32 noundef 1) #54
  br label %37

36:                                               ; preds = %3
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %4) #56
  call void @llvm.memset.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %4, i8 0, i32 12, i1 false)
  br label %70

37:                                               ; preds = %34, %6
  %38 = phi ptr [ %4, %6 ], [ %35, %34 ]
  br label %41

39:                                               ; preds = %56
  %40 = icmp eq i32 %31, 0
  br i1 %40, label %63, label %66

41:                                               ; preds = %37, %56
  %42 = phi i32 [ %61, %56 ], [ 0, %37 ]
  %43 = phi i32 [ %60, %56 ], [ 0, %37 ]
  %44 = getelementptr inbounds %struct.FaString, ptr %0, i32 %42
  %45 = getelementptr inbounds %struct.FaString, ptr %0, i32 %42, i32 2
  %46 = load i32, ptr %45, align 4, !tbaa !19
  %47 = and i32 %46, 1073741824
  %48 = icmp eq i32 %47, 0
  br i1 %48, label %52, label %49

49:                                               ; preds = %41
  %50 = lshr i32 %46, 24
  %51 = and i32 %50, 31
  br label %56

52:                                               ; preds = %41
  %53 = getelementptr inbounds %struct.FaString, ptr %0, i32 %42, i32 1
  %54 = load i32, ptr %53, align 4, !tbaa !18
  %55 = load ptr, ptr %44, align 4, !tbaa !13
  br label %56

56:                                               ; preds = %49, %52
  %57 = phi i32 [ %54, %52 ], [ %51, %49 ]
  %58 = phi ptr [ %55, %52 ], [ %44, %49 ]
  %59 = getelementptr inbounds i8, ptr %38, i32 %43
  call fastcc void @copy_bytes(ptr noundef %59, ptr noundef %58, i32 noundef %57) #53
  %60 = add i32 %57, %43
  %61 = add nuw i32 %42, 1
  %62 = icmp eq i32 %61, %1
  br i1 %62, label %39, label %41, !llvm.loop !33

63:                                               ; preds = %39
  %64 = call fastcc i32 @simple_flag(ptr noundef %38, i32 noundef %28) #53, !range !20
  %65 = freeze i32 %64
  br i1 %7, label %67, label %76

66:                                               ; preds = %39
  br i1 %7, label %70, label %76

67:                                               ; preds = %63
  %68 = icmp eq i32 %65, 0
  %69 = select i1 %68, i32 64, i32 192
  br label %70

70:                                               ; preds = %67, %66, %36
  %71 = phi i32 [ %28, %66 ], [ 0, %36 ], [ %28, %67 ]
  %72 = phi i32 [ 192, %66 ], [ 192, %36 ], [ %69, %67 ]
  %73 = or disjoint i32 %72, %71
  %74 = trunc i32 %73 to i8
  %75 = getelementptr inbounds i8, ptr %4, i32 11
  store i8 %74, ptr %75, align 1, !tbaa !1
  br label %81

76:                                               ; preds = %63, %66
  %77 = phi i32 [ %65, %63 ], [ %31, %66 ]
  store ptr %38, ptr %4, align 4, !tbaa !13
  %78 = getelementptr inbounds %struct.FaString, ptr %4, i32 0, i32 1
  store i32 %28, ptr %78, align 4, !tbaa !18
  %79 = or i32 %77, %28
  %80 = getelementptr inbounds %struct.FaString, ptr %4, i32 0, i32 2
  store i32 %79, ptr %80, align 4, !tbaa !19
  br label %81

81:                                               ; preds = %70, %76
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %2, ptr noundef nonnull align 4 dereferenceable(12) %4, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %4) #56
  ret void
}

; Function Attrs: alwaysinline nounwind optsize
define hidden void @fa_string_append(ptr nocapture noundef %0, ptr nocapture noundef readonly %1, i32 noundef %2, i32 noundef %3) local_unnamed_addr #13 {
  %5 = alloca %struct.FaString, align 4
  %6 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 2
  %7 = load i32, ptr %6, align 4, !tbaa !19
  %8 = and i32 %7, 2147483647
  %9 = and i32 %3, -2147483648
  %10 = icmp ne i32 %9, 0
  %11 = icmp ult i32 %8, 1073741823
  %12 = select i1 %10, i1 %11, i1 false
  br i1 %12, label %13, label %23, !prof !34

13:                                               ; preds = %4
  %14 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 1
  %15 = load i32, ptr %14, align 4, !tbaa !18
  %16 = add i32 %15, %2
  %17 = icmp ugt i32 %16, %8
  br i1 %17, label %23, label %18, !prof !35

18:                                               ; preds = %13
  %19 = load ptr, ptr %0, align 4, !tbaa !13
  %20 = getelementptr inbounds i8, ptr %19, i32 %15
  tail call fastcc void @copy_bytes(ptr noundef %20, ptr noundef %1, i32 noundef %2) #53
  %21 = load i32, ptr %14, align 4, !tbaa !18
  %22 = add i32 %21, %2
  store i32 %22, ptr %14, align 4, !tbaa !18
  br label %63

23:                                               ; preds = %4, %13
  %24 = icmp eq i32 %2, 0
  br i1 %24, label %63, label %25

25:                                               ; preds = %23
  %26 = and i32 %7, 1073741824
  %27 = icmp eq i32 %26, 0
  br i1 %27, label %28, label %33

28:                                               ; preds = %25
  %29 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 1
  %30 = load i32, ptr %29, align 4, !tbaa !18
  %31 = add i32 %30, %2
  %32 = icmp ult i32 %31, 12
  br i1 %32, label %48, label %62

33:                                               ; preds = %25
  %34 = lshr i32 %7, 24
  %35 = and i32 %34, 31
  %36 = add i32 %35, %2
  %37 = icmp ult i32 %36, 12
  br i1 %37, label %38, label %62

38:                                               ; preds = %33
  %39 = icmp eq i32 %35, 0
  %40 = select i1 %39, i32 -2147483648, i32 %7
  %41 = and i32 %40, %9
  %42 = getelementptr inbounds i8, ptr %0, i32 %35
  tail call fastcc void @copy_bytes(ptr noundef %42, ptr noundef %1, i32 noundef %2) #53
  %43 = icmp eq i32 %41, 0
  %44 = select i1 %43, i32 64, i32 192
  %45 = or disjoint i32 %44, %36
  %46 = trunc i32 %45 to i8
  %47 = getelementptr inbounds i8, ptr %0, i32 11
  store i8 %46, ptr %47, align 1, !tbaa !1
  br label %63

48:                                               ; preds = %28
  %49 = icmp eq i32 %30, 0
  %50 = select i1 %49, i32 -2147483648, i32 %7
  %51 = and i32 %50, %9
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %5) #56
  call void @llvm.memset.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %5, i8 0, i32 12, i1 false)
  %52 = load ptr, ptr %0, align 4, !tbaa !13
  call fastcc void @copy_bytes(ptr noundef nonnull %5, ptr noundef %52, i32 noundef %30) #53
  %53 = getelementptr inbounds i8, ptr %5, i32 %30
  call fastcc void @copy_bytes(ptr noundef nonnull %53, ptr noundef %1, i32 noundef %2) #53
  %54 = icmp eq i32 %51, 0
  %55 = select i1 %54, i32 64, i32 192
  %56 = or disjoint i32 %55, %31
  %57 = trunc i32 %56 to i8
  %58 = getelementptr inbounds i8, ptr %5, i32 11
  store i8 %57, ptr %58, align 1, !tbaa !1
  %59 = and i32 %7, 1073741823
  switch i32 %59, label %60 [
    i32 1073741823, label %61
    i32 0, label %61
  ]

60:                                               ; preds = %48
  tail call void @fa_free(ptr noundef %52, i32 noundef %59, i32 noundef 1) #54
  br label %61

61:                                               ; preds = %48, %48, %60
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %0, ptr noundef nonnull align 4 dereferenceable(12) %5, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %5) #56
  br label %63

62:                                               ; preds = %33, %28
  tail call fastcc void @string_append_slow(ptr noundef nonnull %0, ptr noundef %1, i32 noundef %2, i32 noundef %3) #53
  br label %63

63:                                               ; preds = %62, %61, %38, %23, %18
  ret void
}

; Function Attrs: noinline nounwind optsize
define internal fastcc void @string_append_slow(ptr nocapture noundef %0, ptr nocapture noundef readonly %1, i32 noundef %2, i32 noundef %3) unnamed_addr #15 {
  %5 = alloca %struct.FaString, align 4
  %6 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 2
  %7 = load i32, ptr %6, align 4, !tbaa !19
  %8 = and i32 %7, 1073741824
  %9 = icmp eq i32 %8, 0
  br i1 %9, label %13, label %10

10:                                               ; preds = %4
  %11 = lshr i32 %7, 24
  %12 = and i32 %11, 31
  br label %16

13:                                               ; preds = %4
  %14 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 1
  %15 = load i32, ptr %14, align 4, !tbaa !18
  br label %16

16:                                               ; preds = %10, %13
  %17 = phi i32 [ %12, %10 ], [ %15, %13 ]
  %18 = add i32 %17, %2
  %19 = icmp sgt i32 %3, -1
  br i1 %19, label %20, label %30

20:                                               ; preds = %16
  %21 = icmp eq i32 %2, 1
  br i1 %21, label %22, label %28

22:                                               ; preds = %20
  %23 = load i8, ptr %1, align 1, !tbaa !1
  %24 = icmp sgt i8 %23, -1
  %25 = icmp eq i8 %23, 13
  %26 = select i1 %25, i32 0, i32 -2147483648
  %27 = select i1 %24, i32 %26, i32 0
  br label %30

28:                                               ; preds = %20
  %29 = tail call fastcc i32 @simple_flag(ptr noundef %1, i32 noundef %2) #53, !range !20
  br label %30

30:                                               ; preds = %22, %28, %16
  %31 = phi i32 [ -2147483648, %16 ], [ %27, %22 ], [ %29, %28 ]
  %32 = icmp eq i32 %17, 0
  %33 = select i1 %32, i32 -1, i32 %7
  %34 = and i32 %31, %33
  %35 = icmp ult i32 %18, 12
  br i1 %35, label %36, label %55

36:                                               ; preds = %30
  br i1 %9, label %44, label %37

37:                                               ; preds = %36
  %38 = getelementptr inbounds i8, ptr %0, i32 %17
  tail call fastcc void @copy_bytes(ptr noundef %38, ptr noundef %1, i32 noundef %2) #53
  %39 = icmp eq i32 %34, 0
  %40 = select i1 %39, i32 64, i32 192
  %41 = or disjoint i32 %40, %18
  %42 = trunc i32 %41 to i8
  %43 = getelementptr inbounds i8, ptr %0, i32 11
  store i8 %42, ptr %43, align 1, !tbaa !1
  br label %69

44:                                               ; preds = %36
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %5) #56
  call void @llvm.memset.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %5, i8 0, i32 12, i1 false)
  %45 = load ptr, ptr %0, align 4, !tbaa !13
  call fastcc void @copy_bytes(ptr noundef nonnull %5, ptr noundef %45, i32 noundef %17) #53
  %46 = getelementptr inbounds i8, ptr %5, i32 %17
  call fastcc void @copy_bytes(ptr noundef nonnull %46, ptr noundef %1, i32 noundef %2) #53
  %47 = icmp eq i32 %34, 0
  %48 = select i1 %47, i32 64, i32 192
  %49 = or disjoint i32 %48, %18
  %50 = trunc i32 %49 to i8
  %51 = getelementptr inbounds i8, ptr %5, i32 11
  store i8 %50, ptr %51, align 1, !tbaa !1
  %52 = and i32 %7, 1073741823
  switch i32 %52, label %53 [
    i32 1073741823, label %54
    i32 0, label %54
  ]

53:                                               ; preds = %44
  tail call void @fa_free(ptr noundef %45, i32 noundef %52, i32 noundef 1) #54
  br label %54

54:                                               ; preds = %44, %44, %53
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %0, ptr noundef nonnull align 4 dereferenceable(12) %5, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %5) #56
  br label %69

55:                                               ; preds = %30
  %56 = and i32 %7, 1073741823
  %57 = icmp ne i32 %56, 0
  %58 = icmp ne i32 %56, 1073741823
  %59 = and i1 %57, %58
  %60 = and i1 %9, %59
  %61 = icmp ule i32 %18, %56
  %62 = select i1 %60, i1 %61, i1 false
  br i1 %62, label %63, label %68

63:                                               ; preds = %55
  %64 = load ptr, ptr %0, align 4, !tbaa !13
  %65 = getelementptr inbounds i8, ptr %64, i32 %17
  tail call fastcc void @copy_bytes(ptr noundef %65, ptr noundef %1, i32 noundef %2) #53
  %66 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 1
  store i32 %18, ptr %66, align 4, !tbaa !18
  %67 = or i32 %34, %56
  store i32 %67, ptr %6, align 4, !tbaa !19
  br label %69

68:                                               ; preds = %55
  tail call fastcc void @string_grow(ptr noundef nonnull %0, ptr noundef %1, i32 noundef %2, i32 noundef %34) #53
  br label %69

69:                                               ; preds = %63, %68, %54, %37
  ret void
}

; Function Attrs: alwaysinline nofree norecurse nosync nounwind optsize memory(argmem: read)
define hidden i32 @fa_string_equal(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef readonly %2, i32 noundef %3) local_unnamed_addr #16 {
  %5 = icmp eq i32 %1, %3
  br i1 %5, label %6, label %14

6:                                                ; preds = %4
  switch i32 %1, label %12 [
    i32 0, label %14
    i32 1, label %7
  ]

7:                                                ; preds = %6
  %8 = load i8, ptr %0, align 1, !tbaa !1
  %9 = load i8, ptr %2, align 1, !tbaa !1
  %10 = icmp eq i8 %8, %9
  %11 = zext i1 %10 to i32
  br label %14

12:                                               ; preds = %6
  %13 = tail call fastcc i32 @string_equal_bytes(ptr noundef %0, ptr noundef %2, i32 noundef %1) #53
  br label %14

14:                                               ; preds = %6, %4, %12, %7
  %15 = phi i32 [ %11, %7 ], [ %13, %12 ], [ 0, %4 ], [ 1, %6 ]
  ret i32 %15
}

; Function Attrs: nofree noinline norecurse nosync nounwind optsize memory(argmem: read)
define internal fastcc noundef i32 @string_equal_bytes(ptr nocapture noundef readonly %0, ptr nocapture noundef readonly %1, i32 noundef %2) unnamed_addr #17 {
  %4 = icmp ugt i32 %2, 7
  br i1 %4, label %5, label %18

5:                                                ; preds = %3
  %6 = and i32 %2, -8
  br label %7

7:                                                ; preds = %5, %14
  %8 = phi i32 [ %15, %14 ], [ 0, %5 ]
  %9 = getelementptr inbounds i8, ptr %0, i32 %8
  %10 = load i64, ptr %9, align 1
  %11 = getelementptr inbounds i8, ptr %1, i32 %8
  %12 = load i64, ptr %11, align 1
  %13 = icmp eq i64 %10, %12
  br i1 %13, label %14, label %50

14:                                               ; preds = %7
  %15 = add i32 %8, 8
  %16 = sub i32 %2, %15
  %17 = icmp ugt i32 %16, 7
  br i1 %17, label %7, label %18, !llvm.loop !36

18:                                               ; preds = %14, %3
  %19 = phi i32 [ 0, %3 ], [ %6, %14 ]
  %20 = phi i32 [ %2, %3 ], [ %16, %14 ]
  %21 = icmp ugt i32 %20, 3
  br i1 %21, label %22, label %29

22:                                               ; preds = %18
  %23 = getelementptr inbounds i8, ptr %0, i32 %19
  %24 = load i32, ptr %23, align 1
  %25 = getelementptr inbounds i8, ptr %1, i32 %19
  %26 = load i32, ptr %25, align 1
  %27 = icmp eq i32 %24, %26
  %28 = or disjoint i32 %19, 4
  br i1 %27, label %29, label %50

29:                                               ; preds = %22, %18
  %30 = phi i32 [ %28, %22 ], [ %19, %18 ]
  %31 = sub i32 %2, %30
  %32 = icmp ugt i32 %31, 1
  br i1 %32, label %33, label %40

33:                                               ; preds = %29
  %34 = getelementptr inbounds i8, ptr %0, i32 %30
  %35 = load i16, ptr %34, align 1
  %36 = getelementptr inbounds i8, ptr %1, i32 %30
  %37 = load i16, ptr %36, align 1
  %38 = icmp eq i16 %35, %37
  %39 = add i32 %30, 2
  br i1 %38, label %40, label %50

40:                                               ; preds = %33, %29
  %41 = phi i32 [ %39, %33 ], [ %30, %29 ]
  %42 = icmp ult i32 %41, %2
  br i1 %42, label %43, label %49

43:                                               ; preds = %40
  %44 = getelementptr inbounds i8, ptr %0, i32 %41
  %45 = load i8, ptr %44, align 1, !tbaa !1
  %46 = getelementptr inbounds i8, ptr %1, i32 %41
  %47 = load i8, ptr %46, align 1, !tbaa !1
  %48 = icmp eq i8 %45, %47
  br i1 %48, label %49, label %50

49:                                               ; preds = %43, %40
  br label %50

50:                                               ; preds = %7, %43, %22, %33, %49
  %51 = phi i32 [ 1, %49 ], [ 0, %33 ], [ 0, %22 ], [ 0, %43 ], [ 0, %7 ]
  ret i32 %51
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: read)
define hidden i32 @fa_string_compare(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef readonly %2, i32 noundef %3) local_unnamed_addr #12 {
  %5 = tail call i32 @llvm.umin.i32(i32 %1, i32 %3)
  %6 = icmp eq i32 %5, 0
  br i1 %6, label %11, label %7

7:                                                ; preds = %4
  %8 = load i8, ptr %0, align 1, !tbaa !1
  %9 = load i8, ptr %2, align 1, !tbaa !1
  %10 = icmp eq i8 %8, %9
  br i1 %10, label %16, label %28

11:                                               ; preds = %4
  %12 = icmp ult i32 %1, %3
  %13 = icmp eq i32 %1, %3
  %14 = select i1 %12, i32 -1, i32 1
  %15 = select i1 %13, i32 0, i32 %14
  br label %44

16:                                               ; preds = %7, %20
  %17 = phi i32 [ %18, %20 ], [ 0, %7 ]
  %18 = add i32 %17, 1
  %19 = icmp eq i32 %18, %5
  br i1 %19, label %39, label %20, !llvm.loop !37

20:                                               ; preds = %16
  %21 = getelementptr inbounds i8, ptr %0, i32 %18
  %22 = load i8, ptr %21, align 1, !tbaa !1
  %23 = getelementptr inbounds i8, ptr %2, i32 %18
  %24 = load i8, ptr %23, align 1, !tbaa !1
  %25 = icmp eq i8 %22, %24
  br i1 %25, label %16, label %26, !llvm.loop !37

26:                                               ; preds = %20
  %27 = icmp ult i32 %18, %5
  br label %28

28:                                               ; preds = %26, %7
  %29 = phi i1 [ true, %7 ], [ %27, %26 ]
  %30 = phi i8 [ %8, %7 ], [ %22, %26 ]
  %31 = phi i8 [ %9, %7 ], [ %24, %26 ]
  %32 = icmp ult i8 %30, %31
  %33 = select i1 %32, i32 -1, i32 1
  %34 = icmp ult i32 %1, %3
  %35 = icmp eq i32 %1, %3
  %36 = select i1 %34, i32 -1, i32 1
  %37 = select i1 %35, i32 0, i32 %36
  %38 = select i1 %29, i32 %33, i32 %37
  br label %44

39:                                               ; preds = %16
  %40 = icmp ult i32 %1, %3
  %41 = icmp eq i32 %1, %3
  %42 = select i1 %40, i32 -1, i32 1
  %43 = select i1 %41, i32 0, i32 %42
  br label %44

44:                                               ; preds = %28, %39, %11
  %45 = phi i32 [ %43, %39 ], [ %15, %11 ], [ %38, %28 ]
  ret i32 %45
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define hidden i32 @fa_next_grapheme(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2) local_unnamed_addr #18 {
  %4 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %2) #53
  ret i32 %4
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define internal fastcc i32 @next_boundary(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2) unnamed_addr #18 {
  %4 = alloca i32, align 4
  %5 = alloca i32, align 4
  store i32 %2, ptr %4, align 4, !tbaa !4
  %6 = getelementptr inbounds i8, ptr %0, i32 %2
  %7 = load i8, ptr %6, align 1, !tbaa !1
  %8 = icmp sgt i8 %7, -1
  br i1 %8, label %9, label %24

9:                                                ; preds = %3
  %10 = add i32 %2, 1
  %11 = icmp ult i32 %10, %1
  br i1 %11, label %12, label %21

12:                                               ; preds = %9
  %13 = getelementptr inbounds i8, ptr %0, i32 %10
  %14 = load i8, ptr %13, align 1, !tbaa !1
  %15 = icmp sgt i8 %14, -1
  br i1 %15, label %16, label %24

16:                                               ; preds = %12
  %17 = icmp eq i8 %7, 13
  br i1 %17, label %18, label %21

18:                                               ; preds = %16
  %19 = icmp eq i8 %14, 10
  %20 = zext i1 %19 to i32
  br label %21

21:                                               ; preds = %9, %18, %16
  %22 = phi i32 [ 0, %16 ], [ %20, %18 ], [ 0, %9 ]
  %23 = add i32 %22, %10
  br label %106

24:                                               ; preds = %12, %3
  %25 = call fastcc i32 @decode(ptr noundef nonnull %0, i32 noundef %1, ptr noundef nonnull %4) #53
  %26 = tail call fastcc i32 @break_kind(i32 noundef %25) #53, !range !38
  %27 = load i32, ptr %4, align 4, !tbaa !4
  %28 = icmp ult i32 %27, %1
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
  %38 = phi i32 [ %102, %101 ], [ %27, %29 ]
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %5) #56
  store i32 %38, ptr %5, align 4, !tbaa !4
  %39 = call fastcc i32 @decode(ptr noundef %0, i32 noundef %1, ptr noundef nonnull %5) #53
  %40 = tail call fastcc i32 @break_kind(i32 noundef %39) #53, !range !38
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
  %75 = load i32, ptr %5, align 4, !tbaa !4
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
  %84 = load i32, ptr %5, align 4, !tbaa !4
  %85 = add nuw nsw i32 %36, 1
  br label %94

86:                                               ; preds = %53, %52, %52, %52, %52, %34
  %87 = load i32, ptr %5, align 4, !tbaa !4
  br label %94

88:                                               ; preds = %59
  %89 = load i32, ptr %5, align 4, !tbaa !4
  %90 = icmp eq i32 %40, 6
  %91 = add nsw i32 %36, 1
  %92 = select i1 %90, i32 %91, i32 0
  %93 = icmp eq i32 %40, 7
  br i1 %93, label %101, label %94

94:                                               ; preds = %83, %86, %88
  %95 = phi i32 [ %87, %86 ], [ %89, %88 ], [ %84, %83 ]
  %96 = phi i32 [ 0, %86 ], [ %92, %88 ], [ %85, %83 ]
  %97 = add nsw i32 %40, -6
  %98 = icmp ult i32 %97, -2
  %99 = select i1 %98, i32 0, i32 %35
  br label %101

100:                                              ; preds = %76, %44
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %5) #56
  br label %106

101:                                              ; preds = %74, %94, %88
  %102 = phi i32 [ %89, %88 ], [ %95, %94 ], [ %75, %74 ]
  %103 = phi i32 [ %92, %88 ], [ %96, %94 ], [ 0, %74 ]
  %104 = phi i32 [ 1, %88 ], [ %99, %94 ], [ 1, %74 ]
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %5) #56
  %105 = icmp ult i32 %102, %1
  br i1 %105, label %34, label %106

106:                                              ; preds = %101, %24, %100, %21
  %107 = phi i32 [ %23, %21 ], [ %38, %100 ], [ %27, %24 ], [ %102, %101 ]
  ret i32 %107
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define hidden i32 @fa_graphemes(ptr nocapture noundef readonly %0, i32 noundef %1) local_unnamed_addr #18 {
  %3 = icmp eq i32 %1, 0
  br i1 %3, label %29, label %4

4:                                                ; preds = %2, %25
  %5 = phi i32 [ %27, %25 ], [ 0, %2 ]
  %6 = phi i32 [ %26, %25 ], [ 0, %2 ]
  %7 = add i32 %6, 8
  %8 = icmp ult i32 %7, %1
  br i1 %8, label %9, label %22

9:                                                ; preds = %4
  %10 = getelementptr inbounds i8, ptr %0, i32 %7
  %11 = load i8, ptr %10, align 1, !tbaa !1
  %12 = icmp sgt i8 %11, -1
  br i1 %12, label %13, label %22

13:                                               ; preds = %9
  %14 = getelementptr inbounds i8, ptr %0, i32 %6
  %15 = load i64, ptr %14, align 1
  %16 = xor i64 %15, 940422246894996749
  %17 = add i64 %16, -72340172838076673
  %18 = or i64 %17, %15
  %19 = and i64 %18, -9187201950435737472
  %20 = icmp eq i64 %19, 0
  %21 = add i32 %5, 8
  br i1 %20, label %25, label %22

22:                                               ; preds = %13, %9, %4
  %23 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %6) #53
  %24 = add i32 %5, 1
  br label %25

25:                                               ; preds = %22, %13
  %26 = phi i32 [ %7, %13 ], [ %23, %22 ]
  %27 = phi i32 [ %21, %13 ], [ %24, %22 ]
  %28 = icmp ult i32 %26, %1
  br i1 %28, label %4, label %29, !llvm.loop !39

29:                                               ; preds = %25, %2
  %30 = phi i32 [ 0, %2 ], [ %27, %25 ]
  ret i32 %30
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_append_into(ptr nocapture noundef %0) local_unnamed_addr #3 {
  %2 = alloca %struct.FaString, align 4
  %3 = load ptr, ptr @builder, align 4, !tbaa !6
  %4 = load i32, ptr @builder_length, align 4, !tbaa !4
  %5 = tail call fastcc i32 @simple_flag(ptr noundef %3, i32 noundef %4) #53, !range !20
  %6 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 2
  %7 = load i32, ptr %6, align 4, !tbaa !19
  %8 = and i32 %7, 2147483647
  %9 = and i32 %5, -2147483648
  %10 = icmp ne i32 %9, 0
  %11 = icmp ult i32 %8, 1073741823
  %12 = select i1 %10, i1 %11, i1 false
  br i1 %12, label %13, label %23, !prof !34

13:                                               ; preds = %1
  %14 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 1
  %15 = load i32, ptr %14, align 4, !tbaa !18
  %16 = add i32 %15, %4
  %17 = icmp ugt i32 %16, %8
  br i1 %17, label %23, label %18, !prof !35

18:                                               ; preds = %13
  %19 = load ptr, ptr %0, align 4, !tbaa !13
  %20 = getelementptr inbounds i8, ptr %19, i32 %15
  tail call fastcc void @copy_bytes(ptr noundef %20, ptr noundef %3, i32 noundef %4) #53
  %21 = load i32, ptr %14, align 4, !tbaa !18
  %22 = add i32 %21, %4
  store i32 %22, ptr %14, align 4, !tbaa !18
  br label %63

23:                                               ; preds = %13, %1
  %24 = icmp eq i32 %4, 0
  br i1 %24, label %63, label %25

25:                                               ; preds = %23
  %26 = and i32 %7, 1073741824
  %27 = icmp eq i32 %26, 0
  br i1 %27, label %28, label %33

28:                                               ; preds = %25
  %29 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 1
  %30 = load i32, ptr %29, align 4, !tbaa !18
  %31 = add i32 %30, %4
  %32 = icmp ult i32 %31, 12
  br i1 %32, label %48, label %62

33:                                               ; preds = %25
  %34 = lshr i32 %7, 24
  %35 = and i32 %34, 31
  %36 = add i32 %35, %4
  %37 = icmp ult i32 %36, 12
  br i1 %37, label %38, label %62

38:                                               ; preds = %33
  %39 = icmp eq i32 %35, 0
  %40 = select i1 %39, i32 -2147483648, i32 %7
  %41 = and i32 %40, %9
  %42 = getelementptr inbounds i8, ptr %0, i32 %35
  tail call fastcc void @copy_bytes(ptr noundef %42, ptr noundef %3, i32 noundef %4) #53
  %43 = icmp eq i32 %41, 0
  %44 = select i1 %43, i32 64, i32 192
  %45 = or disjoint i32 %44, %36
  %46 = trunc i32 %45 to i8
  %47 = getelementptr inbounds i8, ptr %0, i32 11
  store i8 %46, ptr %47, align 1, !tbaa !1
  br label %63

48:                                               ; preds = %28
  %49 = icmp eq i32 %30, 0
  %50 = select i1 %49, i32 -2147483648, i32 %7
  %51 = and i32 %50, %9
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %2) #56
  call void @llvm.memset.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %2, i8 0, i32 12, i1 false)
  %52 = load ptr, ptr %0, align 4, !tbaa !13
  call fastcc void @copy_bytes(ptr noundef nonnull %2, ptr noundef %52, i32 noundef %30) #53
  %53 = getelementptr inbounds i8, ptr %2, i32 %30
  call fastcc void @copy_bytes(ptr noundef nonnull %53, ptr noundef %3, i32 noundef %4) #53
  %54 = icmp eq i32 %51, 0
  %55 = select i1 %54, i32 64, i32 192
  %56 = or disjoint i32 %55, %31
  %57 = trunc i32 %56 to i8
  %58 = getelementptr inbounds i8, ptr %2, i32 11
  store i8 %57, ptr %58, align 1, !tbaa !1
  %59 = and i32 %7, 1073741823
  switch i32 %59, label %60 [
    i32 1073741823, label %61
    i32 0, label %61
  ]

60:                                               ; preds = %48
  tail call void @fa_free(ptr noundef %52, i32 noundef %59, i32 noundef 1) #54
  br label %61

61:                                               ; preds = %48, %48, %60
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %0, ptr noundef nonnull align 4 dereferenceable(12) %2, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %2) #56
  br label %63

62:                                               ; preds = %33, %28
  tail call fastcc void @string_append_slow(ptr noundef nonnull %0, ptr noundef %3, i32 noundef %4, i32 noundef %5) #53
  br label %63

63:                                               ; preds = %18, %23, %38, %61, %62
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: read)
define hidden i64 @fa_bytes_next(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2, i32 noundef %3) local_unnamed_addr #12 {
  %5 = icmp slt i64 %1, %2
  br i1 %5, label %6, label %12

6:                                                ; preds = %4
  %7 = trunc i64 %2 to i32
  %8 = trunc i64 %1 to i32
  %9 = trunc i32 %3 to i8
  %10 = tail call fastcc i32 @next_byte(ptr noundef %0, i32 noundef %7, i32 noundef %8, i8 noundef zeroext %9) #53
  %11 = zext i32 %10 to i64
  br label %12

12:                                               ; preds = %4, %6
  %13 = phi i64 [ %11, %6 ], [ %2, %4 ]
  ret i64 %13
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: read)
define internal fastcc i32 @next_byte(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2, i8 noundef zeroext %3) unnamed_addr #12 {
  %5 = zext i8 %3 to i64
  %6 = mul nuw i64 %5, 72340172838076673
  %7 = insertelement <16 x i8> poison, i8 %3, i64 0
  %8 = shufflevector <16 x i8> %7, <16 x i8> poison, <16 x i32> zeroinitializer
  br label %9

9:                                                ; preds = %13, %4
  %10 = phi i32 [ %2, %4 ], [ %11, %13 ]
  %11 = add i32 %10, 16
  %12 = icmp ugt i32 %11, %1
  br i1 %12, label %31, label %13

13:                                               ; preds = %9
  %14 = getelementptr inbounds i8, ptr %0, i32 %10
  %15 = load <16 x i8>, ptr %14, align 1
  %16 = icmp eq <16 x i8> %15, %8
  %17 = sext <16 x i1> %16 to <16 x i8>
  %18 = bitcast <16 x i8> %17 to <2 x i64>
  %19 = extractelement <2 x i64> %18, i64 0
  %20 = extractelement <2 x i64> %18, i64 1
  %21 = icmp eq i64 %19, 0
  %22 = select i1 %21, i64 %20, i64 %19
  %23 = icmp eq i64 %22, 0
  br i1 %23, label %9, label %24, !llvm.loop !40

24:                                               ; preds = %13
  %25 = select i1 %21, i32 8, i32 0
  %26 = add i32 %25, %10
  %27 = tail call i64 @llvm.cttz.i64(i64 %22, i1 true), !range !41
  %28 = trunc i64 %27 to i32
  %29 = lshr i32 %28, 3
  %30 = add i32 %26, %29
  br label %59

31:                                               ; preds = %9, %37
  %32 = phi i32 [ %33, %37 ], [ %10, %9 ]
  %33 = add i32 %32, 8
  %34 = icmp ugt i32 %33, %1
  br i1 %34, label %35, label %37

35:                                               ; preds = %31
  %36 = icmp ult i32 %32, %1
  br i1 %36, label %46, label %59

37:                                               ; preds = %31
  %38 = getelementptr inbounds i8, ptr %0, i32 %32
  %39 = load i64, ptr %38, align 1
  %40 = xor i64 %39, %6
  %41 = add i64 %40, -72340172838076673
  %42 = and i64 %40, -9187201950435737472
  %43 = xor i64 %42, -9187201950435737472
  %44 = and i64 %43, %41
  %45 = icmp eq i64 %44, 0
  br i1 %45, label %31, label %54, !llvm.loop !42

46:                                               ; preds = %35, %51
  %47 = phi i32 [ %52, %51 ], [ %32, %35 ]
  %48 = getelementptr inbounds i8, ptr %0, i32 %47
  %49 = load i8, ptr %48, align 1, !tbaa !1
  %50 = icmp eq i8 %49, %3
  br i1 %50, label %59, label %51

51:                                               ; preds = %46
  %52 = add nuw i32 %47, 1
  %53 = icmp ult i32 %52, %1
  br i1 %53, label %46, label %59, !llvm.loop !43

54:                                               ; preds = %37
  %55 = tail call i64 @llvm.cttz.i64(i64 %44, i1 true), !range !44
  %56 = trunc i64 %55 to i32
  %57 = lshr i32 %56, 3
  %58 = add i32 %57, %32
  br label %59

59:                                               ; preds = %46, %51, %35, %54, %24
  %60 = phi i32 [ %30, %24 ], [ %58, %54 ], [ %1, %35 ], [ %47, %46 ], [ %1, %51 ]
  ret i32 %60
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden i32 @fa_string_includes(ptr noundef %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3, i32 noundef %4) local_unnamed_addr #19 {
  %6 = and i32 %2, -2147483648
  %7 = tail call fastcc i64 @find(ptr noundef %0, i32 noundef %1, i32 noundef %6, ptr noundef %3, i32 noundef %4, i32 noundef 0) #53
  %8 = icmp sgt i64 %7, -1
  %9 = zext i1 %8 to i32
  ret i32 %9
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc i64 @find(ptr noundef %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3, i32 noundef %4, i32 noundef %5) unnamed_addr #19 {
  %7 = alloca %struct.Boundaries, align 4
  %8 = alloca %struct.Boundaries, align 4
  %9 = icmp ugt i32 %4, %1
  br i1 %9, label %120, label %10

10:                                               ; preds = %6
  %11 = icmp eq i32 %2, 0
  %12 = icmp eq i32 %4, 0
  %13 = or i1 %11, %12
  %14 = icmp ne i32 %5, 0
  %15 = or i1 %13, %14
  br i1 %15, label %45, label %16

16:                                               ; preds = %10
  %17 = load i8, ptr %3, align 1, !tbaa !1
  %18 = tail call fastcc i32 @next_byte(ptr noundef %0, i32 noundef %1, i32 noundef 0, i8 noundef zeroext %17) #53
  %19 = add i32 %18, %4
  %20 = icmp ugt i32 %19, %1
  br i1 %20, label %120, label %21

21:                                               ; preds = %16, %40
  %22 = phi i32 [ %42, %40 ], [ %18, %16 ]
  %23 = getelementptr i8, ptr %0, i32 %22
  %24 = load i8, ptr %23, align 1, !tbaa !1
  %25 = icmp eq i8 %24, %17
  br i1 %25, label %26, label %40

26:                                               ; preds = %21, %30
  %27 = phi i32 [ %28, %30 ], [ 0, %21 ]
  %28 = add nuw i32 %27, 1
  %29 = icmp eq i32 %28, %4
  br i1 %29, label %36, label %30, !llvm.loop !45

30:                                               ; preds = %26
  %31 = getelementptr i8, ptr %23, i32 %28
  %32 = load i8, ptr %31, align 1, !tbaa !1
  %33 = getelementptr inbounds i8, ptr %3, i32 %28
  %34 = load i8, ptr %33, align 1, !tbaa !1
  %35 = icmp eq i8 %32, %34
  br i1 %35, label %26, label %36, !llvm.loop !45

36:                                               ; preds = %30, %26
  %37 = icmp ult i32 %28, %4
  br i1 %37, label %40, label %38

38:                                               ; preds = %36
  %39 = zext i32 %22 to i64
  br label %120

40:                                               ; preds = %21, %36
  %41 = add i32 %22, 1
  %42 = tail call fastcc i32 @next_byte(ptr noundef nonnull %0, i32 noundef %1, i32 noundef %41, i8 noundef zeroext %17) #53
  %43 = add i32 %42, %4
  %44 = icmp ugt i32 %43, %1
  br i1 %44, label %120, label %21, !llvm.loop !46

45:                                               ; preds = %10
  br i1 %13, label %75, label %46

46:                                               ; preds = %45
  %47 = load i8, ptr %3, align 1, !tbaa !1
  %48 = sub i32 %1, %4
  %49 = icmp eq i32 %48, -1
  br i1 %49, label %120, label %50

50:                                               ; preds = %46, %72
  %51 = phi i32 [ %73, %72 ], [ 0, %46 ]
  %52 = sub i32 %48, %51
  %53 = select i1 %14, i32 %52, i32 %51
  %54 = getelementptr inbounds i8, ptr %0, i32 %53
  %55 = load i8, ptr %54, align 1, !tbaa !1
  %56 = icmp ne i8 %55, %47
  %57 = add i32 %53, %4
  %58 = icmp ugt i32 %57, %1
  %59 = or i1 %58, %56
  br i1 %59, label %72, label %60

60:                                               ; preds = %50, %64
  %61 = phi i32 [ %62, %64 ], [ 0, %50 ]
  %62 = add nuw i32 %61, 1
  %63 = icmp eq i32 %62, %4
  br i1 %63, label %70, label %64, !llvm.loop !45

64:                                               ; preds = %60
  %65 = getelementptr i8, ptr %54, i32 %62
  %66 = load i8, ptr %65, align 1, !tbaa !1
  %67 = getelementptr inbounds i8, ptr %3, i32 %62
  %68 = load i8, ptr %67, align 1, !tbaa !1
  %69 = icmp eq i8 %66, %68
  br i1 %69, label %60, label %70, !llvm.loop !45

70:                                               ; preds = %64, %60
  %71 = icmp ult i32 %62, %4
  br i1 %71, label %72, label %118

72:                                               ; preds = %50, %70
  %73 = add nuw i32 %51, 1
  %74 = icmp eq i32 %51, %48
  br i1 %74, label %120, label %50, !llvm.loop !47

75:                                               ; preds = %45
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %7) #56
  store ptr %0, ptr %7, align 4, !tbaa !48
  %76 = getelementptr inbounds %struct.Boundaries, ptr %7, i32 0, i32 1
  store i32 %1, ptr %76, align 4, !tbaa !50
  %77 = getelementptr inbounds %struct.Boundaries, ptr %7, i32 0, i32 2
  store i32 0, ptr %77, align 4, !tbaa !51
  %78 = getelementptr inbounds %struct.Boundaries, ptr %7, i32 0, i32 3
  store i32 %2, ptr %78, align 4, !tbaa !52
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %8) #56
  store ptr %0, ptr %8, align 4, !tbaa !48
  %79 = getelementptr inbounds %struct.Boundaries, ptr %8, i32 0, i32 1
  store i32 %1, ptr %79, align 4, !tbaa !50
  %80 = getelementptr inbounds %struct.Boundaries, ptr %8, i32 0, i32 2
  store i32 0, ptr %80, align 4, !tbaa !51
  %81 = getelementptr inbounds %struct.Boundaries, ptr %8, i32 0, i32 3
  store i32 %2, ptr %81, align 4, !tbaa !52
  br label %82

82:                                               ; preds = %75, %111
  %83 = phi i32 [ %114, %111 ], [ %4, %75 ]
  %84 = phi i32 [ %113, %111 ], [ 0, %75 ]
  %85 = phi i64 [ %112, %111 ], [ -1, %75 ]
  %86 = getelementptr i8, ptr %0, i32 %84
  br i1 %12, label %103, label %87

87:                                               ; preds = %82
  %88 = load i8, ptr %86, align 1, !tbaa !1
  %89 = load i8, ptr %3, align 1, !tbaa !1
  %90 = icmp eq i8 %88, %89
  br i1 %90, label %91, label %111

91:                                               ; preds = %87, %95
  %92 = phi i32 [ %93, %95 ], [ 0, %87 ]
  %93 = add nuw i32 %92, 1
  %94 = icmp eq i32 %93, %4
  br i1 %94, label %101, label %95, !llvm.loop !45

95:                                               ; preds = %91
  %96 = getelementptr i8, ptr %86, i32 %93
  %97 = load i8, ptr %96, align 1, !tbaa !1
  %98 = getelementptr inbounds i8, ptr %3, i32 %93
  %99 = load i8, ptr %98, align 1, !tbaa !1
  %100 = icmp eq i8 %97, %99
  br i1 %100, label %91, label %101, !llvm.loop !45

101:                                              ; preds = %95, %91
  %102 = icmp ult i32 %93, %4
  br i1 %102, label %111, label %103

103:                                              ; preds = %82, %101
  %104 = call fastcc i32 @at_boundary(ptr noundef nonnull %7, i32 noundef %84) #53, !range !53
  %105 = icmp eq i32 %104, 0
  br i1 %105, label %111, label %106

106:                                              ; preds = %103
  %107 = call fastcc i32 @at_boundary(ptr noundef nonnull %8, i32 noundef %83) #53, !range !53
  %108 = icmp eq i32 %107, 0
  br i1 %108, label %111, label %109

109:                                              ; preds = %106
  %110 = zext i32 %84 to i64
  br i1 %14, label %111, label %116

111:                                              ; preds = %87, %101, %103, %106, %109
  %112 = phi i64 [ %110, %109 ], [ %85, %106 ], [ %85, %103 ], [ %85, %101 ], [ %85, %87 ]
  %113 = add i32 %84, 1
  %114 = add i32 %113, %4
  %115 = icmp ugt i32 %114, %1
  br i1 %115, label %116, label %82, !llvm.loop !54

116:                                              ; preds = %111, %109
  %117 = phi i64 [ %112, %111 ], [ %110, %109 ]
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %8) #56
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %7) #56
  br label %120

118:                                              ; preds = %70
  %119 = zext i32 %53 to i64
  br label %120

120:                                              ; preds = %40, %72, %16, %118, %46, %38, %6, %116
  %121 = phi i64 [ %117, %116 ], [ -1, %6 ], [ %39, %38 ], [ %119, %118 ], [ -1, %46 ], [ -1, %16 ], [ -1, %72 ], [ -1, %40 ]
  ret i64 %121
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define hidden i32 @fa_string_starts_with(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3, i32 noundef %4) local_unnamed_addr #18 {
  %6 = icmp ugt i32 %4, %1
  br i1 %6, label %37, label %7

7:                                                ; preds = %5
  %8 = icmp eq i32 %4, 0
  br i1 %8, label %27, label %9

9:                                                ; preds = %7
  %10 = load i8, ptr %0, align 1, !tbaa !1
  %11 = load i8, ptr %3, align 1, !tbaa !1
  %12 = icmp eq i8 %10, %11
  br i1 %12, label %13, label %37

13:                                               ; preds = %9, %17
  %14 = phi i32 [ %15, %17 ], [ 0, %9 ]
  %15 = add nuw i32 %14, 1
  %16 = icmp eq i32 %15, %4
  br i1 %16, label %23, label %17, !llvm.loop !45

17:                                               ; preds = %13
  %18 = getelementptr i8, ptr %0, i32 %15
  %19 = load i8, ptr %18, align 1, !tbaa !1
  %20 = getelementptr inbounds i8, ptr %3, i32 %15
  %21 = load i8, ptr %20, align 1, !tbaa !1
  %22 = icmp eq i8 %19, %21
  br i1 %22, label %13, label %23, !llvm.loop !45

23:                                               ; preds = %17, %13
  %24 = icmp ult i32 %15, %4
  br i1 %24, label %37, label %25

25:                                               ; preds = %23
  %26 = icmp sgt i32 %2, -1
  br i1 %26, label %29, label %37

27:                                               ; preds = %7
  %28 = icmp sgt i32 %2, -1
  br i1 %28, label %33, label %37

29:                                               ; preds = %25, %29
  %30 = phi i32 [ %31, %29 ], [ 0, %25 ]
  %31 = tail call fastcc i32 @next_boundary(ptr noundef nonnull %0, i32 noundef %1, i32 noundef %30) #53
  %32 = icmp ult i32 %31, %4
  br i1 %32, label %29, label %33, !llvm.loop !55

33:                                               ; preds = %29, %27
  %34 = phi i32 [ 0, %27 ], [ %31, %29 ]
  %35 = icmp eq i32 %34, %4
  %36 = zext i1 %35 to i32
  br label %37

37:                                               ; preds = %9, %5, %27, %25, %33, %23
  %38 = phi i32 [ 0, %23 ], [ 1, %25 ], [ %36, %33 ], [ 1, %27 ], [ 0, %5 ], [ 0, %9 ]
  ret i32 %38
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define hidden i32 @fa_string_ends_with(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3, i32 noundef %4) local_unnamed_addr #18 {
  %6 = icmp ugt i32 %4, %1
  br i1 %6, label %39, label %7

7:                                                ; preds = %5
  %8 = sub i32 %1, %4
  %9 = getelementptr i8, ptr %0, i32 %8
  %10 = icmp eq i32 %4, 0
  br i1 %10, label %27, label %11

11:                                               ; preds = %7
  %12 = load i8, ptr %9, align 1, !tbaa !1
  %13 = load i8, ptr %3, align 1, !tbaa !1
  %14 = icmp eq i8 %12, %13
  br i1 %14, label %15, label %39

15:                                               ; preds = %11, %19
  %16 = phi i32 [ %17, %19 ], [ 0, %11 ]
  %17 = add nuw i32 %16, 1
  %18 = icmp eq i32 %17, %4
  br i1 %18, label %25, label %19, !llvm.loop !45

19:                                               ; preds = %15
  %20 = getelementptr i8, ptr %9, i32 %17
  %21 = load i8, ptr %20, align 1, !tbaa !1
  %22 = getelementptr inbounds i8, ptr %3, i32 %17
  %23 = load i8, ptr %22, align 1, !tbaa !1
  %24 = icmp eq i8 %21, %23
  br i1 %24, label %15, label %25, !llvm.loop !45

25:                                               ; preds = %19, %15
  %26 = icmp ult i32 %17, %4
  br i1 %26, label %39, label %27

27:                                               ; preds = %7, %25
  %28 = icmp sgt i32 %2, -1
  br i1 %28, label %29, label %39

29:                                               ; preds = %27
  %30 = icmp eq i32 %1, %4
  br i1 %30, label %35, label %31

31:                                               ; preds = %29, %31
  %32 = phi i32 [ %33, %31 ], [ 0, %29 ]
  %33 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %32) #53
  %34 = icmp ult i32 %33, %8
  br i1 %34, label %31, label %35, !llvm.loop !55

35:                                               ; preds = %31, %29
  %36 = phi i32 [ 0, %29 ], [ %33, %31 ]
  %37 = icmp eq i32 %36, %8
  %38 = zext i1 %37 to i32
  br label %39

39:                                               ; preds = %11, %27, %35, %25, %5
  %40 = phi i32 [ 0, %25 ], [ 0, %5 ], [ 1, %27 ], [ %38, %35 ], [ 0, %11 ]
  ret i32 %40
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden i64 @fa_string_index_of(ptr noundef %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3, i32 noundef %4, i32 noundef %5) local_unnamed_addr #19 {
  %7 = icmp sgt i32 %2, -1
  %8 = icmp ne i32 %4, 1
  %9 = or i1 %7, %8
  %10 = icmp ne i32 %5, 0
  %11 = or i1 %9, %10
  %12 = icmp ugt i32 %1, 32
  %13 = or i1 %12, %11
  br i1 %13, label %28, label %14

14:                                               ; preds = %6
  %15 = icmp eq i32 %1, 0
  br i1 %15, label %30, label %16

16:                                               ; preds = %14
  %17 = load i8, ptr %3, align 1, !tbaa !1
  br label %18

18:                                               ; preds = %16, %25
  %19 = phi i32 [ 0, %16 ], [ %26, %25 ]
  %20 = getelementptr inbounds i8, ptr %0, i32 %19
  %21 = load i8, ptr %20, align 1, !tbaa !1
  %22 = icmp eq i8 %21, %17
  br i1 %22, label %23, label %25

23:                                               ; preds = %18
  %24 = zext nneg i32 %19 to i64
  br label %30

25:                                               ; preds = %18
  %26 = add nuw i32 %19, 1
  %27 = icmp eq i32 %26, %1
  br i1 %27, label %30, label %18, !llvm.loop !56

28:                                               ; preds = %6
  %29 = tail call fastcc i64 @string_index_of(ptr noundef %0, i32 noundef %1, i32 noundef %2, ptr noundef %3, i32 noundef %4, i32 noundef %5) #53
  br label %30

30:                                               ; preds = %25, %14, %23, %28
  %31 = phi i64 [ %29, %28 ], [ %24, %23 ], [ -1, %14 ], [ -1, %25 ]
  ret i64 %31
}

; Function Attrs: nofree noinline norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc i64 @string_index_of(ptr noundef %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3, i32 noundef %4, i32 noundef %5) unnamed_addr #20 {
  %7 = and i32 %2, -2147483648
  %8 = tail call fastcc i64 @find(ptr noundef %0, i32 noundef %1, i32 noundef %7, ptr noundef %3, i32 noundef %4, i32 noundef %5) #53
  %9 = icmp slt i64 %8, 0
  %10 = icmp ne i32 %7, 0
  %11 = or i1 %10, %9
  br i1 %11, label %23, label %12

12:                                               ; preds = %6
  %13 = trunc i64 %8 to i32
  %14 = icmp eq i32 %13, 0
  br i1 %14, label %23, label %15

15:                                               ; preds = %12, %15
  %16 = phi i32 [ %19, %15 ], [ 0, %12 ]
  %17 = phi i32 [ %18, %15 ], [ 0, %12 ]
  %18 = add i32 %17, 1
  %19 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %16) #53
  %20 = icmp ult i32 %19, %13
  br i1 %20, label %15, label %21, !llvm.loop !57

21:                                               ; preds = %15
  %22 = zext i32 %18 to i64
  br label %23

23:                                               ; preds = %12, %21, %6
  %24 = phi i64 [ %8, %6 ], [ 0, %12 ], [ %22, %21 ]
  ret i64 %24
}

; Function Attrs: nounwind optsize
define hidden void @fa_string_replace(ptr noundef %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3, i32 noundef %4, ptr noundef %5, i32 noundef %6, i32 noundef %7, ptr nocapture noundef writeonly %8) local_unnamed_addr #3 {
  %10 = alloca %struct.FaString, align 4
  %11 = alloca %struct.Replacing, align 4
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  call void @llvm.lifetime.start.p0(i64 28, ptr nonnull %11) #56
  store ptr %0, ptr %11, align 4, !tbaa !58
  %12 = getelementptr inbounds %struct.Replacing, ptr %11, i32 0, i32 1
  store i32 0, ptr %12, align 4, !tbaa !60
  %13 = getelementptr inbounds %struct.Replacing, ptr %11, i32 0, i32 2
  store i32 %4, ptr %13, align 4, !tbaa !61
  %14 = getelementptr inbounds %struct.Replacing, ptr %11, i32 0, i32 3
  store ptr %5, ptr %14, align 4, !tbaa !62
  %15 = getelementptr inbounds %struct.Replacing, ptr %11, i32 0, i32 4
  store i32 %6, ptr %15, align 4, !tbaa !63
  %16 = getelementptr inbounds %struct.Replacing, ptr %11, i32 0, i32 5
  store i32 %7, ptr %16, align 4, !tbaa !64
  %17 = getelementptr inbounds %struct.Replacing, ptr %11, i32 0, i32 6
  store i32 0, ptr %17, align 4, !tbaa !65
  %18 = and i32 %2, -2147483648
  call fastcc void @each_match(ptr noundef %0, i32 noundef %1, i32 noundef %18, ptr noundef %3, i32 noundef %4, ptr noundef nonnull @replace_one, ptr noundef nonnull %11) #53
  %19 = load i32, ptr %12, align 4, !tbaa !60
  %20 = getelementptr inbounds i8, ptr %0, i32 %19
  %21 = sub i32 %1, %19
  call fastcc void @reserve(i32 noundef %21) #53
  %22 = load ptr, ptr @builder, align 4, !tbaa !6
  %23 = load i32, ptr @builder_length, align 4, !tbaa !4
  %24 = getelementptr inbounds i8, ptr %22, i32 %23
  call fastcc void @copy_bytes(ptr noundef %24, ptr noundef %20, i32 noundef %21) #53
  %25 = add i32 %23, %21
  store i32 %25, ptr @builder_length, align 4, !tbaa !4
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %10) #56
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %10, ptr noundef %22, i32 noundef %25, i32 noundef 0) #53
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %8, ptr noundef nonnull align 4 dereferenceable(12) %10, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %10) #56
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  call void @llvm.lifetime.end.p0(i64 28, ptr nonnull %11) #56
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @each_match(ptr noundef %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3, i32 noundef %4, ptr nocapture noundef readonly %5, ptr noundef %6) unnamed_addr #3 {
  %8 = alloca %struct.Boundaries, align 4
  %9 = alloca %struct.Boundaries, align 4
  %10 = icmp ne i32 %2, 0
  %11 = icmp ne i32 %4, 0
  %12 = and i1 %10, %11
  br i1 %12, label %13, label %47

13:                                               ; preds = %7
  %14 = load i8, ptr %3, align 1, !tbaa !1
  %15 = tail call fastcc i32 @next_byte(ptr noundef %0, i32 noundef %1, i32 noundef 0, i8 noundef zeroext %14) #53
  %16 = add i32 %15, %4
  %17 = icmp ugt i32 %16, %1
  br i1 %17, label %91, label %18

18:                                               ; preds = %13, %41
  %19 = phi i8 [ %42, %41 ], [ %14, %13 ]
  %20 = phi i32 [ %45, %41 ], [ %16, %13 ]
  %21 = phi i32 [ %44, %41 ], [ %15, %13 ]
  %22 = getelementptr i8, ptr %0, i32 %21
  %23 = load i8, ptr %22, align 1, !tbaa !1
  %24 = icmp eq i8 %23, %19
  br i1 %24, label %25, label %39

25:                                               ; preds = %18, %29
  %26 = phi i32 [ %27, %29 ], [ 0, %18 ]
  %27 = add nuw i32 %26, 1
  %28 = icmp eq i32 %27, %4
  br i1 %28, label %35, label %29, !llvm.loop !45

29:                                               ; preds = %25
  %30 = getelementptr i8, ptr %22, i32 %27
  %31 = load i8, ptr %30, align 1, !tbaa !1
  %32 = getelementptr inbounds i8, ptr %3, i32 %27
  %33 = load i8, ptr %32, align 1, !tbaa !1
  %34 = icmp eq i8 %31, %33
  br i1 %34, label %25, label %35, !llvm.loop !45

35:                                               ; preds = %29, %25
  %36 = icmp ult i32 %27, %4
  br i1 %36, label %39, label %37

37:                                               ; preds = %35
  tail call void %5(ptr noundef %6, i32 noundef %21) #54, !callees !66
  %38 = load i8, ptr %3, align 1, !tbaa !1
  br label %41

39:                                               ; preds = %18, %35
  %40 = add i32 %21, 1
  br label %41

41:                                               ; preds = %39, %37
  %42 = phi i8 [ %38, %37 ], [ %19, %39 ]
  %43 = phi i32 [ %20, %37 ], [ %40, %39 ]
  %44 = tail call fastcc i32 @next_byte(ptr noundef nonnull %0, i32 noundef %1, i32 noundef %43, i8 noundef zeroext %42) #53
  %45 = add i32 %44, %4
  %46 = icmp ugt i32 %45, %1
  br i1 %46, label %91, label %18, !llvm.loop !67

47:                                               ; preds = %7
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %8) #56
  store ptr %0, ptr %8, align 4, !tbaa !48
  %48 = getelementptr inbounds %struct.Boundaries, ptr %8, i32 0, i32 1
  store i32 %1, ptr %48, align 4, !tbaa !50
  %49 = getelementptr inbounds %struct.Boundaries, ptr %8, i32 0, i32 2
  store i32 0, ptr %49, align 4, !tbaa !51
  %50 = getelementptr inbounds %struct.Boundaries, ptr %8, i32 0, i32 3
  store i32 %2, ptr %50, align 4, !tbaa !52
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %9) #56
  store ptr %0, ptr %9, align 4, !tbaa !48
  %51 = getelementptr inbounds %struct.Boundaries, ptr %9, i32 0, i32 1
  store i32 %1, ptr %51, align 4, !tbaa !50
  %52 = getelementptr inbounds %struct.Boundaries, ptr %9, i32 0, i32 2
  store i32 0, ptr %52, align 4, !tbaa !51
  %53 = getelementptr inbounds %struct.Boundaries, ptr %9, i32 0, i32 3
  store i32 %2, ptr %53, align 4, !tbaa !52
  %54 = icmp ugt i32 %4, %1
  br i1 %54, label %90, label %55

55:                                               ; preds = %47
  %56 = icmp eq i32 %4, 0
  %57 = tail call i32 @llvm.umax.i32(i32 %4, i32 1)
  br label %58

58:                                               ; preds = %55, %85
  %59 = phi i32 [ %4, %55 ], [ %88, %85 ]
  %60 = phi i32 [ 0, %55 ], [ %87, %85 ]
  %61 = getelementptr i8, ptr %0, i32 %60
  br i1 %56, label %78, label %62

62:                                               ; preds = %58
  %63 = load i8, ptr %61, align 1, !tbaa !1
  %64 = load i8, ptr %3, align 1, !tbaa !1
  %65 = icmp eq i8 %63, %64
  br i1 %65, label %66, label %85

66:                                               ; preds = %62, %70
  %67 = phi i32 [ %68, %70 ], [ 0, %62 ]
  %68 = add nuw i32 %67, 1
  %69 = icmp eq i32 %68, %4
  br i1 %69, label %76, label %70, !llvm.loop !45

70:                                               ; preds = %66
  %71 = getelementptr i8, ptr %61, i32 %68
  %72 = load i8, ptr %71, align 1, !tbaa !1
  %73 = getelementptr inbounds i8, ptr %3, i32 %68
  %74 = load i8, ptr %73, align 1, !tbaa !1
  %75 = icmp eq i8 %72, %74
  br i1 %75, label %66, label %76, !llvm.loop !45

76:                                               ; preds = %70, %66
  %77 = icmp ult i32 %68, %4
  br i1 %77, label %85, label %78

78:                                               ; preds = %58, %76
  %79 = call fastcc i32 @at_boundary(ptr noundef nonnull %8, i32 noundef %60) #53, !range !53
  %80 = icmp eq i32 %79, 0
  br i1 %80, label %85, label %81

81:                                               ; preds = %78
  %82 = call fastcc i32 @at_boundary(ptr noundef nonnull %9, i32 noundef %59) #53, !range !53
  %83 = icmp eq i32 %82, 0
  br i1 %83, label %85, label %84

84:                                               ; preds = %81
  tail call void %5(ptr noundef %6, i32 noundef %60) #54, !callees !66
  br label %85

85:                                               ; preds = %62, %76, %78, %81, %84
  %86 = phi i32 [ %57, %84 ], [ 1, %81 ], [ 1, %78 ], [ 1, %76 ], [ 1, %62 ]
  %87 = add i32 %86, %60
  %88 = add i32 %87, %4
  %89 = icmp ugt i32 %88, %1
  br i1 %89, label %90, label %58, !llvm.loop !68

90:                                               ; preds = %85, %47
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %9) #56
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %8) #56
  br label %91

91:                                               ; preds = %41, %13, %90
  ret void
}

; Function Attrs: nounwind optsize
define internal void @replace_one(ptr nocapture noundef %0, i32 noundef %1) #3 {
  %3 = getelementptr inbounds %struct.Replacing, ptr %0, i32 0, i32 6
  %4 = load i32, ptr %3, align 4, !tbaa !65
  %5 = icmp eq i32 %4, 0
  br i1 %5, label %6, label %31

6:                                                ; preds = %2
  %7 = load ptr, ptr %0, align 4, !tbaa !58
  %8 = getelementptr inbounds %struct.Replacing, ptr %0, i32 0, i32 1
  %9 = load i32, ptr %8, align 4, !tbaa !60
  %10 = getelementptr inbounds i8, ptr %7, i32 %9
  %11 = sub i32 %1, %9
  tail call fastcc void @reserve(i32 noundef %11) #53
  %12 = load ptr, ptr @builder, align 4, !tbaa !6
  %13 = load i32, ptr @builder_length, align 4, !tbaa !4
  %14 = getelementptr inbounds i8, ptr %12, i32 %13
  tail call fastcc void @copy_bytes(ptr noundef %14, ptr noundef %10, i32 noundef %11) #53
  %15 = add i32 %13, %11
  store i32 %15, ptr @builder_length, align 4, !tbaa !4
  %16 = getelementptr inbounds %struct.Replacing, ptr %0, i32 0, i32 3
  %17 = load ptr, ptr %16, align 4, !tbaa !62
  %18 = getelementptr inbounds %struct.Replacing, ptr %0, i32 0, i32 4
  %19 = load i32, ptr %18, align 4, !tbaa !63
  tail call fastcc void @reserve(i32 noundef %19) #53
  %20 = load ptr, ptr @builder, align 4, !tbaa !6
  %21 = load i32, ptr @builder_length, align 4, !tbaa !4
  %22 = getelementptr inbounds i8, ptr %20, i32 %21
  tail call fastcc void @copy_bytes(ptr noundef %22, ptr noundef %17, i32 noundef %19) #53
  %23 = add i32 %21, %19
  store i32 %23, ptr @builder_length, align 4, !tbaa !4
  %24 = getelementptr inbounds %struct.Replacing, ptr %0, i32 0, i32 2
  %25 = load i32, ptr %24, align 4, !tbaa !61
  %26 = add i32 %25, %1
  store i32 %26, ptr %8, align 4, !tbaa !60
  %27 = getelementptr inbounds %struct.Replacing, ptr %0, i32 0, i32 5
  %28 = load i32, ptr %27, align 4, !tbaa !64
  %29 = icmp eq i32 %28, 0
  %30 = zext i1 %29 to i32
  store i32 %30, ptr %3, align 4, !tbaa !65
  br label %31

31:                                               ; preds = %2, %6
  ret void
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_string_characters(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2) local_unnamed_addr #3 {
  %4 = tail call fastcc ptr @characters_of(ptr noundef %0, i32 noundef %1, i32 noundef %2) #53
  ret ptr %4
}

; Function Attrs: nounwind optsize
define internal fastcc ptr @characters_of(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2) unnamed_addr #3 {
  %4 = alloca %struct.FaString, align 4
  %5 = icmp sgt i32 %2, -1
  br i1 %5, label %29, label %6

6:                                                ; preds = %3
  %7 = load i32, ptr @fa_compact_characters, align 4, !tbaa !23
  %8 = icmp eq i32 %7, 0
  %9 = zext i32 %1 to i64
  br i1 %8, label %10, label %25

10:                                               ; preds = %6
  %11 = tail call ptr @fa_array_new(i64 noundef %9, i64 noundef %9, i64 noundef 12) #53
  %12 = load ptr, ptr %11, align 8, !tbaa !69
  %13 = icmp eq i32 %1, 0
  br i1 %13, label %41, label %14

14:                                               ; preds = %10, %14
  %15 = phi i32 [ %23, %14 ], [ 0, %10 ]
  %16 = getelementptr inbounds %struct.FaString, ptr %12, i32 %15
  %17 = getelementptr inbounds i8, ptr %0, i32 %15
  %18 = load i8, ptr %17, align 1, !tbaa !1
  %19 = zext i8 %18 to i32
  %20 = getelementptr inbounds i8, ptr @ascii_table, i32 %19
  store ptr %20, ptr %16, align 4, !tbaa.struct !10
  %21 = getelementptr inbounds i8, ptr %16, i32 4
  store i32 1, ptr %21, align 4, !tbaa.struct !11
  %22 = getelementptr inbounds %struct.FaString, ptr %12, i32 %15, i32 2
  store i32 -2147483648, ptr %22, align 4, !tbaa !19
  %23 = add nuw i32 %15, 1
  %24 = icmp eq i32 %23, %1
  br i1 %24, label %41, label %14, !llvm.loop !72

25:                                               ; preds = %6
  %26 = tail call ptr @fa_array_new(i64 noundef %9, i64 noundef %9, i64 noundef 1) #53
  %27 = load ptr, ptr %26, align 8, !tbaa !69
  tail call fastcc void @copy_bytes(ptr noundef %27, ptr noundef %0, i32 noundef %1) #53
  %28 = getelementptr inbounds %struct.FaArrayHeader, ptr %26, i32 0, i32 2
  store i64 -1, ptr %28, align 8, !tbaa !73
  br label %41

29:                                               ; preds = %3
  %30 = tail call i32 @fa_graphemes(ptr noundef %0, i32 noundef %1) #53
  %31 = zext i32 %30 to i64
  %32 = tail call ptr @fa_array_new(i64 noundef 0, i64 noundef %31, i64 noundef 12) #53
  %33 = icmp eq i32 %1, 0
  br i1 %33, label %41, label %34

34:                                               ; preds = %29, %34
  %35 = phi i32 [ %36, %34 ], [ 0, %29 ]
  %36 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %35) #53
  %37 = tail call ptr @fa_array_push_slot(ptr noundef %32, i64 noundef 12) #53
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %4) #56
  %38 = getelementptr inbounds i8, ptr %0, i32 %35
  %39 = sub i32 %36, %35
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %4, ptr noundef %38, i32 noundef %39, i32 noundef 0) #53
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %37, ptr noundef nonnull align 4 dereferenceable(12) %4, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %4) #56
  %40 = icmp ult i32 %36, %1
  br i1 %40, label %34, label %41, !llvm.loop !74

41:                                               ; preds = %14, %34, %10, %29, %25
  %42 = phi ptr [ %26, %25 ], [ %32, %29 ], [ %11, %10 ], [ %32, %34 ], [ %11, %14 ]
  ret ptr %42
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_string_split(ptr noundef %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3, i32 noundef %4) local_unnamed_addr #3 {
  %6 = alloca %struct.Splitting, align 4
  %7 = alloca %struct.FaString, align 4
  %8 = icmp eq i32 %4, 0
  br i1 %8, label %9, label %11

9:                                                ; preds = %5
  %10 = tail call fastcc ptr @characters_of(ptr noundef %0, i32 noundef %1, i32 noundef %2) #53
  br label %28

11:                                               ; preds = %5
  call void @llvm.lifetime.start.p0(i64 20, ptr nonnull %6) #56
  store ptr %0, ptr %6, align 4, !tbaa !75
  %12 = getelementptr inbounds %struct.Splitting, ptr %6, i32 0, i32 1
  store i32 0, ptr %12, align 4, !tbaa !77
  %13 = getelementptr inbounds %struct.Splitting, ptr %6, i32 0, i32 2
  store i32 %4, ptr %13, align 4, !tbaa !78
  %14 = getelementptr inbounds %struct.Splitting, ptr %6, i32 0, i32 3
  %15 = tail call ptr @fa_alloc(i32 noundef 80, i32 noundef 16) #54
  %16 = getelementptr inbounds i8, ptr %15, i32 24
  store i64 1, ptr %16, align 1
  %17 = getelementptr inbounds i8, ptr %15, i32 32
  store ptr %17, ptr %15, align 8, !tbaa !69
  %18 = getelementptr inbounds %struct.FaArrayHeader, ptr %15, i32 0, i32 1
  store i64 0, ptr %18, align 8, !tbaa !79
  %19 = getelementptr inbounds %struct.FaArrayHeader, ptr %15, i32 0, i32 2
  store i64 4, ptr %19, align 8, !tbaa !73
  store ptr %15, ptr %14, align 4, !tbaa !80
  %20 = getelementptr inbounds %struct.Splitting, ptr %6, i32 0, i32 4
  %21 = and i32 %2, -2147483648
  store i32 %21, ptr %20, align 4, !tbaa !81
  call fastcc void @each_match(ptr noundef %0, i32 noundef %1, i32 noundef %21, ptr noundef %3, i32 noundef %4, ptr noundef nonnull @split_one, ptr noundef nonnull %6) #53
  %22 = load ptr, ptr %14, align 4, !tbaa !80
  %23 = call ptr @fa_array_push_slot(ptr noundef %22, i64 noundef 12) #53
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %7) #56
  %24 = load i32, ptr %12, align 4, !tbaa !77
  %25 = getelementptr inbounds i8, ptr %0, i32 %24
  %26 = sub i32 %1, %24
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %7, ptr noundef %25, i32 noundef %26, i32 noundef %21) #53
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %23, ptr noundef nonnull align 4 dereferenceable(12) %7, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %7) #56
  %27 = load ptr, ptr %14, align 4, !tbaa !80
  call void @llvm.lifetime.end.p0(i64 20, ptr nonnull %6) #56
  br label %28

28:                                               ; preds = %11, %9
  %29 = phi ptr [ %10, %9 ], [ %27, %11 ]
  ret ptr %29
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_array_new(i64 noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #3 {
  %4 = tail call i64 @llvm.smax.i64(i64 %1, i64 %0)
  %5 = mul nsw i64 %4, %2
  %6 = trunc i64 %5 to i32
  %7 = icmp ult i32 %6, 97
  br i1 %7, label %8, label %13

8:                                                ; preds = %3
  %9 = add nuw nsw i32 %6, 32
  %10 = tail call ptr @fa_alloc(i32 noundef %9, i32 noundef 16) #54
  %11 = getelementptr inbounds i8, ptr %10, i32 24
  store i64 1, ptr %11, align 1
  %12 = getelementptr inbounds i8, ptr %10, i32 32
  store ptr %12, ptr %10, align 8, !tbaa !69
  br label %17

13:                                               ; preds = %3
  %14 = tail call ptr @fa_alloc(i32 noundef 32, i32 noundef 8) #54
  %15 = getelementptr inbounds i8, ptr %14, i32 24
  store i64 0, ptr %15, align 1
  %16 = tail call ptr @fa_alloc(i32 noundef %6, i32 noundef 16) #54
  store ptr %16, ptr %14, align 8, !tbaa !69
  br label %17

17:                                               ; preds = %13, %8
  %18 = phi ptr [ %14, %13 ], [ %10, %8 ]
  %19 = getelementptr inbounds %struct.FaArrayHeader, ptr %18, i32 0, i32 1
  store i64 %0, ptr %19, align 8, !tbaa !79
  %20 = getelementptr inbounds %struct.FaArrayHeader, ptr %18, i32 0, i32 2
  store i64 %4, ptr %20, align 8, !tbaa !73
  ret ptr %18
}

; Function Attrs: nounwind optsize
define internal void @split_one(ptr nocapture noundef %0, i32 noundef %1) #3 {
  %3 = alloca %struct.FaString, align 4
  %4 = getelementptr inbounds %struct.Splitting, ptr %0, i32 0, i32 3
  %5 = load ptr, ptr %4, align 4, !tbaa !80
  %6 = tail call ptr @fa_array_push_slot(ptr noundef %5, i64 noundef 12) #53
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %3) #56
  %7 = load ptr, ptr %0, align 4, !tbaa !75
  %8 = getelementptr inbounds %struct.Splitting, ptr %0, i32 0, i32 1
  %9 = load i32, ptr %8, align 4, !tbaa !77
  %10 = getelementptr inbounds i8, ptr %7, i32 %9
  %11 = sub i32 %1, %9
  %12 = getelementptr inbounds %struct.Splitting, ptr %0, i32 0, i32 4
  %13 = load i32, ptr %12, align 4, !tbaa !81
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %3, ptr noundef %10, i32 noundef %11, i32 noundef %13) #53
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %6, ptr noundef nonnull align 4 dereferenceable(12) %3, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %3) #56
  %14 = getelementptr inbounds %struct.Splitting, ptr %0, i32 0, i32 2
  %15 = load i32, ptr %14, align 4, !tbaa !78
  %16 = add i32 %15, %1
  store i32 %16, ptr %8, align 4, !tbaa !77
  ret void
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_array_push_slot(ptr noundef %0, i64 noundef %1) local_unnamed_addr #3 {
  %3 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %4 = load i64, ptr %3, align 8, !tbaa !79
  %5 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %6 = load i64, ptr %5, align 8, !tbaa !73
  %7 = icmp slt i64 %4, %6
  br i1 %7, label %10, label %8, !prof !82

8:                                                ; preds = %2
  tail call fastcc void @array_grow(ptr noundef nonnull %0, i64 noundef %1) #53
  %9 = load i64, ptr %3, align 8, !tbaa !79
  br label %10

10:                                               ; preds = %8, %2
  %11 = phi i64 [ %9, %8 ], [ %4, %2 ]
  %12 = load ptr, ptr %0, align 8, !tbaa !69
  %13 = mul nsw i64 %11, %1
  %14 = trunc i64 %13 to i32
  %15 = getelementptr inbounds i8, ptr %12, i32 %14
  %16 = add nsw i64 %11, 1
  store i64 %16, ptr %3, align 8, !tbaa !79
  ret ptr %15
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_string_code_points(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2) local_unnamed_addr #3 {
  %4 = alloca i32, align 4
  %5 = icmp sgt i32 %2, -1
  br i1 %5, label %6, label %19

6:                                                ; preds = %3
  %7 = icmp eq i32 %1, 0
  br i1 %7, label %19, label %8

8:                                                ; preds = %6, %8
  %9 = phi i32 [ %16, %8 ], [ 0, %6 ]
  %10 = phi i32 [ %17, %8 ], [ 0, %6 ]
  %11 = getelementptr inbounds i8, ptr %0, i32 %10
  %12 = load i8, ptr %11, align 1, !tbaa !1
  %13 = and i8 %12, -64
  %14 = icmp ne i8 %13, -128
  %15 = zext i1 %14 to i32
  %16 = add i32 %9, %15
  %17 = add nuw i32 %10, 1
  %18 = icmp eq i32 %17, %1
  br i1 %18, label %19, label %8, !llvm.loop !83

19:                                               ; preds = %8, %6, %3
  %20 = phi i32 [ %1, %3 ], [ 0, %6 ], [ %16, %8 ]
  %21 = zext i32 %20 to i64
  %22 = tail call ptr @fa_array_new(i64 noundef %21, i64 noundef %21, i64 noundef 4) #53
  %23 = load ptr, ptr %22, align 8, !tbaa !69
  %24 = icmp eq i32 %20, %1
  br i1 %24, label %25, label %35

25:                                               ; preds = %19
  %26 = icmp eq i32 %1, 0
  br i1 %26, label %45, label %27

27:                                               ; preds = %25, %27
  %28 = phi i32 [ %33, %27 ], [ 0, %25 ]
  %29 = getelementptr inbounds i8, ptr %0, i32 %28
  %30 = load i8, ptr %29, align 1, !tbaa !1
  %31 = zext i8 %30 to i32
  %32 = getelementptr inbounds i32, ptr %23, i32 %28
  store i32 %31, ptr %32, align 4, !tbaa !23
  %33 = add nuw i32 %28, 1
  %34 = icmp eq i32 %33, %1
  br i1 %34, label %45, label %27, !llvm.loop !84

35:                                               ; preds = %19
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %4) #56
  store i32 0, ptr %4, align 4, !tbaa !4
  %36 = icmp eq i32 %1, 0
  br i1 %36, label %37, label %38

37:                                               ; preds = %38, %35
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %4) #56
  br label %45

38:                                               ; preds = %35, %38
  %39 = phi i32 [ %42, %38 ], [ 0, %35 ]
  %40 = call fastcc i32 @decode(ptr noundef %0, i32 noundef %1, ptr noundef nonnull %4) #53
  %41 = getelementptr inbounds i32, ptr %23, i32 %39
  store i32 %40, ptr %41, align 4, !tbaa !23
  %42 = add i32 %39, 1
  %43 = load i32, ptr %4, align 4, !tbaa !4
  %44 = icmp ult i32 %43, %1
  br i1 %44, label %38, label %37, !llvm.loop !85

45:                                               ; preds = %27, %25, %37
  ret ptr %22
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define internal fastcc i32 @decode(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef %2) unnamed_addr #18 {
  %4 = load i32, ptr %2, align 4, !tbaa !4
  %5 = getelementptr inbounds i8, ptr %0, i32 %4
  %6 = load i8, ptr %5, align 1, !tbaa !1
  %7 = zext i8 %6 to i32
  %8 = icmp sgt i8 %6, -1
  br i1 %8, label %21, label %9

9:                                                ; preds = %3
  %10 = icmp ult i8 %6, -32
  %11 = icmp ult i8 %6, -16
  %12 = select i1 %11, i32 3, i32 4
  %13 = select i1 %10, i32 2, i32 %12
  %14 = lshr i32 127, %13
  %15 = and i32 %14, %7
  br label %16

16:                                               ; preds = %9, %25
  %17 = phi i32 [ %32, %25 ], [ 1, %9 ]
  %18 = phi i32 [ %31, %25 ], [ %15, %9 ]
  %19 = add i32 %17, %4
  %20 = icmp ult i32 %19, %1
  br i1 %20, label %25, label %21

21:                                               ; preds = %16, %25, %3
  %22 = phi i32 [ 1, %3 ], [ %13, %25 ], [ %13, %16 ]
  %23 = phi i32 [ %7, %3 ], [ %18, %16 ], [ %31, %25 ]
  %24 = add i32 %22, %4
  store i32 %24, ptr %2, align 4, !tbaa !4
  ret i32 %23

25:                                               ; preds = %16
  %26 = shl i32 %18, 6
  %27 = getelementptr inbounds i8, ptr %0, i32 %19
  %28 = load i8, ptr %27, align 1, !tbaa !1
  %29 = and i8 %28, 63
  %30 = zext nneg i8 %29 to i32
  %31 = or disjoint i32 %26, %30
  %32 = add nuw nsw i32 %17, 1
  %33 = icmp eq i32 %32, %13
  br i1 %33, label %21, label %16, !llvm.loop !86
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define hidden i64 @fa_string_code_point_at(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2, i64 noundef %3) local_unnamed_addr #18 {
  %5 = alloca i32, align 4
  %6 = icmp sgt i32 %2, -1
  %7 = icmp slt i64 %3, 0
  br i1 %6, label %18, label %8

8:                                                ; preds = %4
  %9 = zext i32 %1 to i64
  %10 = select i1 %7, i64 %9, i64 0
  %11 = add nsw i64 %10, %3
  %12 = icmp ult i64 %11, %9
  br i1 %12, label %13, label %51

13:                                               ; preds = %8
  %14 = trunc i64 %11 to i32
  %15 = getelementptr inbounds i8, ptr %0, i32 %14
  %16 = load i8, ptr %15, align 1, !tbaa !1
  %17 = zext i8 %16 to i64
  br label %51

18:                                               ; preds = %4
  br i1 %7, label %19, label %36

19:                                               ; preds = %18
  %20 = icmp eq i32 %1, 0
  br i1 %20, label %32, label %21

21:                                               ; preds = %19, %21
  %22 = phi i32 [ %30, %21 ], [ 0, %19 ]
  %23 = phi i64 [ %29, %21 ], [ 0, %19 ]
  %24 = getelementptr inbounds i8, ptr %0, i32 %22
  %25 = load i8, ptr %24, align 1, !tbaa !1
  %26 = and i8 %25, -64
  %27 = icmp ne i8 %26, -128
  %28 = zext i1 %27 to i64
  %29 = add nuw nsw i64 %23, %28
  %30 = add nuw i32 %22, 1
  %31 = icmp eq i32 %30, %1
  br i1 %31, label %32, label %21, !llvm.loop !87

32:                                               ; preds = %21, %19
  %33 = phi i64 [ 0, %19 ], [ %29, %21 ]
  %34 = add nsw i64 %33, %3
  %35 = icmp slt i64 %34, 0
  br i1 %35, label %51, label %36

36:                                               ; preds = %32, %18
  %37 = phi i64 [ %34, %32 ], [ %3, %18 ]
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %5) #56
  store i32 0, ptr %5, align 4, !tbaa !4
  %38 = icmp eq i32 %1, 0
  br i1 %38, label %49, label %43

39:                                               ; preds = %43
  %40 = add nsw i64 %44, -1
  %41 = load i32, ptr %5, align 4, !tbaa !4
  %42 = icmp ult i32 %41, %1
  br i1 %42, label %43, label %49, !llvm.loop !88

43:                                               ; preds = %36, %39
  %44 = phi i64 [ %40, %39 ], [ %37, %36 ]
  %45 = call fastcc i32 @decode(ptr noundef %0, i32 noundef %1, ptr noundef nonnull %5) #53
  %46 = icmp eq i64 %44, 0
  br i1 %46, label %47, label %39

47:                                               ; preds = %43
  %48 = zext i32 %45 to i64
  br label %49

49:                                               ; preds = %39, %36, %47
  %50 = phi i64 [ %48, %47 ], [ -1, %36 ], [ -1, %39 ]
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %5) #56
  br label %51

51:                                               ; preds = %49, %32, %13, %8
  %52 = phi i64 [ %17, %13 ], [ -1, %8 ], [ -1, %32 ], [ %50, %49 ]
  ret i64 %52
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: read)
define hidden i64 @fa_string_code_point_count(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2) local_unnamed_addr #12 {
  %4 = icmp sgt i32 %2, -1
  br i1 %4, label %5, label %7

5:                                                ; preds = %3
  %6 = icmp eq i32 %1, 0
  br i1 %6, label %20, label %9

7:                                                ; preds = %3
  %8 = zext i32 %1 to i64
  br label %20

9:                                                ; preds = %5, %9
  %10 = phi i32 [ %18, %9 ], [ 0, %5 ]
  %11 = phi i64 [ %17, %9 ], [ 0, %5 ]
  %12 = getelementptr inbounds i8, ptr %0, i32 %10
  %13 = load i8, ptr %12, align 1, !tbaa !1
  %14 = and i8 %13, -64
  %15 = icmp ne i8 %14, -128
  %16 = zext i1 %15 to i64
  %17 = add nuw nsw i64 %11, %16
  %18 = add nuw i32 %10, 1
  %19 = icmp eq i32 %18, %1
  br i1 %19, label %20, label %9, !llvm.loop !87

20:                                               ; preds = %9, %5, %7
  %21 = phi i64 [ %8, %7 ], [ 0, %5 ], [ %17, %9 ]
  ret i64 %21
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define hidden i32 @fa_next_code_point(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef %2) local_unnamed_addr #18 {
  %4 = alloca i32, align 4
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %4) #56
  %5 = load i64, ptr %2, align 8, !tbaa !89
  %6 = trunc i64 %5 to i32
  store i32 %6, ptr %4, align 4, !tbaa !4
  %7 = getelementptr inbounds i8, ptr %0, i32 %6
  %8 = load i8, ptr %7, align 1, !tbaa !1
  %9 = icmp sgt i8 %8, -1
  br i1 %9, label %10, label %14

10:                                               ; preds = %3
  %11 = zext nneg i8 %8 to i32
  %12 = and i64 %5, 4294967295
  %13 = add nuw nsw i64 %12, 1
  br label %18

14:                                               ; preds = %3
  %15 = call fastcc i32 @decode(ptr noundef nonnull %0, i32 noundef %1, ptr noundef nonnull %4) #53
  %16 = load i32, ptr %4, align 4, !tbaa !4
  %17 = zext i32 %16 to i64
  br label %18

18:                                               ; preds = %14, %10
  %19 = phi i64 [ %17, %14 ], [ %13, %10 ]
  %20 = phi i32 [ %15, %14 ], [ %11, %10 ]
  store i64 %19, ptr %2, align 8, !tbaa !89
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %4) #56
  ret i32 %20
}

; Function Attrs: nounwind optsize
define hidden noundef i32 @fa_string_get(ptr noundef %0, i32 noundef %1, i32 noundef %2, i64 noundef %3, ptr nocapture noundef writeonly %4) local_unnamed_addr #3 {
  %6 = icmp sgt i32 %2, -1
  br i1 %6, label %21, label %7

7:                                                ; preds = %5
  %8 = icmp slt i64 %3, 0
  %9 = zext i32 %1 to i64
  %10 = select i1 %8, i64 %9, i64 0
  %11 = add nsw i64 %10, %3
  %12 = icmp ult i64 %11, %9
  br i1 %12, label %13, label %23

13:                                               ; preds = %7
  %14 = trunc i64 %11 to i32
  %15 = getelementptr inbounds i8, ptr %0, i32 %14
  %16 = load i8, ptr %15, align 1, !tbaa !1
  %17 = zext i8 %16 to i32
  %18 = getelementptr inbounds i8, ptr @ascii_table, i32 %17
  store ptr %18, ptr %4, align 4, !tbaa.struct !10
  %19 = getelementptr inbounds i8, ptr %4, i32 4
  store i32 1, ptr %19, align 4, !tbaa.struct !11
  %20 = getelementptr inbounds i8, ptr %4, i32 8
  store i32 -2147483648, ptr %20, align 4, !tbaa !19
  br label %23

21:                                               ; preds = %5
  %22 = tail call fastcc i32 @string_get_slow(ptr noundef %0, i32 noundef %1, i32 noundef %2, i64 noundef %3, ptr noundef %4) #53
  br label %23

23:                                               ; preds = %7, %21, %13
  %24 = phi i32 [ 1, %13 ], [ %22, %21 ], [ 0, %7 ]
  ret i32 %24
}

; Function Attrs: noinline nounwind optsize
define internal fastcc noundef i32 @string_get_slow(ptr noundef %0, i32 noundef %1, i32 noundef %2, i64 noundef %3, ptr nocapture noundef writeonly %4) unnamed_addr #15 {
  %6 = alloca %struct.FaString, align 4
  %7 = icmp slt i64 %3, 0
  br i1 %7, label %8, label %13

8:                                                ; preds = %5
  %9 = tail call i32 @fa_graphemes(ptr noundef %0, i32 noundef %1) #53
  %10 = zext i32 %9 to i64
  %11 = add nsw i64 %10, %3
  %12 = icmp slt i64 %11, 0
  br i1 %12, label %36, label %13

13:                                               ; preds = %5, %8
  %14 = phi i64 [ %11, %8 ], [ %3, %5 ]
  %15 = icmp eq i32 %1, 0
  br i1 %15, label %36, label %19

16:                                               ; preds = %19
  %17 = add nuw nsw i64 %20, 1
  %18 = icmp ult i32 %22, %1
  br i1 %18, label %19, label %36, !llvm.loop !90

19:                                               ; preds = %13, %16
  %20 = phi i64 [ %17, %16 ], [ 0, %13 ]
  %21 = phi i32 [ %22, %16 ], [ 0, %13 ]
  %22 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %21) #53
  %23 = icmp eq i64 %20, %14
  br i1 %23, label %24, label %16

24:                                               ; preds = %19
  %25 = and i32 %2, 2147483647
  %26 = icmp eq i32 %25, 0
  %27 = sub i32 %22, %21
  %28 = icmp ugt i32 %27, 1
  %29 = select i1 %26, i1 %28, i1 false
  br i1 %29, label %30, label %34

30:                                               ; preds = %24
  %31 = getelementptr inbounds i8, ptr %0, i32 %21
  store ptr %31, ptr %4, align 4, !tbaa !13
  %32 = getelementptr inbounds %struct.FaString, ptr %4, i32 0, i32 1
  store i32 %27, ptr %32, align 4, !tbaa !18
  %33 = getelementptr inbounds %struct.FaString, ptr %4, i32 0, i32 2
  store i32 0, ptr %33, align 4, !tbaa !19
  br label %36

34:                                               ; preds = %24
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %6) #56
  %35 = getelementptr inbounds i8, ptr %0, i32 %21
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %6, ptr noundef %35, i32 noundef %27, i32 noundef 0) #53
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %4, ptr noundef nonnull align 4 dereferenceable(12) %6, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %6) #56
  br label %36

36:                                               ; preds = %16, %30, %34, %13, %8
  %37 = phi i32 [ 0, %8 ], [ 1, %34 ], [ 1, %30 ], [ 0, %13 ], [ 0, %16 ]
  ret i32 %37
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define hidden i32 @fa_string_length(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2) local_unnamed_addr #18 {
  %4 = icmp sgt i32 %2, -1
  br i1 %4, label %5, label %7

5:                                                ; preds = %3
  %6 = tail call i32 @fa_graphemes(ptr noundef %0, i32 noundef %1) #53
  br label %7

7:                                                ; preds = %3, %5
  %8 = phi i32 [ %6, %5 ], [ %1, %3 ]
  ret i32 %8
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_string_slice(ptr noundef %0, i32 noundef %1, i32 noundef %2, i64 noundef %3, i64 noundef %4, i8 noundef signext %5, ptr nocapture noundef writeonly %6) local_unnamed_addr #3 {
  %8 = alloca %struct.FaString, align 4
  %9 = and i32 %2, -2147483648
  %10 = icmp eq i32 %9, 0
  br i1 %10, label %11, label %13

11:                                               ; preds = %7
  %12 = tail call i32 @fa_graphemes(ptr noundef %0, i32 noundef %1) #53
  br label %13

13:                                               ; preds = %7, %11
  %14 = phi i32 [ %12, %11 ], [ %1, %7 ]
  %15 = zext i32 %14 to i64
  %16 = zext i8 %5 to i32
  %17 = and i32 %16, 1
  %18 = icmp eq i32 %17, 0
  br i1 %18, label %30, label %19

19:                                               ; preds = %13
  %20 = sub nsw i64 0, %15
  %21 = icmp sgt i64 %20, %3
  %22 = icmp slt i64 %15, %3
  %23 = or i1 %21, %22
  br i1 %23, label %24, label %26

24:                                               ; preds = %19
  %25 = tail call ptr @fa_error_index(i64 noundef %3, i64 noundef %15) #53
  br label %89

26:                                               ; preds = %19
  %27 = icmp slt i64 %3, 0
  %28 = select i1 %27, i64 %15, i64 0
  %29 = add nsw i64 %28, %3
  br label %30

30:                                               ; preds = %26, %13
  %31 = phi i64 [ %29, %26 ], [ 0, %13 ]
  %32 = and i32 %16, 2
  %33 = icmp eq i32 %32, 0
  br i1 %33, label %53, label %34

34:                                               ; preds = %30
  %35 = and i32 %16, 4
  %36 = lshr exact i32 %35, 2
  %37 = sub nsw i64 0, %15
  %38 = icmp sgt i64 %37, %4
  br i1 %38, label %51, label %39

39:                                               ; preds = %34
  %40 = shl nuw i32 %35, 29
  %41 = ashr exact i32 %40, 31
  %42 = sext i32 %41 to i64
  %43 = add nsw i64 %15, %42
  %44 = icmp slt i64 %43, %4
  br i1 %44, label %51, label %45

45:                                               ; preds = %39
  %46 = icmp slt i64 %4, 0
  %47 = select i1 %46, i64 %15, i64 0
  %48 = zext nneg i32 %36 to i64
  %49 = add nsw i64 %48, %4
  %50 = add nsw i64 %49, %47
  br label %53

51:                                               ; preds = %34, %39
  %52 = tail call ptr @fa_error_index(i64 noundef %4, i64 noundef %15) #53
  br label %89

53:                                               ; preds = %45, %30
  %54 = phi i64 [ %15, %30 ], [ %50, %45 ]
  %55 = tail call i64 @llvm.smax.i64(i64 %54, i64 %31)
  %56 = trunc i64 %31 to i32
  %57 = trunc i64 %55 to i32
  br i1 %10, label %58, label %75

58:                                               ; preds = %53
  %59 = icmp eq i32 %56, 0
  br i1 %59, label %60, label %63

60:                                               ; preds = %63, %58
  %61 = phi i32 [ 0, %58 ], [ %66, %63 ]
  %62 = icmp ult i32 %56, %57
  br i1 %62, label %69, label %75

63:                                               ; preds = %58, %63
  %64 = phi i32 [ %67, %63 ], [ 0, %58 ]
  %65 = phi i32 [ %66, %63 ], [ 0, %58 ]
  %66 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %65) #53
  %67 = add nuw i32 %64, 1
  %68 = icmp eq i32 %67, %56
  br i1 %68, label %60, label %63, !llvm.loop !91

69:                                               ; preds = %60, %69
  %70 = phi i32 [ %73, %69 ], [ %56, %60 ]
  %71 = phi i32 [ %72, %69 ], [ %61, %60 ]
  %72 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %71) #53
  %73 = add i32 %70, 1
  %74 = icmp eq i32 %73, %57
  br i1 %74, label %75, label %69, !llvm.loop !92

75:                                               ; preds = %69, %60, %53
  %76 = phi i32 [ %56, %53 ], [ %61, %60 ], [ %61, %69 ]
  %77 = phi i32 [ %57, %53 ], [ %61, %60 ], [ %72, %69 ]
  %78 = and i32 %2, 2147483647
  %79 = icmp eq i32 %78, 0
  %80 = sub i32 %77, %76
  %81 = icmp ugt i32 %80, 1
  %82 = select i1 %79, i1 %81, i1 false
  br i1 %82, label %83, label %87

83:                                               ; preds = %75
  %84 = getelementptr inbounds i8, ptr %0, i32 %76
  store ptr %84, ptr %6, align 4, !tbaa !13
  %85 = getelementptr inbounds %struct.FaString, ptr %6, i32 0, i32 1
  store i32 %80, ptr %85, align 4, !tbaa !18
  %86 = getelementptr inbounds %struct.FaString, ptr %6, i32 0, i32 2
  store i32 %9, ptr %86, align 4, !tbaa !19
  br label %89

87:                                               ; preds = %75
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %8) #56
  %88 = getelementptr inbounds i8, ptr %0, i32 %76
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %8, ptr noundef %88, i32 noundef %80, i32 noundef 0) #53
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %6, ptr noundef nonnull align 4 dereferenceable(12) %8, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %8) #56
  br label %89

89:                                               ; preds = %51, %83, %87, %24
  %90 = phi ptr [ %25, %24 ], [ %52, %51 ], [ null, %87 ], [ null, %83 ]
  ret ptr %90
}

; Function Attrs: cold noinline nounwind optsize
define hidden noundef ptr @fa_error_index(i64 noundef %0, i64 noundef %1) local_unnamed_addr #21 {
  %3 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #54
  store i64 1, ptr %3, align 8, !tbaa !93
  %4 = getelementptr inbounds %struct.IndexError, ptr %3, i32 0, i32 1
  store i64 %0, ptr %4, align 8, !tbaa !95
  %5 = getelementptr inbounds %struct.IndexError, ptr %3, i32 0, i32 2
  store i64 %1, ptr %5, align 8, !tbaa !96
  ret ptr %3
}

; Function Attrs: nounwind optsize
define hidden void @fa_string_case(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef writeonly %3) local_unnamed_addr #3 {
  %5 = alloca %struct.FaString, align 4
  %6 = alloca i32, align 4
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %6) #56
  store i32 0, ptr %6, align 4, !tbaa !4
  %7 = icmp eq i32 %1, 0
  br i1 %7, label %147, label %8

8:                                                ; preds = %4
  %9 = icmp eq i32 %2, 0
  br label %10

10:                                               ; preds = %8, %142
  %11 = call fastcc i32 @decode(ptr noundef %0, i32 noundef %1, ptr noundef nonnull %6) #53
  br i1 %9, label %39, label %12

12:                                               ; preds = %10
  %13 = add i32 %11, -97
  %14 = icmp ult i32 %13, 26
  br i1 %14, label %28, label %15

15:                                               ; preds = %12
  %16 = add i32 %11, -224
  %17 = icmp ult i32 %16, 31
  %18 = icmp ne i32 %11, 247
  %19 = and i1 %18, %17
  br i1 %19, label %28, label %20

20:                                               ; preds = %15
  %21 = add i32 %11, -945
  %22 = icmp ult i32 %21, 25
  %23 = icmp ne i32 %11, 962
  %24 = and i1 %23, %22
  %25 = add i32 %11, -1072
  %26 = icmp ult i32 %25, 32
  %27 = or i1 %26, %24
  br i1 %27, label %28, label %30

28:                                               ; preds = %20, %15, %12
  %29 = add nsw i32 %11, -32
  br label %64

30:                                               ; preds = %20
  %31 = and i32 %11, -16
  %32 = icmp eq i32 %31, 1104
  br i1 %32, label %33, label %35

33:                                               ; preds = %30
  %34 = add nsw i32 %11, -80
  br label %74

35:                                               ; preds = %30
  switch i32 %11, label %64 [
    i32 255, label %74
    i32 181, label %36
    i32 962, label %37
    i32 223, label %38
  ]

36:                                               ; preds = %35
  br label %74

37:                                               ; preds = %35
  br label %74

38:                                               ; preds = %35
  tail call fastcc void @append_c(ptr noundef nonnull @.str.12) #53
  br label %142, !llvm.loop !97

39:                                               ; preds = %10
  %40 = add i32 %11, -65
  %41 = icmp ult i32 %40, 26
  br i1 %41, label %55, label %42

42:                                               ; preds = %39
  %43 = add i32 %11, -192
  %44 = icmp ult i32 %43, 31
  %45 = icmp ne i32 %11, 215
  %46 = and i1 %45, %44
  br i1 %46, label %55, label %47

47:                                               ; preds = %42
  %48 = add i32 %11, -913
  %49 = icmp ult i32 %48, 25
  %50 = icmp ne i32 %11, 930
  %51 = and i1 %50, %49
  %52 = add i32 %11, -1040
  %53 = icmp ult i32 %52, 32
  %54 = or i1 %53, %51
  br i1 %54, label %55, label %57

55:                                               ; preds = %47, %42, %39
  %56 = add nuw nsw i32 %11, 32
  br label %64

57:                                               ; preds = %47
  %58 = and i32 %11, -16
  %59 = icmp eq i32 %58, 1024
  br i1 %59, label %60, label %62

60:                                               ; preds = %57
  %61 = or disjoint i32 %11, 80
  br label %74

62:                                               ; preds = %57
  %63 = icmp eq i32 %11, 376
  br i1 %63, label %74, label %64

64:                                               ; preds = %35, %62, %55, %28
  %65 = phi i32 [ %29, %28 ], [ %56, %55 ], [ %11, %62 ], [ %11, %35 ]
  %66 = icmp ult i32 %65, 128
  br i1 %66, label %67, label %72

67:                                               ; preds = %64
  %68 = trunc i32 %65 to i8
  tail call fastcc void @reserve(i32 noundef 1) #53
  %69 = load ptr, ptr @builder, align 4, !tbaa !6
  %70 = load i32, ptr @builder_length, align 4, !tbaa !4
  %71 = getelementptr inbounds i8, ptr %69, i32 %70
  store i8 %68, ptr %71, align 1, !tbaa !1
  br label %139

72:                                               ; preds = %64
  %73 = icmp ult i32 %65, 2048
  br i1 %73, label %74, label %89

74:                                               ; preds = %33, %36, %37, %60, %35, %62, %72
  %75 = phi i32 [ %65, %72 ], [ 376, %35 ], [ %61, %60 ], [ 931, %37 ], [ 924, %36 ], [ %34, %33 ], [ 255, %62 ]
  %76 = lshr i32 %75, 6
  %77 = trunc i32 %76 to i8
  %78 = or disjoint i8 %77, -64
  tail call fastcc void @reserve(i32 noundef 1) #53
  %79 = load ptr, ptr @builder, align 4, !tbaa !6
  %80 = load i32, ptr @builder_length, align 4, !tbaa !4
  %81 = getelementptr inbounds i8, ptr %79, i32 %80
  store i8 %78, ptr %81, align 1, !tbaa !1
  %82 = add i32 %80, 1
  store i32 %82, ptr @builder_length, align 4, !tbaa !4
  %83 = trunc i32 %75 to i8
  %84 = and i8 %83, 63
  %85 = or disjoint i8 %84, -128
  tail call fastcc void @reserve(i32 noundef 1) #53
  %86 = load ptr, ptr @builder, align 4, !tbaa !6
  %87 = load i32, ptr @builder_length, align 4, !tbaa !4
  %88 = getelementptr inbounds i8, ptr %86, i32 %87
  store i8 %85, ptr %88, align 1, !tbaa !1
  br label %139

89:                                               ; preds = %72
  %90 = icmp ult i32 %65, 65536
  tail call fastcc void @reserve(i32 noundef 1) #53
  %91 = load ptr, ptr @builder, align 4, !tbaa !6
  %92 = load i32, ptr @builder_length, align 4, !tbaa !4
  %93 = getelementptr inbounds i8, ptr %91, i32 %92
  %94 = add i32 %92, 1
  br i1 %90, label %95, label %113

95:                                               ; preds = %89
  %96 = lshr i32 %65, 12
  %97 = trunc i32 %96 to i8
  %98 = or disjoint i8 %97, -32
  store i8 %98, ptr %93, align 1, !tbaa !1
  store i32 %94, ptr @builder_length, align 4, !tbaa !4
  %99 = lshr i32 %65, 6
  %100 = trunc i32 %99 to i8
  %101 = and i8 %100, 63
  %102 = or disjoint i8 %101, -128
  tail call fastcc void @reserve(i32 noundef 1) #53
  %103 = load ptr, ptr @builder, align 4, !tbaa !6
  %104 = load i32, ptr @builder_length, align 4, !tbaa !4
  %105 = getelementptr inbounds i8, ptr %103, i32 %104
  store i8 %102, ptr %105, align 1, !tbaa !1
  %106 = add i32 %104, 1
  store i32 %106, ptr @builder_length, align 4, !tbaa !4
  %107 = trunc i32 %65 to i8
  %108 = and i8 %107, 63
  %109 = or disjoint i8 %108, -128
  tail call fastcc void @reserve(i32 noundef 1) #53
  %110 = load ptr, ptr @builder, align 4, !tbaa !6
  %111 = load i32, ptr @builder_length, align 4, !tbaa !4
  %112 = getelementptr inbounds i8, ptr %110, i32 %111
  store i8 %109, ptr %112, align 1, !tbaa !1
  br label %139

113:                                              ; preds = %89
  %114 = lshr i32 %65, 18
  %115 = trunc i32 %114 to i8
  %116 = or i8 %115, -16
  store i8 %116, ptr %93, align 1, !tbaa !1
  store i32 %94, ptr @builder_length, align 4, !tbaa !4
  %117 = lshr i32 %65, 12
  %118 = trunc i32 %117 to i8
  %119 = and i8 %118, 63
  %120 = or disjoint i8 %119, -128
  tail call fastcc void @reserve(i32 noundef 1) #53
  %121 = load ptr, ptr @builder, align 4, !tbaa !6
  %122 = load i32, ptr @builder_length, align 4, !tbaa !4
  %123 = getelementptr inbounds i8, ptr %121, i32 %122
  store i8 %120, ptr %123, align 1, !tbaa !1
  %124 = add i32 %122, 1
  store i32 %124, ptr @builder_length, align 4, !tbaa !4
  %125 = lshr i32 %65, 6
  %126 = trunc i32 %125 to i8
  %127 = and i8 %126, 63
  %128 = or disjoint i8 %127, -128
  tail call fastcc void @reserve(i32 noundef 1) #53
  %129 = load ptr, ptr @builder, align 4, !tbaa !6
  %130 = load i32, ptr @builder_length, align 4, !tbaa !4
  %131 = getelementptr inbounds i8, ptr %129, i32 %130
  store i8 %128, ptr %131, align 1, !tbaa !1
  %132 = add i32 %130, 1
  store i32 %132, ptr @builder_length, align 4, !tbaa !4
  %133 = trunc i32 %65 to i8
  %134 = and i8 %133, 63
  %135 = or disjoint i8 %134, -128
  tail call fastcc void @reserve(i32 noundef 1) #53
  %136 = load ptr, ptr @builder, align 4, !tbaa !6
  %137 = load i32, ptr @builder_length, align 4, !tbaa !4
  %138 = getelementptr inbounds i8, ptr %136, i32 %137
  store i8 %135, ptr %138, align 1, !tbaa !1
  br label %139

139:                                              ; preds = %67, %74, %95, %113
  %140 = phi i32 [ %87, %74 ], [ %137, %113 ], [ %111, %95 ], [ %70, %67 ]
  %141 = add i32 %140, 1
  store i32 %141, ptr @builder_length, align 4, !tbaa !4
  br label %142

142:                                              ; preds = %139, %38
  %143 = load i32, ptr %6, align 4, !tbaa !4
  %144 = icmp ult i32 %143, %1
  br i1 %144, label %10, label %145

145:                                              ; preds = %142
  %146 = load i32, ptr @builder_length, align 4, !tbaa !4
  br label %147

147:                                              ; preds = %145, %4
  %148 = phi i32 [ %146, %145 ], [ 0, %4 ]
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %5) #56
  %149 = load ptr, ptr @builder, align 4, !tbaa !6
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %5, ptr noundef %149, i32 noundef %148, i32 noundef 0) #53
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %3, ptr noundef nonnull align 4 dereferenceable(12) %5, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %5) #56
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %6) #56
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_string_trim(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef writeonly %3) local_unnamed_addr #3 {
  %5 = alloca i32, align 4
  %6 = alloca i32, align 4
  %7 = alloca %struct.FaString, align 4
  %8 = and i32 %2, 1
  %9 = icmp eq i32 %8, 0
  br i1 %9, label %18, label %10

10:                                               ; preds = %4, %13
  %11 = phi i32 [ %17, %13 ], [ 0, %4 ]
  %12 = icmp ult i32 %11, %1
  br i1 %12, label %13, label %18

13:                                               ; preds = %10
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %5) #56
  store i32 %11, ptr %5, align 4, !tbaa !4
  %14 = call fastcc i32 @decode(ptr noundef %0, i32 noundef %1, ptr noundef nonnull %5) #53
  %15 = tail call fastcc i32 @is_space(i32 noundef %14) #53, !range !53
  %16 = icmp eq i32 %15, 0
  %17 = load i32, ptr %5, align 4
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %5) #56
  br i1 %16, label %18, label %10

18:                                               ; preds = %13, %10, %4
  %19 = phi i32 [ 0, %4 ], [ %11, %10 ], [ %11, %13 ]
  %20 = and i32 %2, 2
  %21 = icmp eq i32 %20, 0
  br i1 %21, label %39, label %22

22:                                               ; preds = %18, %34
  %23 = phi i32 [ %35, %34 ], [ %1, %18 ]
  %24 = icmp ugt i32 %23, %19
  br i1 %24, label %25, label %39

25:                                               ; preds = %22, %29
  %26 = phi i32 [ %27, %29 ], [ %23, %22 ]
  %27 = add i32 %26, -1
  %28 = icmp ugt i32 %27, %19
  br i1 %28, label %29, label %34

29:                                               ; preds = %25
  %30 = getelementptr inbounds i8, ptr %0, i32 %27
  %31 = load i8, ptr %30, align 1, !tbaa !1
  %32 = and i8 %31, -64
  %33 = icmp eq i8 %32, -128
  br i1 %33, label %25, label %34, !llvm.loop !98

34:                                               ; preds = %25, %29
  %35 = phi i32 [ %19, %25 ], [ %27, %29 ]
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %6) #56
  store i32 %35, ptr %6, align 4, !tbaa !4
  %36 = call fastcc i32 @decode(ptr noundef %0, i32 noundef %23, ptr noundef nonnull %6) #53
  %37 = tail call fastcc i32 @is_space(i32 noundef %36) #53, !range !53
  %38 = icmp eq i32 %37, 0
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %6) #56
  br i1 %38, label %39, label %22

39:                                               ; preds = %34, %22, %18
  %40 = phi i32 [ %1, %18 ], [ %23, %22 ], [ %23, %34 ]
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %7) #56
  %41 = getelementptr inbounds i8, ptr %0, i32 %19
  %42 = sub i32 %40, %19
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %7, ptr noundef %41, i32 noundef %42, i32 noundef 0) #53
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %3, ptr noundef nonnull align 4 dereferenceable(12) %7, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %7) #56
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(none)
define internal fastcc noundef i32 @is_space(i32 noundef %0) unnamed_addr #22 {
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
define hidden void @fa_string_repeat(ptr nocapture noundef readonly %0, i32 noundef %1, i64 noundef %2, ptr nocapture noundef writeonly %3) local_unnamed_addr #3 {
  %5 = alloca %struct.FaString, align 4
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  %6 = icmp sgt i64 %2, 0
  br i1 %6, label %12, label %7

7:                                                ; preds = %4
  %8 = load ptr, ptr @builder, align 4, !tbaa !6
  br label %9

9:                                                ; preds = %12, %7
  %10 = phi i32 [ 0, %7 ], [ %17, %12 ]
  %11 = phi ptr [ %8, %7 ], [ %14, %12 ]
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %5) #56
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %5, ptr noundef %11, i32 noundef %10, i32 noundef 0) #53
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %3, ptr noundef nonnull align 4 dereferenceable(12) %5, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %5) #56
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  ret void

12:                                               ; preds = %4, %12
  %13 = phi i64 [ %18, %12 ], [ 0, %4 ]
  tail call fastcc void @reserve(i32 noundef %1) #53
  %14 = load ptr, ptr @builder, align 4, !tbaa !6
  %15 = load i32, ptr @builder_length, align 4, !tbaa !4
  %16 = getelementptr inbounds i8, ptr %14, i32 %15
  tail call fastcc void @copy_bytes(ptr noundef %16, ptr noundef %0, i32 noundef %1) #53
  %17 = add i32 %15, %1
  store i32 %17, ptr @builder_length, align 4, !tbaa !4
  %18 = add nuw nsw i64 %13, 1
  %19 = icmp eq i64 %18, %2
  br i1 %19, label %9, label %12, !llvm.loop !99
}

; Function Attrs: nounwind optsize
define hidden void @fa_array_join(ptr noundef %0, ptr nocapture noundef readonly %1, i32 noundef %2, ptr nocapture noundef writeonly %3) local_unnamed_addr #3 {
  %5 = alloca %struct.FaString, align 4
  %6 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %7 = load i64, ptr %6, align 8, !tbaa !73
  %8 = icmp slt i64 %7, 0
  br i1 %8, label %9, label %12, !prof !35

9:                                                ; preds = %4
  %10 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %11 = load i64, ptr %10, align 8, !tbaa !79
  tail call fastcc void @materialize_characters(ptr noundef nonnull %0, i64 noundef %11) #53
  br label %12

12:                                               ; preds = %4, %9
  %13 = load ptr, ptr %0, align 8, !tbaa !69
  %14 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %15 = load i64, ptr %14, align 8, !tbaa !79
  %16 = trunc i64 %15 to i32
  %17 = icmp ne i32 %2, 0
  %18 = icmp ugt i32 %16, 1
  %19 = and i1 %17, %18
  br i1 %19, label %20, label %25

20:                                               ; preds = %12
  %21 = add i32 %16, -1
  %22 = udiv i32 1073741823, %2
  %23 = icmp ult i32 %21, %22
  br i1 %23, label %28, label %24

24:                                               ; preds = %20
  tail call fastcc void @out_of_memory() #57
  unreachable

25:                                               ; preds = %12
  br i1 %18, label %26, label %32

26:                                               ; preds = %25
  %27 = add i32 %16, -1
  br label %28

28:                                               ; preds = %26, %20
  %29 = phi i32 [ %27, %26 ], [ %21, %20 ]
  %30 = mul i32 %29, %2
  %31 = tail call fastcc i32 @simple_flag(ptr noundef %1, i32 noundef %2) #53, !range !20
  br label %34

32:                                               ; preds = %25
  %33 = icmp eq i32 %16, 0
  br i1 %33, label %37, label %34

34:                                               ; preds = %28, %32
  %35 = phi i32 [ 0, %32 ], [ %30, %28 ]
  %36 = phi i32 [ -2147483648, %32 ], [ %31, %28 ]
  br label %40

37:                                               ; preds = %32
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %5) #56
  call void @llvm.memset.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %5, i8 0, i32 12, i1 false)
  br label %103

38:                                               ; preds = %59
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %5) #56
  call void @llvm.memset.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %5, i8 0, i32 12, i1 false)
  %39 = icmp ult i32 %60, 12
  br i1 %39, label %68, label %66

40:                                               ; preds = %34, %59
  %41 = phi i32 [ %60, %59 ], [ %35, %34 ]
  %42 = phi i32 [ %63, %59 ], [ %36, %34 ]
  %43 = phi i32 [ %64, %59 ], [ 0, %34 ]
  %44 = getelementptr inbounds %struct.FaString, ptr %13, i32 %43, i32 2
  %45 = load i32, ptr %44, align 4, !tbaa !19
  %46 = and i32 %45, 1073741824
  %47 = icmp eq i32 %46, 0
  br i1 %47, label %51, label %48

48:                                               ; preds = %40
  %49 = lshr i32 %45, 24
  %50 = and i32 %49, 31
  br label %54

51:                                               ; preds = %40
  %52 = getelementptr inbounds %struct.FaString, ptr %13, i32 %43, i32 1
  %53 = load i32, ptr %52, align 4, !tbaa !18
  br label %54

54:                                               ; preds = %48, %51
  %55 = phi i32 [ %50, %48 ], [ %53, %51 ]
  %56 = sub i32 1073741823, %41
  %57 = icmp ult i32 %55, %56
  br i1 %57, label %59, label %58

58:                                               ; preds = %54
  tail call fastcc void @out_of_memory() #57
  unreachable

59:                                               ; preds = %54
  %60 = add i32 %55, %41
  %61 = icmp eq i32 %55, 0
  %62 = select i1 %61, i32 -1, i32 %45
  %63 = and i32 %42, %62
  %64 = add nuw i32 %43, 1
  %65 = icmp eq i32 %64, %16
  br i1 %65, label %38, label %40, !llvm.loop !32

66:                                               ; preds = %38
  %67 = tail call ptr @fa_alloc(i32 noundef %60, i32 noundef 1) #54
  br label %68

68:                                               ; preds = %38, %66
  %69 = phi ptr [ %67, %66 ], [ %5, %38 ]
  br label %72

70:                                               ; preds = %93
  %71 = icmp eq i32 %63, 0
  br i1 %71, label %100, label %102

72:                                               ; preds = %68, %93
  %73 = phi i32 [ %98, %93 ], [ 0, %68 ]
  %74 = phi i32 [ %97, %93 ], [ 0, %68 ]
  %75 = icmp eq i32 %73, 0
  br i1 %75, label %79, label %76

76:                                               ; preds = %72
  %77 = getelementptr inbounds i8, ptr %69, i32 %74
  call fastcc void @copy_bytes(ptr noundef %77, ptr noundef %1, i32 noundef %2) #53
  %78 = add i32 %74, %2
  br label %79

79:                                               ; preds = %76, %72
  %80 = phi i32 [ %78, %76 ], [ %74, %72 ]
  %81 = getelementptr inbounds %struct.FaString, ptr %13, i32 %73
  %82 = getelementptr inbounds %struct.FaString, ptr %13, i32 %73, i32 2
  %83 = load i32, ptr %82, align 4, !tbaa !19
  %84 = and i32 %83, 1073741824
  %85 = icmp eq i32 %84, 0
  br i1 %85, label %89, label %86

86:                                               ; preds = %79
  %87 = lshr i32 %83, 24
  %88 = and i32 %87, 31
  br label %93

89:                                               ; preds = %79
  %90 = getelementptr inbounds %struct.FaString, ptr %13, i32 %73, i32 1
  %91 = load i32, ptr %90, align 4, !tbaa !18
  %92 = load ptr, ptr %81, align 4, !tbaa !13
  br label %93

93:                                               ; preds = %86, %89
  %94 = phi i32 [ %91, %89 ], [ %88, %86 ]
  %95 = phi ptr [ %92, %89 ], [ %81, %86 ]
  %96 = getelementptr inbounds i8, ptr %69, i32 %80
  call fastcc void @copy_bytes(ptr noundef %96, ptr noundef %95, i32 noundef %94) #53
  %97 = add i32 %94, %80
  %98 = add nuw i32 %73, 1
  %99 = icmp eq i32 %98, %16
  br i1 %99, label %70, label %72, !llvm.loop !33

100:                                              ; preds = %70
  %101 = call fastcc i32 @simple_flag(ptr noundef %69, i32 noundef %60) #53, !range !20
  br i1 %39, label %103, label %111

102:                                              ; preds = %70
  br i1 %39, label %103, label %111

103:                                              ; preds = %100, %37, %102
  %104 = phi i32 [ -2147483648, %37 ], [ -2147483648, %102 ], [ %101, %100 ]
  %105 = phi i32 [ 0, %37 ], [ %60, %102 ], [ %60, %100 ]
  %106 = icmp eq i32 %104, 0
  %107 = select i1 %106, i32 64, i32 192
  %108 = or disjoint i32 %107, %105
  %109 = trunc i32 %108 to i8
  %110 = getelementptr inbounds i8, ptr %5, i32 11
  store i8 %109, ptr %110, align 1, !tbaa !1
  br label %116

111:                                              ; preds = %100, %102
  %112 = phi i32 [ %101, %100 ], [ %63, %102 ]
  store ptr %69, ptr %5, align 4, !tbaa !13
  %113 = getelementptr inbounds %struct.FaString, ptr %5, i32 0, i32 1
  store i32 %60, ptr %113, align 4, !tbaa !18
  %114 = or i32 %112, %60
  %115 = getelementptr inbounds %struct.FaString, ptr %5, i32 0, i32 2
  store i32 %114, ptr %115, align 4, !tbaa !19
  br label %116

116:                                              ; preds = %103, %111
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %3, ptr noundef nonnull align 4 dereferenceable(12) %5, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %5) #56
  ret void
}

; Function Attrs: alwaysinline nounwind optsize
define hidden void @fa_array_materialize(ptr noundef %0) local_unnamed_addr #13 {
  %2 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %3 = load i64, ptr %2, align 8, !tbaa !73
  %4 = icmp slt i64 %3, 0
  br i1 %4, label %5, label %8, !prof !35

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %7 = load i64, ptr %6, align 8, !tbaa !79
  tail call fastcc void @materialize_characters(ptr noundef nonnull %0, i64 noundef %7) #53
  br label %8

8:                                                ; preds = %5, %1
  ret void
}

; Function Attrs: nounwind optsize
define hidden i32 @fa_array_join_length(ptr noundef %0, ptr nocapture noundef readonly %1, i32 noundef %2) local_unnamed_addr #3 {
  %4 = alloca %struct.FaString, align 4
  %5 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %6 = load i64, ptr %5, align 8, !tbaa !73
  %7 = icmp slt i64 %6, 0
  br i1 %7, label %8, label %11, !prof !35

8:                                                ; preds = %3
  %9 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %10 = load i64, ptr %9, align 8, !tbaa !79
  tail call fastcc void @materialize_characters(ptr noundef nonnull %0, i64 noundef %10) #53
  br label %11

11:                                               ; preds = %3, %8
  %12 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %13 = load i64, ptr %12, align 8, !tbaa !79
  %14 = trunc i64 %13 to i32
  %15 = icmp ugt i32 %14, 1
  br i1 %15, label %16, label %29

16:                                               ; preds = %11
  %17 = tail call fastcc i32 @simple_flag(ptr noundef %1, i32 noundef %2) #53, !range !20
  %18 = icmp eq i32 %17, 0
  br i1 %18, label %60, label %19

19:                                               ; preds = %16
  %20 = icmp eq i32 %2, 0
  br i1 %20, label %26, label %21

21:                                               ; preds = %19
  %22 = add i32 %14, -1
  %23 = udiv i32 1073741823, %2
  %24 = icmp ult i32 %22, %23
  br i1 %24, label %26, label %25

25:                                               ; preds = %21
  tail call fastcc void @out_of_memory() #57
  unreachable

26:                                               ; preds = %21, %19
  %27 = add i32 %14, -1
  %28 = mul i32 %27, %2
  br label %31

29:                                               ; preds = %11
  %30 = icmp eq i32 %14, 0
  br i1 %30, label %82, label %31

31:                                               ; preds = %26, %29
  %32 = phi i32 [ %28, %26 ], [ 0, %29 ]
  %33 = load ptr, ptr %0, align 8, !tbaa !69
  br label %34

34:                                               ; preds = %31, %56
  %35 = phi i32 [ %57, %56 ], [ %32, %31 ]
  %36 = phi i32 [ %58, %56 ], [ 0, %31 ]
  %37 = getelementptr inbounds %struct.FaString, ptr %33, i32 %36, i32 2
  %38 = load i32, ptr %37, align 4, !tbaa !19
  %39 = and i32 %38, 1073741824
  %40 = icmp eq i32 %39, 0
  br i1 %40, label %44, label %41

41:                                               ; preds = %34
  %42 = lshr i32 %38, 24
  %43 = and i32 %42, 31
  br label %47

44:                                               ; preds = %34
  %45 = getelementptr inbounds %struct.FaString, ptr %33, i32 %36, i32 1
  %46 = load i32, ptr %45, align 4, !tbaa !18
  br label %47

47:                                               ; preds = %41, %44
  %48 = phi i32 [ %43, %41 ], [ %46, %44 ]
  %49 = icmp ne i32 %48, 0
  %50 = icmp sgt i32 %38, -1
  %51 = and i1 %49, %50
  br i1 %51, label %60, label %52

52:                                               ; preds = %47
  %53 = sub i32 1073741823, %35
  %54 = icmp ugt i32 %53, %48
  br i1 %54, label %56, label %55

55:                                               ; preds = %52
  tail call fastcc void @out_of_memory() #57
  unreachable

56:                                               ; preds = %52
  %57 = add i32 %48, %35
  %58 = add nuw i32 %36, 1
  %59 = icmp eq i32 %58, %14
  br i1 %59, label %82, label %34, !llvm.loop !100

60:                                               ; preds = %47, %16
  call void @fa_array_join(ptr noundef %0, ptr noundef %1, i32 noundef %2, ptr noundef nonnull %4) #53
  %61 = getelementptr inbounds %struct.FaString, ptr %4, i32 0, i32 2
  %62 = load i32, ptr %61, align 4, !tbaa !19
  %63 = freeze i32 %62
  %64 = and i32 %63, 1073741824
  %65 = icmp eq i32 %64, 0
  %66 = lshr i32 %63, 24
  %67 = and i32 %66, 31
  %68 = getelementptr inbounds %struct.FaString, ptr %4, i32 0, i32 1
  %69 = load i32, ptr %68, align 4
  %70 = select i1 %65, i32 %69, i32 %67
  %71 = icmp sgt i32 %63, -1
  br i1 %71, label %72, label %76

72:                                               ; preds = %60
  %73 = load ptr, ptr %4, align 4
  %74 = select i1 %65, ptr %73, ptr %4
  %75 = call i32 @fa_graphemes(ptr noundef %74, i32 noundef %70) #53
  br label %76

76:                                               ; preds = %60, %72
  %77 = phi i32 [ %75, %72 ], [ %70, %60 ]
  %78 = and i32 %63, 1073741823
  br i1 %65, label %79, label %82

79:                                               ; preds = %76
  switch i32 %78, label %80 [
    i32 1073741823, label %82
    i32 0, label %82
  ]

80:                                               ; preds = %79
  %81 = load ptr, ptr %4, align 4, !tbaa !13
  tail call void @fa_free(ptr noundef %81, i32 noundef %78, i32 noundef 1) #54
  br label %82

82:                                               ; preds = %56, %29, %79, %79, %76, %80
  %83 = phi i32 [ %77, %80 ], [ %77, %79 ], [ %77, %76 ], [ %77, %79 ], [ 0, %29 ], [ %57, %56 ]
  ret i32 %83
}

; Function Attrs: alwaysinline nounwind optsize
define hidden void @fa_string_pad(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2, i64 noundef %3, ptr nocapture noundef readonly %4, i32 noundef %5, i32 noundef %6, ptr nocapture noundef writeonly %7) local_unnamed_addr #13 {
  %9 = alloca %struct.FaString, align 4
  %10 = alloca %struct.FaString, align 4
  %11 = alloca %struct.FaString, align 4
  %12 = icmp slt i64 %3, 1
  br i1 %12, label %13, label %15

13:                                               ; preds = %8
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %9) #56
  %14 = and i32 %2, -2147483648
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %9, ptr noundef %0, i32 noundef %1, i32 noundef %14) #53
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %7, ptr noundef nonnull align 4 dereferenceable(12) %9, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %9) #56
  br label %57

15:                                               ; preds = %8
  %16 = icmp sgt i32 %2, -1
  %17 = icmp ne i32 %5, 1
  %18 = or i1 %16, %17
  br i1 %18, label %24, label %19

19:                                               ; preds = %15
  %20 = load i8, ptr %4, align 1, !tbaa !1
  %21 = icmp slt i8 %20, 0
  %22 = icmp eq i8 %20, 13
  %23 = or i1 %21, %22
  br i1 %23, label %24, label %25

24:                                               ; preds = %19, %15
  tail call fastcc void @string_pad_slow(ptr noundef %0, i32 noundef %1, i64 noundef %3, ptr noundef %4, i32 noundef %5, i32 noundef %6, ptr noundef %7) #53
  br label %57

25:                                               ; preds = %19
  %26 = zext i32 %1 to i64
  %27 = icmp ult i64 %26, %3
  br i1 %27, label %29, label %28

28:                                               ; preds = %25
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %10) #56
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %10, ptr noundef %0, i32 noundef %1, i32 noundef -2147483648) #53
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %7, ptr noundef nonnull align 4 dereferenceable(12) %10, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %10) #56
  br label %57

29:                                               ; preds = %25
  %30 = icmp ugt i64 %3, 1073741822
  br i1 %30, label %31, label %32

31:                                               ; preds = %29
  tail call fastcc void @out_of_memory() #57
  unreachable

32:                                               ; preds = %29
  %33 = trunc i64 %3 to i32
  %34 = sub i32 %33, %1
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %11) #56
  call void @llvm.memset.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %11, i8 0, i32 12, i1 false)
  %35 = icmp ult i32 %33, 12
  br i1 %35, label %38, label %36

36:                                               ; preds = %32
  %37 = tail call ptr @fa_alloc(i32 noundef %33, i32 noundef 1) #54
  br label %38

38:                                               ; preds = %32, %36
  %39 = phi ptr [ %37, %36 ], [ %11, %32 ]
  %40 = icmp eq i32 %6, 0
  br i1 %40, label %44, label %41

41:                                               ; preds = %38
  %42 = load i8, ptr %4, align 1, !tbaa !1
  call fastcc void @fill_bytes(ptr noundef %39, i8 noundef zeroext %42, i32 noundef %34) #53
  %43 = getelementptr inbounds i8, ptr %39, i32 %34
  call fastcc void @copy_bytes(ptr noundef %43, ptr noundef %0, i32 noundef %1) #53
  br label %47

44:                                               ; preds = %38
  call fastcc void @copy_bytes(ptr noundef %39, ptr noundef %0, i32 noundef %1) #53
  %45 = getelementptr inbounds i8, ptr %39, i32 %1
  %46 = load i8, ptr %4, align 1, !tbaa !1
  call fastcc void @fill_bytes(ptr noundef %45, i8 noundef zeroext %46, i32 noundef %34) #53
  br label %47

47:                                               ; preds = %44, %41
  br i1 %35, label %48, label %52

48:                                               ; preds = %47
  %49 = trunc i64 %3 to i8
  %50 = or i8 %49, -64
  %51 = getelementptr inbounds i8, ptr %11, i32 11
  store i8 %50, ptr %51, align 1, !tbaa !1
  br label %56

52:                                               ; preds = %47
  store ptr %39, ptr %11, align 4, !tbaa !13
  %53 = getelementptr inbounds %struct.FaString, ptr %11, i32 0, i32 1
  store i32 %33, ptr %53, align 4, !tbaa !18
  %54 = or disjoint i32 %33, -2147483648
  %55 = getelementptr inbounds %struct.FaString, ptr %11, i32 0, i32 2
  store i32 %54, ptr %55, align 4, !tbaa !19
  br label %56

56:                                               ; preds = %52, %48
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %7, ptr noundef nonnull align 4 dereferenceable(12) %11, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %11) #56
  br label %57

57:                                               ; preds = %56, %28, %24, %13
  ret void
}

; Function Attrs: noinline nounwind optsize
define internal fastcc void @string_pad_slow(ptr nocapture noundef readonly %0, i32 noundef %1, i64 noundef %2, ptr nocapture noundef readonly %3, i32 noundef %4, i32 noundef %5, ptr nocapture noundef writeonly %6) unnamed_addr #15 {
  %8 = alloca %struct.FaString, align 4
  %9 = tail call i32 @fa_graphemes(ptr noundef %0, i32 noundef %1) #53
  %10 = zext i32 %9 to i64
  %11 = sub nsw i64 %2, %10
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  %12 = icmp eq i32 %5, 0
  br i1 %12, label %13, label %18

13:                                               ; preds = %7
  tail call fastcc void @reserve(i32 noundef %1) #53
  %14 = load ptr, ptr @builder, align 4, !tbaa !6
  %15 = load i32, ptr @builder_length, align 4, !tbaa !4
  %16 = getelementptr inbounds i8, ptr %14, i32 %15
  tail call fastcc void @copy_bytes(ptr noundef %16, ptr noundef %0, i32 noundef %1) #53
  %17 = add i32 %15, %1
  store i32 %17, ptr @builder_length, align 4, !tbaa !4
  br label %18

18:                                               ; preds = %13, %7
  %19 = phi i32 [ %17, %13 ], [ 0, %7 ]
  %20 = icmp ne i32 %4, 0
  %21 = icmp sgt i64 %11, 0
  %22 = and i1 %20, %21
  br i1 %22, label %27, label %23

23:                                               ; preds = %27, %18
  %24 = phi i32 [ %19, %18 ], [ %36, %27 ]
  br i1 %12, label %25, label %41

25:                                               ; preds = %23
  %26 = load ptr, ptr @builder, align 4, !tbaa !6
  br label %46

27:                                               ; preds = %18, %27
  %28 = phi i32 [ %38, %27 ], [ 0, %18 ]
  %29 = phi i64 [ %39, %27 ], [ 0, %18 ]
  %30 = tail call fastcc i32 @next_boundary(ptr noundef %3, i32 noundef %4, i32 noundef %28) #53
  %31 = getelementptr inbounds i8, ptr %3, i32 %28
  %32 = sub i32 %30, %28
  tail call fastcc void @reserve(i32 noundef %32) #53
  %33 = load ptr, ptr @builder, align 4, !tbaa !6
  %34 = load i32, ptr @builder_length, align 4, !tbaa !4
  %35 = getelementptr inbounds i8, ptr %33, i32 %34
  tail call fastcc void @copy_bytes(ptr noundef %35, ptr noundef %31, i32 noundef %32) #53
  %36 = add i32 %34, %32
  store i32 %36, ptr @builder_length, align 4, !tbaa !4
  %37 = icmp eq i32 %30, %4
  %38 = select i1 %37, i32 0, i32 %30
  %39 = add nuw nsw i64 %29, 1
  %40 = icmp slt i64 %39, %11
  br i1 %40, label %27, label %23, !llvm.loop !101

41:                                               ; preds = %23
  tail call fastcc void @reserve(i32 noundef %1) #53
  %42 = load ptr, ptr @builder, align 4, !tbaa !6
  %43 = load i32, ptr @builder_length, align 4, !tbaa !4
  %44 = getelementptr inbounds i8, ptr %42, i32 %43
  tail call fastcc void @copy_bytes(ptr noundef %44, ptr noundef %0, i32 noundef %1) #53
  %45 = add i32 %43, %1
  store i32 %45, ptr @builder_length, align 4, !tbaa !4
  br label %46

46:                                               ; preds = %25, %41
  %47 = phi i32 [ %24, %25 ], [ %45, %41 ]
  %48 = phi ptr [ %26, %25 ], [ %42, %41 ]
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %8) #56
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %8, ptr noundef %48, i32 noundef %47, i32 noundef 0) #53
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %6, ptr noundef nonnull align 4 dereferenceable(12) %8, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %8) #56
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: write)
define internal fastcc void @fill_bytes(ptr nocapture noundef writeonly %0, i8 noundef zeroext %1, i32 noundef %2) unnamed_addr #23 {
  %4 = icmp ult i32 %2, 17
  br i1 %4, label %5, label %25

5:                                                ; preds = %3
  %6 = zext i8 %1 to i64
  %7 = mul nuw i64 %6, 72340172838076673
  %8 = icmp ugt i32 %2, 7
  br i1 %8, label %9, label %12

9:                                                ; preds = %5
  store i64 %7, ptr %0, align 1
  %10 = getelementptr inbounds i8, ptr %0, i32 %2
  %11 = getelementptr inbounds i8, ptr %10, i32 -8
  store i64 %7, ptr %11, align 1
  br label %26

12:                                               ; preds = %5
  %13 = icmp ugt i32 %2, 3
  br i1 %13, label %14, label %18

14:                                               ; preds = %12
  %15 = trunc i64 %7 to i32
  store i32 %15, ptr %0, align 1
  %16 = getelementptr inbounds i8, ptr %0, i32 %2
  %17 = getelementptr inbounds i8, ptr %16, i32 -4
  store i32 %15, ptr %17, align 1
  br label %26

18:                                               ; preds = %12
  %19 = icmp eq i32 %2, 0
  br i1 %19, label %26, label %20

20:                                               ; preds = %18
  store i8 %1, ptr %0, align 1, !tbaa !1
  %21 = lshr i32 %2, 1
  %22 = getelementptr inbounds i8, ptr %0, i32 %21
  store i8 %1, ptr %22, align 1, !tbaa !1
  %23 = getelementptr i8, ptr %0, i32 %2
  %24 = getelementptr i8, ptr %23, i32 -1
  store i8 %1, ptr %24, align 1, !tbaa !1
  br label %26

25:                                               ; preds = %3
  tail call void @llvm.memset.p0.i32(ptr align 1 %0, i8 %1, i32 %2, i1 false)
  br label %26

26:                                               ; preds = %9, %18, %20, %14, %25
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_arguments_init(i32 noundef %0, ptr nocapture noundef readnone %1) local_unnamed_addr #3 {
  %3 = tail call i32 @fa_host_arguments_size() #54
  %4 = tail call i32 @llvm.umax.i32(i32 %3, i32 1)
  %5 = tail call ptr @fa_alloc(i32 noundef %4, i32 noundef 1) #54
  store ptr %5, ptr @argument_bytes, align 4, !tbaa !6
  tail call void @fa_host_arguments_copy(ptr noundef %5) #54
  store i32 0, ptr @argument_count, align 4, !tbaa !4
  %6 = icmp eq i32 %3, 0
  br i1 %6, label %7, label %13

7:                                                ; preds = %2
  %8 = tail call ptr @fa_alloc(i32 noundef 12, i32 noundef 4) #54
  store ptr %8, ptr @argument_items, align 4, !tbaa !6
  br label %23

9:                                                ; preds = %13
  store i32 %20, ptr @argument_count, align 4, !tbaa !4
  %10 = mul i32 %20, 12
  %11 = add i32 %10, 12
  %12 = tail call ptr @fa_alloc(i32 noundef %11, i32 noundef 4) #54
  store ptr %12, ptr @argument_items, align 4, !tbaa !6
  br label %24

13:                                               ; preds = %2, %13
  %14 = phi i32 [ %21, %13 ], [ 0, %2 ]
  %15 = phi i32 [ %20, %13 ], [ 0, %2 ]
  %16 = getelementptr inbounds i8, ptr %5, i32 %14
  %17 = load i8, ptr %16, align 1, !tbaa !1
  %18 = icmp eq i8 %17, 0
  %19 = zext i1 %18 to i32
  %20 = add i32 %15, %19
  %21 = add nuw i32 %14, 1
  %22 = icmp eq i32 %21, %3
  br i1 %22, label %9, label %13, !llvm.loop !102

23:                                               ; preds = %42, %7
  ret void

24:                                               ; preds = %9, %42
  %25 = phi i32 [ %43, %42 ], [ 0, %9 ]
  %26 = phi i32 [ %45, %42 ], [ 0, %9 ]
  %27 = phi i32 [ %44, %42 ], [ 0, %9 ]
  %28 = getelementptr inbounds i8, ptr %5, i32 %25
  %29 = load i8, ptr %28, align 1, !tbaa !1
  %30 = icmp eq i8 %29, 0
  br i1 %30, label %33, label %31

31:                                               ; preds = %24
  %32 = add nuw i32 %25, 1
  br label %42

33:                                               ; preds = %24
  %34 = getelementptr inbounds i8, ptr %5, i32 %27
  %35 = getelementptr inbounds %struct.FaString, ptr %12, i32 %26
  store ptr %34, ptr %35, align 4, !tbaa !13
  %36 = sub i32 %25, %27
  %37 = getelementptr inbounds %struct.FaString, ptr %12, i32 %26, i32 1
  store i32 %36, ptr %37, align 4, !tbaa !18
  %38 = tail call fastcc i32 @simple_flag(ptr noundef %34, i32 noundef %36) #53, !range !20
  %39 = getelementptr inbounds %struct.FaString, ptr %12, i32 %26, i32 2
  store i32 %38, ptr %39, align 4, !tbaa !19
  %40 = add i32 %26, 1
  %41 = add nuw i32 %25, 1
  br label %42

42:                                               ; preds = %31, %33
  %43 = phi i32 [ %32, %31 ], [ %41, %33 ]
  %44 = phi i32 [ %27, %31 ], [ %41, %33 ]
  %45 = phi i32 [ %26, %31 ], [ %40, %33 ]
  %46 = icmp eq i32 %43, %3
  br i1 %46, label %23, label %24, !llvm.loop !103
}

; Function Attrs: optsize
declare i32 @fa_host_arguments_size() local_unnamed_addr #24

; Function Attrs: optsize
declare void @fa_host_arguments_copy(ptr noundef) local_unnamed_addr #25

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read)
define hidden i32 @fa_array_storage(ptr nocapture noundef readonly %0) local_unnamed_addr #26 {
  %2 = getelementptr inbounds i8, ptr %0, i32 24
  %3 = load i64, ptr %2, align 1
  %4 = icmp ugt i64 %3, 1
  %5 = trunc i64 %3 to i32
  %6 = select i1 %4, i32 %5, i32 0
  ret i32 %6
}

; Function Attrs: noinline nounwind optsize
define internal fastcc void @materialize_characters(ptr noundef %0, i64 noundef %1) unnamed_addr #15 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %29, label %4

4:                                                ; preds = %2
  %5 = icmp ugt i64 %1, 357913941
  br i1 %5, label %6, label %7

6:                                                ; preds = %4
  tail call fastcc void @out_of_memory() #57
  unreachable

7:                                                ; preds = %4
  %8 = trunc i64 %1 to i32
  %9 = mul nuw i32 %8, 12
  %10 = tail call ptr @fa_alloc(i32 noundef %9, i32 noundef 16) #54
  %11 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %12 = load i64, ptr %11, align 8, !tbaa !79
  %13 = icmp sgt i64 %12, 0
  br i1 %13, label %15, label %14

14:                                               ; preds = %15, %7
  tail call fastcc void @free_array_items(ptr noundef nonnull %0, i64 noundef 1) #53
  store ptr %10, ptr %0, align 8, !tbaa !69
  br label %29

15:                                               ; preds = %7, %15
  %16 = phi i64 [ %26, %15 ], [ 0, %7 ]
  %17 = trunc i64 %16 to i32
  %18 = getelementptr inbounds %struct.FaString, ptr %10, i32 %17
  %19 = load ptr, ptr %0, align 8, !tbaa !69
  %20 = getelementptr inbounds i8, ptr %19, i32 %17
  %21 = load i8, ptr %20, align 1, !tbaa !1
  %22 = zext i8 %21 to i32
  %23 = getelementptr inbounds i8, ptr @ascii_table, i32 %22
  store ptr %23, ptr %18, align 4, !tbaa.struct !10
  %24 = getelementptr inbounds i8, ptr %18, i32 4
  store i32 1, ptr %24, align 4, !tbaa.struct !11
  %25 = getelementptr inbounds %struct.FaString, ptr %10, i32 %17, i32 2
  store i32 -2147483648, ptr %25, align 4, !tbaa !19
  %26 = add nuw nsw i64 %16, 1
  %27 = load i64, ptr %11, align 8, !tbaa !79
  %28 = icmp slt i64 %26, %27
  br i1 %28, label %15, label %14, !llvm.loop !104

29:                                               ; preds = %2, %14
  %30 = phi i64 [ %1, %14 ], [ 0, %2 ]
  %31 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  store i64 %30, ptr %31, align 8, !tbaa !73
  ret void
}

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define hidden noundef i32 @fa_string_array_get(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef writeonly %2) local_unnamed_addr #27 {
  %4 = icmp slt i64 %1, 0
  %5 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %6 = load i64, ptr %5, align 8, !tbaa !79
  %7 = select i1 %4, i64 %6, i64 0
  %8 = add nsw i64 %7, %1
  %9 = icmp ult i64 %8, %6
  br i1 %9, label %10, label %23

10:                                               ; preds = %3
  %11 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %12 = load i64, ptr %11, align 8, !tbaa !73
  %13 = icmp slt i64 %12, 0
  %14 = load ptr, ptr %0, align 8, !tbaa !69
  %15 = trunc i64 %8 to i32
  br i1 %13, label %16, label %21

16:                                               ; preds = %10
  %17 = getelementptr inbounds i8, ptr %14, i32 %15
  %18 = load i8, ptr %17, align 1, !tbaa !1
  store i8 %18, ptr %2, align 4, !tbaa.struct !10
  %19 = getelementptr inbounds i8, ptr %2, i32 1
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 1 dereferenceable(10) %19, i8 0, i32 10, i1 false)
  %20 = getelementptr inbounds i8, ptr %2, i32 11
  store i8 -63, ptr %20, align 1, !tbaa.struct !105
  br label %23

21:                                               ; preds = %10
  %22 = getelementptr inbounds %struct.FaString, ptr %14, i32 %15
  tail call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %2, ptr noundef nonnull align 4 dereferenceable(12) %22, i32 12, i1 false), !tbaa.struct !10
  br label %23

23:                                               ; preds = %16, %21, %3
  %24 = phi i32 [ 0, %3 ], [ 1, %21 ], [ 1, %16 ]
  ret i32 %24
}

; Function Attrs: noinline nounwind optsize
define internal fastcc void @array_grow(ptr noundef %0, i64 noundef %1) unnamed_addr #15 {
  %3 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %4 = load i64, ptr %3, align 8, !tbaa !73
  %5 = icmp slt i64 %4, 0
  br i1 %5, label %6, label %15

6:                                                ; preds = %2
  %7 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %8 = load i64, ptr %7, align 8, !tbaa !79
  %9 = icmp sgt i64 %8, 4611686018427387903
  br i1 %9, label %10, label %11

10:                                               ; preds = %6
  tail call fastcc void @out_of_memory() #57
  unreachable

11:                                               ; preds = %6
  %12 = icmp slt i64 %8, 4
  %13 = shl nuw nsw i64 %8, 1
  %14 = select i1 %12, i64 4, i64 %13
  tail call fastcc void @materialize_characters(ptr noundef nonnull %0, i64 noundef %14) #53
  br label %27

15:                                               ; preds = %2
  %16 = icmp ult i64 %4, 4
  %17 = shl nuw nsw i64 %4, 1
  %18 = select i1 %16, i64 4, i64 %17
  %19 = mul nsw i64 %18, %1
  %20 = trunc i64 %19 to i32
  %21 = tail call ptr @fa_alloc(i32 noundef %20, i32 noundef 16) #54
  %22 = load ptr, ptr %0, align 8, !tbaa !69
  %23 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %24 = load i64, ptr %23, align 8, !tbaa !79
  %25 = mul nsw i64 %24, %1
  %26 = trunc i64 %25 to i32
  tail call fastcc void @copy_bytes(ptr noundef %21, ptr noundef %22, i32 noundef %26) #53
  tail call fastcc void @free_array_items(ptr noundef nonnull %0, i64 noundef %1) #53
  store ptr %21, ptr %0, align 8, !tbaa !69
  store i64 %18, ptr %3, align 8, !tbaa !73
  br label %27

27:                                               ; preds = %15, %11
  ret void
}

; Function Attrs: noinline nounwind optsize
define hidden void @fa_array_grow(ptr noundef %0, i64 noundef %1) local_unnamed_addr #15 {
  tail call fastcc void @array_grow(ptr noundef %0, i64 noundef %1) #53
  ret void
}

; Function Attrs: alwaysinline nounwind optsize
define hidden void @fa_array_room(ptr noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #13 {
  %4 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %5 = load i64, ptr %4, align 8, !tbaa !73
  %6 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %7 = load i64, ptr %6, align 8, !tbaa !79
  %8 = sub nsw i64 %5, %7
  %9 = icmp slt i64 %8, %1
  br i1 %9, label %10, label %11, !prof !35

10:                                               ; preds = %3
  tail call fastcc void @array_extend_room(ptr noundef nonnull %0, i64 noundef %1, i64 noundef %2) #53
  br label %11

11:                                               ; preds = %10, %3
  ret void
}

; Function Attrs: noinline nounwind optsize
define internal fastcc void @array_extend_room(ptr noundef %0, i64 noundef %1, i64 noundef %2) unnamed_addr #15 {
  %4 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %5 = load i64, ptr %4, align 8, !tbaa !79
  %6 = sub nsw i64 9223372036854775807, %1
  %7 = icmp sgt i64 %5, %6
  br i1 %7, label %12, label %8

8:                                                ; preds = %3
  %9 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %10 = load i64, ptr %9, align 8, !tbaa !73
  %11 = icmp sgt i64 %10, 4611686018427387903
  br i1 %11, label %12, label %13

12:                                               ; preds = %8, %3
  tail call fastcc void @out_of_memory() #57
  unreachable

13:                                               ; preds = %8
  %14 = icmp slt i64 %10, 2
  %15 = shl nuw nsw i64 %10, 1
  %16 = select i1 %14, i64 4, i64 %15
  %17 = add nsw i64 %5, %1
  %18 = tail call i64 @llvm.smax.i64(i64 %16, i64 %17)
  tail call void @fa_array_reserve(ptr noundef nonnull %0, i64 noundef %18, i64 noundef %2) #53
  ret void
}

; Function Attrs: alwaysinline nounwind optsize
define hidden void @fa_array_fill(ptr noundef %0, i64 noundef %1, i32 noundef %2) local_unnamed_addr #13 {
  %4 = icmp slt i64 %1, 1
  br i1 %4, label %23, label %5

5:                                                ; preds = %3
  %6 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %7 = load i64, ptr %6, align 8, !tbaa !73
  %8 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %9 = load i64, ptr %8, align 8, !tbaa !79
  %10 = sub nsw i64 %7, %9
  %11 = icmp slt i64 %10, %1
  br i1 %11, label %12, label %14, !prof !35

12:                                               ; preds = %5
  tail call fastcc void @array_extend_room(ptr noundef nonnull %0, i64 noundef %1, i64 noundef 1) #53
  %13 = load i64, ptr %8, align 8, !tbaa !79
  br label %14

14:                                               ; preds = %5, %12
  %15 = phi i64 [ %9, %5 ], [ %13, %12 ]
  %16 = load ptr, ptr %0, align 8, !tbaa !69
  %17 = trunc i64 %15 to i32
  %18 = getelementptr inbounds i8, ptr %16, i32 %17
  %19 = trunc i32 %2 to i8
  %20 = trunc i64 %1 to i32
  tail call fastcc void @fill_bytes(ptr noundef %18, i8 noundef zeroext %19, i32 noundef %20) #53
  %21 = load i64, ptr %8, align 8, !tbaa !79
  %22 = add nsw i64 %21, %1
  store i64 %22, ptr %8, align 8, !tbaa !79
  br label %23

23:                                               ; preds = %3, %14
  ret void
}

; Function Attrs: alwaysinline nounwind optsize
define hidden void @fa_array_extend(ptr noundef %0, ptr nocapture noundef readonly %1, i64 noundef %2, i64 noundef %3, i64 noundef %4) local_unnamed_addr #13 {
  %6 = icmp slt i64 %3, 1
  br i1 %6, label %29, label %7

7:                                                ; preds = %5
  %8 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %9 = load i64, ptr %8, align 8, !tbaa !73
  %10 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %11 = load i64, ptr %10, align 8, !tbaa !79
  %12 = sub nsw i64 %9, %11
  %13 = icmp slt i64 %12, %3
  br i1 %13, label %14, label %16, !prof !35

14:                                               ; preds = %7
  tail call fastcc void @array_extend_room(ptr noundef nonnull %0, i64 noundef %3, i64 noundef %4) #53
  %15 = load i64, ptr %10, align 8, !tbaa !79
  br label %16

16:                                               ; preds = %7, %14
  %17 = phi i64 [ %11, %7 ], [ %15, %14 ]
  %18 = load ptr, ptr %0, align 8, !tbaa !69
  %19 = mul nsw i64 %17, %4
  %20 = trunc i64 %19 to i32
  %21 = getelementptr inbounds i8, ptr %18, i32 %20
  %22 = load ptr, ptr %1, align 8, !tbaa !69
  %23 = mul nsw i64 %4, %2
  %24 = trunc i64 %23 to i32
  %25 = getelementptr inbounds i8, ptr %22, i32 %24
  %26 = mul nsw i64 %4, %3
  %27 = trunc i64 %26 to i32
  tail call fastcc void @copy_bytes(ptr noundef %21, ptr noundef %25, i32 noundef %27) #53
  %28 = add nsw i64 %17, %3
  store i64 %28, ptr %10, align 8, !tbaa !79
  br label %29

29:                                               ; preds = %5, %16
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_array_append(ptr noundef %0, ptr noundef %1, i64 noundef %2) local_unnamed_addr #3 {
  %4 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %5 = load i64, ptr %4, align 8, !tbaa !73
  %6 = icmp slt i64 %5, 0
  br i1 %6, label %7, label %10, !prof !35

7:                                                ; preds = %3
  %8 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %9 = load i64, ptr %8, align 8, !tbaa !79
  tail call fastcc void @materialize_characters(ptr noundef nonnull %0, i64 noundef %9) #53
  br label %10

10:                                               ; preds = %3, %7
  %11 = getelementptr inbounds %struct.FaArrayHeader, ptr %1, i32 0, i32 2
  %12 = load i64, ptr %11, align 8, !tbaa !73
  %13 = icmp slt i64 %12, 0
  br i1 %13, label %14, label %17, !prof !35

14:                                               ; preds = %10
  %15 = getelementptr inbounds %struct.FaArrayHeader, ptr %1, i32 0, i32 1
  %16 = load i64, ptr %15, align 8, !tbaa !79
  tail call fastcc void @materialize_characters(ptr noundef nonnull %1, i64 noundef %16) #53
  br label %17

17:                                               ; preds = %10, %14
  %18 = getelementptr inbounds %struct.FaArrayHeader, ptr %1, i32 0, i32 1
  %19 = load i64, ptr %18, align 8, !tbaa !79
  %20 = icmp sgt i64 %19, 0
  br i1 %20, label %21, label %23

21:                                               ; preds = %17
  %22 = trunc i64 %2 to i32
  br label %24

23:                                               ; preds = %24, %17
  ret void

24:                                               ; preds = %21, %24
  %25 = phi i64 [ 0, %21 ], [ %31, %24 ]
  %26 = tail call ptr @fa_array_push_slot(ptr noundef %0, i64 noundef %2) #53
  %27 = load ptr, ptr %1, align 8, !tbaa !69
  %28 = mul nsw i64 %25, %2
  %29 = trunc i64 %28 to i32
  %30 = getelementptr inbounds i8, ptr %27, i32 %29
  tail call fastcc void @copy_bytes(ptr noundef %26, ptr noundef %30, i32 noundef %22) #53
  %31 = add nuw nsw i64 %25, 1
  %32 = icmp eq i64 %31, %19
  br i1 %32, label %23, label %24, !llvm.loop !106
}

; Function Attrs: mustprogress nofree noinline norecurse nosync nounwind optsize willreturn memory(none)
define hidden i64 @fa_index_from_end(i64 noundef %0, i64 noundef %1) local_unnamed_addr #28 {
  %3 = add nsw i64 %1, %0
  %4 = tail call i64 @llvm.smax.i64(i64 %3, i64 -1)
  %5 = icmp sgt i64 %0, -1
  %6 = select i1 %5, i64 -1, i64 %4
  ret i64 %6
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_array_slice(ptr noundef %0, i64 noundef %1, i64 noundef %2, i8 noundef signext %3, i64 noundef %4, ptr nocapture noundef writeonly %5) local_unnamed_addr #3 {
  %7 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %8 = load i64, ptr %7, align 8, !tbaa !73
  %9 = icmp slt i64 %8, 0
  br i1 %9, label %10, label %13, !prof !35

10:                                               ; preds = %6
  %11 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %12 = load i64, ptr %11, align 8, !tbaa !79
  tail call fastcc void @materialize_characters(ptr noundef nonnull %0, i64 noundef %12) #53
  br label %13

13:                                               ; preds = %6, %10
  %14 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %15 = load i64, ptr %14, align 8, !tbaa !79
  %16 = zext i8 %3 to i32
  %17 = and i32 %16, 1
  %18 = icmp eq i32 %17, 0
  br i1 %18, label %30, label %19

19:                                               ; preds = %13
  %20 = sub nsw i64 0, %15
  %21 = icmp sgt i64 %20, %1
  %22 = icmp slt i64 %15, %1
  %23 = or i1 %21, %22
  br i1 %23, label %24, label %26

24:                                               ; preds = %19
  %25 = tail call ptr @fa_error_index(i64 noundef %1, i64 noundef %15) #53
  br label %65

26:                                               ; preds = %19
  %27 = icmp slt i64 %1, 0
  %28 = select i1 %27, i64 %15, i64 0
  %29 = add nsw i64 %28, %1
  br label %30

30:                                               ; preds = %26, %13
  %31 = phi i64 [ %29, %26 ], [ 0, %13 ]
  %32 = and i32 %16, 2
  %33 = icmp eq i32 %32, 0
  br i1 %33, label %53, label %34

34:                                               ; preds = %30
  %35 = and i32 %16, 4
  %36 = lshr exact i32 %35, 2
  %37 = sub nsw i64 0, %15
  %38 = icmp sgt i64 %37, %2
  br i1 %38, label %51, label %39

39:                                               ; preds = %34
  %40 = shl nuw i32 %35, 29
  %41 = ashr exact i32 %40, 31
  %42 = sext i32 %41 to i64
  %43 = add nsw i64 %15, %42
  %44 = icmp slt i64 %43, %2
  br i1 %44, label %51, label %45

45:                                               ; preds = %39
  %46 = icmp slt i64 %2, 0
  %47 = select i1 %46, i64 %15, i64 0
  %48 = zext nneg i32 %36 to i64
  %49 = add i64 %48, %2
  %50 = add i64 %49, %47
  br label %53

51:                                               ; preds = %34, %39
  %52 = tail call ptr @fa_error_index(i64 noundef %2, i64 noundef %15) #53
  br label %65

53:                                               ; preds = %45, %30
  %54 = phi i64 [ %15, %30 ], [ %50, %45 ]
  %55 = tail call i64 @llvm.smax.i64(i64 %54, i64 %31)
  %56 = sub nsw i64 %55, %31
  %57 = tail call ptr @fa_array_new(i64 noundef %56, i64 noundef %56, i64 noundef %4) #53
  %58 = load ptr, ptr %57, align 8, !tbaa !69
  %59 = load ptr, ptr %0, align 8, !tbaa !69
  %60 = mul nsw i64 %31, %4
  %61 = trunc i64 %60 to i32
  %62 = getelementptr inbounds i8, ptr %59, i32 %61
  %63 = mul nsw i64 %56, %4
  %64 = trunc i64 %63 to i32
  tail call fastcc void @copy_bytes(ptr noundef %58, ptr noundef %62, i32 noundef %64) #53
  store ptr %57, ptr %5, align 4, !tbaa !6
  br label %65

65:                                               ; preds = %51, %53, %24
  %66 = phi ptr [ %25, %24 ], [ null, %53 ], [ %52, %51 ]
  ret ptr %66
}

; Function Attrs: nounwind optsize
define hidden void @fa_array_reserve(ptr noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #3 {
  %4 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %5 = load i64, ptr %4, align 8, !tbaa !73
  %6 = icmp slt i64 %5, 0
  br i1 %6, label %7, label %12

7:                                                ; preds = %3
  %8 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %9 = load i64, ptr %8, align 8, !tbaa !79
  %10 = icmp slt i64 %9, %1
  br i1 %10, label %11, label %29

11:                                               ; preds = %7
  tail call fastcc void @materialize_characters(ptr noundef nonnull %0, i64 noundef %1) #53
  br label %29

12:                                               ; preds = %3
  %13 = icmp slt i64 %5, %1
  br i1 %13, label %14, label %29

14:                                               ; preds = %12
  %15 = icmp sgt i64 %2, 0
  br i1 %15, label %16, label %20

16:                                               ; preds = %14
  %17 = udiv i64 2147483647, %2
  %18 = icmp ult i64 %17, %1
  br i1 %18, label %19, label %20

19:                                               ; preds = %16
  tail call fastcc void @out_of_memory() #57
  unreachable

20:                                               ; preds = %16, %14
  %21 = mul nsw i64 %2, %1
  %22 = trunc i64 %21 to i32
  %23 = tail call ptr @fa_alloc(i32 noundef %22, i32 noundef 16) #54
  %24 = load ptr, ptr %0, align 8, !tbaa !69
  %25 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %26 = load i64, ptr %25, align 8, !tbaa !79
  %27 = mul nsw i64 %26, %2
  %28 = trunc i64 %27 to i32
  tail call fastcc void @copy_bytes(ptr noundef %23, ptr noundef %24, i32 noundef %28) #53
  tail call fastcc void @free_array_items(ptr noundef nonnull %0, i64 noundef %2) #53
  store ptr %23, ptr %0, align 8, !tbaa !69
  store i64 %1, ptr %4, align 8, !tbaa !73
  br label %29

29:                                               ; preds = %12, %7, %11, %20
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @free_array_items(ptr noundef %0, i64 noundef %1) unnamed_addr #3 {
  %3 = getelementptr inbounds i8, ptr %0, i32 24
  %4 = load i64, ptr %3, align 1
  %5 = icmp ne i64 %4, 0
  %6 = load ptr, ptr %0, align 8, !tbaa !69
  %7 = getelementptr inbounds i8, ptr %0, i32 32
  %8 = icmp eq ptr %6, %7
  %9 = select i1 %5, i1 %8, i1 false
  br i1 %9, label %10, label %11

10:                                               ; preds = %2
  store i64 0, ptr %3, align 1
  br label %23

11:                                               ; preds = %2
  %12 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %13 = load i64, ptr %12, align 8, !tbaa !73
  %14 = icmp slt i64 %13, 0
  br i1 %14, label %15, label %18

15:                                               ; preds = %11
  %16 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %17 = load i64, ptr %16, align 8, !tbaa !79
  br label %20

18:                                               ; preds = %11
  %19 = mul nsw i64 %13, %1
  br label %20

20:                                               ; preds = %18, %15
  %21 = phi i64 [ %17, %15 ], [ %19, %18 ]
  %22 = trunc i64 %21 to i32
  tail call void @fa_free(ptr noundef %6, i32 noundef %22, i32 noundef 16) #54
  br label %23

23:                                               ; preds = %20, %10
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_array_assign(ptr noundef %0, ptr noundef %1, i64 noundef %2) local_unnamed_addr #3 {
  %4 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %5 = load i64, ptr %4, align 8, !tbaa !73
  %6 = icmp slt i64 %5, 0
  br i1 %6, label %7, label %10, !prof !35

7:                                                ; preds = %3
  %8 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %9 = load i64, ptr %8, align 8, !tbaa !79
  tail call fastcc void @materialize_characters(ptr noundef nonnull %0, i64 noundef %9) #53
  br label %10

10:                                               ; preds = %3, %7
  %11 = getelementptr inbounds %struct.FaArrayHeader, ptr %1, i32 0, i32 2
  %12 = load i64, ptr %11, align 8, !tbaa !73
  %13 = icmp slt i64 %12, 0
  br i1 %13, label %14, label %17, !prof !35

14:                                               ; preds = %10
  %15 = getelementptr inbounds %struct.FaArrayHeader, ptr %1, i32 0, i32 1
  %16 = load i64, ptr %15, align 8, !tbaa !79
  tail call fastcc void @materialize_characters(ptr noundef nonnull %1, i64 noundef %16) #53
  br label %17

17:                                               ; preds = %10, %14
  %18 = icmp eq ptr %0, %1
  br i1 %18, label %39, label %19

19:                                               ; preds = %17
  %20 = load i64, ptr %4, align 8, !tbaa !73
  %21 = getelementptr inbounds %struct.FaArrayHeader, ptr %1, i32 0, i32 1
  %22 = load i64, ptr %21, align 8, !tbaa !79
  %23 = icmp slt i64 %20, %22
  br i1 %23, label %26, label %24

24:                                               ; preds = %19
  %25 = load ptr, ptr %0, align 8, !tbaa !69
  br label %31

26:                                               ; preds = %19
  %27 = mul nsw i64 %22, %2
  %28 = trunc i64 %27 to i32
  %29 = tail call ptr @fa_alloc(i32 noundef %28, i32 noundef 16) #54
  tail call fastcc void @free_array_items(ptr noundef nonnull %0, i64 noundef %2) #53
  store ptr %29, ptr %0, align 8, !tbaa !69
  %30 = load i64, ptr %21, align 8, !tbaa !79
  store i64 %30, ptr %4, align 8, !tbaa !73
  br label %31

31:                                               ; preds = %24, %26
  %32 = phi i64 [ %22, %24 ], [ %30, %26 ]
  %33 = phi ptr [ %25, %24 ], [ %29, %26 ]
  %34 = load ptr, ptr %1, align 8, !tbaa !69
  %35 = mul nsw i64 %32, %2
  %36 = trunc i64 %35 to i32
  tail call fastcc void @copy_bytes(ptr noundef %33, ptr noundef %34, i32 noundef %36) #53
  %37 = load i64, ptr %21, align 8, !tbaa !79
  %38 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  store i64 %37, ptr %38, align 8, !tbaa !79
  br label %39

39:                                               ; preds = %17, %31
  ret void
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_array_copy(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #3 {
  %3 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %4 = load i64, ptr %3, align 8, !tbaa !73
  %5 = icmp slt i64 %4, 0
  %6 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %7 = load i64, ptr %6, align 8, !tbaa !79
  br i1 %5, label %8, label %15

8:                                                ; preds = %2
  %9 = tail call ptr @fa_array_new(i64 noundef %7, i64 noundef %7, i64 noundef 1) #53
  %10 = load ptr, ptr %9, align 8, !tbaa !69
  %11 = load ptr, ptr %0, align 8, !tbaa !69
  %12 = load i64, ptr %6, align 8, !tbaa !79
  %13 = trunc i64 %12 to i32
  tail call fastcc void @copy_bytes(ptr noundef %10, ptr noundef %11, i32 noundef %13) #53
  %14 = getelementptr inbounds %struct.FaArrayHeader, ptr %9, i32 0, i32 2
  store i64 -1, ptr %14, align 8, !tbaa !73
  br label %22

15:                                               ; preds = %2
  %16 = tail call ptr @fa_array_new(i64 noundef %7, i64 noundef %7, i64 noundef %1) #53
  %17 = load ptr, ptr %16, align 8, !tbaa !69
  %18 = load ptr, ptr %0, align 8, !tbaa !69
  %19 = load i64, ptr %6, align 8, !tbaa !79
  %20 = mul nsw i64 %19, %1
  %21 = trunc i64 %20 to i32
  tail call fastcc void @copy_bytes(ptr noundef %17, ptr noundef %18, i32 noundef %21) #53
  br label %22

22:                                               ; preds = %15, %8
  %23 = phi ptr [ %9, %8 ], [ %16, %15 ]
  ret ptr %23
}

; Function Attrs: nounwind optsize
define hidden void @fa_array_reverse(ptr noundef %0, i64 noundef %1) local_unnamed_addr #3 {
  %3 = alloca [64 x i8], align 16
  %4 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %5 = load i64, ptr %4, align 8, !tbaa !73
  %6 = icmp slt i64 %5, 0
  br i1 %6, label %7, label %10, !prof !35

7:                                                ; preds = %2
  %8 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %9 = load i64, ptr %8, align 8, !tbaa !79
  tail call fastcc void @materialize_characters(ptr noundef nonnull %0, i64 noundef %9) #53
  br label %10

10:                                               ; preds = %2, %7
  %11 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %12 = load i64, ptr %11, align 8, !tbaa !79
  %13 = icmp slt i64 %12, 2
  br i1 %13, label %43, label %14

14:                                               ; preds = %10
  call void @llvm.lifetime.start.p0(i64 64, ptr nonnull %3) #56
  %15 = add nuw i64 %12, 4294967295
  %16 = mul i64 %15, %1
  %17 = trunc i64 %16 to i32
  %18 = icmp sgt i32 %17, 0
  br i1 %18, label %19, label %42

19:                                               ; preds = %14
  %20 = load ptr, ptr %0, align 8, !tbaa !69
  %21 = getelementptr inbounds i8, ptr %20, i32 %17
  %22 = icmp sgt i64 %1, 0
  %23 = trunc i64 %1 to i32
  %24 = sub i32 0, %23
  br label %25

25:                                               ; preds = %19, %28
  %26 = phi ptr [ %20, %19 ], [ %29, %28 ]
  %27 = phi ptr [ %21, %19 ], [ %30, %28 ]
  br i1 %22, label %32, label %28

28:                                               ; preds = %32, %25
  %29 = getelementptr inbounds i8, ptr %26, i32 %23
  %30 = getelementptr inbounds i8, ptr %27, i32 %24
  %31 = icmp ult ptr %29, %30
  br i1 %31, label %25, label %42, !llvm.loop !107

32:                                               ; preds = %25, %32
  %33 = phi i64 [ %40, %32 ], [ 0, %25 ]
  %34 = sub nsw i64 %1, %33
  %35 = trunc i64 %34 to i32
  %36 = tail call i32 @llvm.umin.i32(i32 %35, i32 64)
  %37 = trunc i64 %33 to i32
  %38 = getelementptr inbounds i8, ptr %26, i32 %37
  call fastcc void @copy_bytes(ptr noundef nonnull %3, ptr noundef %38, i32 noundef %36) #53
  %39 = getelementptr inbounds i8, ptr %27, i32 %37
  tail call fastcc void @copy_bytes(ptr noundef %38, ptr noundef nonnull %39, i32 noundef %36) #53
  call fastcc void @copy_bytes(ptr noundef nonnull %39, ptr noundef nonnull %3, i32 noundef %36) #53
  %40 = add nuw nsw i64 %33, 64
  %41 = icmp slt i64 %40, %1
  br i1 %41, label %32, label %28, !llvm.loop !108

42:                                               ; preds = %28, %14
  call void @llvm.lifetime.end.p0(i64 64, ptr nonnull %3) #56
  br label %43

43:                                               ; preds = %10, %42
  ret void
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_array_insert_slot(ptr noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #3 {
  %4 = tail call ptr @fa_array_push_slot(ptr noundef %0, i64 noundef %2) #53
  %5 = load ptr, ptr %0, align 8, !tbaa !69
  %6 = mul nsw i64 %2, %1
  %7 = trunc i64 %6 to i32
  %8 = getelementptr inbounds i8, ptr %5, i32 %7
  %9 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %10 = load i64, ptr %9, align 8, !tbaa !79
  %11 = xor i64 %1, -1
  %12 = add i64 %10, %11
  %13 = mul nsw i64 %12, %2
  %14 = trunc i64 %13 to i32
  %15 = trunc i64 %2 to i32
  %16 = getelementptr inbounds i8, ptr %8, i32 %15
  tail call void @llvm.memmove.p0.p0.i32(ptr align 1 %16, ptr align 1 %8, i32 %14, i1 false)
  ret ptr %8
}

; Function Attrs: mustprogress nocallback nofree nounwind willreturn memory(argmem: readwrite)
declare void @llvm.memmove.p0.p0.i32(ptr nocapture writeonly, ptr nocapture readonly, i32, i1 immarg) #9

; Function Attrs: nounwind optsize
define hidden void @fa_array_remove(ptr noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #3 {
  %4 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %5 = load i64, ptr %4, align 8, !tbaa !73
  %6 = icmp slt i64 %5, 0
  br i1 %6, label %7, label %10, !prof !35

7:                                                ; preds = %3
  %8 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %9 = load i64, ptr %8, align 8, !tbaa !79
  tail call fastcc void @materialize_characters(ptr noundef nonnull %0, i64 noundef %9) #53
  br label %10

10:                                               ; preds = %3, %7
  %11 = load ptr, ptr %0, align 8, !tbaa !69
  %12 = mul nsw i64 %2, %1
  %13 = trunc i64 %12 to i32
  %14 = getelementptr inbounds i8, ptr %11, i32 %13
  %15 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %16 = load i64, ptr %15, align 8, !tbaa !79
  %17 = xor i64 %1, -1
  %18 = add i64 %16, %17
  %19 = mul nsw i64 %18, %2
  %20 = trunc i64 %19 to i32
  %21 = trunc i64 %2 to i32
  %22 = getelementptr inbounds i8, ptr %14, i32 %21
  tail call void @llvm.memmove.p0.p0.i32(ptr align 1 %14, ptr align 1 %22, i32 %20, i1 false)
  %23 = load i64, ptr %15, align 8, !tbaa !79
  %24 = add nsw i64 %23, -1
  store i64 %24, ptr %15, align 8, !tbaa !79
  ret void
}

; Function Attrs: alwaysinline nounwind optsize
define hidden void @fa_array_sort(ptr noundef %0, i64 noundef %1, ptr nocapture noundef readonly %2, ptr noundef %3) local_unnamed_addr #13 {
  %5 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %6 = load i64, ptr %5, align 8, !tbaa !73
  %7 = icmp slt i64 %6, 0
  br i1 %7, label %8, label %11, !prof !35

8:                                                ; preds = %4
  %9 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %10 = load i64, ptr %9, align 8, !tbaa !79
  tail call fastcc void @materialize_characters(ptr noundef nonnull %0, i64 noundef %10) #53
  br label %11

11:                                               ; preds = %4, %8
  %12 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %13 = load i64, ptr %12, align 8, !tbaa !79
  %14 = icmp slt i64 %13, 2
  br i1 %14, label %93, label %15

15:                                               ; preds = %11
  %16 = mul nsw i64 %13, %1
  %17 = trunc i64 %16 to i32
  %18 = load ptr, ptr %0, align 8, !tbaa !69
  %19 = tail call ptr @fa_alloc(i32 noundef %17, i32 noundef 16) #54
  %20 = trunc i64 %1 to i32
  br label %21

21:                                               ; preds = %29, %15
  %22 = phi ptr [ %18, %15 ], [ %23, %29 ]
  %23 = phi ptr [ %19, %15 ], [ %22, %29 ]
  %24 = phi i64 [ 1, %15 ], [ %25, %29 ]
  %25 = shl nsw i64 %24, 1
  br label %31

26:                                               ; preds = %29
  %27 = load ptr, ptr %0, align 8, !tbaa !69
  %28 = icmp eq ptr %23, %27
  br i1 %28, label %91, label %90

29:                                               ; preds = %66
  %30 = icmp slt i64 %25, %13
  br i1 %30, label %21, label %26, !llvm.loop !109

31:                                               ; preds = %21, %66
  %32 = phi i64 [ 0, %21 ], [ %35, %66 ]
  %33 = add nsw i64 %32, %24
  %34 = tail call i64 @llvm.smin.i64(i64 %33, i64 %13)
  %35 = add nsw i64 %32, %25
  %36 = tail call i64 @llvm.smin.i64(i64 %35, i64 %13)
  %37 = icmp slt i64 %32, %34
  %38 = icmp slt i64 %33, %36
  %39 = select i1 %37, i1 %38, i1 false
  br i1 %39, label %40, label %66

40:                                               ; preds = %31, %59
  %41 = phi i64 [ %62, %59 ], [ %32, %31 ]
  %42 = phi i64 [ %61, %59 ], [ %34, %31 ]
  %43 = phi i64 [ %60, %59 ], [ %32, %31 ]
  %44 = mul nsw i64 %42, %1
  %45 = trunc i64 %44 to i32
  %46 = getelementptr inbounds i8, ptr %22, i32 %45
  %47 = mul nsw i64 %43, %1
  %48 = trunc i64 %47 to i32
  %49 = getelementptr inbounds i8, ptr %22, i32 %48
  %50 = tail call i32 %2(ptr noundef %3, ptr noundef %46, ptr noundef %49) #54
  %51 = icmp slt i32 %50, 0
  %52 = mul nsw i64 %41, %1
  %53 = trunc i64 %52 to i32
  %54 = getelementptr inbounds i8, ptr %23, i32 %53
  br i1 %51, label %55, label %57

55:                                               ; preds = %40
  tail call fastcc void @copy_bytes(ptr noundef %54, ptr noundef %46, i32 noundef %20) #53
  %56 = add nsw i64 %42, 1
  br label %59

57:                                               ; preds = %40
  tail call fastcc void @copy_bytes(ptr noundef %54, ptr noundef %49, i32 noundef %20) #53
  %58 = add nsw i64 %43, 1
  br label %59

59:                                               ; preds = %57, %55
  %60 = phi i64 [ %43, %55 ], [ %58, %57 ]
  %61 = phi i64 [ %56, %55 ], [ %42, %57 ]
  %62 = add nsw i64 %41, 1
  %63 = icmp slt i64 %60, %34
  %64 = icmp slt i64 %61, %36
  %65 = select i1 %63, i1 %64, i1 false
  br i1 %65, label %40, label %66, !llvm.loop !110

66:                                               ; preds = %59, %31
  %67 = phi i64 [ %32, %31 ], [ %60, %59 ]
  %68 = phi i64 [ %34, %31 ], [ %61, %59 ]
  %69 = phi i64 [ %32, %31 ], [ %62, %59 ]
  %70 = mul nsw i64 %69, %1
  %71 = trunc i64 %70 to i32
  %72 = getelementptr inbounds i8, ptr %23, i32 %71
  %73 = mul nsw i64 %67, %1
  %74 = trunc i64 %73 to i32
  %75 = getelementptr inbounds i8, ptr %22, i32 %74
  %76 = sub nsw i64 %34, %67
  %77 = mul nsw i64 %76, %1
  %78 = trunc i64 %77 to i32
  tail call fastcc void @copy_bytes(ptr noundef %72, ptr noundef %75, i32 noundef %78) #53
  %79 = add nsw i64 %69, %76
  %80 = mul nsw i64 %79, %1
  %81 = trunc i64 %80 to i32
  %82 = getelementptr inbounds i8, ptr %23, i32 %81
  %83 = mul nsw i64 %68, %1
  %84 = trunc i64 %83 to i32
  %85 = getelementptr inbounds i8, ptr %22, i32 %84
  %86 = sub nsw i64 %36, %68
  %87 = mul nsw i64 %86, %1
  %88 = trunc i64 %87 to i32
  tail call fastcc void @copy_bytes(ptr noundef %82, ptr noundef %85, i32 noundef %88) #53
  %89 = icmp slt i64 %35, %13
  br i1 %89, label %31, label %29, !llvm.loop !111

90:                                               ; preds = %26
  tail call fastcc void @copy_bytes(ptr noundef %27, ptr noundef %23, i32 noundef %17) #53
  br label %91

91:                                               ; preds = %26, %90
  %92 = phi ptr [ %23, %90 ], [ %22, %26 ]
  tail call void @fa_free(ptr noundef %92, i32 noundef %17, i32 noundef 16) #54
  br label %93

93:                                               ; preds = %91, %11
  ret void
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_arguments() local_unnamed_addr #3 {
  %1 = load i32, ptr @argument_count, align 4, !tbaa !4
  %2 = zext i32 %1 to i64
  %3 = tail call ptr @fa_array_new(i64 noundef %2, i64 noundef %2, i64 noundef 12) #53
  %4 = load ptr, ptr %3, align 8, !tbaa !69
  %5 = load ptr, ptr @argument_items, align 4, !tbaa !6
  %6 = mul i32 %1, 12
  tail call fastcc void @copy_bytes(ptr noundef %4, ptr noundef %5, i32 noundef %6) #53
  %7 = getelementptr inbounds %struct.FaArrayHeader, ptr %3, i32 0, i32 1
  store i64 %2, ptr %7, align 8, !tbaa !79
  %8 = getelementptr inbounds %struct.FaArrayHeader, ptr %3, i32 0, i32 2
  store i64 %2, ptr %8, align 8, !tbaa !73
  ret ptr %3
}

; Function Attrs: nounwind optsize
define hidden void @fa_array_free(ptr noundef %0, i64 noundef %1) local_unnamed_addr #3 {
  %3 = getelementptr inbounds i8, ptr %0, i32 24
  %4 = load i64, ptr %3, align 1
  %5 = icmp ult i64 %4, 2
  %6 = and i64 %4, 4294967295
  %7 = icmp eq i64 %6, 0
  %8 = or i1 %5, %7
  br i1 %8, label %9, label %28

9:                                                ; preds = %2
  %10 = icmp ne i64 %4, 0
  %11 = load ptr, ptr %0, align 8, !tbaa !69
  %12 = getelementptr inbounds i8, ptr %0, i32 32
  %13 = icmp eq ptr %11, %12
  %14 = select i1 %10, i1 %13, i1 false
  br i1 %14, label %27, label %15

15:                                               ; preds = %9
  %16 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %17 = load i64, ptr %16, align 8, !tbaa !73
  %18 = icmp slt i64 %17, 0
  br i1 %18, label %19, label %22

19:                                               ; preds = %15
  %20 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %21 = load i64, ptr %20, align 8, !tbaa !79
  br label %24

22:                                               ; preds = %15
  %23 = mul nsw i64 %17, %1
  br label %24

24:                                               ; preds = %22, %19
  %25 = phi i64 [ %21, %19 ], [ %23, %22 ]
  %26 = trunc i64 %25 to i32
  tail call void @fa_free(ptr noundef %11, i32 noundef %26, i32 noundef 16) #54
  br label %27

27:                                               ; preds = %9, %24
  tail call void @fa_free(ptr noundef nonnull %0, i32 noundef 32, i32 noundef 8) #54
  br label %28

28:                                               ; preds = %2, %27
  ret void
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_string_split_into(ptr noundef %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3, i32 noundef %4, ptr noundef %5, i32 noundef %6) local_unnamed_addr #3 {
  %8 = alloca %struct.Boundaries, align 4
  %9 = alloca %struct.Boundaries, align 4
  %10 = alloca %struct.Splitting, align 4
  %11 = alloca %struct.FaString, align 4
  %12 = icmp eq i32 %4, 0
  %13 = icmp eq ptr %5, null
  %14 = or i1 %12, %13
  %15 = icmp ult i32 %6, 32
  %16 = or i1 %14, %15
  br i1 %16, label %17, label %19

17:                                               ; preds = %7
  %18 = tail call ptr @fa_string_split(ptr noundef %0, i32 noundef %1, i32 noundef %2, ptr noundef %3, i32 noundef %4) #53
  br label %102

19:                                               ; preds = %7
  %20 = add i32 %6, -32
  %21 = udiv i32 %20, 12
  %22 = udiv i32 %1, %4
  %23 = icmp ult i32 %22, %21
  br i1 %23, label %24, label %26

24:                                               ; preds = %19
  %25 = add nuw nsw i32 %22, 1
  br label %82

26:                                               ; preds = %19
  %27 = and i32 %2, -2147483648
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %8) #56
  store ptr %0, ptr %8, align 4, !tbaa !48
  %28 = getelementptr inbounds %struct.Boundaries, ptr %8, i32 0, i32 1
  store i32 %1, ptr %28, align 4, !tbaa !50
  %29 = getelementptr inbounds %struct.Boundaries, ptr %8, i32 0, i32 2
  store i32 0, ptr %29, align 4, !tbaa !51
  %30 = getelementptr inbounds %struct.Boundaries, ptr %8, i32 0, i32 3
  store i32 %27, ptr %30, align 4, !tbaa !52
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %9) #56
  store ptr %0, ptr %9, align 4, !tbaa !48
  %31 = getelementptr inbounds %struct.Boundaries, ptr %9, i32 0, i32 1
  store i32 %1, ptr %31, align 4, !tbaa !50
  %32 = getelementptr inbounds %struct.Boundaries, ptr %9, i32 0, i32 2
  store i32 0, ptr %32, align 4, !tbaa !51
  %33 = getelementptr inbounds %struct.Boundaries, ptr %9, i32 0, i32 3
  store i32 %27, ptr %33, align 4, !tbaa !52
  %34 = icmp ugt i32 %4, %1
  br i1 %34, label %80, label %35

35:                                               ; preds = %26
  %36 = icmp eq i32 %27, 0
  br label %37

37:                                               ; preds = %75, %35
  %38 = phi i32 [ 0, %35 ], [ %77, %75 ]
  %39 = phi i32 [ 1, %35 ], [ %76, %75 ]
  br i1 %36, label %43, label %40

40:                                               ; preds = %37
  %41 = load i8, ptr %3, align 1, !tbaa !1
  %42 = tail call fastcc i32 @next_byte(ptr noundef %0, i32 noundef %1, i32 noundef %38, i8 noundef zeroext %41) #53
  br label %43

43:                                               ; preds = %40, %37
  %44 = phi i32 [ %42, %40 ], [ %38, %37 ]
  %45 = add i32 %44, %4
  %46 = icmp ugt i32 %45, %1
  br i1 %46, label %80, label %47

47:                                               ; preds = %43
  %48 = getelementptr i8, ptr %0, i32 %44
  %49 = load i8, ptr %48, align 1, !tbaa !1
  %50 = load i8, ptr %3, align 1, !tbaa !1
  %51 = icmp eq i8 %49, %50
  br i1 %51, label %52, label %73

52:                                               ; preds = %47, %56
  %53 = phi i32 [ %54, %56 ], [ 0, %47 ]
  %54 = add nuw i32 %53, 1
  %55 = icmp eq i32 %54, %4
  br i1 %55, label %64, label %56, !llvm.loop !45

56:                                               ; preds = %52
  %57 = getelementptr i8, ptr %48, i32 %54
  %58 = load i8, ptr %57, align 1, !tbaa !1
  %59 = getelementptr inbounds i8, ptr %3, i32 %54
  %60 = load i8, ptr %59, align 1, !tbaa !1
  %61 = icmp eq i8 %58, %60
  br i1 %61, label %52, label %62, !llvm.loop !45

62:                                               ; preds = %56
  %63 = icmp ult i32 %54, %4
  br i1 %63, label %73, label %64

64:                                               ; preds = %52, %62
  %65 = call fastcc i32 @at_boundary(ptr noundef nonnull %8, i32 noundef %44) #53, !range !53
  %66 = icmp eq i32 %65, 0
  br i1 %66, label %73, label %67

67:                                               ; preds = %64
  %68 = call fastcc i32 @at_boundary(ptr noundef nonnull %9, i32 noundef %45) #53, !range !53
  %69 = icmp eq i32 %68, 0
  br i1 %69, label %73, label %70

70:                                               ; preds = %67
  %71 = add i32 %39, 1
  %72 = icmp ugt i32 %71, %21
  br i1 %72, label %80, label %75

73:                                               ; preds = %67, %64, %62, %47
  %74 = add i32 %44, 1
  br label %75

75:                                               ; preds = %73, %70
  %76 = phi i32 [ %39, %73 ], [ %71, %70 ]
  %77 = phi i32 [ %74, %73 ], [ %45, %70 ]
  %78 = add i32 %77, %4
  %79 = icmp ugt i32 %78, %1
  br i1 %79, label %80, label %37, !llvm.loop !112

80:                                               ; preds = %43, %70, %75, %26
  %81 = phi i32 [ 1, %26 ], [ %71, %70 ], [ %39, %43 ], [ %76, %75 ]
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %9) #56
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %8) #56
  br label %82

82:                                               ; preds = %80, %24
  %83 = phi i32 [ %25, %24 ], [ %81, %80 ]
  %84 = icmp ugt i32 %83, %21
  br i1 %84, label %85, label %87

85:                                               ; preds = %82
  %86 = tail call ptr @fa_string_split(ptr noundef %0, i32 noundef %1, i32 noundef %2, ptr noundef %3, i32 noundef %4) #53
  br label %102

87:                                               ; preds = %82
  %88 = getelementptr inbounds i8, ptr %5, i32 32
  store ptr %88, ptr %5, align 8, !tbaa !69
  %89 = getelementptr inbounds %struct.FaArrayHeader, ptr %5, i32 0, i32 1
  store i64 0, ptr %89, align 8, !tbaa !79
  %90 = zext nneg i32 %83 to i64
  %91 = getelementptr inbounds %struct.FaArrayHeader, ptr %5, i32 0, i32 2
  store i64 %90, ptr %91, align 8, !tbaa !73
  %92 = getelementptr inbounds i8, ptr %5, i32 24
  store i64 3, ptr %92, align 1
  call void @llvm.lifetime.start.p0(i64 20, ptr nonnull %10) #56
  store ptr %0, ptr %10, align 4, !tbaa !75
  %93 = getelementptr inbounds %struct.Splitting, ptr %10, i32 0, i32 1
  store i32 0, ptr %93, align 4, !tbaa !77
  %94 = getelementptr inbounds %struct.Splitting, ptr %10, i32 0, i32 2
  store i32 %4, ptr %94, align 4, !tbaa !78
  %95 = getelementptr inbounds %struct.Splitting, ptr %10, i32 0, i32 3
  store ptr %5, ptr %95, align 4, !tbaa !80
  %96 = getelementptr inbounds %struct.Splitting, ptr %10, i32 0, i32 4
  %97 = and i32 %2, -2147483648
  store i32 %97, ptr %96, align 4, !tbaa !81
  call fastcc void @each_match(ptr noundef %0, i32 noundef %1, i32 noundef %97, ptr noundef %3, i32 noundef %4, ptr noundef nonnull @split_one, ptr noundef nonnull %10) #53
  %98 = call ptr @fa_array_push_slot(ptr noundef nonnull %5, i64 noundef 12) #53
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %11) #56
  %99 = load i32, ptr %93, align 4, !tbaa !77
  %100 = getelementptr inbounds i8, ptr %0, i32 %99
  %101 = sub i32 %1, %99
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %11, ptr noundef %100, i32 noundef %101, i32 noundef %97) #53
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %98, ptr noundef nonnull align 4 dereferenceable(12) %11, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %11) #56
  call void @llvm.lifetime.end.p0(i64 20, ptr nonnull %10) #56
  br label %102

102:                                              ; preds = %85, %87, %17
  %103 = phi ptr [ %18, %17 ], [ %86, %85 ], [ %5, %87 ]
  ret ptr %103
}

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read)
define hidden ptr @fa_segmented_at(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2, i32 noundef %3) local_unnamed_addr #26 {
  %5 = zext nneg i32 %3 to i64
  %6 = lshr i64 %1, %5
  %7 = icmp eq i64 %6, 0
  br i1 %7, label %16, label %8

8:                                                ; preds = %4
  %9 = tail call i64 @llvm.ctlz.i64(i64 %6, i1 true), !range !41
  %10 = sext i32 %3 to i64
  %11 = sub nsw i64 %10, %9
  %12 = add nsw i64 %11, 63
  %13 = shl nsw i64 -1, %12
  %14 = trunc i64 %9 to i32
  %15 = sub nuw nsw i32 64, %14
  br label %16

16:                                               ; preds = %4, %8
  %17 = phi i64 [ %13, %8 ], [ 0, %4 ]
  %18 = phi i32 [ %15, %8 ], [ 0, %4 ]
  %19 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 4, i32 %18
  %20 = load ptr, ptr %19, align 4, !tbaa !6
  %21 = add i64 %17, %1
  %22 = mul nsw i64 %21, %2
  %23 = trunc i64 %22 to i32
  %24 = getelementptr inbounds i8, ptr %20, i32 %23
  ret ptr %24
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_segmented_new() local_unnamed_addr #3 {
  %1 = tail call ptr @fa_alloc(i32 noundef 224, i32 noundef 8) #54
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(224) %1, i8 0, i32 224, i1 false)
  ret ptr %1
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: write)
define hidden noundef ptr @fa_segmented_local(ptr noundef returned %0, i64 noundef %1, i64 noundef %2, i32 noundef %3) local_unnamed_addr #10 {
  %5 = getelementptr inbounds i8, ptr %0, i32 8
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(224) %5, i8 0, i32 208, i1 false)
  %6 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 5
  store i64 3, ptr %6, align 8, !tbaa !113
  store i64 %1, ptr %0, align 8, !tbaa !115
  %7 = icmp sgt i64 %1, 0
  br i1 %7, label %8, label %31

8:                                                ; preds = %4
  %9 = ptrtoint ptr %0 to i32
  %10 = add i32 %9, 239
  %11 = and i32 %10, -16
  %12 = inttoptr i32 %11 to ptr
  %13 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 1
  %14 = sext i32 %3 to i64
  br label %15

15:                                               ; preds = %8, %15
  %16 = phi i64 [ 0, %8 ], [ %22, %15 ]
  %17 = phi ptr [ %12, %8 ], [ %27, %15 ]
  %18 = phi i64 [ 0, %8 ], [ %28, %15 ]
  %19 = tail call i64 @llvm.usub.sat.i64(i64 %16, i64 1)
  %20 = add i64 %19, %14
  %21 = shl nuw i64 1, %20
  %22 = add nuw nsw i64 %16, 1
  %23 = trunc i64 %16 to i32
  %24 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 4, i32 %23
  store ptr %17, ptr %24, align 4, !tbaa !6
  %25 = shl i64 %2, %20
  %26 = trunc i64 %25 to i32
  %27 = getelementptr inbounds i8, ptr %17, i32 %26
  %28 = add nsw i64 %21, %18
  %29 = icmp slt i64 %28, %1
  br i1 %29, label %15, label %30, !llvm.loop !116

30:                                               ; preds = %15
  store i64 %22, ptr %13, align 8, !tbaa !117
  br label %31

31:                                               ; preds = %30, %4
  ret ptr %0
}

; Function Attrs: noinline nounwind optsize
define hidden void @fa_segmented_next(ptr nocapture noundef %0, i64 noundef %1, i32 noundef %2) local_unnamed_addr #15 {
  %4 = load i64, ptr %0, align 8, !tbaa !115
  %5 = zext nneg i32 %2 to i64
  %6 = lshr i64 %4, %5
  %7 = icmp eq i64 %6, 0
  br i1 %7, label %15, label %8

8:                                                ; preds = %3
  %9 = tail call i64 @llvm.ctlz.i64(i64 %6, i1 true), !range !41
  %10 = sub nuw nsw i64 64, %9
  %11 = sext i32 %2 to i64
  %12 = sub nsw i64 %11, %9
  %13 = add nsw i64 %12, 63
  %14 = shl nsw i64 -1, %13
  br label %15

15:                                               ; preds = %3, %8
  %16 = phi i64 [ %14, %8 ], [ 0, %3 ]
  %17 = phi i64 [ %10, %8 ], [ 0, %3 ]
  %18 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 1
  %19 = load i64, ptr %18, align 8, !tbaa !117
  %20 = icmp slt i64 %17, %19
  br i1 %20, label %21, label %28

21:                                               ; preds = %15
  %22 = trunc i64 %17 to i32
  %23 = sext i32 %2 to i64
  %24 = tail call i64 @llvm.usub.sat.i64(i64 %17, i64 1)
  %25 = add nsw i64 %24, %23
  %26 = shl i64 %1, %25
  %27 = trunc i64 %26 to i32
  br label %38

28:                                               ; preds = %15
  %29 = sext i32 %2 to i64
  %30 = tail call i64 @llvm.usub.sat.i64(i64 %17, i64 1)
  %31 = add nsw i64 %30, %29
  %32 = shl i64 %1, %31
  %33 = trunc i64 %32 to i32
  %34 = tail call ptr @fa_alloc(i32 noundef %33, i32 noundef 16) #54
  %35 = trunc i64 %17 to i32
  %36 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 4, i32 %35
  store ptr %34, ptr %36, align 4, !tbaa !6
  %37 = add nuw nsw i64 %17, 1
  store i64 %37, ptr %18, align 8, !tbaa !117
  br label %38

38:                                               ; preds = %21, %28
  %39 = phi i32 [ %27, %21 ], [ %33, %28 ]
  %40 = phi i32 [ %22, %21 ], [ %35, %28 ]
  %41 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 4, i32 %40
  %42 = load ptr, ptr %41, align 4, !tbaa !6
  %43 = add i64 %16, %4
  %44 = mul nsw i64 %43, %1
  %45 = trunc i64 %44 to i32
  %46 = getelementptr inbounds i8, ptr %42, i32 %45
  %47 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 2
  store ptr %46, ptr %47, align 8, !tbaa !118
  %48 = load ptr, ptr %41, align 4, !tbaa !6
  %49 = getelementptr inbounds i8, ptr %48, i32 %39
  %50 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 3
  store ptr %49, ptr %50, align 4, !tbaa !119
  ret void
}

; Function Attrs: alwaysinline nounwind optsize
define hidden ptr @fa_segmented_push_slot(ptr nocapture noundef %0, i64 noundef %1, i32 noundef %2) local_unnamed_addr #13 {
  %4 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 2
  %5 = load ptr, ptr %4, align 8, !tbaa !118
  %6 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 3
  %7 = load ptr, ptr %6, align 4, !tbaa !119
  %8 = icmp eq ptr %5, %7
  br i1 %8, label %9, label %11, !prof !35

9:                                                ; preds = %3
  tail call void @fa_segmented_next(ptr noundef nonnull %0, i64 noundef %1, i32 noundef %2) #53
  %10 = load ptr, ptr %4, align 8, !tbaa !118
  br label %11

11:                                               ; preds = %9, %3
  %12 = phi ptr [ %10, %9 ], [ %5, %3 ]
  %13 = trunc i64 %1 to i32
  %14 = getelementptr inbounds i8, ptr %12, i32 %13
  store ptr %14, ptr %4, align 8, !tbaa !118
  %15 = load i64, ptr %0, align 8, !tbaa !115
  %16 = add nsw i64 %15, 1
  store i64 %16, ptr %0, align 8, !tbaa !115
  ret ptr %12
}

; Function Attrs: alwaysinline nounwind optsize
define hidden void @fa_segmented_append(ptr nocapture noundef %0, ptr nocapture noundef readonly %1, i64 noundef %2, i64 noundef %3, i32 noundef %4) local_unnamed_addr #13 {
  %6 = icmp sgt i64 %2, 0
  br i1 %6, label %7, label %24

7:                                                ; preds = %5
  %8 = mul nsw i64 %3, %2
  %9 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 3
  %10 = load ptr, ptr %9, align 4, !tbaa !119
  %11 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 2
  %12 = load ptr, ptr %11, align 8, !tbaa !118
  %13 = ptrtoint ptr %10 to i32
  %14 = ptrtoint ptr %12 to i32
  %15 = sub i32 %13, %14
  %16 = sext i32 %15 to i64
  %17 = icmp sgt i64 %8, %16
  br i1 %17, label %24, label %18

18:                                               ; preds = %7
  %19 = trunc i64 %8 to i32
  tail call fastcc void @copy_bytes(ptr noundef %12, ptr noundef %1, i32 noundef %19) #53
  %20 = load ptr, ptr %11, align 8, !tbaa !118
  %21 = getelementptr inbounds i8, ptr %20, i32 %19
  store ptr %21, ptr %11, align 8, !tbaa !118
  %22 = load i64, ptr %0, align 8, !tbaa !115
  %23 = add nsw i64 %22, %2
  store i64 %23, ptr %0, align 8, !tbaa !115
  br label %25

24:                                               ; preds = %7, %5
  tail call fastcc void @segmented_append_runs(ptr noundef %0, ptr noundef %1, i64 noundef %2, i64 noundef %3, i32 noundef %4) #53
  br label %25

25:                                               ; preds = %24, %18
  ret void
}

; Function Attrs: noinline nounwind optsize
define internal fastcc void @segmented_append_runs(ptr nocapture noundef %0, ptr nocapture noundef readonly %1, i64 noundef %2, i64 noundef %3, i32 noundef %4) unnamed_addr #15 {
  %6 = icmp sgt i64 %2, 0
  br i1 %6, label %7, label %38

7:                                                ; preds = %5
  %8 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 2
  %9 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 3
  %10 = load ptr, ptr %8, align 8, !tbaa !118
  br label %11

11:                                               ; preds = %7, %20
  %12 = phi ptr [ %10, %7 ], [ %32, %20 ]
  %13 = phi ptr [ %1, %7 ], [ %35, %20 ]
  %14 = phi i64 [ %2, %7 ], [ %36, %20 ]
  %15 = load ptr, ptr %9, align 4, !tbaa !119
  %16 = icmp eq ptr %12, %15
  br i1 %16, label %17, label %20

17:                                               ; preds = %11
  tail call void @fa_segmented_next(ptr noundef nonnull %0, i64 noundef %3, i32 noundef %4) #53
  %18 = load ptr, ptr %9, align 4, !tbaa !119
  %19 = load ptr, ptr %8, align 8, !tbaa !118
  br label %20

20:                                               ; preds = %17, %11
  %21 = phi ptr [ %19, %17 ], [ %12, %11 ]
  %22 = phi ptr [ %18, %17 ], [ %15, %11 ]
  %23 = ptrtoint ptr %22 to i32
  %24 = ptrtoint ptr %21 to i32
  %25 = sub i32 %23, %24
  %26 = sext i32 %25 to i64
  %27 = sdiv i64 %26, %3
  %28 = tail call i64 @llvm.smin.i64(i64 %14, i64 %27)
  %29 = mul nsw i64 %28, %3
  %30 = trunc i64 %29 to i32
  tail call fastcc void @copy_bytes(ptr noundef %21, ptr noundef %13, i32 noundef %30) #53
  %31 = load ptr, ptr %8, align 8, !tbaa !118
  %32 = getelementptr inbounds i8, ptr %31, i32 %30
  store ptr %32, ptr %8, align 8, !tbaa !118
  %33 = load i64, ptr %0, align 8, !tbaa !115
  %34 = add nsw i64 %33, %28
  store i64 %34, ptr %0, align 8, !tbaa !115
  %35 = getelementptr inbounds i8, ptr %13, i32 %30
  %36 = sub nsw i64 %14, %28
  %37 = icmp sgt i64 %36, 0
  br i1 %37, label %11, label %38, !llvm.loop !120

38:                                               ; preds = %20, %5
  ret void
}

; Function Attrs: alwaysinline nounwind optsize
define hidden void @fa_segmented_fill(ptr nocapture noundef %0, i64 noundef %1, i32 noundef %2, i32 noundef %3) local_unnamed_addr #13 {
  %5 = icmp sgt i64 %1, 0
  br i1 %5, label %6, label %23

6:                                                ; preds = %4
  %7 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 3
  %8 = load ptr, ptr %7, align 4, !tbaa !119
  %9 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 2
  %10 = load ptr, ptr %9, align 8, !tbaa !118
  %11 = ptrtoint ptr %8 to i32
  %12 = ptrtoint ptr %10 to i32
  %13 = sub i32 %11, %12
  %14 = sext i32 %13 to i64
  %15 = icmp slt i64 %14, %1
  br i1 %15, label %23, label %16

16:                                               ; preds = %6
  %17 = trunc i32 %2 to i8
  %18 = trunc i64 %1 to i32
  tail call fastcc void @fill_bytes(ptr noundef %10, i8 noundef zeroext %17, i32 noundef %18) #53
  %19 = load ptr, ptr %9, align 8, !tbaa !118
  %20 = getelementptr inbounds i8, ptr %19, i32 %18
  store ptr %20, ptr %9, align 8, !tbaa !118
  %21 = load i64, ptr %0, align 8, !tbaa !115
  %22 = add nsw i64 %21, %1
  store i64 %22, ptr %0, align 8, !tbaa !115
  br label %24

23:                                               ; preds = %6, %4
  tail call fastcc void @segmented_fill_runs(ptr noundef %0, i64 noundef %1, i32 noundef %2, i32 noundef %3) #53
  br label %24

24:                                               ; preds = %23, %16
  ret void
}

; Function Attrs: noinline nounwind optsize
define internal fastcc void @segmented_fill_runs(ptr nocapture noundef %0, i64 noundef %1, i32 noundef %2, i32 noundef %3) unnamed_addr #15 {
  %5 = icmp sgt i64 %1, 0
  br i1 %5, label %6, label %34

6:                                                ; preds = %4
  %7 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 2
  %8 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 3
  %9 = trunc i32 %2 to i8
  %10 = load ptr, ptr %7, align 8, !tbaa !118
  br label %11

11:                                               ; preds = %6, %19
  %12 = phi ptr [ %10, %6 ], [ %29, %19 ]
  %13 = phi i64 [ %1, %6 ], [ %32, %19 ]
  %14 = load ptr, ptr %8, align 4, !tbaa !119
  %15 = icmp eq ptr %12, %14
  br i1 %15, label %16, label %19

16:                                               ; preds = %11
  tail call void @fa_segmented_next(ptr noundef nonnull %0, i64 noundef 1, i32 noundef %3) #53
  %17 = load ptr, ptr %8, align 4, !tbaa !119
  %18 = load ptr, ptr %7, align 8, !tbaa !118
  br label %19

19:                                               ; preds = %16, %11
  %20 = phi ptr [ %18, %16 ], [ %12, %11 ]
  %21 = phi ptr [ %17, %16 ], [ %14, %11 ]
  %22 = ptrtoint ptr %21 to i32
  %23 = ptrtoint ptr %20 to i32
  %24 = sub i32 %22, %23
  %25 = sext i32 %24 to i64
  %26 = tail call i64 @llvm.smin.i64(i64 %13, i64 %25)
  %27 = trunc i64 %26 to i32
  tail call fastcc void @fill_bytes(ptr noundef %20, i8 noundef zeroext %9, i32 noundef %27) #53
  %28 = load ptr, ptr %7, align 8, !tbaa !118
  %29 = getelementptr inbounds i8, ptr %28, i32 %27
  store ptr %29, ptr %7, align 8, !tbaa !118
  %30 = load i64, ptr %0, align 8, !tbaa !115
  %31 = add nsw i64 %30, %26
  store i64 %31, ptr %0, align 8, !tbaa !115
  %32 = sub nsw i64 %13, %26
  %33 = icmp sgt i64 %32, 0
  br i1 %33, label %11, label %34, !llvm.loop !121

34:                                               ; preds = %19, %4
  ret void
}

; Function Attrs: alwaysinline nounwind optsize
define hidden void @fa_segmented_extend_to(ptr noundef %0, i32 noundef %1, ptr nocapture noundef readonly %2, i64 noundef %3, i64 noundef %4, i64 noundef %5, i32 noundef %6) local_unnamed_addr #13 {
  %8 = icmp slt i64 %4, 1
  br i1 %8, label %78, label %9

9:                                                ; preds = %7
  %10 = zext nneg i32 %6 to i64
  %11 = lshr i64 %3, %10
  %12 = icmp eq i64 %11, 0
  br i1 %12, label %13, label %15

13:                                               ; preds = %9
  %14 = sext i32 %6 to i64
  br label %22

15:                                               ; preds = %9
  %16 = tail call i64 @llvm.ctlz.i64(i64 %11, i1 true), !range !41
  %17 = sub nuw nsw i64 64, %16
  %18 = sext i32 %6 to i64
  %19 = sub nsw i64 %18, %16
  %20 = add nsw i64 %19, 63
  %21 = shl nsw i64 -1, %20
  br label %22

22:                                               ; preds = %13, %15
  %23 = phi i64 [ %14, %13 ], [ %18, %15 ]
  %24 = phi i64 [ 0, %13 ], [ %21, %15 ]
  %25 = phi i64 [ 0, %13 ], [ %17, %15 ]
  %26 = add i64 %24, %3
  %27 = add nsw i64 %26, %4
  %28 = tail call i64 @llvm.usub.sat.i64(i64 %25, i64 1)
  %29 = add nsw i64 %28, %23
  %30 = shl nuw i64 1, %29
  %31 = icmp sgt i64 %27, %30
  br i1 %31, label %77, label %32

32:                                               ; preds = %22
  %33 = trunc i64 %25 to i32
  %34 = getelementptr inbounds %struct.FaSegmented, ptr %2, i32 0, i32 4, i32 %33
  %35 = load ptr, ptr %34, align 4, !tbaa !6
  %36 = mul nsw i64 %26, %5
  %37 = trunc i64 %36 to i32
  %38 = getelementptr inbounds i8, ptr %35, i32 %37
  %39 = icmp slt i32 %1, 0
  br i1 %39, label %40, label %59

40:                                               ; preds = %32
  %41 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %42 = load i64, ptr %41, align 8, !tbaa !73
  %43 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %44 = load i64, ptr %43, align 8, !tbaa !79
  %45 = sub nsw i64 %42, %44
  %46 = icmp slt i64 %45, %4
  br i1 %46, label %47, label %49, !prof !35

47:                                               ; preds = %40
  tail call fastcc void @array_extend_room(ptr noundef nonnull %0, i64 noundef %4, i64 noundef %5) #53
  %48 = load i64, ptr %43, align 8, !tbaa !79
  br label %49

49:                                               ; preds = %40, %47
  %50 = phi i64 [ %44, %40 ], [ %48, %47 ]
  %51 = load ptr, ptr %0, align 8, !tbaa !69
  %52 = mul nsw i64 %50, %5
  %53 = trunc i64 %52 to i32
  %54 = getelementptr inbounds i8, ptr %51, i32 %53
  %55 = mul nsw i64 %5, %4
  %56 = trunc i64 %55 to i32
  tail call fastcc void @copy_bytes(ptr noundef %54, ptr noundef %38, i32 noundef %56) #53
  %57 = load i64, ptr %43, align 8, !tbaa !79
  %58 = add nsw i64 %57, %4
  store i64 %58, ptr %43, align 8, !tbaa !79
  br label %78

59:                                               ; preds = %32
  %60 = mul nsw i64 %5, %4
  %61 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 3
  %62 = load ptr, ptr %61, align 4, !tbaa !119
  %63 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 2
  %64 = load ptr, ptr %63, align 8, !tbaa !118
  %65 = ptrtoint ptr %62 to i32
  %66 = ptrtoint ptr %64 to i32
  %67 = sub i32 %65, %66
  %68 = sext i32 %67 to i64
  %69 = icmp sgt i64 %60, %68
  br i1 %69, label %76, label %70

70:                                               ; preds = %59
  %71 = trunc i64 %60 to i32
  tail call fastcc void @copy_bytes(ptr noundef %64, ptr noundef %38, i32 noundef %71) #53
  %72 = load ptr, ptr %63, align 8, !tbaa !118
  %73 = getelementptr inbounds i8, ptr %72, i32 %71
  store ptr %73, ptr %63, align 8, !tbaa !118
  %74 = load i64, ptr %0, align 8, !tbaa !115
  %75 = add nsw i64 %74, %4
  store i64 %75, ptr %0, align 8, !tbaa !115
  br label %78

76:                                               ; preds = %59
  tail call fastcc void @segmented_append_runs(ptr noundef nonnull %0, ptr noundef %38, i64 noundef %4, i64 noundef %5, i32 noundef %1) #53
  br label %78

77:                                               ; preds = %22
  tail call fastcc void @segmented_extend_runs(ptr noundef %0, i32 noundef %1, ptr noundef %2, i64 noundef %3, i64 noundef %4, i64 noundef %5, i32 noundef %6) #53
  br label %78

78:                                               ; preds = %77, %49, %70, %76, %7
  ret void
}

; Function Attrs: noinline nounwind optsize
define internal fastcc void @segmented_extend_runs(ptr noundef %0, i32 noundef %1, ptr nocapture noundef readonly %2, i64 noundef %3, i64 noundef %4, i64 noundef %5, i32 noundef %6) unnamed_addr #15 {
  %8 = icmp slt i32 %1, 0
  br i1 %8, label %9, label %17

9:                                                ; preds = %7
  %10 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %11 = load i64, ptr %10, align 8, !tbaa !73
  %12 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %13 = load i64, ptr %12, align 8, !tbaa !79
  %14 = sub nsw i64 %11, %13
  %15 = icmp slt i64 %14, %4
  br i1 %15, label %16, label %17, !prof !35

16:                                               ; preds = %9
  tail call fastcc void @array_extend_room(ptr noundef nonnull %0, i64 noundef %4, i64 noundef %5) #53
  br label %17

17:                                               ; preds = %16, %9, %7
  %18 = icmp sgt i64 %4, 0
  br i1 %18, label %19, label %81

19:                                               ; preds = %17
  %20 = zext nneg i32 %6 to i64
  %21 = sext i32 %6 to i64
  %22 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 3
  %23 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 2
  %24 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  br label %25

25:                                               ; preds = %19, %77
  %26 = phi i64 [ %3, %19 ], [ %78, %77 ]
  %27 = phi i64 [ %4, %19 ], [ %79, %77 ]
  %28 = lshr i64 %26, %20
  %29 = icmp eq i64 %28, 0
  %30 = tail call i64 @llvm.ctlz.i64(i64 %28, i1 true), !range !41
  %31 = sub nuw nsw i64 64, %30
  %32 = sub nsw i64 %21, %30
  %33 = add nsw i64 %32, 63
  %34 = shl nsw i64 -1, %33
  %35 = select i1 %29, i64 0, i64 %34
  %36 = select i1 %29, i64 0, i64 %31
  %37 = tail call i64 @llvm.usub.sat.i64(i64 %36, i64 1)
  %38 = add nsw i64 %37, %21
  %39 = shl nuw i64 1, %38
  %40 = add i64 %35, %26
  %41 = sub nsw i64 %39, %40
  %42 = tail call i64 @llvm.smin.i64(i64 %27, i64 %41)
  %43 = trunc i64 %36 to i32
  %44 = getelementptr inbounds %struct.FaSegmented, ptr %2, i32 0, i32 4, i32 %43
  %45 = load ptr, ptr %44, align 4, !tbaa !6
  %46 = mul nsw i64 %40, %5
  %47 = trunc i64 %46 to i32
  %48 = getelementptr inbounds i8, ptr %45, i32 %47
  br i1 %8, label %49, label %59

49:                                               ; preds = %25
  %50 = load ptr, ptr %0, align 8, !tbaa !69
  %51 = load i64, ptr %24, align 8, !tbaa !79
  %52 = mul nsw i64 %51, %5
  %53 = trunc i64 %52 to i32
  %54 = getelementptr inbounds i8, ptr %50, i32 %53
  %55 = mul nsw i64 %42, %5
  %56 = trunc i64 %55 to i32
  tail call fastcc void @copy_bytes(ptr noundef %54, ptr noundef %48, i32 noundef %56) #53
  %57 = load i64, ptr %24, align 8, !tbaa !79
  %58 = add nsw i64 %57, %42
  store i64 %58, ptr %24, align 8, !tbaa !79
  br label %77

59:                                               ; preds = %25
  %60 = icmp sgt i64 %41, 0
  br i1 %60, label %61, label %76

61:                                               ; preds = %59
  %62 = mul nsw i64 %42, %5
  %63 = load ptr, ptr %22, align 4, !tbaa !119
  %64 = load ptr, ptr %23, align 8, !tbaa !118
  %65 = ptrtoint ptr %63 to i32
  %66 = ptrtoint ptr %64 to i32
  %67 = sub i32 %65, %66
  %68 = sext i32 %67 to i64
  %69 = icmp sgt i64 %62, %68
  br i1 %69, label %76, label %70

70:                                               ; preds = %61
  %71 = trunc i64 %62 to i32
  tail call fastcc void @copy_bytes(ptr noundef %64, ptr noundef %48, i32 noundef %71) #53
  %72 = load ptr, ptr %23, align 8, !tbaa !118
  %73 = getelementptr inbounds i8, ptr %72, i32 %71
  store ptr %73, ptr %23, align 8, !tbaa !118
  %74 = load i64, ptr %0, align 8, !tbaa !115
  %75 = add nsw i64 %74, %42
  store i64 %75, ptr %0, align 8, !tbaa !115
  br label %77

76:                                               ; preds = %61, %59
  tail call fastcc void @segmented_append_runs(ptr noundef %0, ptr noundef %48, i64 noundef %42, i64 noundef %5, i32 noundef %1) #53
  br label %77

77:                                               ; preds = %76, %70, %49
  %78 = add nsw i64 %42, %26
  %79 = sub nsw i64 %27, %42
  %80 = icmp sgt i64 %79, 0
  br i1 %80, label %25, label %81, !llvm.loop !122

81:                                               ; preds = %77, %17
  ret void
}

; Function Attrs: mustprogress nofree noinline norecurse nosync nounwind optsize willreturn memory(argmem: readwrite)
define hidden ptr @fa_segmented_run(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2, i32 noundef %3, ptr nocapture noundef writeonly %4) local_unnamed_addr #29 {
  %6 = zext nneg i32 %3 to i64
  %7 = lshr i64 %1, %6
  %8 = icmp eq i64 %7, 0
  br i1 %8, label %9, label %11

9:                                                ; preds = %5
  %10 = sext i32 %3 to i64
  br label %18

11:                                               ; preds = %5
  %12 = tail call i64 @llvm.ctlz.i64(i64 %7, i1 true), !range !41
  %13 = sub nuw nsw i64 64, %12
  %14 = sext i32 %3 to i64
  %15 = sub nsw i64 %14, %12
  %16 = add nsw i64 %15, 63
  %17 = shl nsw i64 -1, %16
  br label %18

18:                                               ; preds = %9, %11
  %19 = phi i64 [ %10, %9 ], [ %14, %11 ]
  %20 = phi i64 [ 0, %9 ], [ %17, %11 ]
  %21 = phi i64 [ 0, %9 ], [ %13, %11 ]
  %22 = trunc i64 %21 to i32
  %23 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 4, i32 %22
  %24 = load ptr, ptr %23, align 4, !tbaa !6
  %25 = tail call i64 @llvm.usub.sat.i64(i64 %21, i64 1)
  %26 = add nsw i64 %25, %19
  %27 = shl i64 %2, %26
  %28 = trunc i64 %27 to i32
  %29 = getelementptr inbounds i8, ptr %24, i32 %28
  store ptr %29, ptr %4, align 4, !tbaa !6
  %30 = load ptr, ptr %23, align 4, !tbaa !6
  %31 = add i64 %20, %1
  %32 = mul nsw i64 %31, %2
  %33 = trunc i64 %32 to i32
  %34 = getelementptr inbounds i8, ptr %30, i32 %33
  ret ptr %34
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(read, argmem: readwrite, inaccessiblemem: none)
define hidden i64 @fa_segmented_next_byte(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2, i32 noundef %3, i32 noundef %4, ptr nocapture noundef %5, ptr nocapture noundef %6) local_unnamed_addr #14 {
  %8 = icmp slt i64 %1, %2
  br i1 %8, label %9, label %40

9:                                                ; preds = %7
  %10 = trunc i32 %3 to i8
  %11 = load ptr, ptr %5, align 4, !tbaa !6
  br label %12

12:                                               ; preds = %9, %36
  %13 = phi ptr [ %11, %9 ], [ %37, %36 ]
  %14 = phi i64 [ %1, %9 ], [ %38, %36 ]
  %15 = load ptr, ptr %6, align 4, !tbaa !6
  %16 = icmp eq ptr %13, %15
  br i1 %16, label %17, label %20

17:                                               ; preds = %12
  %18 = tail call ptr @fa_segmented_run(ptr noundef %0, i64 noundef %14, i64 noundef 1, i32 noundef %4, ptr noundef nonnull %6) #53
  store ptr %18, ptr %5, align 4, !tbaa !6
  %19 = load ptr, ptr %6, align 4, !tbaa !6
  br label %20

20:                                               ; preds = %17, %12
  %21 = phi ptr [ %18, %17 ], [ %13, %12 ]
  %22 = phi ptr [ %19, %17 ], [ %15, %12 ]
  %23 = ptrtoint ptr %22 to i32
  %24 = ptrtoint ptr %21 to i32
  %25 = sub i32 %23, %24
  %26 = sext i32 %25 to i64
  %27 = sub nsw i64 %2, %14
  %28 = tail call i64 @llvm.smin.i64(i64 %27, i64 %26)
  %29 = trunc i64 %28 to i32
  %30 = tail call fastcc i32 @next_byte(ptr noundef %21, i32 noundef %29, i32 noundef 0, i8 noundef zeroext %10) #53
  %31 = zext i32 %30 to i64
  %32 = icmp sgt i64 %28, %31
  br i1 %32, label %33, label %36

33:                                               ; preds = %20
  %34 = getelementptr inbounds i8, ptr %21, i32 %30
  store ptr %34, ptr %5, align 4, !tbaa !6
  %35 = add nsw i64 %14, %31
  br label %40

36:                                               ; preds = %20
  %37 = getelementptr inbounds i8, ptr %21, i32 %29
  store ptr %37, ptr %5, align 4, !tbaa !6
  %38 = add nsw i64 %28, %14
  %39 = icmp slt i64 %38, %2
  br i1 %39, label %12, label %40

40:                                               ; preds = %36, %7, %33
  %41 = phi i64 [ %35, %33 ], [ %2, %7 ], [ %2, %36 ]
  ret i64 %41
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: readwrite)
define hidden void @fa_segmented_reset(ptr nocapture noundef %0, i64 noundef %1, i32 noundef %2) local_unnamed_addr #1 {
  store i64 0, ptr %0, align 8, !tbaa !115
  %4 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 1
  %5 = load i64, ptr %4, align 8, !tbaa !117
  %6 = icmp sgt i64 %5, 0
  br i1 %6, label %7, label %14

7:                                                ; preds = %3
  %8 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 4
  %9 = load ptr, ptr %8, align 8, !tbaa !6
  %10 = zext nneg i32 %2 to i64
  %11 = shl i64 %1, %10
  %12 = trunc i64 %11 to i32
  %13 = getelementptr inbounds i8, ptr %9, i32 %12
  br label %14

14:                                               ; preds = %3, %7
  %15 = phi ptr [ %9, %7 ], [ null, %3 ]
  %16 = phi ptr [ %13, %7 ], [ null, %3 ]
  %17 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 2
  store ptr %15, ptr %17, align 8
  %18 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 3
  store ptr %16, ptr %18, align 4
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_segmented_free(ptr noundef %0, i64 noundef %1, i32 noundef %2) local_unnamed_addr #3 {
  %4 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 5
  %5 = load i64, ptr %4, align 8, !tbaa !113
  %6 = icmp eq i64 %5, 0
  br i1 %6, label %7, label %26

7:                                                ; preds = %3
  %8 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 1
  %9 = load i64, ptr %8, align 8, !tbaa !117
  %10 = icmp sgt i64 %9, 0
  br i1 %10, label %11, label %13

11:                                               ; preds = %7
  %12 = sext i32 %2 to i64
  br label %14

13:                                               ; preds = %14, %7
  tail call void @fa_free(ptr noundef nonnull %0, i32 noundef 224, i32 noundef 8) #54
  br label %26

14:                                               ; preds = %11, %14
  %15 = phi i64 [ 0, %11 ], [ %23, %14 ]
  %16 = trunc i64 %15 to i32
  %17 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 4, i32 %16
  %18 = load ptr, ptr %17, align 4, !tbaa !6
  %19 = tail call i64 @llvm.usub.sat.i64(i64 %15, i64 1)
  %20 = add i64 %19, %12
  %21 = shl i64 %1, %20
  %22 = trunc i64 %21 to i32
  tail call void @fa_free(ptr noundef %18, i32 noundef %22, i32 noundef 16) #54
  %23 = add nuw nsw i64 %15, 1
  %24 = load i64, ptr %8, align 8, !tbaa !117
  %25 = icmp slt i64 %23, %24
  br i1 %25, label %14, label %13, !llvm.loop !123

26:                                               ; preds = %3, %13
  ret void
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_segmented_flatten(ptr nocapture noundef readonly %0, i64 noundef %1, i32 noundef %2) local_unnamed_addr #3 {
  %4 = load i64, ptr %0, align 8, !tbaa !115
  %5 = tail call ptr @fa_array_new(i64 noundef %4, i64 noundef %4, i64 noundef %1) #53
  %6 = load i64, ptr %0, align 8, !tbaa !115
  %7 = icmp sgt i64 %6, 0
  br i1 %7, label %8, label %10

8:                                                ; preds = %3
  %9 = sext i32 %2 to i64
  br label %11

10:                                               ; preds = %11, %3
  ret ptr %5

11:                                               ; preds = %8, %11
  %12 = phi i64 [ %6, %8 ], [ %31, %11 ]
  %13 = phi i64 [ 0, %8 ], [ %30, %11 ]
  %14 = phi i64 [ 0, %8 ], [ %29, %11 ]
  %15 = tail call i64 @llvm.usub.sat.i64(i64 %13, i64 1)
  %16 = add i64 %15, %9
  %17 = shl nuw i64 1, %16
  %18 = sub nsw i64 %12, %14
  %19 = tail call i64 @llvm.smin.i64(i64 %17, i64 %18)
  %20 = load ptr, ptr %5, align 8, !tbaa !69
  %21 = mul nsw i64 %14, %1
  %22 = trunc i64 %21 to i32
  %23 = getelementptr inbounds i8, ptr %20, i32 %22
  %24 = trunc i64 %13 to i32
  %25 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 4, i32 %24
  %26 = load ptr, ptr %25, align 4, !tbaa !6
  %27 = mul nsw i64 %19, %1
  %28 = trunc i64 %27 to i32
  tail call fastcc void @copy_bytes(ptr noundef %23, ptr noundef %26, i32 noundef %28) #53
  %29 = add nsw i64 %19, %14
  %30 = add nuw nsw i64 %13, 1
  %31 = load i64, ptr %0, align 8, !tbaa !115
  %32 = icmp slt i64 %29, %31
  br i1 %32, label %11, label %10, !llvm.loop !124
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_segmented_from_array(ptr noundef %0, i64 noundef %1, i32 noundef %2) local_unnamed_addr #3 {
  %4 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %5 = load i64, ptr %4, align 8, !tbaa !73
  %6 = icmp slt i64 %5, 0
  br i1 %6, label %7, label %10, !prof !35

7:                                                ; preds = %3
  %8 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %9 = load i64, ptr %8, align 8, !tbaa !79
  tail call fastcc void @materialize_characters(ptr noundef nonnull %0, i64 noundef %9) #53
  br label %10

10:                                               ; preds = %3, %7
  %11 = tail call noundef ptr @fa_alloc(i32 noundef 224, i32 noundef 8) #54
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(224) %11, i8 0, i32 224, i1 false)
  %12 = load ptr, ptr %0, align 8, !tbaa !69
  %13 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %14 = load i64, ptr %13, align 8, !tbaa !79
  %15 = icmp sgt i64 %14, 0
  br i1 %15, label %16, label %23

16:                                               ; preds = %10
  %17 = mul nsw i64 %14, %1
  %18 = icmp sgt i64 %17, 0
  br i1 %18, label %23, label %19

19:                                               ; preds = %16
  %20 = getelementptr inbounds %struct.FaSegmented, ptr %11, i32 0, i32 2
  %21 = trunc i64 %17 to i32
  tail call fastcc void @copy_bytes(ptr noundef null, ptr noundef %12, i32 noundef %21) #53
  %22 = getelementptr inbounds i8, ptr null, i32 %21
  store ptr %22, ptr %20, align 8, !tbaa !118
  store i64 %14, ptr %11, align 8, !tbaa !115
  br label %24

23:                                               ; preds = %16, %10
  tail call fastcc void @segmented_append_runs(ptr noundef nonnull %11, ptr noundef %12, i64 noundef %14, i64 noundef %1, i32 noundef %2) #53
  br label %24

24:                                               ; preds = %19, %23
  tail call void @fa_array_free(ptr noundef nonnull %0, i64 noundef %1) #53
  ret ptr %11
}

; Function Attrs: alwaysinline nounwind optsize
define hidden ptr @fa_heap_alloc(i32 noundef %0, i32 noundef %1) local_unnamed_addr #13 {
  %3 = icmp ugt i32 %1, 8
  %4 = icmp eq i32 %0, 0
  %5 = add i32 %0, 15
  %6 = and i32 %5, -16
  %7 = select i1 %4, i32 16, i32 %6
  %8 = select i1 %3, i32 %7, i32 %0
  %9 = icmp ult i32 %8, 129
  br i1 %9, label %10, label %36

10:                                               ; preds = %2
  %11 = icmp ult i32 %8, 9
  %12 = add nuw nsw i32 %8, 7
  %13 = lshr i32 %12, 3
  %14 = add nsw i32 %13, -1
  %15 = select i1 %11, i32 0, i32 %14
  %16 = getelementptr inbounds [40 x ptr], ptr @free_blocks, i32 0, i32 %15
  %17 = load ptr, ptr %16, align 4, !tbaa !6
  %18 = icmp eq ptr %17, null
  br i1 %18, label %21, label %19, !prof !35

19:                                               ; preds = %10
  %20 = load ptr, ptr %17, align 4, !tbaa !125
  store ptr %20, ptr %16, align 4, !tbaa !6
  br label %38

21:                                               ; preds = %10
  %22 = getelementptr inbounds [40 x ptr], ptr @class_next, i32 0, i32 %15
  %23 = load ptr, ptr %22, align 4, !tbaa !6
  %24 = shl nsw i32 %15, 3
  %25 = add nsw i32 %24, 8
  %26 = icmp eq ptr %23, null
  br i1 %26, label %36, label %27

27:                                               ; preds = %21
  %28 = getelementptr inbounds [40 x ptr], ptr @class_end, i32 0, i32 %15
  %29 = load ptr, ptr %28, align 4, !tbaa !6
  %30 = ptrtoint ptr %29 to i32
  %31 = ptrtoint ptr %23 to i32
  %32 = sub i32 %30, %31
  %33 = icmp ult i32 %32, %25
  br i1 %33, label %36, label %34

34:                                               ; preds = %27
  %35 = getelementptr inbounds i8, ptr %23, i32 %25
  store ptr %35, ptr %22, align 4, !tbaa !6
  br label %38

36:                                               ; preds = %21, %27, %2
  %37 = tail call fastcc ptr @heap_alloc_slow(i32 noundef %8) #53
  br label %38

38:                                               ; preds = %19, %34, %36
  %39 = phi ptr [ %37, %36 ], [ %17, %19 ], [ %23, %34 ]
  ret ptr %39
}

; Function Attrs: noinline nounwind optsize
define internal fastcc ptr @heap_alloc_slow(i32 noundef %0) unnamed_addr #15 {
  %2 = icmp ugt i32 %0, 8192
  br i1 %2, label %3, label %13

3:                                                ; preds = %1
  %4 = add i32 %0, 65551
  %5 = lshr i32 %4, 16
  %6 = icmp ult i32 %4, 65536
  br i1 %6, label %69, label %7

7:                                                ; preds = %3
  %8 = tail call fastcc ptr @take_pages(i32 noundef %5) #53
  %9 = icmp eq ptr %8, null
  br i1 %9, label %69, label %10

10:                                               ; preds = %7
  store i32 65535, ptr %8, align 4, !tbaa !127
  %11 = getelementptr inbounds %struct.PageHeader, ptr %8, i32 0, i32 1
  store i32 %5, ptr %11, align 4, !tbaa !129
  %12 = getelementptr inbounds i8, ptr %8, i32 16
  br label %69

13:                                               ; preds = %1
  %14 = icmp ult i32 %0, 129
  br i1 %14, label %15, label %22

15:                                               ; preds = %13
  %16 = add nuw nsw i32 %0, 7
  %17 = and i32 %16, 504
  %18 = icmp ugt i32 %0, 8
  %19 = select i1 %18, i32 %17, i32 8
  %20 = lshr exact i32 %19, 3
  %21 = add nsw i32 %20, -1
  br label %39

22:                                               ; preds = %13, %37
  %23 = phi i32 [ %26, %37 ], [ 16, %13 ]
  %24 = phi i32 [ %38, %37 ], [ 128, %13 ]
  %25 = lshr exact i32 %24, 2
  %26 = add i32 %23, 4
  br label %27

27:                                               ; preds = %33, %22
  %28 = phi i32 [ 1, %22 ], [ %35, %33 ]
  %29 = phi i32 [ %23, %22 ], [ %34, %33 ]
  %30 = mul nuw i32 %28, %25
  %31 = add i32 %30, %24
  %32 = icmp ult i32 %31, %0
  br i1 %32, label %33, label %39

33:                                               ; preds = %27
  %34 = add i32 %29, 1
  %35 = add nuw nsw i32 %28, 1
  %36 = icmp eq i32 %35, 5
  br i1 %36, label %37, label %27, !llvm.loop !130

37:                                               ; preds = %33
  %38 = shl i32 %24, 1
  br label %22

39:                                               ; preds = %27, %15
  %40 = phi i32 [ %19, %15 ], [ %31, %27 ]
  %41 = phi i32 [ %21, %15 ], [ %29, %27 ]
  %42 = getelementptr inbounds [40 x ptr], ptr @free_blocks, i32 0, i32 %41
  %43 = load ptr, ptr %42, align 4, !tbaa !6
  %44 = icmp eq ptr %43, null
  br i1 %44, label %47, label %45

45:                                               ; preds = %39
  %46 = load ptr, ptr %43, align 4, !tbaa !125
  store ptr %46, ptr %42, align 4, !tbaa !6
  br label %69

47:                                               ; preds = %39
  %48 = getelementptr inbounds [40 x ptr], ptr @class_next, i32 0, i32 %41
  %49 = load ptr, ptr %48, align 4, !tbaa !6
  %50 = icmp eq ptr %49, null
  br i1 %50, label %58, label %51

51:                                               ; preds = %47
  %52 = getelementptr inbounds [40 x ptr], ptr @class_end, i32 0, i32 %41
  %53 = load ptr, ptr %52, align 4, !tbaa !6
  %54 = ptrtoint ptr %53 to i32
  %55 = ptrtoint ptr %49 to i32
  %56 = sub i32 %54, %55
  %57 = icmp ult i32 %56, %40
  br i1 %57, label %58, label %66

58:                                               ; preds = %51, %47
  %59 = tail call fastcc ptr @take_pages(i32 noundef 1) #53
  %60 = icmp eq ptr %59, null
  br i1 %60, label %69, label %61

61:                                               ; preds = %58
  store i32 %41, ptr %59, align 4, !tbaa !127
  %62 = getelementptr inbounds %struct.PageHeader, ptr %59, i32 0, i32 1
  store i32 1, ptr %62, align 4, !tbaa !129
  %63 = getelementptr inbounds i8, ptr %59, i32 16
  %64 = getelementptr inbounds i8, ptr %59, i32 65536
  %65 = getelementptr inbounds [40 x ptr], ptr @class_end, i32 0, i32 %41
  store ptr %64, ptr %65, align 4, !tbaa !6
  br label %66

66:                                               ; preds = %61, %51
  %67 = phi ptr [ %63, %61 ], [ %49, %51 ]
  %68 = getelementptr inbounds i8, ptr %67, i32 %40
  store ptr %68, ptr %48, align 4, !tbaa !6
  br label %69

69:                                               ; preds = %45, %66, %58, %3, %7, %10
  %70 = phi ptr [ null, %3 ], [ %12, %10 ], [ null, %7 ], [ %43, %45 ], [ %67, %66 ], [ null, %58 ]
  ret ptr %70
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden void @fa_heap_free(ptr noundef %0) local_unnamed_addr #19 {
  %2 = ptrtoint ptr %0 to i32
  %3 = and i32 %2, -65536
  %4 = inttoptr i32 %3 to ptr
  %5 = load i32, ptr %4, align 65536, !tbaa !127
  %6 = icmp eq i32 %5, 65535
  br i1 %6, label %7, label %10

7:                                                ; preds = %1
  %8 = getelementptr inbounds %struct.PageHeader, ptr %4, i32 0, i32 1
  %9 = load i32, ptr %8, align 4, !tbaa !129
  tail call fastcc void @give_pages(ptr noundef nonnull %4, i32 noundef %9) #53
  br label %13

10:                                               ; preds = %1
  %11 = getelementptr inbounds [40 x ptr], ptr @free_blocks, i32 0, i32 %5
  %12 = load ptr, ptr %11, align 4, !tbaa !6
  store ptr %12, ptr %0, align 4, !tbaa !125
  store ptr %0, ptr %11, align 4, !tbaa !6
  br label %13

13:                                               ; preds = %10, %7
  ret void
}

; Function Attrs: nofree noinline norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc void @give_pages(ptr noundef %0, i32 noundef %1) unnamed_addr #20 {
  %3 = getelementptr inbounds %struct.FreeSpan, ptr %0, i32 0, i32 1
  store i32 %1, ptr %3, align 4, !tbaa !131
  br label %4

4:                                                ; preds = %4, %2
  %5 = phi ptr [ null, %2 ], [ %7, %4 ]
  %6 = phi ptr [ @free_spans, %2 ], [ %7, %4 ]
  %7 = load ptr, ptr %6, align 4, !tbaa !6
  %8 = icmp ne ptr %7, null
  %9 = icmp ult ptr %7, %0
  %10 = and i1 %8, %9
  br i1 %10, label %4, label %11, !llvm.loop !133

11:                                               ; preds = %4
  store ptr %7, ptr %0, align 4, !tbaa !134
  br i1 %8, label %12, label %21

12:                                               ; preds = %11
  %13 = shl i32 %1, 16
  %14 = getelementptr inbounds i8, ptr %0, i32 %13
  %15 = icmp eq ptr %14, %7
  br i1 %15, label %16, label %21

16:                                               ; preds = %12
  %17 = getelementptr inbounds %struct.FreeSpan, ptr %7, i32 0, i32 1
  %18 = load i32, ptr %17, align 4, !tbaa !131
  %19 = add i32 %18, %1
  store i32 %19, ptr %3, align 4, !tbaa !131
  %20 = load ptr, ptr %7, align 4, !tbaa !134
  store ptr %20, ptr %0, align 4, !tbaa !134
  br label %21

21:                                               ; preds = %16, %12, %11
  %22 = phi ptr [ %20, %16 ], [ %7, %12 ], [ null, %11 ]
  %23 = phi i32 [ %19, %16 ], [ %1, %12 ], [ %1, %11 ]
  %24 = icmp eq ptr %5, null
  br i1 %24, label %34, label %25

25:                                               ; preds = %21
  %26 = getelementptr inbounds %struct.FreeSpan, ptr %5, i32 0, i32 1
  %27 = load i32, ptr %26, align 4, !tbaa !131
  %28 = shl i32 %27, 16
  %29 = getelementptr inbounds i8, ptr %5, i32 %28
  %30 = icmp eq ptr %29, %0
  br i1 %30, label %31, label %33

31:                                               ; preds = %25
  %32 = add i32 %27, %23
  store i32 %32, ptr %26, align 4, !tbaa !131
  store ptr %22, ptr %5, align 4, !tbaa !134
  br label %35

33:                                               ; preds = %25
  store ptr %0, ptr %5, align 4, !tbaa !134
  br label %35

34:                                               ; preds = %21
  store ptr %0, ptr @free_spans, align 4, !tbaa !6
  br label %35

35:                                               ; preds = %31, %33, %34
  ret void
}

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(readwrite, argmem: none, inaccessiblemem: none)
define hidden void @fa_arena_enter() local_unnamed_addr #30 {
  %1 = load i32, ptr @tree_arena_depth, align 4, !tbaa !4
  %2 = add i32 %1, 1
  store i32 %2, ptr @tree_arena_depth, align 4, !tbaa !4
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(readwrite, argmem: read, inaccessiblemem: none)
define hidden void @fa_arena_enter_owned(ptr noundef %0) local_unnamed_addr #31 {
  %2 = load i32, ptr @tree_arena_depth, align 4, !tbaa !4
  %3 = icmp eq i32 %2, 0
  br i1 %3, label %4, label %13

4:                                                ; preds = %1
  %5 = icmp eq ptr %0, null
  br i1 %5, label %11, label %6

6:                                                ; preds = %4
  %7 = ptrtoint ptr %0 to i32
  %8 = and i32 %7, -65536
  %9 = inttoptr i32 %8 to ptr
  %10 = load ptr, ptr %9, align 65536, !tbaa !135
  br label %11

11:                                               ; preds = %4, %6
  %12 = phi ptr [ %10, %6 ], [ null, %4 ]
  store ptr %12, ptr @tree_arena, align 4, !tbaa !6
  br label %13

13:                                               ; preds = %11, %1
  %14 = add i32 %2, 1
  store i32 %14, ptr @tree_arena_depth, align 4, !tbaa !4
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_arena_drop(ptr noundef %0) local_unnamed_addr #3 {
  %2 = icmp eq ptr %0, null
  br i1 %2, label %13, label %3

3:                                                ; preds = %1
  %4 = ptrtoint ptr %0 to i32
  %5 = and i32 %4, -65536
  %6 = inttoptr i32 %5 to ptr
  %7 = load ptr, ptr %6, align 65536, !tbaa !135
  %8 = icmp eq ptr %7, null
  %9 = load ptr, ptr @tree_arena, align 4
  %10 = icmp eq ptr %7, %9
  %11 = select i1 %8, i1 true, i1 %10
  br i1 %11, label %13, label %12

12:                                               ; preds = %3
  tail call fastcc void @release_arena(ptr noundef nonnull %7) #53
  br label %13

13:                                               ; preds = %1, %12, %3
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @release_arena(ptr noundef %0) unnamed_addr #3 {
  %2 = atomicrmw sub ptr @live_tree_arenas, i32 1 monotonic, align 4
  %3 = getelementptr inbounds %struct.FaArena, ptr %0, i32 0, i32 4
  %4 = load ptr, ptr %3, align 4, !tbaa !6
  %5 = icmp eq ptr %4, null
  br i1 %5, label %8, label %9

6:                                                ; preds = %9
  %7 = icmp eq ptr %0, null
  br i1 %7, label %22, label %8

8:                                                ; preds = %1, %6
  br label %17

9:                                                ; preds = %1, %9
  %10 = phi ptr [ %15, %9 ], [ %4, %1 ]
  %11 = getelementptr inbounds %struct.FaArenaCleanup, ptr %10, i32 0, i32 1
  %12 = load ptr, ptr %11, align 4, !tbaa !137
  %13 = getelementptr inbounds %struct.FaArenaCleanup, ptr %10, i32 0, i32 2
  %14 = load ptr, ptr %13, align 4, !tbaa !139
  tail call void %12(ptr noundef %14) #54
  %15 = load ptr, ptr %10, align 4, !tbaa !6
  %16 = icmp eq ptr %15, null
  br i1 %16, label %6, label %9, !llvm.loop !140

17:                                               ; preds = %8, %17
  %18 = phi ptr [ %20, %17 ], [ %0, %8 ]
  %19 = getelementptr inbounds %struct.FaArena, ptr %18, i32 0, i32 1
  %20 = load ptr, ptr %19, align 4, !tbaa !141
  tail call fastcc void @give_pages(ptr noundef nonnull %18, i32 noundef 1) #53
  %21 = icmp eq ptr %20, null
  br i1 %21, label %22, label %17, !llvm.loop !142

22:                                               ; preds = %17, %6
  ret void
}

; Function Attrs: alwaysinline nounwind optsize
define hidden void @fa_arena_leave(ptr noundef %0) local_unnamed_addr #13 {
  %2 = load i32, ptr @tree_arena_depth, align 4, !tbaa !4
  %3 = add i32 %2, -1
  store i32 %3, ptr @tree_arena_depth, align 4, !tbaa !4
  %4 = icmp eq i32 %3, 0
  br i1 %4, label %5, label %19

5:                                                ; preds = %1
  %6 = load ptr, ptr @tree_arena, align 4, !tbaa !6
  store ptr null, ptr @tree_arena, align 4, !tbaa !6
  %7 = icmp eq ptr %6, null
  br i1 %7, label %19, label %8

8:                                                ; preds = %5
  %9 = icmp eq ptr %0, null
  br i1 %9, label %15, label %10

10:                                               ; preds = %8
  %11 = ptrtoint ptr %0 to i32
  %12 = and i32 %11, -65536
  %13 = inttoptr i32 %12 to ptr
  %14 = load ptr, ptr %13, align 65536, !tbaa !135
  br label %15

15:                                               ; preds = %8, %10
  %16 = phi ptr [ %14, %10 ], [ null, %8 ]
  %17 = icmp eq ptr %16, %6
  br i1 %17, label %19, label %18

18:                                               ; preds = %15
  tail call fastcc void @release_arena(ptr noundef nonnull %6) #53
  br label %19

19:                                               ; preds = %5, %15, %18, %1
  ret void
}

; Function Attrs: alwaysinline nounwind optsize
define hidden ptr @fa_arena_alloc(i32 noundef %0, i32 noundef %1) local_unnamed_addr #13 {
  %3 = add i32 %0, 7
  %4 = and i32 %3, -8
  %5 = load ptr, ptr @tree_arena, align 4, !tbaa !6
  %6 = icmp eq ptr %5, null
  br i1 %6, label %28, label %7, !prof !35

7:                                                ; preds = %2
  %8 = icmp ult i32 %1, 9
  %9 = getelementptr inbounds %struct.FaArena, ptr %5, i32 0, i32 2
  %10 = load ptr, ptr %9, align 4, !tbaa !143
  %11 = ptrtoint ptr %10 to i32
  %12 = sub i32 0, %11
  %13 = add i32 %1, -1
  %14 = and i32 %13, %12
  %15 = select i1 %8, i32 0, i32 %14
  %16 = getelementptr inbounds %struct.FaArena, ptr %5, i32 0, i32 3
  %17 = load ptr, ptr %16, align 4, !tbaa !144
  %18 = ptrtoint ptr %17 to i32
  %19 = sub i32 %18, %11
  %20 = icmp ule i32 %15, %19
  %21 = sub i32 %19, %15
  %22 = icmp ule i32 %4, %21
  %23 = and i1 %20, %22
  br i1 %23, label %24, label %28, !prof !82

24:                                               ; preds = %7
  %25 = getelementptr inbounds %struct.FaArena, ptr %5, i32 0, i32 2
  %26 = getelementptr inbounds i8, ptr %10, i32 %15
  %27 = getelementptr inbounds i8, ptr %26, i32 %4
  store ptr %27, ptr %25, align 4, !tbaa !143
  br label %30

28:                                               ; preds = %7, %2
  %29 = tail call fastcc ptr @arena_refill(i32 noundef %4, i32 noundef %1) #53
  br label %30

30:                                               ; preds = %24, %28
  %31 = phi ptr [ %29, %28 ], [ %26, %24 ]
  ret ptr %31
}

; Function Attrs: noinline nounwind optsize
define internal fastcc ptr @arena_refill(i32 noundef %0, i32 noundef %1) unnamed_addr #15 {
  %3 = add i32 %1, 31
  %4 = sub i32 0, %1
  %5 = and i32 %3, %4
  %6 = tail call fastcc ptr @take_pages(i32 noundef 1) #53
  %7 = icmp eq ptr %6, null
  %8 = icmp ugt i32 %5, 65536
  %9 = or i1 %8, %7
  %10 = sub nuw nsw i32 65536, %5
  %11 = icmp ult i32 %10, %0
  %12 = select i1 %9, i1 true, i1 %11
  br i1 %12, label %13, label %14

13:                                               ; preds = %2
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @arena_refill.message, i32 noundef 20) #54
  tail call void @fa_exit(i32 noundef 101) #55
  unreachable

14:                                               ; preds = %2
  %15 = load ptr, ptr @tree_arena, align 4, !tbaa !6
  %16 = icmp eq ptr %15, null
  br i1 %16, label %17, label %21

17:                                               ; preds = %14
  %18 = getelementptr inbounds %struct.FaArena, ptr %6, i32 0, i32 1
  store ptr null, ptr %18, align 4, !tbaa !141
  %19 = getelementptr inbounds %struct.FaArena, ptr %6, i32 0, i32 4
  store ptr null, ptr %19, align 4, !tbaa !145
  store ptr %6, ptr @tree_arena, align 4, !tbaa !6
  %20 = atomicrmw add ptr @live_tree_arenas, i32 1 monotonic, align 4
  br label %25

21:                                               ; preds = %14
  %22 = getelementptr inbounds %struct.FaArena, ptr %15, i32 0, i32 1
  %23 = load ptr, ptr %22, align 4, !tbaa !141
  %24 = getelementptr inbounds %struct.FaArena, ptr %6, i32 0, i32 1
  store ptr %23, ptr %24, align 4, !tbaa !141
  store ptr %6, ptr %22, align 4, !tbaa !141
  br label %25

25:                                               ; preds = %21, %17
  %26 = phi ptr [ %6, %17 ], [ %15, %21 ]
  store ptr %26, ptr %6, align 4, !tbaa !135
  %27 = getelementptr inbounds i8, ptr %6, i32 %5
  %28 = getelementptr inbounds i8, ptr %27, i32 %0
  %29 = getelementptr inbounds %struct.FaArena, ptr %26, i32 0, i32 2
  store ptr %28, ptr %29, align 4, !tbaa !143
  %30 = getelementptr inbounds i8, ptr %6, i32 65536
  %31 = getelementptr inbounds %struct.FaArena, ptr %26, i32 0, i32 3
  store ptr %30, ptr %31, align 4, !tbaa !144
  ret ptr %27
}

; Function Attrs: cold noinline nounwind optsize
define hidden void @fa_arena_cleanup(ptr noundef %0, ptr noundef %1) local_unnamed_addr #21 {
  %3 = load ptr, ptr @tree_arena, align 4, !tbaa !6
  %4 = icmp eq ptr %3, null
  br i1 %4, label %16, label %5, !prof !35

5:                                                ; preds = %2
  %6 = getelementptr inbounds %struct.FaArena, ptr %3, i32 0, i32 3
  %7 = load ptr, ptr %6, align 4, !tbaa !144
  %8 = getelementptr inbounds %struct.FaArena, ptr %3, i32 0, i32 2
  %9 = load ptr, ptr %8, align 4, !tbaa !143
  %10 = ptrtoint ptr %7 to i32
  %11 = ptrtoint ptr %9 to i32
  %12 = sub i32 %10, %11
  %13 = icmp ugt i32 %12, 15
  br i1 %13, label %14, label %16, !prof !82

14:                                               ; preds = %5
  %15 = getelementptr inbounds i8, ptr %9, i32 16
  store ptr %15, ptr %8, align 4, !tbaa !143
  br label %19

16:                                               ; preds = %5, %2
  %17 = tail call fastcc ptr @arena_refill(i32 noundef 16, i32 noundef 4) #53
  %18 = load ptr, ptr @tree_arena, align 4, !tbaa !6
  br label %19

19:                                               ; preds = %14, %16
  %20 = phi ptr [ %18, %16 ], [ %3, %14 ]
  %21 = phi ptr [ %17, %16 ], [ %9, %14 ]
  %22 = getelementptr inbounds %struct.FaArena, ptr %20, i32 0, i32 4
  %23 = load ptr, ptr %22, align 4, !tbaa !145
  store ptr %23, ptr %21, align 4, !tbaa !146
  %24 = getelementptr inbounds %struct.FaArenaCleanup, ptr %21, i32 0, i32 1
  store ptr %1, ptr %24, align 4, !tbaa !137
  %25 = getelementptr inbounds %struct.FaArenaCleanup, ptr %21, i32 0, i32 2
  store ptr %0, ptr %25, align 4, !tbaa !139
  store ptr %21, ptr %22, align 4, !tbaa !145
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_release_builder() local_unnamed_addr #3 {
  %1 = load ptr, ptr @builder, align 4, !tbaa !6
  %2 = icmp eq ptr %1, null
  br i1 %2, label %5, label %3

3:                                                ; preds = %0
  %4 = load i32, ptr @builder_capacity, align 4, !tbaa !4
  tail call void @fa_free(ptr noundef nonnull %1, i32 noundef %4, i32 noundef 1) #54
  store ptr null, ptr @builder, align 4, !tbaa !6
  store i32 0, ptr @builder_capacity, align 4, !tbaa !4
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  br label %5

5:                                                ; preds = %3, %0
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_leak_check() local_unnamed_addr #3 {
  %1 = load i32, ptr @output_length, align 4, !tbaa !4
  %2 = icmp eq i32 %1, 0
  br i1 %2, label %4, label %3

3:                                                ; preds = %0
  tail call void @fa_write(i32 noundef 1, ptr noundef nonnull @output, i32 noundef %1) #54
  store i32 0, ptr @output_length, align 4, !tbaa !4
  br label %4

4:                                                ; preds = %0, %3
  %5 = load ptr, ptr @builder, align 4, !tbaa !6
  %6 = icmp eq ptr %5, null
  br i1 %6, label %9, label %7

7:                                                ; preds = %4
  %8 = load i32, ptr @builder_capacity, align 4, !tbaa !4
  tail call void @fa_free(ptr noundef nonnull %5, i32 noundef %8, i32 noundef 1) #54
  store ptr null, ptr @builder, align 4, !tbaa !6
  store i32 0, ptr @builder_capacity, align 4, !tbaa !4
  br label %9

9:                                                ; preds = %7, %4
  %10 = load ptr, ptr @argument_items, align 4, !tbaa !6
  %11 = icmp eq ptr %10, null
  br i1 %11, label %16, label %12

12:                                               ; preds = %9
  %13 = load i32, ptr @argument_count, align 4, !tbaa !4
  %14 = mul i32 %13, 12
  %15 = add i32 %14, 12
  tail call void @fa_free(ptr noundef nonnull %10, i32 noundef %15, i32 noundef 4) #54
  store ptr null, ptr @argument_items, align 4, !tbaa !6
  br label %16

16:                                               ; preds = %12, %9
  %17 = load ptr, ptr @argument_bytes, align 4, !tbaa !6
  %18 = icmp eq ptr %17, null
  br i1 %18, label %20, label %19

19:                                               ; preds = %16
  tail call void @fa_free(ptr noundef nonnull %17, i32 noundef 1, i32 noundef 1) #54
  store ptr null, ptr @argument_bytes, align 4, !tbaa !6
  br label %20

20:                                               ; preds = %19, %16
  %21 = load i32, ptr @fa_live_blocks, align 4, !tbaa !4
  %22 = icmp ne i32 %21, 0
  %23 = load i32, ptr @live_tree_arenas, align 4
  %24 = icmp ne i32 %23, 0
  %25 = select i1 %22, i1 true, i1 %24
  br i1 %25, label %26, label %31

26:                                               ; preds = %20
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  tail call fastcc void @append_c(ptr noundef nonnull @.str.13) #53
  %27 = load i32, ptr @fa_live_blocks, align 4, !tbaa !4
  %28 = load i32, ptr @live_tree_arenas, align 4, !tbaa !4
  %29 = add i32 %28, %27
  %30 = zext i32 %29 to i64
  tail call void @fa_b_u64(i64 noundef %30) #53
  tail call fastcc void @append_c(ptr noundef nonnull @.str.14) #53
  tail call void @fa_b_line(i32 noundef 2) #53
  tail call void @fa_exit(i32 noundef 102) #55
  unreachable

31:                                               ; preds = %20
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden noundef ptr @fa_pool_local(ptr noundef returned %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #19 {
  %4 = getelementptr inbounds i8, ptr %0, i32 16
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(56) %4, i8 0, i32 32, i1 false)
  %5 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 10
  store i64 3, ptr %5, align 8, !tbaa !147
  store i64 %1, ptr %0, align 8, !tbaa !149
  %6 = tail call i64 @llvm.smax.i64(i64 %2, i64 4)
  %7 = trunc i64 %6 to i32
  %8 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 6
  store i32 %7, ptr %8, align 4, !tbaa !150
  %9 = trunc i64 %1 to i32
  %10 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 7
  store i32 %9, ptr %10, align 8, !tbaa !151
  %11 = icmp ne i64 %1, 0
  %12 = zext i1 %11 to i32
  %13 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 4
  store i32 %12, ptr %13, align 4, !tbaa !152
  %14 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 5
  store i32 1, ptr %14, align 8, !tbaa !153
  %15 = getelementptr inbounds i8, ptr %0, i32 56
  %16 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 1
  store ptr %15, ptr %16, align 8, !tbaa !154
  %17 = getelementptr inbounds i8, ptr %0, i32 60
  %18 = ptrtoint ptr %17 to i32
  %19 = add i32 %18, 7
  %20 = and i32 %19, -8
  %21 = inttoptr i32 %20 to ptr
  %22 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 2
  store ptr %21, ptr %22, align 4, !tbaa !155
  store i64 0, ptr %21, align 8, !tbaa !89
  %23 = getelementptr inbounds i8, ptr %21, i32 8
  store ptr %23, ptr %15, align 4, !tbaa !6
  %24 = add i32 %9, 63
  %25 = lshr i32 %24, 3
  %26 = and i32 %25, 536870904
  %27 = add nuw nsw i32 %26, 24
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(1) %23, i8 0, i32 %27, i1 false)
  %28 = getelementptr inbounds i8, ptr %23, i32 %27
  %29 = ptrtoint ptr %28 to i32
  %30 = add i32 %29, 15
  %31 = and i32 %30, -16
  %32 = inttoptr i32 %31 to ptr
  store ptr %32, ptr %23, align 8, !tbaa !6
  %33 = getelementptr inbounds i8, ptr %21, i32 16
  store i32 %9, ptr %33, align 8, !tbaa !23
  %34 = getelementptr inbounds i8, ptr %21, i32 20
  store i32 %9, ptr %34, align 4, !tbaa !23
  %35 = icmp sgt i64 %1, 0
  br i1 %35, label %36, label %38

36:                                               ; preds = %3
  %37 = getelementptr inbounds i8, ptr %21, i32 32
  br label %39

38:                                               ; preds = %39, %3
  ret ptr %0

39:                                               ; preds = %36, %39
  %40 = phi i64 [ 0, %36 ], [ %48, %39 ]
  %41 = and i64 %40, 63
  %42 = shl nuw i64 1, %41
  %43 = lshr i64 %40, 6
  %44 = trunc i64 %43 to i32
  %45 = getelementptr inbounds [0 x i64], ptr %37, i32 0, i32 %44
  %46 = load i64, ptr %45, align 8, !tbaa !89
  %47 = or i64 %46, %42
  store i64 %47, ptr %45, align 8, !tbaa !89
  %48 = add nuw nsw i64 %40, 1
  %49 = icmp eq i64 %48, %1
  br i1 %49, label %38, label %39, !llvm.loop !156
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_pool_new(i64 noundef %0) local_unnamed_addr #3 {
  %2 = tail call ptr @fa_alloc(i32 noundef 56, i32 noundef 8) #54
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(56) %2, i8 0, i32 56, i1 false)
  %3 = tail call i64 @llvm.smax.i64(i64 %0, i64 4)
  %4 = trunc i64 %3 to i32
  %5 = getelementptr inbounds %struct.FaPool, ptr %2, i32 0, i32 6
  store i32 %4, ptr %5, align 4, !tbaa !150
  %6 = shl i32 %4, 6
  %7 = add i32 %6, 104
  br label %8

8:                                                ; preds = %8, %1
  %9 = phi i32 [ 1, %1 ], [ %12, %8 ]
  %10 = shl i32 %9, 16
  %11 = icmp ult i32 %10, %7
  %12 = shl i32 %9, 1
  br i1 %11, label %8, label %13, !llvm.loop !157

13:                                               ; preds = %8
  %14 = shl i32 %9, 19
  %15 = add i32 %14, -320
  %16 = shl i32 %4, 3
  %17 = or disjoint i32 %16, 1
  %18 = udiv i32 %15, %17
  br label %19

19:                                               ; preds = %19, %13
  %20 = phi i32 [ %18, %13 ], [ %29, %19 ]
  %21 = add i32 %20, 63
  %22 = lshr i32 %21, 3
  %23 = and i32 %22, 536870904
  %24 = add nuw nsw i32 %23, 39
  %25 = and i32 %24, 1073741808
  %26 = mul i32 %20, %4
  %27 = add i32 %25, %26
  %28 = icmp ugt i32 %27, %10
  %29 = add i32 %20, -1
  br i1 %28, label %19, label %30

30:                                               ; preds = %19
  %31 = getelementptr inbounds %struct.FaPool, ptr %2, i32 0, i32 7
  store i32 %20, ptr %31, align 8, !tbaa !151
  %32 = getelementptr inbounds %struct.FaPool, ptr %2, i32 0, i32 8
  store i32 %9, ptr %32, align 4, !tbaa !158
  %33 = getelementptr inbounds %struct.FaPool, ptr %2, i32 0, i32 9
  store i32 %25, ptr %33, align 8, !tbaa !159
  ret ptr %2
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_columnar_new(i64 noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #3 {
  %4 = tail call ptr @fa_alloc(i32 noundef 56, i32 noundef 8) #54
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(56) %4, i8 0, i32 56, i1 false)
  %5 = getelementptr inbounds %struct.FaPool, ptr %4, i32 0, i32 6
  store i32 4, ptr %5, align 4, !tbaa !150
  %6 = trunc i64 %0 to i32
  %7 = getelementptr inbounds %struct.FaPool, ptr %4, i32 0, i32 7
  store i32 %6, ptr %7, align 8, !tbaa !151
  %8 = trunc i64 %1 to i32
  %9 = getelementptr inbounds %struct.FaPool, ptr %4, i32 0, i32 8
  store i32 %8, ptr %9, align 4, !tbaa !158
  %10 = trunc i64 %2 to i32
  %11 = getelementptr inbounds %struct.FaPool, ptr %4, i32 0, i32 9
  store i32 %10, ptr %11, align 8, !tbaa !159
  ret ptr %4
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_pool_add_slot(ptr nocapture noundef %0) local_unnamed_addr #3 {
  %2 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 4
  %3 = load i32, ptr %2, align 4, !tbaa !152
  %4 = add i32 %3, 63
  %5 = lshr i32 %4, 6
  %6 = icmp ult i32 %4, 64
  br i1 %6, label %28, label %7

7:                                                ; preds = %1
  %8 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 2
  %9 = load ptr, ptr %8, align 4, !tbaa !155
  br label %10

10:                                               ; preds = %7, %15
  %11 = phi i32 [ 0, %7 ], [ %16, %15 ]
  %12 = getelementptr inbounds i64, ptr %9, i32 %11
  %13 = load i64, ptr %12, align 8, !tbaa !89
  %14 = icmp eq i64 %13, 0
  br i1 %14, label %15, label %18

15:                                               ; preds = %10
  %16 = add nuw nsw i32 %11, 1
  %17 = icmp eq i32 %16, %5
  br i1 %17, label %28, label %10, !llvm.loop !160

18:                                               ; preds = %10
  %19 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 1
  %20 = load ptr, ptr %19, align 8, !tbaa !154
  %21 = shl nuw i32 %11, 6
  %22 = tail call i64 @llvm.cttz.i64(i64 %13, i1 true), !range !41
  %23 = trunc i64 %22 to i32
  %24 = or disjoint i32 %21, %23
  %25 = getelementptr inbounds ptr, ptr %20, i32 %24
  %26 = load ptr, ptr %25, align 4, !tbaa !6
  %27 = icmp eq ptr %26, null
  br i1 %27, label %28, label %30

28:                                               ; preds = %15, %1, %18
  %29 = tail call fastcc ptr @pool_new_chunk(ptr noundef nonnull %0) #58
  br label %30

30:                                               ; preds = %28, %18
  %31 = phi ptr [ %29, %28 ], [ %26, %18 ]
  %32 = getelementptr inbounds %struct.PoolChunk, ptr %31, i32 0, i32 4
  %33 = load i32, ptr %32, align 8, !tbaa !23
  %34 = icmp eq i32 %33, 0
  br i1 %34, label %44, label %35

35:                                               ; preds = %30
  %36 = add i32 %33, -1
  %37 = load ptr, ptr %31, align 8, !tbaa !6
  %38 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 6
  %39 = load i32, ptr %38, align 4, !tbaa !150
  %40 = mul i32 %39, %36
  %41 = getelementptr inbounds i8, ptr %37, i32 %40
  %42 = load i32, ptr %41, align 1
  store i32 %42, ptr %32, align 8, !tbaa !23
  %43 = icmp eq i32 %42, 0
  br label %48

44:                                               ; preds = %30
  %45 = getelementptr inbounds %struct.PoolChunk, ptr %31, i32 0, i32 3
  %46 = load i32, ptr %45, align 4, !tbaa !23
  %47 = add i32 %46, 1
  store i32 %47, ptr %45, align 4, !tbaa !23
  br label %48

48:                                               ; preds = %44, %35
  %49 = phi i1 [ %43, %35 ], [ true, %44 ]
  %50 = phi i32 [ %36, %35 ], [ %46, %44 ]
  %51 = and i32 %50, 63
  %52 = zext nneg i32 %51 to i64
  %53 = shl nuw i64 1, %52
  %54 = lshr i32 %50, 6
  %55 = getelementptr inbounds %struct.PoolChunk, ptr %31, i32 0, i32 5, i32 %54
  %56 = load i64, ptr %55, align 8, !tbaa !89
  %57 = or i64 %53, %56
  store i64 %57, ptr %55, align 8, !tbaa !89
  %58 = getelementptr inbounds %struct.PoolChunk, ptr %31, i32 0, i32 2
  %59 = load i32, ptr %58, align 8, !tbaa !23
  %60 = add i32 %59, 1
  store i32 %60, ptr %58, align 8, !tbaa !23
  %61 = load i64, ptr %0, align 8, !tbaa !149
  %62 = add nsw i64 %61, 1
  store i64 %62, ptr %0, align 8, !tbaa !149
  br i1 %49, label %63, label %82

63:                                               ; preds = %48
  %64 = getelementptr inbounds %struct.PoolChunk, ptr %31, i32 0, i32 3
  %65 = load i32, ptr %64, align 4, !tbaa !23
  %66 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 7
  %67 = load i32, ptr %66, align 8, !tbaa !151
  %68 = icmp eq i32 %65, %67
  br i1 %68, label %69, label %82

69:                                               ; preds = %63
  %70 = getelementptr inbounds %struct.PoolChunk, ptr %31, i32 0, i32 1
  %71 = load i32, ptr %70, align 4, !tbaa !23
  %72 = and i32 %71, 63
  %73 = zext nneg i32 %72 to i64
  %74 = shl nuw i64 1, %73
  %75 = xor i64 %74, -1
  %76 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 2
  %77 = load ptr, ptr %76, align 4, !tbaa !155
  %78 = lshr i32 %71, 6
  %79 = getelementptr inbounds i64, ptr %77, i32 %78
  %80 = load i64, ptr %79, align 8, !tbaa !89
  %81 = and i64 %80, %75
  store i64 %81, ptr %79, align 8, !tbaa !89
  br label %82

82:                                               ; preds = %69, %63, %48
  %83 = load ptr, ptr %31, align 8, !tbaa !6
  %84 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 6
  %85 = load i32, ptr %84, align 4, !tbaa !150
  %86 = mul i32 %85, %50
  %87 = getelementptr inbounds i8, ptr %83, i32 %86
  ret ptr %87
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.cttz.i64(i64, i1 immarg) #32

; Function Attrs: cold noinline nounwind optsize
define internal fastcc noundef ptr @pool_new_chunk(ptr nocapture noundef %0) unnamed_addr #21 {
  %2 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 3
  %3 = load ptr, ptr %2, align 8, !tbaa !161
  store ptr null, ptr %2, align 8, !tbaa !161
  %4 = icmp eq ptr %3, null
  br i1 %4, label %5, label %40

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 8
  %7 = load i32, ptr %6, align 4, !tbaa !158
  %8 = icmp eq i32 %7, 1
  br i1 %8, label %9, label %11

9:                                                ; preds = %5
  %10 = tail call fastcc ptr @take_pages(i32 noundef 1) #53
  br label %34

11:                                               ; preds = %5
  %12 = shl i32 %7, 1
  %13 = add i32 %12, -1
  %14 = tail call fastcc ptr @take_pages(i32 noundef %13) #53
  %15 = icmp eq ptr %14, null
  br i1 %15, label %37, label %16

16:                                               ; preds = %11
  %17 = shl i32 %7, 16
  %18 = ptrtoint ptr %14 to i32
  %19 = add i32 %17, -1
  %20 = add i32 %19, %18
  %21 = sub i32 0, %17
  %22 = and i32 %20, %21
  %23 = inttoptr i32 %22 to ptr
  %24 = sub i32 %22, %18
  %25 = lshr i32 %24, 16
  %26 = icmp ult i32 %24, 65536
  br i1 %26, label %28, label %27

27:                                               ; preds = %16
  tail call fastcc void @give_pages(ptr noundef nonnull %14, i32 noundef %25) #53
  br label %28

28:                                               ; preds = %27, %16
  %29 = xor i32 %25, -1
  %30 = add i32 %7, %29
  %31 = icmp eq i32 %30, 0
  br i1 %31, label %34, label %32

32:                                               ; preds = %28
  %33 = getelementptr inbounds i8, ptr %23, i32 %17
  tail call fastcc void @give_pages(ptr noundef %33, i32 noundef %30) #53
  br label %34

34:                                               ; preds = %9, %28, %32
  %35 = phi ptr [ %10, %9 ], [ %23, %32 ], [ %23, %28 ]
  %36 = icmp eq ptr %35, null
  br i1 %36, label %37, label %38

37:                                               ; preds = %11, %34
  tail call fastcc void @out_of_memory() #57
  unreachable

38:                                               ; preds = %34
  %39 = atomicrmw add ptr @fa_live_blocks, i32 1 monotonic, align 4
  br label %40

40:                                               ; preds = %38, %1
  %41 = phi ptr [ %35, %38 ], [ %3, %1 ]
  %42 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 7
  %43 = load i32, ptr %42, align 8, !tbaa !151
  %44 = add i32 %43, 63
  %45 = lshr i32 %44, 3
  %46 = and i32 %45, 536870904
  %47 = add nuw nsw i32 %46, 24
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(1) %41, i8 0, i32 %47, i1 false)
  %48 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 9
  %49 = load i32, ptr %48, align 8, !tbaa !159
  %50 = getelementptr inbounds i8, ptr %41, i32 %49
  store ptr %50, ptr %41, align 8, !tbaa !6
  %51 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 4
  %52 = load i32, ptr %51, align 4, !tbaa !152
  %53 = icmp eq i32 %52, 0
  br i1 %53, label %65, label %54

54:                                               ; preds = %40
  %55 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 1
  %56 = load ptr, ptr %55, align 8, !tbaa !154
  br label %57

57:                                               ; preds = %54, %62
  %58 = phi i32 [ 0, %54 ], [ %63, %62 ]
  %59 = getelementptr inbounds ptr, ptr %56, i32 %58
  %60 = load ptr, ptr %59, align 4, !tbaa !6
  %61 = icmp eq ptr %60, null
  br i1 %61, label %65, label %62

62:                                               ; preds = %57
  %63 = add nuw i32 %58, 1
  %64 = icmp eq i32 %63, %52
  br i1 %64, label %68, label %57, !llvm.loop !162

65:                                               ; preds = %57, %40
  %66 = phi i32 [ 0, %40 ], [ %58, %57 ]
  %67 = icmp eq i32 %66, %52
  br i1 %67, label %68, label %110

68:                                               ; preds = %62, %65
  %69 = phi i32 [ %66, %65 ], [ %52, %62 ]
  %70 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 5
  %71 = load i32, ptr %70, align 8, !tbaa !153
  %72 = icmp eq i32 %52, %71
  br i1 %72, label %73, label %107

73:                                               ; preds = %68
  %74 = shl i32 %52, 1
  %75 = select i1 %53, i32 4, i32 %74
  %76 = shl i32 %75, 2
  %77 = tail call ptr @fa_alloc(i32 noundef %76, i32 noundef 4) #54
  %78 = add i32 %75, 63
  %79 = lshr i32 %78, 3
  %80 = and i32 %79, 536870904
  %81 = tail call ptr @fa_alloc(i32 noundef %80, i32 noundef 8) #54
  tail call void @llvm.memset.p0.i32(ptr align 4 %77, i8 0, i32 %76, i1 false)
  tail call void @llvm.memset.p0.i32(ptr align 8 %81, i8 0, i32 %80, i1 false)
  %82 = load i32, ptr %70, align 8, !tbaa !153
  %83 = icmp eq i32 %82, 0
  br i1 %83, label %103, label %84

84:                                               ; preds = %73
  %85 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 1
  %86 = load ptr, ptr %85, align 8, !tbaa !154
  %87 = load i32, ptr %51, align 4, !tbaa !152
  %88 = shl i32 %87, 2
  tail call fastcc void @copy_bytes(ptr noundef %77, ptr noundef %86, i32 noundef %88) #53
  %89 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 2
  %90 = load ptr, ptr %89, align 4, !tbaa !155
  %91 = load i32, ptr %70, align 8, !tbaa !153
  %92 = add i32 %91, 63
  %93 = lshr i32 %92, 3
  %94 = and i32 %93, 536870904
  tail call fastcc void @copy_bytes(ptr noundef %81, ptr noundef %90, i32 noundef %94) #53
  %95 = load ptr, ptr %85, align 8, !tbaa !154
  %96 = load i32, ptr %70, align 8, !tbaa !153
  %97 = shl i32 %96, 2
  tail call void @fa_free(ptr noundef %95, i32 noundef %97, i32 noundef 4) #54
  %98 = load ptr, ptr %89, align 4, !tbaa !155
  %99 = load i32, ptr %70, align 8, !tbaa !153
  %100 = add i32 %99, 63
  %101 = lshr i32 %100, 3
  %102 = and i32 %101, 536870904
  tail call void @fa_free(ptr noundef %98, i32 noundef %102, i32 noundef 8) #54
  br label %103

103:                                              ; preds = %84, %73
  %104 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 1
  store ptr %77, ptr %104, align 8, !tbaa !154
  %105 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 2
  store ptr %81, ptr %105, align 4, !tbaa !155
  store i32 %75, ptr %70, align 8, !tbaa !153
  %106 = load i32, ptr %51, align 4, !tbaa !152
  br label %107

107:                                              ; preds = %103, %68
  %108 = phi i32 [ %106, %103 ], [ %52, %68 ]
  %109 = add i32 %108, 1
  store i32 %109, ptr %51, align 4, !tbaa !152
  br label %110

110:                                              ; preds = %107, %65
  %111 = phi i32 [ %69, %107 ], [ %66, %65 ]
  %112 = getelementptr inbounds %struct.PoolChunk, ptr %41, i32 0, i32 1
  store i32 %111, ptr %112, align 4, !tbaa !23
  %113 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 1
  %114 = load ptr, ptr %113, align 8, !tbaa !154
  %115 = getelementptr inbounds ptr, ptr %114, i32 %111
  store ptr %41, ptr %115, align 4, !tbaa !6
  %116 = and i32 %111, 63
  %117 = zext nneg i32 %116 to i64
  %118 = shl nuw i64 1, %117
  %119 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 2
  %120 = load ptr, ptr %119, align 4, !tbaa !155
  %121 = lshr i32 %111, 6
  %122 = getelementptr inbounds i64, ptr %120, i32 %121
  %123 = load i64, ptr %122, align 8, !tbaa !89
  %124 = or i64 %123, %118
  store i64 %124, ptr %122, align 8, !tbaa !89
  ret ptr %41
}

; Function Attrs: nofree norecurse nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden void @fa_pool_remove(ptr nocapture noundef %0, ptr noundef %1) local_unnamed_addr #33 {
  %3 = ptrtoint ptr %1 to i32
  %4 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 8
  %5 = load i32, ptr %4, align 4, !tbaa !158
  %6 = mul i32 %5, -65536
  %7 = and i32 %6, %3
  %8 = inttoptr i32 %7 to ptr
  %9 = load ptr, ptr %8, align 65536, !tbaa !6
  %10 = ptrtoint ptr %9 to i32
  %11 = sub i32 %3, %10
  %12 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 6
  %13 = load i32, ptr %12, align 4, !tbaa !150
  %14 = udiv i32 %11, %13
  %15 = and i32 %14, 63
  %16 = zext nneg i32 %15 to i64
  %17 = shl nuw i64 1, %16
  %18 = xor i64 %17, -1
  %19 = lshr i32 %14, 6
  %20 = getelementptr inbounds %struct.PoolChunk, ptr %8, i32 0, i32 5, i32 %19
  %21 = load i64, ptr %20, align 8, !tbaa !89
  %22 = and i64 %21, %18
  store i64 %22, ptr %20, align 8, !tbaa !89
  %23 = getelementptr inbounds %struct.PoolChunk, ptr %8, i32 0, i32 4
  %24 = load i32, ptr %23, align 16
  store i32 %24, ptr %1, align 1
  %25 = add i32 %14, 1
  store i32 %25, ptr %23, align 16, !tbaa !23
  %26 = getelementptr inbounds %struct.PoolChunk, ptr %8, i32 0, i32 2
  %27 = load i32, ptr %26, align 8, !tbaa !23
  %28 = add i32 %27, -1
  store i32 %28, ptr %26, align 8, !tbaa !23
  %29 = load i64, ptr %0, align 8, !tbaa !149
  %30 = add nsw i64 %29, -1
  store i64 %30, ptr %0, align 8, !tbaa !149
  %31 = getelementptr inbounds %struct.PoolChunk, ptr %8, i32 0, i32 1
  %32 = load i32, ptr %31, align 4, !tbaa !23
  %33 = icmp eq i32 %28, 0
  br i1 %33, label %34, label %69

34:                                               ; preds = %2
  %35 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 1
  %36 = load ptr, ptr %35, align 8, !tbaa !154
  %37 = getelementptr inbounds ptr, ptr %36, i32 %32
  store ptr null, ptr %37, align 4, !tbaa !6
  %38 = and i32 %32, 63
  %39 = zext nneg i32 %38 to i64
  %40 = shl nuw i64 1, %39
  %41 = xor i64 %40, -1
  %42 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 2
  %43 = load ptr, ptr %42, align 4, !tbaa !155
  %44 = lshr i32 %32, 6
  %45 = getelementptr inbounds i64, ptr %43, i32 %44
  %46 = load i64, ptr %45, align 8, !tbaa !89
  %47 = and i64 %46, %41
  store i64 %47, ptr %45, align 8, !tbaa !89
  %48 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 4
  %49 = load i32, ptr %48, align 4, !tbaa !152
  %50 = icmp eq i32 %49, 0
  br i1 %50, label %61, label %51

51:                                               ; preds = %34
  %52 = load ptr, ptr %35, align 8, !tbaa !154
  br label %53

53:                                               ; preds = %51, %59
  %54 = phi i32 [ %49, %51 ], [ %55, %59 ]
  %55 = add i32 %54, -1
  %56 = getelementptr inbounds ptr, ptr %52, i32 %55
  %57 = load ptr, ptr %56, align 4, !tbaa !6
  %58 = icmp eq ptr %57, null
  br i1 %58, label %59, label %61

59:                                               ; preds = %53
  store i32 %55, ptr %48, align 4, !tbaa !152
  %60 = icmp eq i32 %55, 0
  br i1 %60, label %61, label %53, !llvm.loop !163

61:                                               ; preds = %53, %59, %34
  %62 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 3
  %63 = load ptr, ptr %62, align 8, !tbaa !161
  %64 = icmp eq ptr %63, null
  br i1 %64, label %65, label %66

65:                                               ; preds = %61
  store ptr %8, ptr %62, align 8, !tbaa !161
  br label %79

66:                                               ; preds = %61
  %67 = load i32, ptr %4, align 4, !tbaa !158
  tail call fastcc void @give_pages(ptr noundef nonnull %8, i32 noundef %67) #53
  %68 = atomicrmw sub ptr @fa_live_blocks, i32 1 monotonic, align 4
  br label %79

69:                                               ; preds = %2
  %70 = and i32 %32, 63
  %71 = zext nneg i32 %70 to i64
  %72 = shl nuw i64 1, %71
  %73 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 2
  %74 = load ptr, ptr %73, align 4, !tbaa !155
  %75 = lshr i32 %32, 6
  %76 = getelementptr inbounds i64, ptr %74, i32 %75
  %77 = load i64, ptr %76, align 8, !tbaa !89
  %78 = or i64 %77, %72
  store i64 %78, ptr %76, align 8, !tbaa !89
  br label %79

79:                                               ; preds = %65, %66, %69
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(read, inaccessiblemem: none)
define hidden i64 @fa_pool_seek(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #34 {
  %3 = lshr i64 %1, 32
  %4 = trunc i64 %3 to i32
  %5 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 4
  %6 = load i32, ptr %5, align 4, !tbaa !152
  %7 = icmp ugt i32 %6, %4
  br i1 %7, label %8, label %48

8:                                                ; preds = %2
  %9 = trunc i64 %1 to i32
  %10 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 1
  %11 = load ptr, ptr %10, align 8, !tbaa !154
  br label %12

12:                                               ; preds = %8, %45
  %13 = phi i32 [ %4, %8 ], [ %46, %45 ]
  %14 = phi i32 [ %9, %8 ], [ 0, %45 ]
  %15 = getelementptr inbounds ptr, ptr %11, i32 %13
  %16 = load ptr, ptr %15, align 4, !tbaa !6
  %17 = icmp eq ptr %16, null
  br i1 %17, label %45, label %18

18:                                               ; preds = %12
  %19 = getelementptr inbounds %struct.PoolChunk, ptr %16, i32 0, i32 3
  %20 = load i32, ptr %19, align 4, !tbaa !23
  %21 = icmp ult i32 %14, %20
  br i1 %21, label %22, label %45

22:                                               ; preds = %18, %36
  %23 = phi i32 [ %38, %36 ], [ %14, %18 ]
  %24 = lshr i32 %23, 6
  %25 = getelementptr inbounds %struct.PoolChunk, ptr %16, i32 0, i32 5, i32 %24
  %26 = load i64, ptr %25, align 8, !tbaa !89
  %27 = and i32 %23, 63
  %28 = zext nneg i32 %27 to i64
  %29 = lshr i64 %26, %28
  %30 = icmp eq i64 %29, 0
  br i1 %30, label %36, label %31

31:                                               ; preds = %22
  %32 = tail call i64 @llvm.cttz.i64(i64 %29, i1 true), !range !41
  %33 = trunc i64 %32 to i32
  %34 = add i32 %23, %33
  %35 = icmp ult i32 %34, %20
  br i1 %35, label %40, label %45

36:                                               ; preds = %22
  %37 = and i32 %23, -64
  %38 = add i32 %37, 64
  %39 = icmp ult i32 %38, %20
  br i1 %39, label %22, label %45

40:                                               ; preds = %31
  %41 = zext i32 %13 to i64
  %42 = shl nuw i64 %41, 32
  %43 = zext i32 %34 to i64
  %44 = or disjoint i64 %42, %43
  br label %48

45:                                               ; preds = %36, %18, %31, %12
  %46 = add nuw i32 %13, 1
  %47 = icmp eq i32 %46, %6
  br i1 %47, label %48, label %12, !llvm.loop !164

48:                                               ; preds = %45, %2, %40
  %49 = phi i64 [ %44, %40 ], [ -1, %2 ], [ -1, %45 ]
  ret i64 %49
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(read, inaccessiblemem: none)
define hidden ptr @fa_pool_at(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #35 {
  %3 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 1
  %4 = load ptr, ptr %3, align 8, !tbaa !154
  %5 = lshr i64 %1, 32
  %6 = trunc i64 %5 to i32
  %7 = getelementptr inbounds ptr, ptr %4, i32 %6
  %8 = load ptr, ptr %7, align 4, !tbaa !6
  %9 = load ptr, ptr %8, align 8, !tbaa !6
  %10 = trunc i64 %1 to i32
  %11 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 6
  %12 = load i32, ptr %11, align 4, !tbaa !150
  %13 = mul i32 %12, %10
  %14 = getelementptr inbounds i8, ptr %9, i32 %13
  ret ptr %14
}

; Function Attrs: nofree norecurse nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden void @fa_pool_clear(ptr nocapture noundef %0) local_unnamed_addr #33 {
  tail call fastcc void @pool_release_chunks(ptr noundef %0) #53
  ret void
}

; Function Attrs: nofree norecurse nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc void @pool_release_chunks(ptr nocapture noundef %0) unnamed_addr #33 {
  %2 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 4
  %3 = load i32, ptr %2, align 4, !tbaa !152
  %4 = icmp eq i32 %3, 0
  br i1 %4, label %8, label %5

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 1
  %7 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 8
  br label %12

8:                                                ; preds = %23, %1
  %9 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 5
  %10 = load i32, ptr %9, align 8, !tbaa !153
  %11 = icmp eq i32 %10, 0
  br i1 %11, label %37, label %27

12:                                               ; preds = %5, %23
  %13 = phi i32 [ %3, %5 ], [ %24, %23 ]
  %14 = phi i32 [ 0, %5 ], [ %25, %23 ]
  %15 = load ptr, ptr %6, align 8, !tbaa !154
  %16 = getelementptr inbounds ptr, ptr %15, i32 %14
  %17 = load ptr, ptr %16, align 4, !tbaa !6
  %18 = icmp eq ptr %17, null
  br i1 %18, label %23, label %19

19:                                               ; preds = %12
  %20 = load i32, ptr %7, align 4, !tbaa !158
  tail call fastcc void @give_pages(ptr noundef nonnull %17, i32 noundef %20) #53
  %21 = atomicrmw sub ptr @fa_live_blocks, i32 1 monotonic, align 4
  %22 = load i32, ptr %2, align 4, !tbaa !152
  br label %23

23:                                               ; preds = %12, %19
  %24 = phi i32 [ %13, %12 ], [ %22, %19 ]
  %25 = add nuw i32 %14, 1
  %26 = icmp ult i32 %25, %24
  br i1 %26, label %12, label %8, !llvm.loop !165

27:                                               ; preds = %8
  %28 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 1
  %29 = load ptr, ptr %28, align 8, !tbaa !154
  %30 = shl i32 %10, 2
  tail call void @llvm.memset.p0.i32(ptr align 4 %29, i8 0, i32 %30, i1 false)
  %31 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 2
  %32 = load ptr, ptr %31, align 4, !tbaa !155
  %33 = load i32, ptr %9, align 8, !tbaa !153
  %34 = add i32 %33, 63
  %35 = lshr i32 %34, 3
  %36 = and i32 %35, 536870904
  tail call void @llvm.memset.p0.i32(ptr align 8 %32, i8 0, i32 %36, i1 false)
  br label %37

37:                                               ; preds = %27, %8
  store i32 0, ptr %2, align 4, !tbaa !152
  store i64 0, ptr %0, align 8, !tbaa !149
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_pool_free(ptr noundef %0) local_unnamed_addr #3 {
  %2 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 10
  %3 = load i64, ptr %2, align 8, !tbaa !147
  %4 = icmp eq i64 %3, 0
  br i1 %4, label %5, label %28

5:                                                ; preds = %1
  tail call fastcc void @pool_release_chunks(ptr noundef nonnull %0) #53
  %6 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 3
  %7 = load ptr, ptr %6, align 8, !tbaa !161
  %8 = icmp eq ptr %7, null
  br i1 %8, label %13, label %9

9:                                                ; preds = %5
  %10 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 8
  %11 = load i32, ptr %10, align 4, !tbaa !158
  tail call fastcc void @give_pages(ptr noundef nonnull %7, i32 noundef %11) #53
  %12 = atomicrmw sub ptr @fa_live_blocks, i32 1 monotonic, align 4
  br label %13

13:                                               ; preds = %9, %5
  %14 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 5
  %15 = load i32, ptr %14, align 8, !tbaa !153
  %16 = icmp eq i32 %15, 0
  br i1 %16, label %27, label %17

17:                                               ; preds = %13
  %18 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 1
  %19 = load ptr, ptr %18, align 8, !tbaa !154
  %20 = shl i32 %15, 2
  tail call void @fa_free(ptr noundef %19, i32 noundef %20, i32 noundef 4) #54
  %21 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 2
  %22 = load ptr, ptr %21, align 4, !tbaa !155
  %23 = load i32, ptr %14, align 8, !tbaa !153
  %24 = add i32 %23, 63
  %25 = lshr i32 %24, 3
  %26 = and i32 %25, 536870904
  tail call void @fa_free(ptr noundef %22, i32 noundef %26, i32 noundef 8) #54
  br label %27

27:                                               ; preds = %17, %13
  tail call void @fa_free(ptr noundef nonnull %0, i32 noundef 56, i32 noundef 8) #54
  br label %28

28:                                               ; preds = %1, %27
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(read, inaccessiblemem: none)
define hidden i32 @fa_links_has(ptr noundef readonly %0, ptr noundef readonly %1) local_unnamed_addr #34 {
  %3 = tail call fastcc i64 @links_find(ptr noundef %0, ptr noundef %1) #53
  %4 = icmp sgt i64 %3, -1
  %5 = zext i1 %4 to i32
  ret i32 %5
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(read, inaccessiblemem: none)
define internal fastcc i64 @links_find(ptr noundef readonly %0, ptr noundef %1) unnamed_addr #34 {
  %3 = icmp eq ptr %0, null
  br i1 %3, label %57, label %4

4:                                                ; preds = %2
  %5 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 4
  %6 = load ptr, ptr %5, align 8, !tbaa !166
  %7 = icmp eq ptr %6, null
  br i1 %7, label %8, label %24

8:                                                ; preds = %4
  %9 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 2
  %10 = load i64, ptr %9, align 8, !tbaa !168
  %11 = icmp sgt i64 %10, 0
  br i1 %11, label %12, label %57

12:                                               ; preds = %8
  %13 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 1
  %14 = load ptr, ptr %13, align 8, !tbaa !169
  br label %15

15:                                               ; preds = %12, %21
  %16 = phi i64 [ 0, %12 ], [ %22, %21 ]
  %17 = trunc i64 %16 to i32
  %18 = getelementptr inbounds ptr, ptr %14, i32 %17
  %19 = load ptr, ptr %18, align 4, !tbaa !6
  %20 = icmp eq ptr %19, %1
  br i1 %20, label %57, label %21

21:                                               ; preds = %15
  %22 = add nuw nsw i64 %16, 1
  %23 = icmp eq i64 %22, %10
  br i1 %23, label %57, label %15, !llvm.loop !170

24:                                               ; preds = %4
  %25 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 5
  %26 = load i64, ptr %25, align 8, !tbaa !171
  %27 = ptrtoint ptr %1 to i32
  %28 = zext i32 %27 to i64
  %29 = mul i64 %28, -49064778989728563
  %30 = lshr i64 %29, 33
  %31 = xor i64 %30, %29
  %32 = and i64 %31, %26
  %33 = trunc i64 %32 to i32
  %34 = getelementptr inbounds i32, ptr %6, i32 %33
  %35 = load i32, ptr %34, align 4, !tbaa !23
  %36 = icmp eq i32 %35, 0
  br i1 %36, label %57, label %37

37:                                               ; preds = %24
  %38 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 1
  %39 = load ptr, ptr %38, align 8, !tbaa !169
  %40 = getelementptr ptr, ptr %39, i32 -1
  br label %48

41:                                               ; preds = %48
  %42 = add i64 %50, 1
  %43 = and i64 %42, %26
  %44 = trunc i64 %43 to i32
  %45 = getelementptr inbounds i32, ptr %6, i32 %44
  %46 = load i32, ptr %45, align 4, !tbaa !23
  %47 = icmp eq i32 %46, 0
  br i1 %47, label %57, label %48

48:                                               ; preds = %37, %41
  %49 = phi i32 [ %35, %37 ], [ %46, %41 ]
  %50 = phi i64 [ %32, %37 ], [ %43, %41 ]
  %51 = getelementptr ptr, ptr %40, i32 %49
  %52 = load ptr, ptr %51, align 4, !tbaa !6
  %53 = icmp eq ptr %52, %1
  br i1 %53, label %54, label %41

54:                                               ; preds = %48
  %55 = zext i32 %49 to i64
  %56 = add nsw i64 %55, -1
  br label %57

57:                                               ; preds = %41, %15, %21, %24, %8, %54, %2
  %58 = phi i64 [ -1, %2 ], [ %56, %54 ], [ -1, %8 ], [ -1, %24 ], [ %16, %15 ], [ -1, %21 ], [ -1, %41 ]
  ret i64 %58
}

; Function Attrs: nounwind optsize
define hidden noundef i32 @fa_links_add(ptr nocapture noundef %0, ptr noundef %1) local_unnamed_addr #3 {
  %3 = load ptr, ptr %0, align 4, !tbaa !6
  %4 = icmp eq ptr %3, null
  br i1 %4, label %5, label %7

5:                                                ; preds = %2
  %6 = tail call ptr @fa_alloc(i32 noundef 48, i32 noundef 8) #54
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(48) %6, i8 0, i32 48, i1 false)
  store ptr %6, ptr %0, align 4, !tbaa !6
  br label %10

7:                                                ; preds = %2
  %8 = tail call fastcc i64 @links_find(ptr noundef nonnull %3, ptr noundef %1) #53
  %9 = icmp sgt i64 %8, -1
  br i1 %9, label %157, label %10

10:                                               ; preds = %7, %5
  %11 = phi ptr [ %6, %5 ], [ %3, %7 ]
  %12 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 2
  %13 = load i64, ptr %12, align 8, !tbaa !168
  %14 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 3
  %15 = load i64, ptr %14, align 8, !tbaa !172
  %16 = icmp eq i64 %13, %15
  br i1 %16, label %17, label %124

17:                                               ; preds = %10
  %18 = icmp sgt i64 %13, 0
  br i1 %18, label %19, label %42

19:                                               ; preds = %17
  %20 = load i64, ptr %11, align 8, !tbaa !173
  %21 = shl nsw i64 %20, 1
  %22 = icmp sgt i64 %21, %13
  br i1 %22, label %42, label %23

23:                                               ; preds = %19
  %24 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 1
  br label %26

25:                                               ; preds = %38
  store i64 %39, ptr %12, align 8, !tbaa !168
  br label %63

26:                                               ; preds = %23, %38
  %27 = phi i64 [ 0, %23 ], [ %40, %38 ]
  %28 = phi i64 [ 0, %23 ], [ %39, %38 ]
  %29 = load ptr, ptr %24, align 8, !tbaa !169
  %30 = trunc i64 %27 to i32
  %31 = getelementptr inbounds ptr, ptr %29, i32 %30
  %32 = load ptr, ptr %31, align 4, !tbaa !6
  %33 = icmp eq ptr %32, null
  br i1 %33, label %38, label %34

34:                                               ; preds = %26
  %35 = trunc i64 %28 to i32
  %36 = getelementptr inbounds ptr, ptr %29, i32 %35
  store ptr %32, ptr %36, align 4, !tbaa !6
  %37 = add nsw i64 %28, 1
  br label %38

38:                                               ; preds = %26, %34
  %39 = phi i64 [ %37, %34 ], [ %28, %26 ]
  %40 = add nuw nsw i64 %27, 1
  %41 = icmp eq i64 %40, %13
  br i1 %41, label %25, label %26, !llvm.loop !174

42:                                               ; preds = %19, %17
  %43 = icmp eq i64 %13, 0
  %44 = shl nsw i64 %13, 1
  %45 = select i1 %43, i64 4, i64 %44
  %46 = trunc i64 %45 to i32
  %47 = shl i32 %46, 2
  %48 = tail call ptr @fa_alloc(i32 noundef %47, i32 noundef 4) #54
  %49 = load i64, ptr %14, align 8, !tbaa !172
  %50 = icmp sgt i64 %49, 0
  br i1 %50, label %51, label %61

51:                                               ; preds = %42
  %52 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 1
  %53 = load ptr, ptr %52, align 8, !tbaa !169
  %54 = load i64, ptr %12, align 8, !tbaa !168
  %55 = trunc i64 %54 to i32
  %56 = shl i32 %55, 2
  tail call fastcc void @copy_bytes(ptr noundef %48, ptr noundef %53, i32 noundef %56) #53
  %57 = load ptr, ptr %52, align 8, !tbaa !169
  %58 = load i64, ptr %14, align 8, !tbaa !172
  %59 = trunc i64 %58 to i32
  %60 = shl i32 %59, 2
  tail call void @fa_free(ptr noundef %57, i32 noundef %60, i32 noundef 4) #54
  br label %61

61:                                               ; preds = %51, %42
  %62 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 1
  store ptr %48, ptr %62, align 8, !tbaa !169
  store i64 %45, ptr %14, align 8, !tbaa !172
  br label %63

63:                                               ; preds = %61, %25
  %64 = phi i64 [ %45, %61 ], [ %13, %25 ]
  %65 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 4
  %66 = load ptr, ptr %65, align 8, !tbaa !166
  %67 = icmp eq ptr %66, null
  br i1 %67, label %75, label %68

68:                                               ; preds = %63
  %69 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 5
  %70 = load i64, ptr %69, align 8, !tbaa !171
  %71 = trunc i64 %70 to i32
  %72 = shl i32 %71, 2
  %73 = add i32 %72, 4
  tail call void @fa_free(ptr noundef nonnull %66, i32 noundef %73, i32 noundef 4) #54
  store ptr null, ptr %65, align 8, !tbaa !166
  %74 = load i64, ptr %14, align 8, !tbaa !172
  br label %75

75:                                               ; preds = %68, %63
  %76 = phi i64 [ %74, %68 ], [ %64, %63 ]
  %77 = icmp slt i64 %76, 9
  br i1 %77, label %78, label %80

78:                                               ; preds = %75
  %79 = load i64, ptr %12, align 8, !tbaa !168
  br label %124

80:                                               ; preds = %75
  %81 = shl nuw nsw i64 %76, 1
  br label %82

82:                                               ; preds = %82, %80
  %83 = phi i64 [ %85, %82 ], [ 16, %80 ]
  %84 = icmp slt i64 %83, %81
  %85 = shl nsw i64 %83, 1
  br i1 %84, label %82, label %86, !llvm.loop !175

86:                                               ; preds = %82
  %87 = add nsw i64 %83, -1
  %88 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 5
  store i64 %87, ptr %88, align 8, !tbaa !171
  %89 = trunc i64 %83 to i32
  %90 = shl i32 %89, 2
  %91 = tail call ptr @fa_alloc(i32 noundef %90, i32 noundef 4) #54
  store ptr %91, ptr %65, align 8, !tbaa !166
  tail call void @llvm.memset.p0.i32(ptr align 4 %91, i8 0, i32 %90, i1 false)
  %92 = load i64, ptr %12, align 8, !tbaa !168
  %93 = icmp sgt i64 %92, 0
  br i1 %93, label %94, label %124

94:                                               ; preds = %86
  %95 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 1
  %96 = load ptr, ptr %95, align 8, !tbaa !169
  br label %97

97:                                               ; preds = %121, %94
  %98 = phi i64 [ 0, %94 ], [ %122, %121 ]
  %99 = trunc i64 %98 to i32
  %100 = getelementptr inbounds ptr, ptr %96, i32 %99
  %101 = load ptr, ptr %100, align 4, !tbaa !6
  %102 = icmp eq ptr %101, null
  br i1 %102, label %121, label %103

103:                                              ; preds = %97
  %104 = load i64, ptr %88, align 8, !tbaa !171
  %105 = ptrtoint ptr %101 to i32
  %106 = zext i32 %105 to i64
  %107 = mul i64 %106, -49064778989728563
  %108 = lshr i64 %107, 33
  %109 = xor i64 %108, %107
  %110 = load ptr, ptr %65, align 8, !tbaa !166
  br label %111

111:                                              ; preds = %111, %103
  %112 = phi i64 [ %109, %103 ], [ %118, %111 ]
  %113 = and i64 %112, %104
  %114 = trunc i64 %113 to i32
  %115 = getelementptr inbounds i32, ptr %110, i32 %114
  %116 = load i32, ptr %115, align 4, !tbaa !23
  %117 = icmp eq i32 %116, 0
  %118 = add i64 %113, 1
  br i1 %117, label %119, label %111, !llvm.loop !176

119:                                              ; preds = %111
  %120 = add i32 %99, 1
  store i32 %120, ptr %115, align 4, !tbaa !23
  br label %121

121:                                              ; preds = %119, %97
  %122 = add nuw nsw i64 %98, 1
  %123 = icmp eq i64 %122, %92
  br i1 %123, label %124, label %97, !llvm.loop !177

124:                                              ; preds = %121, %78, %86, %10
  %125 = phi i64 [ %79, %78 ], [ %92, %86 ], [ %13, %10 ], [ %92, %121 ]
  %126 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 1
  %127 = load ptr, ptr %126, align 8, !tbaa !169
  %128 = trunc i64 %125 to i32
  %129 = getelementptr inbounds ptr, ptr %127, i32 %128
  store ptr %1, ptr %129, align 4, !tbaa !6
  %130 = add nsw i64 %125, 1
  store i64 %130, ptr %12, align 8, !tbaa !168
  %131 = load i64, ptr %11, align 8, !tbaa !173
  %132 = add nsw i64 %131, 1
  store i64 %132, ptr %11, align 8, !tbaa !173
  %133 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 4
  %134 = load ptr, ptr %133, align 8, !tbaa !166
  %135 = icmp eq ptr %134, null
  br i1 %135, label %157, label %136

136:                                              ; preds = %124
  %137 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 5
  %138 = load i64, ptr %137, align 8, !tbaa !171
  %139 = load ptr, ptr %126, align 8, !tbaa !169
  %140 = getelementptr inbounds ptr, ptr %139, i32 %128
  %141 = load ptr, ptr %140, align 4, !tbaa !6
  %142 = ptrtoint ptr %141 to i32
  %143 = zext i32 %142 to i64
  %144 = mul i64 %143, -49064778989728563
  %145 = lshr i64 %144, 33
  %146 = xor i64 %145, %144
  br label %147

147:                                              ; preds = %147, %136
  %148 = phi i64 [ %146, %136 ], [ %154, %147 ]
  %149 = and i64 %148, %138
  %150 = trunc i64 %149 to i32
  %151 = getelementptr inbounds i32, ptr %134, i32 %150
  %152 = load i32, ptr %151, align 4, !tbaa !23
  %153 = icmp eq i32 %152, 0
  %154 = add i64 %149, 1
  br i1 %153, label %155, label %147, !llvm.loop !176

155:                                              ; preds = %147
  %156 = add i32 %128, 1
  store i32 %156, ptr %151, align 4, !tbaa !23
  br label %157

157:                                              ; preds = %124, %155, %7
  %158 = phi i32 [ 0, %7 ], [ 1, %155 ], [ 1, %124 ]
  ret i32 %158
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden noundef i32 @fa_links_delete(ptr noundef %0, ptr noundef %1) local_unnamed_addr #19 {
  %3 = tail call fastcc i64 @links_find(ptr noundef %0, ptr noundef %1) #53
  %4 = icmp slt i64 %3, 0
  br i1 %4, label %78, label %5

5:                                                ; preds = %2
  %6 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 1
  %7 = load ptr, ptr %6, align 8, !tbaa !169
  %8 = trunc i64 %3 to i32
  %9 = getelementptr inbounds ptr, ptr %7, i32 %8
  store ptr null, ptr %9, align 4, !tbaa !6
  %10 = load i64, ptr %0, align 8, !tbaa !173
  %11 = add nsw i64 %10, -1
  store i64 %11, ptr %0, align 8, !tbaa !173
  %12 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 4
  %13 = load ptr, ptr %12, align 8, !tbaa !166
  %14 = icmp eq ptr %13, null
  br i1 %14, label %74, label %15

15:                                               ; preds = %5
  %16 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 5
  %17 = load i64, ptr %16, align 8, !tbaa !171
  %18 = ptrtoint ptr %1 to i32
  %19 = zext i32 %18 to i64
  %20 = mul i64 %19, -49064778989728563
  %21 = lshr i64 %20, 33
  %22 = xor i64 %21, %20
  %23 = add i32 %8, 1
  br label %24

24:                                               ; preds = %24, %15
  %25 = phi i64 [ %22, %15 ], [ %31, %24 ]
  %26 = and i64 %25, %17
  %27 = trunc i64 %26 to i32
  %28 = getelementptr inbounds i32, ptr %13, i32 %27
  %29 = load i32, ptr %28, align 4, !tbaa !23
  %30 = icmp eq i32 %29, %23
  %31 = add i64 %26, 1
  br i1 %30, label %32, label %24, !llvm.loop !178

32:                                               ; preds = %24
  %33 = and i64 %31, %17
  %34 = trunc i64 %33 to i32
  %35 = getelementptr inbounds i32, ptr %13, i32 %34
  %36 = load i32, ptr %35, align 4, !tbaa !23
  %37 = icmp eq i32 %36, 0
  br i1 %37, label %71, label %38

38:                                               ; preds = %32
  %39 = load ptr, ptr %6, align 8, !tbaa !169
  %40 = getelementptr ptr, ptr %39, i32 -1
  br label %41

41:                                               ; preds = %38, %61
  %42 = phi i32 [ %36, %38 ], [ %67, %61 ]
  %43 = phi i64 [ %33, %38 ], [ %64, %61 ]
  %44 = phi i64 [ %26, %38 ], [ %62, %61 ]
  %45 = getelementptr ptr, ptr %40, i32 %42
  %46 = load ptr, ptr %45, align 4, !tbaa !6
  %47 = ptrtoint ptr %46 to i32
  %48 = zext i32 %47 to i64
  %49 = mul i64 %48, -49064778989728563
  %50 = lshr i64 %49, 33
  %51 = xor i64 %50, %49
  %52 = and i64 %51, %17
  %53 = sub i64 %43, %52
  %54 = and i64 %53, %17
  %55 = sub i64 %43, %44
  %56 = and i64 %55, %17
  %57 = icmp ult i64 %54, %56
  br i1 %57, label %61, label %58

58:                                               ; preds = %41
  %59 = trunc i64 %44 to i32
  %60 = getelementptr inbounds i32, ptr %13, i32 %59
  store i32 %42, ptr %60, align 4, !tbaa !23
  br label %61

61:                                               ; preds = %41, %58
  %62 = phi i64 [ %43, %58 ], [ %44, %41 ]
  %63 = add i64 %43, 1
  %64 = and i64 %63, %17
  %65 = trunc i64 %64 to i32
  %66 = getelementptr inbounds i32, ptr %13, i32 %65
  %67 = load i32, ptr %66, align 4, !tbaa !23
  %68 = icmp eq i32 %67, 0
  br i1 %68, label %69, label %41

69:                                               ; preds = %61
  %70 = trunc i64 %62 to i32
  br label %71

71:                                               ; preds = %69, %32
  %72 = phi i32 [ %70, %69 ], [ %27, %32 ]
  %73 = getelementptr inbounds i32, ptr %13, i32 %72
  store i32 0, ptr %73, align 4, !tbaa !23
  br label %74

74:                                               ; preds = %71, %5
  %75 = icmp eq i64 %11, 0
  br i1 %75, label %76, label %78

76:                                               ; preds = %74
  %77 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 2
  store i64 0, ptr %77, align 8, !tbaa !168
  br label %78

78:                                               ; preds = %74, %76, %2
  %79 = phi i32 [ 0, %2 ], [ 1, %76 ], [ 1, %74 ]
  ret i32 %79
}

; Function Attrs: nounwind optsize
define hidden void @fa_links_free(ptr noundef %0) local_unnamed_addr #3 {
  %2 = icmp eq ptr %0, null
  br i1 %2, label %23, label %3

3:                                                ; preds = %1
  %4 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 4
  %5 = load ptr, ptr %4, align 8, !tbaa !166
  %6 = icmp eq ptr %5, null
  br i1 %6, label %13, label %7

7:                                                ; preds = %3
  %8 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 5
  %9 = load i64, ptr %8, align 8, !tbaa !171
  %10 = trunc i64 %9 to i32
  %11 = shl i32 %10, 2
  %12 = add i32 %11, 4
  tail call void @fa_free(ptr noundef nonnull %5, i32 noundef %12, i32 noundef 4) #54
  br label %13

13:                                               ; preds = %7, %3
  %14 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 3
  %15 = load i64, ptr %14, align 8, !tbaa !172
  %16 = icmp sgt i64 %15, 0
  br i1 %16, label %17, label %22

17:                                               ; preds = %13
  %18 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 1
  %19 = load ptr, ptr %18, align 8, !tbaa !169
  %20 = trunc i64 %15 to i32
  %21 = shl i32 %20, 2
  tail call void @fa_free(ptr noundef %19, i32 noundef %21, i32 noundef 4) #54
  br label %22

22:                                               ; preds = %17, %13
  tail call void @fa_free(ptr noundef nonnull %0, i32 noundef 48, i32 noundef 8) #54
  br label %23

23:                                               ; preds = %1, %22
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(read, inaccessiblemem: none)
define hidden i64 @fa_links_seek(ptr noundef readonly %0, i64 noundef %1) local_unnamed_addr #34 {
  %3 = icmp eq ptr %0, null
  br i1 %3, label %20, label %4

4:                                                ; preds = %2
  %5 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 2
  %6 = load i64, ptr %5, align 8, !tbaa !168
  %7 = icmp ugt i64 %6, %1
  br i1 %7, label %8, label %20

8:                                                ; preds = %4
  %9 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 1
  %10 = load ptr, ptr %9, align 8, !tbaa !169
  br label %11

11:                                               ; preds = %8, %17
  %12 = phi i64 [ %1, %8 ], [ %18, %17 ]
  %13 = trunc i64 %12 to i32
  %14 = getelementptr inbounds ptr, ptr %10, i32 %13
  %15 = load ptr, ptr %14, align 4, !tbaa !6
  %16 = icmp eq ptr %15, null
  br i1 %16, label %17, label %20

17:                                               ; preds = %11
  %18 = add i64 %12, 1
  %19 = icmp eq i64 %18, %6
  br i1 %19, label %20, label %11, !llvm.loop !179

20:                                               ; preds = %11, %17, %4, %2
  %21 = phi i64 [ -1, %2 ], [ -1, %4 ], [ %12, %11 ], [ -1, %17 ]
  ret i64 %21
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(read, inaccessiblemem: none)
define hidden ptr @fa_links_at(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #35 {
  %3 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 1
  %4 = load ptr, ptr %3, align 8, !tbaa !169
  %5 = trunc i64 %1 to i32
  %6 = getelementptr inbounds ptr, ptr %4, i32 %5
  %7 = load ptr, ptr %6, align 4, !tbaa !6
  ret ptr %7
}

; Function Attrs: nounwind optsize
define hidden void @fa_relate_one(ptr noundef %0, i64 noundef %1, i64 noundef %2, ptr noundef %3, i32 noundef %4) local_unnamed_addr #3 {
  %6 = trunc i64 %1 to i32
  %7 = getelementptr inbounds i8, ptr %0, i32 %6
  %8 = load ptr, ptr %7, align 4, !tbaa !6
  %9 = icmp eq ptr %8, %3
  br i1 %9, label %40, label %10

10:                                               ; preds = %5
  %11 = icmp eq i32 %4, 0
  %12 = icmp eq ptr %8, null
  br i1 %11, label %25, label %13

13:                                               ; preds = %10
  br i1 %12, label %19, label %14

14:                                               ; preds = %13
  %15 = trunc i64 %2 to i32
  %16 = getelementptr inbounds i8, ptr %8, i32 %15
  %17 = load ptr, ptr %16, align 4, !tbaa !6
  %18 = tail call i32 @fa_links_delete(ptr noundef %17, ptr noundef nonnull %0) #53, !range !53
  br label %19

19:                                               ; preds = %14, %13
  store ptr %3, ptr %7, align 4, !tbaa !6
  %20 = icmp eq ptr %3, null
  br i1 %20, label %40, label %21

21:                                               ; preds = %19
  %22 = trunc i64 %2 to i32
  %23 = getelementptr inbounds i8, ptr %3, i32 %22
  %24 = tail call i32 @fa_links_add(ptr noundef nonnull %23, ptr noundef nonnull %0) #53, !range !53
  br label %40

25:                                               ; preds = %10
  br i1 %12, label %29, label %26

26:                                               ; preds = %25
  %27 = trunc i64 %2 to i32
  %28 = getelementptr inbounds i8, ptr %8, i32 %27
  store ptr null, ptr %28, align 4, !tbaa !6
  br label %29

29:                                               ; preds = %26, %25
  %30 = icmp eq ptr %3, null
  br i1 %30, label %39, label %31

31:                                               ; preds = %29
  %32 = trunc i64 %2 to i32
  %33 = getelementptr inbounds i8, ptr %3, i32 %32
  %34 = load ptr, ptr %33, align 4, !tbaa !6
  %35 = icmp eq ptr %34, null
  br i1 %35, label %38, label %36

36:                                               ; preds = %31
  %37 = getelementptr inbounds i8, ptr %34, i32 %6
  store ptr null, ptr %37, align 4, !tbaa !6
  br label %38

38:                                               ; preds = %36, %31
  store ptr %0, ptr %33, align 4, !tbaa !6
  br label %39

39:                                               ; preds = %38, %29
  store ptr %3, ptr %7, align 4, !tbaa !6
  br label %40

40:                                               ; preds = %19, %21, %5, %39
  ret void
}

; Function Attrs: nounwind optsize
define hidden noundef i32 @fa_relate_add(ptr noundef %0, i64 noundef %1, i64 noundef %2, ptr noundef %3) local_unnamed_addr #3 {
  %5 = trunc i64 %1 to i32
  %6 = getelementptr inbounds i8, ptr %0, i32 %5
  %7 = tail call i32 @fa_links_add(ptr noundef %6, ptr noundef %3) #53, !range !53
  %8 = icmp eq i32 %7, 0
  br i1 %8, label %13, label %9

9:                                                ; preds = %4
  %10 = trunc i64 %2 to i32
  %11 = getelementptr inbounds i8, ptr %3, i32 %10
  %12 = tail call i32 @fa_links_add(ptr noundef %11, ptr noundef %0) #53, !range !53
  br label %13

13:                                               ; preds = %4, %9
  %14 = phi i32 [ 1, %9 ], [ 0, %4 ]
  ret i32 %14
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden noundef i32 @fa_relate_delete(ptr noundef %0, i64 noundef %1, i64 noundef %2, ptr noundef %3) local_unnamed_addr #19 {
  %5 = trunc i64 %1 to i32
  %6 = getelementptr inbounds i8, ptr %0, i32 %5
  %7 = load ptr, ptr %6, align 4, !tbaa !6
  %8 = tail call i32 @fa_links_delete(ptr noundef %7, ptr noundef %3) #53, !range !53
  %9 = icmp eq i32 %8, 0
  br i1 %9, label %15, label %10

10:                                               ; preds = %4
  %11 = trunc i64 %2 to i32
  %12 = getelementptr inbounds i8, ptr %3, i32 %11
  %13 = load ptr, ptr %12, align 4, !tbaa !6
  %14 = tail call i32 @fa_links_delete(ptr noundef %13, ptr noundef nonnull %0) #53, !range !53
  br label %15

15:                                               ; preds = %4, %10
  %16 = phi i32 [ 1, %10 ], [ 0, %4 ]
  ret i32 %16
}

; Function Attrs: nounwind optsize
define hidden void @fa_relate_clear(ptr noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #3 {
  %4 = trunc i64 %1 to i32
  %5 = getelementptr inbounds i8, ptr %0, i32 %4
  %6 = load ptr, ptr %5, align 4, !tbaa !6
  %7 = icmp eq ptr %6, null
  br i1 %7, label %33, label %8

8:                                                ; preds = %3
  %9 = getelementptr inbounds %struct.FaLinks, ptr %6, i32 0, i32 2
  %10 = load i64, ptr %9, align 8, !tbaa !168
  %11 = icmp sgt i64 %10, 0
  br i1 %11, label %12, label %15

12:                                               ; preds = %8
  %13 = getelementptr inbounds %struct.FaLinks, ptr %6, i32 0, i32 1
  %14 = trunc i64 %2 to i32
  br label %16

15:                                               ; preds = %29, %8
  tail call void @fa_links_free(ptr noundef nonnull %6) #53
  store ptr null, ptr %5, align 4, !tbaa !6
  br label %33

16:                                               ; preds = %12, %29
  %17 = phi i64 [ %10, %12 ], [ %30, %29 ]
  %18 = phi i64 [ 0, %12 ], [ %31, %29 ]
  %19 = load ptr, ptr %13, align 8, !tbaa !169
  %20 = trunc i64 %18 to i32
  %21 = getelementptr inbounds ptr, ptr %19, i32 %20
  %22 = load ptr, ptr %21, align 4, !tbaa !6
  %23 = icmp eq ptr %22, null
  br i1 %23, label %29, label %24

24:                                               ; preds = %16
  %25 = getelementptr inbounds i8, ptr %22, i32 %14
  %26 = load ptr, ptr %25, align 4, !tbaa !6
  %27 = tail call i32 @fa_links_delete(ptr noundef %26, ptr noundef %0) #53, !range !53
  %28 = load i64, ptr %9, align 8, !tbaa !168
  br label %29

29:                                               ; preds = %24, %16
  %30 = phi i64 [ %28, %24 ], [ %17, %16 ]
  %31 = add nuw nsw i64 %18, 1
  %32 = icmp slt i64 %31, %30
  br i1 %32, label %16, label %15, !llvm.loop !180

33:                                               ; preds = %3, %15
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden void @fa_unlink(ptr noundef %0, i32 noundef %1, i32 noundef %2, i64 noundef %3, i64 noundef %4, ptr noundef %5) local_unnamed_addr #19 {
  %7 = icmp eq i32 %2, 0
  br i1 %7, label %46, label %8

8:                                                ; preds = %6
  %9 = trunc i64 %3 to i32
  %10 = getelementptr inbounds i8, ptr %0, i32 %9
  %11 = load ptr, ptr %10, align 4, !tbaa !6
  %12 = icmp eq i32 %1, 2
  br i1 %12, label %13, label %36

13:                                               ; preds = %8
  %14 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 2
  %15 = icmp eq ptr %11, null
  br i1 %15, label %99, label %16

16:                                               ; preds = %13
  %17 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 1
  %18 = trunc i64 %4 to i32
  %19 = load i64, ptr %14, align 8, !tbaa !168
  %20 = icmp sgt i64 %19, 0
  br i1 %20, label %21, label %99

21:                                               ; preds = %16, %32
  %22 = phi i64 [ %33, %32 ], [ 0, %16 ]
  %23 = load ptr, ptr %17, align 8, !tbaa !169
  %24 = trunc i64 %22 to i32
  %25 = getelementptr inbounds ptr, ptr %23, i32 %24
  %26 = load ptr, ptr %25, align 4, !tbaa !6
  %27 = icmp eq ptr %26, null
  br i1 %27, label %32, label %28

28:                                               ; preds = %21
  %29 = getelementptr inbounds i8, ptr %26, i32 %18
  %30 = load ptr, ptr %29, align 4, !tbaa !6
  %31 = tail call i32 @fa_links_delete(ptr noundef %30, ptr noundef %0) #53, !range !53
  br label %32

32:                                               ; preds = %28, %21
  %33 = add nuw nsw i64 %22, 1
  %34 = load i64, ptr %14, align 8, !tbaa !168
  %35 = icmp slt i64 %33, %34
  br i1 %35, label %21, label %99

36:                                               ; preds = %8
  %37 = icmp eq ptr %11, null
  br i1 %37, label %99, label %38

38:                                               ; preds = %36
  %39 = icmp eq i32 %1, 0
  %40 = trunc i64 %4 to i32
  %41 = getelementptr inbounds i8, ptr %11, i32 %40
  br i1 %39, label %42, label %45

42:                                               ; preds = %38
  %43 = load ptr, ptr %41, align 4, !tbaa !6
  %44 = tail call i32 @fa_links_delete(ptr noundef %43, ptr noundef nonnull %0) #53, !range !53
  br label %99

45:                                               ; preds = %38
  store ptr null, ptr %41, align 4, !tbaa !6
  br label %99

46:                                               ; preds = %6
  switch i32 %1, label %93 [
    i32 0, label %47
    i32 2, label %47
    i32 1, label %76
    i32 3, label %84
  ]

47:                                               ; preds = %46, %46
  %48 = trunc i64 %4 to i32
  %49 = getelementptr inbounds i8, ptr %0, i32 %48
  %50 = load ptr, ptr %49, align 4, !tbaa !6
  %51 = getelementptr inbounds %struct.FaLinks, ptr %50, i32 0, i32 2
  %52 = icmp eq ptr %50, null
  br i1 %52, label %99, label %53

53:                                               ; preds = %47
  %54 = getelementptr inbounds %struct.FaLinks, ptr %50, i32 0, i32 1
  %55 = icmp eq i32 %1, 0
  %56 = trunc i64 %3 to i32
  %57 = load i64, ptr %51, align 8, !tbaa !168
  %58 = icmp sgt i64 %57, 0
  br i1 %58, label %59, label %99

59:                                               ; preds = %53, %72
  %60 = phi i64 [ %73, %72 ], [ 0, %53 ]
  %61 = load ptr, ptr %54, align 8, !tbaa !169
  %62 = trunc i64 %60 to i32
  %63 = getelementptr inbounds ptr, ptr %61, i32 %62
  %64 = load ptr, ptr %63, align 4, !tbaa !6
  %65 = icmp eq ptr %64, null
  br i1 %65, label %72, label %66

66:                                               ; preds = %59
  %67 = getelementptr inbounds i8, ptr %64, i32 %56
  br i1 %55, label %68, label %69

68:                                               ; preds = %66
  store ptr null, ptr %67, align 4, !tbaa !6
  br label %72

69:                                               ; preds = %66
  %70 = load ptr, ptr %67, align 4, !tbaa !6
  %71 = tail call i32 @fa_links_delete(ptr noundef %70, ptr noundef %0) #53, !range !53
  br label %72

72:                                               ; preds = %68, %69, %59
  %73 = add nuw nsw i64 %60, 1
  %74 = load i64, ptr %51, align 8, !tbaa !168
  %75 = icmp slt i64 %73, %74
  br i1 %75, label %59, label %99

76:                                               ; preds = %46
  %77 = trunc i64 %4 to i32
  %78 = getelementptr inbounds i8, ptr %0, i32 %77
  %79 = load ptr, ptr %78, align 4, !tbaa !6
  %80 = icmp eq ptr %79, null
  br i1 %80, label %99, label %81

81:                                               ; preds = %76
  %82 = trunc i64 %3 to i32
  %83 = getelementptr inbounds i8, ptr %79, i32 %82
  store ptr null, ptr %83, align 4, !tbaa !6
  br label %99

84:                                               ; preds = %46
  %85 = trunc i64 %3 to i32
  %86 = getelementptr inbounds i8, ptr %0, i32 %85
  %87 = load ptr, ptr %86, align 4, !tbaa !6
  %88 = icmp eq ptr %87, null
  %89 = icmp eq ptr %87, %0
  %90 = or i1 %88, %89
  br i1 %90, label %99, label %91

91:                                               ; preds = %84
  %92 = getelementptr inbounds i8, ptr %87, i32 %85
  store ptr null, ptr %92, align 4, !tbaa !6
  br label %99

93:                                               ; preds = %46
  %94 = icmp eq ptr %5, null
  br i1 %94, label %99, label %95

95:                                               ; preds = %93
  %96 = load ptr, ptr %5, align 4, !tbaa !6
  %97 = icmp eq ptr %96, %0
  br i1 %97, label %98, label %99

98:                                               ; preds = %95
  store ptr null, ptr %5, align 4, !tbaa !6
  br label %99

99:                                               ; preds = %32, %72, %16, %53, %13, %47, %84, %91, %76, %81, %42, %45, %36, %93, %95, %98
  ret void
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_address_map_new(i64 noundef %0) local_unnamed_addr #3 {
  %2 = tail call ptr @fa_alloc(i32 noundef 16, i32 noundef 8) #54
  %3 = shl nsw i64 %0, 1
  br label %4

4:                                                ; preds = %4, %1
  %5 = phi i64 [ 16, %1 ], [ %7, %4 ]
  %6 = icmp slt i64 %5, %3
  %7 = shl nsw i64 %5, 1
  br i1 %6, label %4, label %8, !llvm.loop !181

8:                                                ; preds = %4
  %9 = add nsw i64 %5, -1
  %10 = getelementptr inbounds %struct.FaAddressMap, ptr %2, i32 0, i32 2
  store i64 %9, ptr %10, align 8, !tbaa !182
  %11 = trunc i64 %5 to i32
  %12 = shl i32 %11, 2
  %13 = tail call ptr @fa_alloc(i32 noundef %12, i32 noundef 4) #54
  store ptr %13, ptr %2, align 8, !tbaa !184
  %14 = tail call ptr @fa_alloc(i32 noundef %12, i32 noundef 4) #54
  %15 = getelementptr inbounds %struct.FaAddressMap, ptr %2, i32 0, i32 1
  store ptr %14, ptr %15, align 4, !tbaa !185
  %16 = load ptr, ptr %2, align 8, !tbaa !184
  tail call void @llvm.memset.p0.i32(ptr align 4 %16, i8 0, i32 %12, i1 false)
  ret ptr %2
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden void @fa_address_map_put(ptr nocapture noundef readonly %0, ptr noundef %1, ptr noundef %2) local_unnamed_addr #19 {
  %4 = getelementptr inbounds %struct.FaAddressMap, ptr %0, i32 0, i32 2
  %5 = load i64, ptr %4, align 8, !tbaa !182
  %6 = ptrtoint ptr %1 to i32
  %7 = zext i32 %6 to i64
  %8 = mul i64 %7, -49064778989728563
  %9 = lshr i64 %8, 33
  %10 = xor i64 %9, %8
  %11 = load ptr, ptr %0, align 8, !tbaa !184
  br label %12

12:                                               ; preds = %12, %3
  %13 = phi i64 [ %10, %3 ], [ %21, %12 ]
  %14 = and i64 %13, %5
  %15 = trunc i64 %14 to i32
  %16 = getelementptr inbounds ptr, ptr %11, i32 %15
  %17 = load ptr, ptr %16, align 4, !tbaa !6
  %18 = icmp eq ptr %17, null
  %19 = icmp eq ptr %17, %1
  %20 = or i1 %18, %19
  %21 = add i64 %14, 1
  br i1 %20, label %22, label %12, !llvm.loop !186

22:                                               ; preds = %12
  store ptr %1, ptr %16, align 4, !tbaa !6
  %23 = getelementptr inbounds %struct.FaAddressMap, ptr %0, i32 0, i32 1
  %24 = load ptr, ptr %23, align 4, !tbaa !185
  %25 = getelementptr inbounds ptr, ptr %24, i32 %15
  store ptr %2, ptr %25, align 4, !tbaa !6
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(read, inaccessiblemem: none)
define hidden ptr @fa_address_map_get(ptr nocapture noundef readonly %0, ptr noundef %1) local_unnamed_addr #34 {
  %3 = icmp eq ptr %1, null
  br i1 %3, label %36, label %4

4:                                                ; preds = %2
  %5 = getelementptr inbounds %struct.FaAddressMap, ptr %0, i32 0, i32 2
  %6 = load i64, ptr %5, align 8, !tbaa !182
  %7 = ptrtoint ptr %1 to i32
  %8 = zext i32 %7 to i64
  %9 = mul i64 %8, -49064778989728563
  %10 = lshr i64 %9, 33
  %11 = xor i64 %10, %9
  %12 = load ptr, ptr %0, align 8, !tbaa !184
  %13 = and i64 %11, %6
  %14 = trunc i64 %13 to i32
  %15 = getelementptr inbounds ptr, ptr %12, i32 %14
  %16 = load ptr, ptr %15, align 4, !tbaa !6
  %17 = icmp eq ptr %16, null
  br i1 %17, label %36, label %18

18:                                               ; preds = %4
  %19 = icmp eq ptr %16, %1
  br i1 %19, label %30, label %20

20:                                               ; preds = %18, %28
  %21 = phi i64 [ %23, %28 ], [ %13, %18 ]
  %22 = add i64 %21, 1
  %23 = and i64 %22, %6
  %24 = trunc i64 %23 to i32
  %25 = getelementptr inbounds ptr, ptr %12, i32 %24
  %26 = load ptr, ptr %25, align 4, !tbaa !6
  %27 = icmp eq ptr %26, null
  br i1 %27, label %36, label %28, !llvm.loop !187

28:                                               ; preds = %20
  %29 = icmp eq ptr %26, %1
  br i1 %29, label %30, label %20, !llvm.loop !187

30:                                               ; preds = %28, %18
  %31 = phi i32 [ %14, %18 ], [ %24, %28 ]
  %32 = getelementptr inbounds %struct.FaAddressMap, ptr %0, i32 0, i32 1
  %33 = load ptr, ptr %32, align 4, !tbaa !185
  %34 = getelementptr inbounds ptr, ptr %33, i32 %31
  %35 = load ptr, ptr %34, align 4, !tbaa !6
  br label %36

36:                                               ; preds = %20, %4, %30, %2
  %37 = phi ptr [ null, %2 ], [ %35, %30 ], [ null, %4 ], [ null, %20 ]
  ret ptr %37
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_links_relink(ptr noundef readonly %0, ptr nocapture noundef readonly %1) local_unnamed_addr #3 {
  %3 = alloca ptr, align 4
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %3) #56
  store ptr null, ptr %3, align 4, !tbaa !6
  %4 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 2
  %5 = icmp eq ptr %0, null
  br i1 %5, label %12, label %6

6:                                                ; preds = %2
  %7 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 1
  %8 = load i64, ptr %4, align 8, !tbaa !168
  %9 = icmp sgt i64 %8, 0
  br i1 %9, label %14, label %10

10:                                               ; preds = %24, %6
  %11 = load ptr, ptr %3, align 4, !tbaa !6
  br label %12

12:                                               ; preds = %10, %2
  %13 = phi ptr [ %11, %10 ], [ null, %2 ]
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %3) #56
  ret ptr %13

14:                                               ; preds = %6, %24
  %15 = phi i64 [ %25, %24 ], [ 0, %6 ]
  %16 = load ptr, ptr %7, align 8, !tbaa !169
  %17 = trunc i64 %15 to i32
  %18 = getelementptr inbounds ptr, ptr %16, i32 %17
  %19 = load ptr, ptr %18, align 4, !tbaa !6
  %20 = tail call ptr @fa_address_map_get(ptr noundef %1, ptr noundef %19) #53
  %21 = icmp eq ptr %20, null
  br i1 %21, label %24, label %22

22:                                               ; preds = %14
  %23 = call i32 @fa_links_add(ptr noundef nonnull %3, ptr noundef nonnull %20) #53, !range !53
  br label %24

24:                                               ; preds = %22, %14
  %25 = add nuw nsw i64 %15, 1
  %26 = load i64, ptr %4, align 8, !tbaa !168
  %27 = icmp slt i64 %25, %26
  br i1 %27, label %14, label %10
}

; Function Attrs: nounwind optsize
define hidden void @fa_address_map_free(ptr noundef %0) local_unnamed_addr #3 {
  %2 = load ptr, ptr %0, align 8, !tbaa !184
  %3 = getelementptr inbounds %struct.FaAddressMap, ptr %0, i32 0, i32 2
  %4 = load i64, ptr %3, align 8, !tbaa !182
  %5 = trunc i64 %4 to i32
  %6 = shl i32 %5, 2
  %7 = add i32 %6, 4
  tail call void @fa_free(ptr noundef %2, i32 noundef %7, i32 noundef 4) #54
  %8 = getelementptr inbounds %struct.FaAddressMap, ptr %0, i32 0, i32 1
  %9 = load ptr, ptr %8, align 4, !tbaa !185
  %10 = load i64, ptr %3, align 8, !tbaa !182
  %11 = trunc i64 %10 to i32
  %12 = shl i32 %11, 2
  %13 = add i32 %12, 4
  tail call void @fa_free(ptr noundef %9, i32 noundef %13, i32 noundef 4) #54
  tail call void @fa_free(ptr noundef nonnull %0, i32 noundef 16, i32 noundef 8) #54
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, argmem: none, inaccessiblemem: none)
define hidden void @fa_argument_source_set(i32 noundef %0, ptr noundef %1) local_unnamed_addr #7 {
  %3 = and i32 %0, 15
  %4 = getelementptr inbounds [16 x ptr], ptr @argument_sources, i32 0, i32 %3
  store ptr %1, ptr %4, align 4, !tbaa !6
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(read, argmem: none, inaccessiblemem: none)
define hidden ptr @fa_argument_source_get(i32 noundef %0) local_unnamed_addr #36 {
  %2 = and i32 %0, 15
  %3 = getelementptr inbounds [16 x ptr], ptr @argument_sources, i32 0, i32 %2
  %4 = load ptr, ptr %3, align 4, !tbaa !6
  ret ptr %4
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_source_new() local_unnamed_addr #3 {
  %1 = tail call ptr @fa_alloc(i32 noundef 64, i32 noundef 8) #54
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(64) %1, i8 0, i64 64, i1 false)
  %2 = load ptr, ptr @reactive_made, align 4, !tbaa !6
  %3 = load i64, ptr @reactive_made_count, align 8, !tbaa !89
  %4 = tail call fastcc ptr @grow_items(ptr noundef %2, i64 noundef %3, ptr noundef nonnull @reactive_made_capacity, i32 noundef 4) #53
  store ptr %4, ptr @reactive_made, align 4, !tbaa !6
  %5 = load i64, ptr @reactive_made_count, align 8, !tbaa !89
  %6 = trunc i64 %5 to i32
  %7 = getelementptr inbounds ptr, ptr %4, i32 %6
  store ptr %1, ptr %7, align 4, !tbaa !6
  %8 = add nsw i64 %5, 1
  store i64 %8, ptr @reactive_made_count, align 8, !tbaa !89
  ret ptr %1
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_source_child(ptr noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #3 {
  %4 = icmp eq ptr %0, null
  br i1 %4, label %50, label %5

5:                                                ; preds = %3
  %6 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 7
  %7 = load i64, ptr %6, align 8, !tbaa !188
  %8 = icmp sgt i64 %7, 0
  br i1 %8, label %9, label %28

9:                                                ; preds = %5
  %10 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 6
  %11 = load ptr, ptr %10, align 4, !tbaa !190
  br label %12

12:                                               ; preds = %9, %22
  %13 = phi i64 [ 0, %9 ], [ %23, %22 ]
  %14 = trunc i64 %13 to i32
  %15 = getelementptr inbounds %struct.FaChild, ptr %11, i32 %14
  %16 = load i64, ptr %15, align 8, !tbaa !191
  %17 = icmp eq i64 %16, %1
  br i1 %17, label %18, label %22

18:                                               ; preds = %12
  %19 = getelementptr inbounds %struct.FaChild, ptr %11, i32 %14, i32 1
  %20 = load i64, ptr %19, align 8, !tbaa !193
  %21 = icmp eq i64 %20, %2
  br i1 %21, label %25, label %22

22:                                               ; preds = %18, %12
  %23 = add nuw nsw i64 %13, 1
  %24 = icmp eq i64 %23, %7
  br i1 %24, label %28, label %12, !llvm.loop !194

25:                                               ; preds = %18
  %26 = getelementptr inbounds %struct.FaChild, ptr %11, i32 %14, i32 2
  %27 = load ptr, ptr %26, align 8, !tbaa !195
  br label %50

28:                                               ; preds = %22, %5
  %29 = tail call ptr @fa_alloc(i32 noundef 64, i32 noundef 8) #54
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(64) %29, i8 0, i32 64, i1 false)
  %30 = getelementptr inbounds %struct.FaSource, ptr %29, i32 0, i32 5
  store ptr %0, ptr %30, align 8, !tbaa !196
  %31 = load ptr, ptr @reactive_made, align 4, !tbaa !6
  %32 = load i64, ptr @reactive_made_count, align 8, !tbaa !89
  %33 = tail call fastcc ptr @grow_items(ptr noundef %31, i64 noundef %32, ptr noundef nonnull @reactive_made_capacity, i32 noundef 4) #53
  store ptr %33, ptr @reactive_made, align 4, !tbaa !6
  %34 = load i64, ptr @reactive_made_count, align 8, !tbaa !89
  %35 = trunc i64 %34 to i32
  %36 = getelementptr inbounds ptr, ptr %33, i32 %35
  store ptr %29, ptr %36, align 4, !tbaa !6
  %37 = add nsw i64 %34, 1
  store i64 %37, ptr @reactive_made_count, align 8, !tbaa !89
  %38 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 6
  %39 = load ptr, ptr %38, align 4, !tbaa !190
  %40 = load i64, ptr %6, align 8, !tbaa !188
  %41 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 8
  %42 = tail call fastcc ptr @grow_items(ptr noundef %39, i64 noundef %40, ptr noundef nonnull %41, i32 noundef 24) #53
  store ptr %42, ptr %38, align 4, !tbaa !190
  %43 = load i64, ptr %6, align 8, !tbaa !188
  %44 = trunc i64 %43 to i32
  %45 = getelementptr inbounds %struct.FaChild, ptr %42, i32 %44
  store i64 %1, ptr %45, align 8, !tbaa.struct !197
  %46 = getelementptr inbounds i8, ptr %45, i32 8
  store i64 %2, ptr %46, align 8, !tbaa.struct !198
  %47 = getelementptr inbounds i8, ptr %45, i32 16
  store ptr %29, ptr %47, align 8, !tbaa.struct !199
  %48 = load i64, ptr %6, align 8, !tbaa !188
  %49 = add nsw i64 %48, 1
  store i64 %49, ptr %6, align 8, !tbaa !188
  br label %50

50:                                               ; preds = %25, %3, %28
  %51 = phi ptr [ %27, %25 ], [ %29, %28 ], [ null, %3 ]
  ret ptr %51
}

; Function Attrs: nounwind optsize
define internal fastcc ptr @grow_items(ptr noundef %0, i64 noundef %1, ptr nocapture noundef %2, i32 noundef %3) unnamed_addr #3 {
  %5 = load i64, ptr %2, align 8, !tbaa !89
  %6 = icmp sgt i64 %5, %1
  br i1 %6, label %23, label %7

7:                                                ; preds = %4
  %8 = icmp eq i64 %5, 0
  %9 = shl nsw i64 %5, 1
  %10 = select i1 %8, i64 4, i64 %9
  %11 = trunc i64 %10 to i32
  %12 = mul i32 %11, %3
  %13 = tail call ptr @fa_alloc(i32 noundef %12, i32 noundef 4) #54
  %14 = load i64, ptr %2, align 8, !tbaa !89
  %15 = icmp sgt i64 %14, 0
  br i1 %15, label %16, label %22

16:                                               ; preds = %7
  %17 = trunc i64 %1 to i32
  %18 = mul i32 %17, %3
  tail call fastcc void @copy_bytes(ptr noundef %13, ptr noundef %0, i32 noundef %18) #53
  %19 = load i64, ptr %2, align 8, !tbaa !89
  %20 = trunc i64 %19 to i32
  %21 = mul i32 %20, %3
  tail call void @fa_free(ptr noundef %0, i32 noundef %21, i32 noundef 4) #54
  br label %22

22:                                               ; preds = %16, %7
  store i64 %10, ptr %2, align 8, !tbaa !89
  br label %23

23:                                               ; preds = %4, %22
  %24 = phi ptr [ %13, %22 ], [ %0, %4 ]
  ret ptr %24
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_field_source(ptr noundef %0, i64 noundef %1) local_unnamed_addr #3 {
  %3 = ptrtoint ptr %0 to i32
  %4 = zext i32 %3 to i64
  %5 = load i64, ptr @field_source_count, align 8, !tbaa !89
  %6 = icmp sgt i64 %5, 0
  br i1 %6, label %7, label %25

7:                                                ; preds = %2
  %8 = load ptr, ptr @field_sources, align 4, !tbaa !6
  br label %9

9:                                                ; preds = %7, %22
  %10 = phi i64 [ 0, %7 ], [ %23, %22 ]
  %11 = trunc i64 %10 to i32
  %12 = getelementptr inbounds %struct.FaChild, ptr %8, i32 %11
  %13 = load i64, ptr %12, align 8, !tbaa !191
  %14 = icmp eq i64 %13, %4
  br i1 %14, label %15, label %22

15:                                               ; preds = %9
  %16 = getelementptr inbounds %struct.FaChild, ptr %8, i32 %11, i32 1
  %17 = load i64, ptr %16, align 8, !tbaa !193
  %18 = icmp eq i64 %17, %1
  br i1 %18, label %19, label %22

19:                                               ; preds = %15
  %20 = getelementptr inbounds %struct.FaChild, ptr %8, i32 %11, i32 2
  %21 = load ptr, ptr %20, align 8, !tbaa !195
  br label %36

22:                                               ; preds = %15, %9
  %23 = add nuw nsw i64 %10, 1
  %24 = icmp eq i64 %23, %5
  br i1 %24, label %25, label %9, !llvm.loop !200

25:                                               ; preds = %22, %2
  %26 = tail call ptr @fa_source_new() #53
  %27 = load ptr, ptr @field_sources, align 4, !tbaa !6
  %28 = load i64, ptr @field_source_count, align 8, !tbaa !89
  %29 = tail call fastcc ptr @grow_items(ptr noundef %27, i64 noundef %28, ptr noundef nonnull @field_source_capacity, i32 noundef 24) #53
  store ptr %29, ptr @field_sources, align 4, !tbaa !6
  %30 = load i64, ptr @field_source_count, align 8, !tbaa !89
  %31 = trunc i64 %30 to i32
  %32 = getelementptr inbounds %struct.FaChild, ptr %29, i32 %31
  store i64 %4, ptr %32, align 8, !tbaa.struct !197
  %33 = getelementptr inbounds i8, ptr %32, i32 8
  store i64 %1, ptr %33, align 8, !tbaa.struct !198
  %34 = getelementptr inbounds i8, ptr %32, i32 16
  store ptr %26, ptr %34, align 8, !tbaa.struct !199
  %35 = add nsw i64 %30, 1
  store i64 %35, ptr @field_source_count, align 8, !tbaa !89
  br label %36

36:                                               ; preds = %19, %25
  %37 = phi ptr [ %26, %25 ], [ %21, %19 ]
  ret ptr %37
}

; Function Attrs: nounwind optsize
define hidden void @fa_track(ptr noundef %0) local_unnamed_addr #3 {
  %2 = alloca i64, align 8
  %3 = load ptr, ptr @reactive_observer, align 4, !tbaa !6
  %4 = icmp eq ptr %3, null
  %5 = icmp eq ptr %0, null
  %6 = or i1 %5, %4
  br i1 %6, label %66, label %7

7:                                                ; preds = %1
  %8 = getelementptr inbounds %struct.FaObserver, ptr %3, i32 0, i32 8
  %9 = load i64, ptr %8, align 8, !tbaa !201
  %10 = icmp sgt i64 %9, 0
  %11 = getelementptr inbounds %struct.FaObserver, ptr %3, i32 0, i32 6
  %12 = load ptr, ptr %11, align 4, !tbaa !203
  br i1 %10, label %16, label %22

13:                                               ; preds = %16
  %14 = add nuw nsw i64 %17, 1
  %15 = icmp eq i64 %14, %9
  br i1 %15, label %22, label %16, !llvm.loop !204

16:                                               ; preds = %7, %13
  %17 = phi i64 [ %14, %13 ], [ 0, %7 ]
  %18 = trunc i64 %17 to i32
  %19 = getelementptr inbounds ptr, ptr %12, i32 %18
  %20 = load ptr, ptr %19, align 4, !tbaa !6
  %21 = icmp eq ptr %20, %0
  br i1 %21, label %66, label %13

22:                                               ; preds = %13, %7
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %2) #56
  %23 = getelementptr inbounds %struct.FaObserver, ptr %3, i32 0, i32 9
  %24 = load i64, ptr %23, align 8, !tbaa !205
  store i64 %24, ptr %2, align 8, !tbaa !89
  %25 = getelementptr inbounds %struct.FaObserver, ptr %3, i32 0, i32 6
  %26 = call fastcc ptr @grow_items(ptr noundef %12, i64 noundef %9, ptr noundef nonnull %2, i32 noundef 4) #53
  store ptr %26, ptr %25, align 4, !tbaa !203
  %27 = load i64, ptr %23, align 8, !tbaa !205
  store i64 %27, ptr %2, align 8, !tbaa !89
  %28 = getelementptr inbounds %struct.FaObserver, ptr %3, i32 0, i32 7
  %29 = load ptr, ptr %28, align 8, !tbaa !206
  %30 = load i64, ptr %8, align 8, !tbaa !201
  %31 = call fastcc ptr @grow_items(ptr noundef %29, i64 noundef %30, ptr noundef nonnull %2, i32 noundef 8) #53
  store ptr %31, ptr %28, align 8, !tbaa !206
  %32 = load i64, ptr %2, align 8, !tbaa !89
  store i64 %32, ptr %23, align 8, !tbaa !205
  %33 = load ptr, ptr %25, align 4, !tbaa !203
  %34 = load i64, ptr %8, align 8, !tbaa !201
  %35 = trunc i64 %34 to i32
  %36 = getelementptr inbounds ptr, ptr %33, i32 %35
  store ptr %0, ptr %36, align 4, !tbaa !6
  %37 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 1
  %38 = load i64, ptr %37, align 8, !tbaa !207
  %39 = load ptr, ptr %28, align 8, !tbaa !206
  %40 = getelementptr inbounds i64, ptr %39, i32 %35
  store i64 %38, ptr %40, align 8, !tbaa !89
  %41 = load i64, ptr %8, align 8, !tbaa !201
  %42 = add nsw i64 %41, 1
  store i64 %42, ptr %8, align 8, !tbaa !201
  %43 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 3
  %44 = load i64, ptr %43, align 8, !tbaa !208
  %45 = icmp sgt i64 %44, 0
  %46 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 2
  %47 = load ptr, ptr %46, align 8, !tbaa !209
  br i1 %45, label %51, label %57

48:                                               ; preds = %51
  %49 = add nuw nsw i64 %52, 1
  %50 = icmp eq i64 %49, %44
  br i1 %50, label %57, label %51, !llvm.loop !210

51:                                               ; preds = %22, %48
  %52 = phi i64 [ %49, %48 ], [ 0, %22 ]
  %53 = trunc i64 %52 to i32
  %54 = getelementptr inbounds ptr, ptr %47, i32 %53
  %55 = load ptr, ptr %54, align 4, !tbaa !6
  %56 = icmp eq ptr %55, %3
  br i1 %56, label %65, label %48

57:                                               ; preds = %48, %22
  %58 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 2
  %59 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 4
  %60 = tail call fastcc ptr @grow_items(ptr noundef %47, i64 noundef %44, ptr noundef nonnull %59, i32 noundef 4) #53
  store ptr %60, ptr %58, align 8, !tbaa !209
  %61 = load i64, ptr %43, align 8, !tbaa !208
  %62 = trunc i64 %61 to i32
  %63 = getelementptr inbounds ptr, ptr %60, i32 %62
  store ptr %3, ptr %63, align 4, !tbaa !6
  %64 = add nsw i64 %61, 1
  store i64 %64, ptr %43, align 8, !tbaa !208
  br label %65

65:                                               ; preds = %51, %57
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %2) #56
  br label %66

66:                                               ; preds = %16, %1, %65
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_notify(ptr noundef %0) local_unnamed_addr #3 {
  %2 = icmp eq ptr %0, null
  br i1 %2, label %15, label %3

3:                                                ; preds = %1
  tail call fastcc void @bump(ptr noundef nonnull %0) #53
  %4 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 5
  %5 = load ptr, ptr %4, align 8, !tbaa !196
  %6 = icmp eq ptr %5, null
  br i1 %6, label %15, label %7

7:                                                ; preds = %3, %7
  %8 = phi ptr [ %13, %7 ], [ %5, %3 ]
  %9 = getelementptr inbounds %struct.FaSource, ptr %8, i32 0, i32 1
  %10 = load i64, ptr %9, align 8, !tbaa !207
  %11 = add nsw i64 %10, 1
  store i64 %11, ptr %9, align 8, !tbaa !207
  tail call fastcc void @mark_observers(ptr noundef nonnull %8) #53
  %12 = getelementptr inbounds %struct.FaSource, ptr %8, i32 0, i32 5
  %13 = load ptr, ptr %12, align 8, !tbaa !196
  %14 = icmp eq ptr %13, null
  br i1 %14, label %15, label %7, !llvm.loop !211

15:                                               ; preds = %7, %3, %1
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @bump(ptr nocapture noundef %0) unnamed_addr #3 {
  %2 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 1
  %3 = load i64, ptr %2, align 8, !tbaa !207
  %4 = add nsw i64 %3, 1
  store i64 %4, ptr %2, align 8, !tbaa !207
  tail call fastcc void @mark_observers(ptr noundef %0) #53
  %5 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 7
  %6 = load i64, ptr %5, align 8, !tbaa !188
  %7 = icmp sgt i64 %6, 0
  br i1 %7, label %8, label %10

8:                                                ; preds = %1
  %9 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 6
  br label %11

10:                                               ; preds = %11, %1
  ret void

11:                                               ; preds = %8, %11
  %12 = phi i64 [ 0, %8 ], [ %17, %11 ]
  %13 = load ptr, ptr %9, align 4, !tbaa !190
  %14 = trunc i64 %12 to i32
  %15 = getelementptr inbounds %struct.FaChild, ptr %13, i32 %14, i32 2
  %16 = load ptr, ptr %15, align 8, !tbaa !195
  tail call fastcc void @bump(ptr noundef %16) #53
  %17 = add nuw nsw i64 %12, 1
  %18 = load i64, ptr %5, align 8, !tbaa !188
  %19 = icmp slt i64 %17, %18
  br i1 %19, label %11, label %10, !llvm.loop !212
}

; Function Attrs: nounwind optsize
define internal fastcc void @mark_observers(ptr nocapture noundef readonly %0) unnamed_addr #3 {
  %2 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 3
  %3 = load i64, ptr %2, align 8, !tbaa !208
  %4 = icmp sgt i64 %3, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 2
  br label %8

7:                                                ; preds = %8, %1
  ret void

8:                                                ; preds = %5, %8
  %9 = phi i64 [ 0, %5 ], [ %14, %8 ]
  %10 = load ptr, ptr %6, align 8, !tbaa !209
  %11 = trunc i64 %9 to i32
  %12 = getelementptr inbounds ptr, ptr %10, i32 %11
  %13 = load ptr, ptr %12, align 4, !tbaa !6
  tail call fastcc void @mark(ptr noundef %13) #53
  %14 = add nuw nsw i64 %9, 1
  %15 = load i64, ptr %2, align 8, !tbaa !208
  %16 = icmp slt i64 %14, %15
  br i1 %16, label %8, label %7, !llvm.loop !213
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_derived_new(ptr noundef %0, ptr noundef %1, ptr noundef %2, i64 noundef %3) local_unnamed_addr #3 {
  %5 = tail call fastcc ptr @observer_new(ptr noundef %0, ptr noundef null, ptr noundef %1, ptr noundef %2, i64 noundef %3) #53
  store i64 1, ptr %5, align 8, !tbaa !214
  %6 = getelementptr inbounds %struct.FaObserver, ptr %5, i32 0, i32 11
  store i32 1, ptr %6, align 8, !tbaa !215
  ret ptr %5
}

; Function Attrs: nounwind optsize
define internal fastcc noundef ptr @observer_new(ptr noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef %3, i64 noundef %4) unnamed_addr #3 {
  %6 = tail call ptr @fa_alloc(i32 noundef 168, i32 noundef 8) #54
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(168) %6, i8 0, i32 168, i1 false)
  %7 = getelementptr inbounds %struct.FaObserver, ptr %6, i32 0, i32 3
  store ptr %0, ptr %7, align 8, !tbaa !216
  %8 = getelementptr inbounds %struct.FaObserver, ptr %6, i32 0, i32 4
  store ptr %1, ptr %8, align 4, !tbaa !217
  %9 = getelementptr inbounds %struct.FaObserver, ptr %6, i32 0, i32 5
  store ptr %2, ptr %9, align 8, !tbaa !218
  %10 = getelementptr inbounds %struct.FaObserver, ptr %6, i32 0, i32 2
  store ptr %3, ptr %10, align 4, !tbaa !219
  %11 = icmp sgt i64 %4, 0
  br i1 %11, label %12, label %17

12:                                               ; preds = %5
  %13 = trunc i64 %4 to i32
  %14 = tail call ptr @fa_alloc(i32 noundef %13, i32 noundef 16) #54
  %15 = getelementptr inbounds %struct.FaObserver, ptr %6, i32 0, i32 1
  store ptr %14, ptr %15, align 8, !tbaa !220
  tail call void @llvm.memset.p0.i32(ptr align 1 %14, i8 0, i32 %13, i1 false)
  %16 = getelementptr inbounds %struct.FaObserver, ptr %6, i32 0, i32 19
  store i64 %4, ptr %16, align 8, !tbaa !221
  br label %17

17:                                               ; preds = %12, %5
  %18 = load ptr, ptr @reactive_made, align 4, !tbaa !6
  %19 = load i64, ptr @reactive_made_count, align 8, !tbaa !89
  %20 = tail call fastcc ptr @grow_items(ptr noundef %18, i64 noundef %19, ptr noundef nonnull @reactive_made_capacity, i32 noundef 4) #53
  store ptr %20, ptr @reactive_made, align 4, !tbaa !6
  %21 = load i64, ptr @reactive_made_count, align 8, !tbaa !89
  %22 = trunc i64 %21 to i32
  %23 = getelementptr inbounds ptr, ptr %20, i32 %22
  store ptr %6, ptr %23, align 4, !tbaa !6
  %24 = add nsw i64 %21, 1
  store i64 %24, ptr @reactive_made_count, align 8, !tbaa !89
  ret ptr %6
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_derived_read(ptr noundef %0) local_unnamed_addr #3 {
  tail call fastcc void @refresh(ptr noundef %0) #53
  tail call void @fa_track(ptr noundef %0) #53
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 1
  %3 = load ptr, ptr %2, align 8, !tbaa !220
  ret ptr %3
}

; Function Attrs: nounwind optsize
define internal fastcc void @refresh(ptr noundef %0) unnamed_addr #3 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 11
  %3 = load i32, ptr %2, align 8, !tbaa !215
  %4 = icmp eq i32 %3, 0
  br i1 %4, label %29, label %5

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 12
  %7 = load i32, ptr %6, align 4, !tbaa !222
  %8 = icmp eq i32 %7, 0
  br i1 %8, label %13, label %9

9:                                                ; preds = %5
  %10 = tail call fastcc i32 @outdated(ptr noundef nonnull %0) #53
  %11 = icmp eq i32 %10, 0
  br i1 %11, label %12, label %13

12:                                               ; preds = %9
  store i32 0, ptr %2, align 8, !tbaa !215
  br label %29

13:                                               ; preds = %9, %5
  %14 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 13
  %15 = load i32, ptr %14, align 8, !tbaa !223
  %16 = icmp eq i32 %15, 0
  br i1 %16, label %18, label %17

17:                                               ; preds = %13
  tail call fastcc void @reactive_stop(ptr noundef nonnull @.str.35) #57
  unreachable

18:                                               ; preds = %13
  tail call fastcc void @unsubscribe(ptr noundef nonnull %0) #53
  %19 = load ptr, ptr @reactive_observer, align 4, !tbaa !6
  store ptr %0, ptr @reactive_observer, align 4, !tbaa !6
  store i32 1, ptr %14, align 8, !tbaa !223
  %20 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 16
  store i32 1, ptr %20, align 4, !tbaa !224
  %21 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 3
  %22 = load ptr, ptr %21, align 8, !tbaa !216
  tail call void %22(ptr noundef nonnull %0) #54
  store i32 0, ptr %14, align 8, !tbaa !223
  store ptr %19, ptr @reactive_observer, align 4, !tbaa !6
  store i32 0, ptr %2, align 8, !tbaa !215
  store i32 1, ptr %6, align 4, !tbaa !222
  %23 = load i32, ptr %20, align 4, !tbaa !224
  %24 = icmp eq i32 %23, 0
  br i1 %24, label %29, label %25

25:                                               ; preds = %18
  %26 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 1
  %27 = load i64, ptr %26, align 8, !tbaa !225
  %28 = add nsw i64 %27, 1
  store i64 %28, ptr %26, align 8, !tbaa !225
  br label %29

29:                                               ; preds = %18, %25, %1, %12
  ret void
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_watch_new(ptr noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef %3) local_unnamed_addr #3 {
  %5 = tail call fastcc ptr @observer_new(ptr noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef %3, i64 noundef 0) #53
  store i64 2, ptr %5, align 8, !tbaa !214
  %6 = getelementptr inbounds %struct.FaObserver, ptr %5, i32 0, i32 17
  store i32 1, ptr %6, align 8, !tbaa !226
  %7 = load i64, ptr @reactive_sequence, align 8, !tbaa !89
  %8 = add nsw i64 %7, 1
  store i64 %8, ptr @reactive_sequence, align 8, !tbaa !89
  %9 = getelementptr inbounds %struct.FaObserver, ptr %5, i32 0, i32 10
  store i64 %8, ptr %9, align 8, !tbaa !227
  tail call fastcc void @unsubscribe(ptr noundef nonnull %5) #53
  %10 = load ptr, ptr @reactive_observer, align 4, !tbaa !6
  store ptr null, ptr @reactive_observer, align 4, !tbaa !6
  %11 = getelementptr inbounds %struct.FaObserver, ptr %5, i32 0, i32 4
  %12 = load ptr, ptr %11, align 4, !tbaa !217
  tail call void %12(ptr noundef nonnull %5) #54
  store ptr %5, ptr @reactive_observer, align 4, !tbaa !6
  %13 = getelementptr inbounds %struct.FaObserver, ptr %5, i32 0, i32 3
  %14 = load ptr, ptr %13, align 8, !tbaa !216
  tail call void %14(ptr noundef nonnull %5) #54
  store ptr %10, ptr @reactive_observer, align 4, !tbaa !6
  ret ptr %5
}

; Function Attrs: nounwind optsize
define hidden void @fa_dispose(ptr noundef %0) local_unnamed_addr #3 {
  %2 = icmp eq ptr %0, null
  br i1 %2, label %11, label %3

3:                                                ; preds = %1
  %4 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 15
  %5 = load i32, ptr %4, align 8, !tbaa !228
  %6 = icmp eq i32 %5, 0
  br i1 %6, label %7, label %11

7:                                                ; preds = %3
  store i32 1, ptr %4, align 8, !tbaa !228
  tail call fastcc void @unsubscribe(ptr noundef nonnull %0) #53
  %8 = load ptr, ptr @reactive_observer, align 4, !tbaa !6
  store ptr null, ptr @reactive_observer, align 4, !tbaa !6
  %9 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 4
  %10 = load ptr, ptr %9, align 4, !tbaa !217
  tail call void %10(ptr noundef nonnull %0) #54
  store ptr %8, ptr @reactive_observer, align 4, !tbaa !6
  br label %11

11:                                               ; preds = %1, %3, %7
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc void @unsubscribe(ptr noundef %0) unnamed_addr #19 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 8
  %3 = load i64, ptr %2, align 8, !tbaa !201
  %4 = icmp sgt i64 %3, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 6
  br label %8

7:                                                ; preds = %35, %1
  store i64 0, ptr %2, align 8, !tbaa !201
  ret void

8:                                                ; preds = %5, %35
  %9 = phi i64 [ 0, %5 ], [ %36, %35 ]
  %10 = load ptr, ptr %6, align 4, !tbaa !203
  %11 = trunc i64 %9 to i32
  %12 = getelementptr inbounds ptr, ptr %10, i32 %11
  %13 = load ptr, ptr %12, align 4, !tbaa !6
  %14 = getelementptr inbounds %struct.FaSource, ptr %13, i32 0, i32 3
  %15 = load i64, ptr %14, align 8, !tbaa !208
  %16 = icmp sgt i64 %15, 0
  br i1 %16, label %17, label %35

17:                                               ; preds = %8
  %18 = getelementptr inbounds %struct.FaSource, ptr %13, i32 0, i32 2
  %19 = load ptr, ptr %18, align 8, !tbaa !209
  br label %23

20:                                               ; preds = %23
  %21 = add nuw nsw i64 %24, 1
  %22 = icmp eq i64 %21, %15
  br i1 %22, label %35, label %23, !llvm.loop !229

23:                                               ; preds = %17, %20
  %24 = phi i64 [ 0, %17 ], [ %21, %20 ]
  %25 = trunc i64 %24 to i32
  %26 = getelementptr inbounds ptr, ptr %19, i32 %25
  %27 = load ptr, ptr %26, align 4, !tbaa !6
  %28 = icmp eq ptr %27, %0
  br i1 %28, label %29, label %20

29:                                               ; preds = %23
  %30 = trunc i64 %15 to i32
  %31 = getelementptr ptr, ptr %19, i32 %30
  %32 = getelementptr ptr, ptr %31, i32 -1
  %33 = load ptr, ptr %32, align 4, !tbaa !6
  store ptr %33, ptr %26, align 4, !tbaa !6
  %34 = add nsw i64 %15, -1
  store i64 %34, ptr %14, align 8, !tbaa !208
  br label %35

35:                                               ; preds = %20, %8, %29
  %36 = add nuw nsw i64 %9, 1
  %37 = icmp eq i64 %36, %3
  br i1 %37, label %7, label %8, !llvm.loop !230
}

; Function Attrs: nounwind optsize
define hidden void @fa_reactive_flush() local_unnamed_addr #3 {
  %1 = load i1, ptr @reactive_flushing, align 4
  %2 = load i64, ptr @reactive_pending_count, align 8
  %3 = icmp eq i64 %2, 0
  %4 = select i1 %1, i1 true, i1 %3
  br i1 %4, label %71, label %5

5:                                                ; preds = %0
  store i1 true, ptr @reactive_flushing, align 4
  %6 = icmp sgt i64 %2, 0
  br i1 %6, label %7, label %70

7:                                                ; preds = %5, %46
  %8 = phi i32 [ %10, %46 ], [ 0, %5 ]
  %9 = phi i64 [ %47, %46 ], [ %2, %5 ]
  %10 = add nuw nsw i32 %8, 1
  %11 = icmp eq i32 %8, 100
  br i1 %11, label %12, label %13

12:                                               ; preds = %7
  tail call fastcc void @reactive_stop(ptr noundef nonnull @.str.15) #57
  unreachable

13:                                               ; preds = %7
  %14 = trunc i64 %9 to i32
  %15 = shl i32 %14, 2
  %16 = tail call ptr @fa_alloc(i32 noundef %15, i32 noundef 4) #54
  %17 = load ptr, ptr @reactive_pending, align 4, !tbaa !6
  tail call fastcc void @copy_bytes(ptr noundef %16, ptr noundef %17, i32 noundef %15) #53
  store i64 0, ptr @reactive_pending_count, align 8, !tbaa !89
  %18 = icmp ugt i64 %9, 1
  br i1 %18, label %20, label %19

19:                                               ; preds = %40, %13
  br label %49

20:                                               ; preds = %13, %40
  %21 = phi i64 [ %44, %40 ], [ 1, %13 ]
  %22 = trunc i64 %21 to i32
  %23 = getelementptr inbounds ptr, ptr %16, i32 %22
  %24 = load ptr, ptr %23, align 4, !tbaa !6
  %25 = getelementptr inbounds %struct.FaObserver, ptr %24, i32 0, i32 10
  %26 = load i64, ptr %25, align 8, !tbaa !227
  br label %27

27:                                               ; preds = %20, %36
  %28 = phi i64 [ %21, %20 ], [ %29, %36 ]
  %29 = add nsw i64 %28, -1
  %30 = trunc i64 %29 to i32
  %31 = getelementptr inbounds ptr, ptr %16, i32 %30
  %32 = load ptr, ptr %31, align 4, !tbaa !6
  %33 = getelementptr inbounds %struct.FaObserver, ptr %32, i32 0, i32 10
  %34 = load i64, ptr %33, align 8, !tbaa !227
  %35 = icmp sgt i64 %34, %26
  br i1 %35, label %36, label %40

36:                                               ; preds = %27
  %37 = trunc i64 %28 to i32
  %38 = getelementptr inbounds ptr, ptr %16, i32 %37
  store ptr %32, ptr %38, align 4, !tbaa !6
  %39 = icmp sgt i64 %28, 1
  br i1 %39, label %27, label %40, !llvm.loop !231

40:                                               ; preds = %36, %27
  %41 = phi i64 [ 0, %36 ], [ %28, %27 ]
  %42 = trunc i64 %41 to i32
  %43 = getelementptr inbounds ptr, ptr %16, i32 %42
  store ptr %24, ptr %43, align 4, !tbaa !6
  %44 = add nuw nsw i64 %21, 1
  %45 = icmp eq i64 %44, %9
  br i1 %45, label %19, label %20, !llvm.loop !232

46:                                               ; preds = %67
  tail call void @fa_free(ptr noundef nonnull %16, i32 noundef %15, i32 noundef 4) #54
  %47 = load i64, ptr @reactive_pending_count, align 8, !tbaa !89
  %48 = icmp sgt i64 %47, 0
  br i1 %48, label %7, label %70, !llvm.loop !233

49:                                               ; preds = %19, %67
  %50 = phi i64 [ %68, %67 ], [ 0, %19 ]
  %51 = trunc i64 %50 to i32
  %52 = getelementptr inbounds ptr, ptr %16, i32 %51
  %53 = load ptr, ptr %52, align 4, !tbaa !6
  %54 = getelementptr inbounds %struct.FaObserver, ptr %53, i32 0, i32 14
  store i32 0, ptr %54, align 4, !tbaa !234
  %55 = getelementptr inbounds %struct.FaObserver, ptr %53, i32 0, i32 15
  %56 = load i32, ptr %55, align 8, !tbaa !228
  %57 = icmp eq i32 %56, 0
  br i1 %57, label %58, label %67

58:                                               ; preds = %49
  %59 = tail call fastcc i32 @outdated(ptr noundef nonnull %53) #53
  %60 = icmp eq i32 %59, 0
  br i1 %60, label %67, label %61

61:                                               ; preds = %58
  tail call fastcc void @unsubscribe(ptr noundef nonnull %53) #53
  %62 = load ptr, ptr @reactive_observer, align 4, !tbaa !6
  store ptr null, ptr @reactive_observer, align 4, !tbaa !6
  %63 = getelementptr inbounds %struct.FaObserver, ptr %53, i32 0, i32 4
  %64 = load ptr, ptr %63, align 4, !tbaa !217
  tail call void %64(ptr noundef nonnull %53) #54
  store ptr %53, ptr @reactive_observer, align 4, !tbaa !6
  %65 = getelementptr inbounds %struct.FaObserver, ptr %53, i32 0, i32 3
  %66 = load ptr, ptr %65, align 8, !tbaa !216
  tail call void %66(ptr noundef nonnull %53) #54
  store ptr %62, ptr @reactive_observer, align 4, !tbaa !6
  br label %67

67:                                               ; preds = %61, %58, %49
  %68 = add nuw i64 %50, 1
  %69 = icmp eq i64 %68, %9
  br i1 %69, label %46, label %49, !llvm.loop !235

70:                                               ; preds = %46, %5
  store i1 false, ptr @reactive_flushing, align 4
  br label %71

71:                                               ; preds = %0, %70
  ret void
}

; Function Attrs: noreturn nounwind optsize
define internal fastcc void @reactive_stop(ptr noundef %0) unnamed_addr #5 {
  tail call void @fa_flush() #53
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.36, i32 noundef 6) #54
  br label %2

2:                                                ; preds = %2, %1
  %3 = phi i32 [ 0, %1 ], [ %7, %2 ]
  %4 = getelementptr inbounds i8, ptr %0, i32 %3
  %5 = load i8, ptr %4, align 1, !tbaa !1
  %6 = icmp eq i8 %5, 0
  %7 = add i32 %3, 1
  br i1 %6, label %8, label %2, !llvm.loop !27

8:                                                ; preds = %2
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull %0, i32 noundef %3) #54
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.22, i32 noundef 1) #54
  tail call void @fa_exit(i32 noundef 101) #55
  unreachable
}

; Function Attrs: nounwind optsize
define internal fastcc noundef i32 @outdated(ptr nocapture noundef readonly %0) unnamed_addr #3 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 8
  %3 = load i64, ptr %2, align 8, !tbaa !201
  %4 = icmp sgt i64 %3, 0
  br i1 %4, label %5, label %30

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 6
  %7 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 7
  br label %12

8:                                                ; preds = %21
  %9 = add nuw nsw i64 %13, 1
  %10 = load i64, ptr %2, align 8, !tbaa !201
  %11 = icmp slt i64 %9, %10
  br i1 %11, label %12, label %28, !llvm.loop !236

12:                                               ; preds = %5, %8
  %13 = phi i64 [ 0, %5 ], [ %9, %8 ]
  %14 = load ptr, ptr %6, align 4, !tbaa !203
  %15 = trunc i64 %13 to i32
  %16 = getelementptr inbounds ptr, ptr %14, i32 %15
  %17 = load ptr, ptr %16, align 4, !tbaa !6
  %18 = load i64, ptr %17, align 8, !tbaa !237
  %19 = icmp eq i64 %18, 1
  br i1 %19, label %20, label %21

20:                                               ; preds = %12
  tail call fastcc void @refresh(ptr noundef nonnull %17) #53
  br label %21

21:                                               ; preds = %20, %12
  %22 = getelementptr inbounds %struct.FaSource, ptr %17, i32 0, i32 1
  %23 = load i64, ptr %22, align 8, !tbaa !207
  %24 = load ptr, ptr %7, align 8, !tbaa !206
  %25 = getelementptr inbounds i64, ptr %24, i32 %15
  %26 = load i64, ptr %25, align 8, !tbaa !89
  %27 = icmp ne i64 %23, %26
  br i1 %27, label %28, label %8

28:                                               ; preds = %8, %21
  %29 = zext i1 %27 to i32
  br label %30

30:                                               ; preds = %28, %1
  %31 = phi i32 [ 0, %1 ], [ %29, %28 ]
  ret i32 %31
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read)
define hidden ptr @fa_observer_value(ptr nocapture noundef readonly %0) local_unnamed_addr #37 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 1
  %3 = load ptr, ptr %2, align 8, !tbaa !220
  ret ptr %3
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read)
define hidden ptr @fa_observer_env(ptr nocapture noundef readonly %0) local_unnamed_addr #37 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 2
  %3 = load ptr, ptr %2, align 4, !tbaa !219
  ret ptr %3
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(none)
define hidden nonnull ptr @fa_observer_cleanup(ptr noundef readnone %0) local_unnamed_addr #22 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 20
  ret ptr %2
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read)
define hidden i32 @fa_observer_computed(ptr nocapture noundef readonly %0) local_unnamed_addr #37 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 12
  %3 = load i32, ptr %2, align 4, !tbaa !222
  ret i32 %3
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: write)
define hidden void @fa_observer_changed(ptr nocapture noundef writeonly %0, i32 noundef %1) local_unnamed_addr #23 {
  %3 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 16
  store i32 %1, ptr %3, align 4, !tbaa !224
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_reactive_release() local_unnamed_addr #3 {
  %1 = load i64, ptr @reactive_made_count, align 8, !tbaa !89
  %2 = icmp sgt i64 %1, 0
  br i1 %2, label %25, label %3

3:                                                ; preds = %78, %0
  %4 = load i64, ptr @reactive_made_capacity, align 8, !tbaa !89
  %5 = icmp sgt i64 %4, 0
  br i1 %5, label %6, label %10

6:                                                ; preds = %3
  %7 = load ptr, ptr @reactive_made, align 4, !tbaa !6
  %8 = trunc i64 %4 to i32
  %9 = shl i32 %8, 2
  tail call void @fa_free(ptr noundef %7, i32 noundef %9, i32 noundef 4) #54
  br label %10

10:                                               ; preds = %3, %6
  %11 = load i64, ptr @reactive_pending_capacity, align 8, !tbaa !89
  %12 = icmp sgt i64 %11, 0
  br i1 %12, label %13, label %17

13:                                               ; preds = %10
  %14 = load ptr, ptr @reactive_pending, align 4, !tbaa !6
  %15 = trunc i64 %11 to i32
  %16 = shl i32 %15, 2
  tail call void @fa_free(ptr noundef %14, i32 noundef %16, i32 noundef 4) #54
  br label %17

17:                                               ; preds = %10, %13
  %18 = load i64, ptr @field_source_capacity, align 8, !tbaa !89
  %19 = icmp sgt i64 %18, 0
  br i1 %19, label %20, label %24

20:                                               ; preds = %17
  %21 = load ptr, ptr @field_sources, align 4, !tbaa !6
  %22 = trunc i64 %18 to i32
  %23 = mul i32 %22, 24
  tail call void @fa_free(ptr noundef %21, i32 noundef %23, i32 noundef 4) #54
  br label %24

24:                                               ; preds = %17, %20
  store i64 0, ptr @reactive_made_count, align 8, !tbaa !89
  ret void

25:                                               ; preds = %0, %78
  %26 = phi i64 [ %27, %78 ], [ %1, %0 ]
  %27 = add nsw i64 %26, -1
  %28 = load ptr, ptr @reactive_made, align 4, !tbaa !6
  %29 = trunc i64 %27 to i32
  %30 = getelementptr inbounds ptr, ptr %28, i32 %29
  %31 = load ptr, ptr %30, align 4, !tbaa !6
  %32 = getelementptr inbounds %struct.FaSource, ptr %31, i32 0, i32 4
  %33 = load i64, ptr %32, align 8, !tbaa !238
  %34 = icmp sgt i64 %33, 0
  br i1 %34, label %35, label %40

35:                                               ; preds = %25
  %36 = getelementptr inbounds %struct.FaSource, ptr %31, i32 0, i32 2
  %37 = load ptr, ptr %36, align 8, !tbaa !209
  %38 = trunc i64 %33 to i32
  %39 = shl i32 %38, 2
  tail call void @fa_free(ptr noundef %37, i32 noundef %39, i32 noundef 4) #54
  br label %40

40:                                               ; preds = %25, %35
  %41 = getelementptr inbounds %struct.FaSource, ptr %31, i32 0, i32 8
  %42 = load i64, ptr %41, align 8, !tbaa !239
  %43 = icmp sgt i64 %42, 0
  br i1 %43, label %44, label %49

44:                                               ; preds = %40
  %45 = getelementptr inbounds %struct.FaSource, ptr %31, i32 0, i32 6
  %46 = load ptr, ptr %45, align 4, !tbaa !190
  %47 = trunc i64 %42 to i32
  %48 = mul i32 %47, 24
  tail call void @fa_free(ptr noundef %46, i32 noundef %48, i32 noundef 4) #54
  br label %49

49:                                               ; preds = %40, %44
  %50 = load i64, ptr %31, align 8, !tbaa !237
  %51 = icmp eq i64 %50, 0
  br i1 %51, label %78, label %52

52:                                               ; preds = %49
  %53 = getelementptr inbounds %struct.FaObserver, ptr %31, i32 0, i32 5
  %54 = load ptr, ptr %53, align 8, !tbaa !218
  tail call void %54(ptr noundef nonnull %31) #54
  %55 = getelementptr inbounds %struct.FaObserver, ptr %31, i32 0, i32 9
  %56 = load i64, ptr %55, align 8, !tbaa !205
  %57 = icmp sgt i64 %56, 0
  br i1 %57, label %58, label %70

58:                                               ; preds = %52
  %59 = getelementptr inbounds %struct.FaObserver, ptr %31, i32 0, i32 6
  %60 = load ptr, ptr %59, align 4, !tbaa !203
  %61 = trunc i64 %56 to i32
  %62 = shl i32 %61, 2
  tail call void @fa_free(ptr noundef %60, i32 noundef %62, i32 noundef 4) #54
  %63 = load i64, ptr %55, align 8, !tbaa !205
  %64 = icmp sgt i64 %63, 0
  br i1 %64, label %65, label %70

65:                                               ; preds = %58
  %66 = getelementptr inbounds %struct.FaObserver, ptr %31, i32 0, i32 7
  %67 = load ptr, ptr %66, align 8, !tbaa !206
  %68 = trunc i64 %63 to i32
  %69 = shl i32 %68, 3
  tail call void @fa_free(ptr noundef %67, i32 noundef %69, i32 noundef 4) #54
  br label %70

70:                                               ; preds = %52, %58, %65
  %71 = getelementptr inbounds %struct.FaObserver, ptr %31, i32 0, i32 19
  %72 = load i64, ptr %71, align 8, !tbaa !221
  %73 = icmp sgt i64 %72, 0
  br i1 %73, label %74, label %78

74:                                               ; preds = %70
  %75 = getelementptr inbounds %struct.FaObserver, ptr %31, i32 0, i32 1
  %76 = load ptr, ptr %75, align 8, !tbaa !220
  %77 = trunc i64 %72 to i32
  tail call void @fa_free(ptr noundef %76, i32 noundef %77, i32 noundef 16) #54
  br label %78

78:                                               ; preds = %70, %74, %49
  %79 = phi i32 [ 64, %49 ], [ 168, %74 ], [ 168, %70 ]
  tail call void @fa_free(ptr noundef nonnull %31, i32 noundef %79, i32 noundef 8) #54
  %80 = icmp ugt i64 %26, 1
  br i1 %80, label %25, label %3, !llvm.loop !240
}

; Function Attrs: cold noinline noreturn nounwind optsize
define hidden void @fa_overflow(i64 noundef %0, i64 noundef %1, i32 noundef %2, ptr noundef %3) local_unnamed_addr #38 {
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  tail call fastcc void @append_c(ptr noundef nonnull @.str.16) #53
  %5 = icmp eq i32 %2, 0
  %6 = zext i64 %1 to i128
  %7 = shl nuw i128 %6, 64
  %8 = zext i64 %0 to i128
  %9 = or disjoint i128 %7, %8
  br i1 %5, label %16, label %10

10:                                               ; preds = %4
  %11 = icmp slt i64 %1, 0
  %12 = sub i128 0, %9
  %13 = select i1 %11, i128 %12, i128 %9
  %14 = lshr i64 %1, 63
  %15 = trunc i64 %14 to i32
  tail call fastcc void @append_u128(i128 noundef %13, i32 noundef %15) #53
  br label %17

16:                                               ; preds = %4
  tail call fastcc void @append_u128(i128 noundef %9, i32 noundef 0) #53
  br label %17

17:                                               ; preds = %16, %10
  tail call fastcc void @append_c(ptr noundef nonnull @.str.17) #53
  tail call fastcc void @append_c(ptr noundef %3) #53
  br label %18

18:                                               ; preds = %18, %17
  %19 = phi i32 [ 0, %17 ], [ %23, %18 ]
  %20 = getelementptr inbounds i8, ptr %3, i32 %19
  %21 = load i8, ptr %20, align 1, !tbaa !1
  %22 = icmp eq i8 %21, 0
  %23 = add i32 %19, 1
  br i1 %22, label %24, label %18, !llvm.loop !27

24:                                               ; preds = %18
  %25 = getelementptr inbounds i8, ptr %20, i32 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %25) #57
  unreachable
}

; Function Attrs: noreturn nounwind optsize
define internal fastcc void @stop_with_builder(ptr noundef %0) unnamed_addr #5 {
  tail call void @fa_flush() #53
  %2 = load ptr, ptr @builder, align 4, !tbaa !6
  %3 = load i32, ptr @builder_length, align 4, !tbaa !4
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.36, i32 noundef 6) #54
  tail call void @fa_write(i32 noundef 2, ptr noundef %2, i32 noundef %3) #54
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.37, i32 noundef 6) #54
  br label %4

4:                                                ; preds = %4, %1
  %5 = phi i32 [ 0, %1 ], [ %9, %4 ]
  %6 = getelementptr inbounds i8, ptr %0, i32 %5
  %7 = load i8, ptr %6, align 1, !tbaa !1
  %8 = icmp eq i8 %7, 0
  %9 = add i32 %5, 1
  br i1 %8, label %10, label %4, !llvm.loop !27

10:                                               ; preds = %4
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull %0, i32 noundef %5) #54
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.22, i32 noundef 1) #54
  tail call void @fa_exit(i32 noundef 101) #55
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define hidden void @fa_overflow_unknown(ptr noundef %0) local_unnamed_addr #38 {
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  tail call fastcc void @append_c(ptr noundef nonnull @.str.18) #53
  tail call fastcc void @append_c(ptr noundef %0) #53
  br label %2

2:                                                ; preds = %2, %1
  %3 = phi i32 [ 0, %1 ], [ %7, %2 ]
  %4 = getelementptr inbounds i8, ptr %0, i32 %3
  %5 = load i8, ptr %4, align 1, !tbaa !1
  %6 = icmp eq i8 %5, 0
  %7 = add i32 %3, 1
  br i1 %6, label %8, label %2, !llvm.loop !27

8:                                                ; preds = %2
  %9 = getelementptr inbounds i8, ptr %4, i32 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %9) #57
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define hidden void @fa_float_overflow(double noundef %0, ptr noundef %1) local_unnamed_addr #38 {
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  tail call fastcc void @append_c(ptr noundef nonnull @.str.16) #53
  tail call void @fa_b_f64(double noundef %0) #53
  tail call fastcc void @append_c(ptr noundef nonnull @.str.17) #53
  tail call fastcc void @append_c(ptr noundef %1) #53
  br label %3

3:                                                ; preds = %3, %2
  %4 = phi i32 [ 0, %2 ], [ %8, %3 ]
  %5 = getelementptr inbounds i8, ptr %1, i32 %4
  %6 = load i8, ptr %5, align 1, !tbaa !1
  %7 = icmp eq i8 %6, 0
  %8 = add i32 %4, 1
  br i1 %7, label %9, label %3, !llvm.loop !27

9:                                                ; preds = %3
  %10 = getelementptr inbounds i8, ptr %5, i32 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %10) #57
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define hidden void @fa_assert_failed(ptr noundef %0) local_unnamed_addr #38 {
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  tail call fastcc void @append_c(ptr noundef nonnull @.str.19) #53
  tail call fastcc void @append_c(ptr noundef %0) #53
  br label %2

2:                                                ; preds = %2, %1
  %3 = phi i32 [ 0, %1 ], [ %7, %2 ]
  %4 = getelementptr inbounds i8, ptr %0, i32 %3
  %5 = load i8, ptr %4, align 1, !tbaa !1
  %6 = icmp eq i8 %5, 0
  %7 = add i32 %3, 1
  br i1 %6, label %8, label %2, !llvm.loop !27

8:                                                ; preds = %2
  %9 = getelementptr inbounds i8, ptr %4, i32 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %9) #57
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define hidden void @fa_assert_failed_operands(ptr noundef %0) local_unnamed_addr #38 {
  %2 = alloca %struct.FaString, align 4
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %2) #56
  call void @fa_b_take(ptr noundef nonnull %2) #53
  tail call fastcc void @append_c(ptr noundef nonnull @.str.19) #53
  tail call fastcc void @append_c(ptr noundef %0) #53
  tail call fastcc void @append_c(ptr noundef nonnull @.str.20) #53
  %3 = getelementptr inbounds %struct.FaString, ptr %2, i32 0, i32 2
  %4 = load i32, ptr %3, align 4, !tbaa !19
  %5 = and i32 %4, 1073741824
  %6 = icmp eq i32 %5, 0
  %7 = lshr i32 %4, 24
  %8 = and i32 %7, 31
  %9 = load ptr, ptr %2, align 4
  %10 = getelementptr inbounds %struct.FaString, ptr %2, i32 0, i32 1
  %11 = load i32, ptr %10, align 4
  %12 = select i1 %6, ptr %9, ptr %2
  %13 = select i1 %6, i32 %11, i32 %8
  call void @fa_b_bytes(ptr noundef %12, i32 noundef %13) #53
  br label %14

14:                                               ; preds = %14, %1
  %15 = phi i32 [ 0, %1 ], [ %19, %14 ]
  %16 = getelementptr inbounds i8, ptr %0, i32 %15
  %17 = load i8, ptr %16, align 1, !tbaa !1
  %18 = icmp eq i8 %17, 0
  %19 = add i32 %15, 1
  br i1 %18, label %20, label %14, !llvm.loop !27

20:                                               ; preds = %14
  %21 = getelementptr inbounds i8, ptr %16, i32 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %21) #57
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define hidden void @fa_uncaught(ptr noundef %0) local_unnamed_addr #38 {
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  tail call void @fa_format_union(ptr noundef %0, i32 noundef 1) #54
  tail call void @fa_flush() #53
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.21, i32 noundef 7) #54
  %2 = load ptr, ptr @builder, align 4, !tbaa !6
  %3 = load i32, ptr @builder_length, align 4, !tbaa !4
  tail call void @fa_write(i32 noundef 2, ptr noundef %2, i32 noundef %3) #54
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.22, i32 noundef 1) #54
  tail call void @fa_exit(i32 noundef 1) #55
  unreachable
}

; Function Attrs: optsize
declare void @fa_format_union(ptr noundef, i32 noundef) local_unnamed_addr #4

; Function Attrs: cold mustprogress nofree noinline norecurse nosync nounwind optsize willreturn memory(none)
define hidden noundef nonnull ptr @fa_error_division_by_zero() local_unnamed_addr #39 {
  ret ptr @division_by_zero
}

; Function Attrs: cold noinline nounwind optsize
define hidden noundef ptr @fa_error_conversion(i64 noundef %0, i64 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3) local_unnamed_addr #21 {
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  %5 = icmp eq i32 %2, 0
  %6 = zext i64 %1 to i128
  %7 = shl nuw i128 %6, 64
  %8 = zext i64 %0 to i128
  %9 = or disjoint i128 %7, %8
  br i1 %5, label %16, label %10

10:                                               ; preds = %4
  %11 = icmp slt i64 %1, 0
  %12 = sub i128 0, %9
  %13 = select i1 %11, i128 %12, i128 %9
  %14 = lshr i64 %1, 63
  %15 = trunc i64 %14 to i32
  tail call fastcc void @append_u128(i128 noundef %13, i32 noundef %15) #53
  br label %17

16:                                               ; preds = %4
  tail call fastcc void @append_u128(i128 noundef %9, i32 noundef 0) #53
  br label %17

17:                                               ; preds = %16, %10
  tail call fastcc void @append_c(ptr noundef nonnull @.str.17) #53
  tail call fastcc void @append_c(ptr noundef %3) #53
  %18 = tail call fastcc ptr @arithmetic_error_from_builder() #53
  ret ptr %18
}

; Function Attrs: nounwind optsize
define internal fastcc noundef ptr @arithmetic_error_from_builder() unnamed_addr #3 {
  %1 = alloca %struct.FaString, align 4
  %2 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #54
  store i64 4, ptr %2, align 8, !tbaa !241
  %3 = getelementptr inbounds %struct.MessageError, ptr %2, i32 0, i32 1
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %1) #56
  %4 = load ptr, ptr @builder, align 4, !tbaa !6
  %5 = load i32, ptr @builder_length, align 4, !tbaa !4
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %1, ptr noundef %4, i32 noundef %5, i32 noundef 0) #53
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %3, ptr noundef nonnull align 4 dereferenceable(12) %1, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %1) #56
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  ret ptr %2
}

; Function Attrs: cold noinline nounwind optsize
define hidden noundef ptr @fa_error_negative_exponent(i64 noundef %0) local_unnamed_addr #21 {
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  tail call fastcc void @append_c(ptr noundef nonnull @.str.23) #53
  %2 = tail call i64 @llvm.abs.i64(i64 %0, i1 false)
  %3 = lshr i64 %0, 63
  %4 = trunc i64 %3 to i32
  tail call fastcc void @append_u64(i64 noundef %2, i32 noundef %4) #53
  %5 = tail call fastcc ptr @arithmetic_error_from_builder() #53
  ret ptr %5
}

; Function Attrs: cold noinline nounwind optsize
define hidden noundef ptr @fa_error_float(double noundef %0, i32 noundef %1) local_unnamed_addr #21 {
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  tail call void @fa_b_f64(double noundef %0) #53
  %3 = icmp eq i32 %1, 0
  %4 = select i1 %3, ptr @.str.25, ptr @.str.24
  tail call fastcc void @append_c(ptr noundef nonnull %4) #53
  %5 = tail call fastcc ptr @arithmetic_error_from_builder() #53
  ret ptr %5
}

; Function Attrs: cold noinline nounwind optsize
define hidden noundef ptr @fa_error_shift(i64 noundef %0, i64 noundef %1) local_unnamed_addr #21 {
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  tail call fastcc void @append_c(ptr noundef nonnull @.str.26) #53
  %3 = tail call i64 @llvm.abs.i64(i64 %0, i1 false)
  %4 = lshr i64 %0, 63
  %5 = trunc i64 %4 to i32
  tail call fastcc void @append_u64(i64 noundef %3, i32 noundef %5) #53
  tail call fastcc void @append_c(ptr noundef nonnull @.str.27) #53
  %6 = tail call i64 @llvm.abs.i64(i64 %1, i1 false)
  %7 = lshr i64 %1, 63
  %8 = trunc i64 %7 to i32
  tail call fastcc void @append_u64(i64 noundef %6, i32 noundef %8) #53
  %9 = tail call fastcc ptr @arithmetic_error_from_builder() #53
  ret ptr %9
}

; Function Attrs: cold noinline nounwind optsize
define hidden noundef ptr @fa_error_checked(i64 noundef %0, i64 noundef %1, ptr nocapture noundef readonly %2) local_unnamed_addr #21 {
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  tail call fastcc void @append_c(ptr noundef nonnull @.str.16) #53
  %4 = zext i64 %1 to i128
  %5 = shl nuw i128 %4, 64
  %6 = zext i64 %0 to i128
  %7 = or disjoint i128 %5, %6
  %8 = icmp slt i64 %1, 0
  %9 = sub i128 0, %7
  %10 = select i1 %8, i128 %9, i128 %7
  %11 = lshr i64 %1, 63
  %12 = trunc i64 %11 to i32
  tail call fastcc void @append_u128(i128 noundef %10, i32 noundef %12) #53
  tail call fastcc void @append_c(ptr noundef nonnull @.str.17) #53
  tail call fastcc void @append_c(ptr noundef %2) #53
  %13 = tail call fastcc ptr @arithmetic_error_from_builder() #53
  ret ptr %13
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_parse_integer(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef writeonly %2) local_unnamed_addr #3 {
  %4 = icmp eq i32 %1, 0
  br i1 %4, label %13, label %5

5:                                                ; preds = %3
  %6 = load i8, ptr %0, align 1, !tbaa !1
  switch i8 %6, label %9 [
    i8 43, label %7
    i8 45, label %7
  ]

7:                                                ; preds = %5, %5
  %8 = icmp ne i8 %6, 45
  br label %9

9:                                                ; preds = %5, %7
  %10 = phi i1 [ %8, %7 ], [ true, %5 ]
  %11 = phi i32 [ 1, %7 ], [ 0, %5 ]
  %12 = icmp ult i32 %11, %1
  br i1 %12, label %22, label %32

13:                                               ; preds = %3
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  br label %54

14:                                               ; preds = %22
  %15 = icmp eq i32 %29, 0
  br i1 %15, label %16, label %17

16:                                               ; preds = %14
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  br label %33

17:                                               ; preds = %14
  br i1 %12, label %18, label %75

18:                                               ; preds = %17
  %19 = getelementptr inbounds i8, ptr %0, i32 %11
  %20 = load i8, ptr %19, align 1, !tbaa !1
  %21 = zext i8 %20 to i64
  br label %64

22:                                               ; preds = %9, %22
  %23 = phi i32 [ %30, %22 ], [ %11, %9 ]
  %24 = phi i32 [ %29, %22 ], [ 1, %9 ]
  %25 = getelementptr inbounds i8, ptr %0, i32 %23
  %26 = load i8, ptr %25, align 1, !tbaa !1
  %27 = add i8 %26, -48
  %28 = icmp ult i8 %27, 10
  %29 = select i1 %28, i32 %24, i32 0
  %30 = add nuw i32 %23, 1
  %31 = icmp eq i32 %30, %1
  br i1 %31, label %14, label %22, !llvm.loop !243

32:                                               ; preds = %9
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  br i1 %4, label %54, label %33

33:                                               ; preds = %16, %32
  tail call fastcc void @reserve(i32 noundef 1) #53
  %34 = load ptr, ptr @builder, align 4, !tbaa !6
  %35 = load i32, ptr @builder_length, align 4, !tbaa !4
  %36 = getelementptr inbounds i8, ptr %34, i32 %35
  store i8 34, ptr %36, align 1, !tbaa !1
  %37 = add i32 %35, 1
  store i32 %37, ptr @builder_length, align 4, !tbaa !4
  br label %38

38:                                               ; preds = %33, %47
  %39 = phi i32 [ 0, %33 ], [ %52, %47 ]
  %40 = getelementptr inbounds i8, ptr %0, i32 %39
  %41 = load i8, ptr %40, align 1, !tbaa !1
  switch i8 %41, label %47 [
    i8 92, label %42
    i8 34, label %42
  ]

42:                                               ; preds = %38, %38
  tail call fastcc void @reserve(i32 noundef 1) #53
  %43 = load ptr, ptr @builder, align 4, !tbaa !6
  %44 = load i32, ptr @builder_length, align 4, !tbaa !4
  %45 = getelementptr inbounds i8, ptr %43, i32 %44
  store i8 92, ptr %45, align 1, !tbaa !1
  %46 = add i32 %44, 1
  store i32 %46, ptr @builder_length, align 4, !tbaa !4
  br label %47

47:                                               ; preds = %38, %42
  tail call fastcc void @reserve(i32 noundef 1) #53
  %48 = load ptr, ptr @builder, align 4, !tbaa !6
  %49 = load i32, ptr @builder_length, align 4, !tbaa !4
  %50 = getelementptr inbounds i8, ptr %48, i32 %49
  store i8 %41, ptr %50, align 1, !tbaa !1
  %51 = add i32 %49, 1
  store i32 %51, ptr @builder_length, align 4, !tbaa !4
  %52 = add nuw i32 %39, 1
  %53 = icmp eq i32 %52, %1
  br i1 %53, label %54, label %38, !llvm.loop !244

54:                                               ; preds = %47, %32, %13
  %55 = phi ptr [ @.str.28, %13 ], [ @.str.28, %32 ], [ @.str.29, %47 ]
  tail call fastcc void @append_c(ptr noundef nonnull %55) #53
  %56 = tail call fastcc ptr @parse_error(ptr noundef %0, i32 noundef %1) #53
  br label %90

57:                                               ; preds = %64
  %58 = getelementptr inbounds i8, ptr %0, i32 %71
  %59 = load i8, ptr %58, align 1, !tbaa !1
  %60 = zext i8 %59 to i64
  %61 = sub nsw i64 47, %60
  %62 = udiv i64 %61, 10
  %63 = icmp ugt i64 %70, %62
  br i1 %63, label %73, label %64, !llvm.loop !245

64:                                               ; preds = %18, %57
  %65 = phi i64 [ %21, %18 ], [ %60, %57 ]
  %66 = phi i64 [ 0, %18 ], [ %70, %57 ]
  %67 = phi i32 [ %11, %18 ], [ %71, %57 ]
  %68 = mul nuw i64 %66, 10
  %69 = add i64 %68, -48
  %70 = add i64 %69, %65
  %71 = add nuw i32 %67, 1
  %72 = icmp eq i32 %71, %1
  br i1 %72, label %73, label %57, !llvm.loop !245

73:                                               ; preds = %57, %64
  %74 = icmp ult i32 %71, %1
  br label %75

75:                                               ; preds = %73, %17
  %76 = phi i64 [ 0, %17 ], [ %70, %73 ]
  %77 = phi i1 [ false, %17 ], [ %74, %73 ]
  %78 = select i1 %10, i64 9223372036854775807, i64 -9223372036854775808
  %79 = icmp ugt i64 %76, %78
  %80 = select i1 %77, i1 true, i1 %79
  br i1 %80, label %81, label %87

81:                                               ; preds = %75
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  tail call fastcc void @reserve(i32 noundef %1) #53
  %82 = load ptr, ptr @builder, align 4, !tbaa !6
  %83 = load i32, ptr @builder_length, align 4, !tbaa !4
  %84 = getelementptr inbounds i8, ptr %82, i32 %83
  tail call fastcc void @copy_bytes(ptr noundef %84, ptr noundef nonnull %0, i32 noundef %1) #53
  %85 = add i32 %83, %1
  store i32 %85, ptr @builder_length, align 4, !tbaa !4
  tail call fastcc void @append_c(ptr noundef nonnull @.str.24) #53
  %86 = tail call fastcc ptr @parse_error(ptr noundef nonnull %0, i32 noundef %1) #53
  br label %90

87:                                               ; preds = %75
  %88 = sub i64 0, %76
  %89 = select i1 %10, i64 %76, i64 %88
  store i64 %89, ptr %2, align 8, !tbaa !89
  br label %90

90:                                               ; preds = %81, %87, %54
  %91 = phi ptr [ %56, %54 ], [ %86, %81 ], [ null, %87 ]
  ret ptr %91
}

; Function Attrs: nounwind optsize
define internal fastcc noundef ptr @parse_error(ptr nocapture noundef readonly %0, i32 noundef %1) unnamed_addr #3 {
  %3 = alloca %struct.FaString, align 4
  %4 = alloca %struct.FaString, align 4
  %5 = tail call ptr @fa_alloc(i32 noundef 32, i32 noundef 8) #54
  store i64 5, ptr %5, align 8, !tbaa !246
  %6 = getelementptr inbounds %struct.ParseError, ptr %5, i32 0, i32 1
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %4) #56
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %4, ptr noundef %0, i32 noundef %1, i32 noundef 0) #53
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 8 dereferenceable(12) %6, ptr noundef nonnull align 4 dereferenceable(12) %4, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %4) #56
  %7 = getelementptr inbounds %struct.ParseError, ptr %5, i32 0, i32 2
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %3) #56
  %8 = load ptr, ptr @builder, align 4, !tbaa !6
  %9 = load i32, ptr @builder_length, align 4, !tbaa !4
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %3, ptr noundef %8, i32 noundef %9, i32 noundef 0) #53
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %7, ptr noundef nonnull align 4 dereferenceable(12) %3, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %3) #56
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  ret ptr %5
}

; Function Attrs: nounwind optsize
define hidden i64 @fa_pow_i64(i64 noundef %0, i64 noundef %1, i32 noundef %2, ptr noundef %3) local_unnamed_addr #3 {
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
  tail call void @fa_overflow_unknown(ptr noundef %3) #59
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
  tail call void @fa_overflow_unknown(ptr noundef %3) #59
  unreachable
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare { i64, i1 } @llvm.smul.with.overflow.i64(i64, i64) #32

; Function Attrs: nounwind optsize
define weak hidden double @pow(double noundef %0, double noundef %1) local_unnamed_addr #3 {
  %3 = tail call double @fa_host_pow(double noundef %0, double noundef %1) #54
  ret double %3
}

; Function Attrs: optsize
declare double @fa_host_pow(double noundef, double noundef) local_unnamed_addr #40

; Function Attrs: nounwind optsize
define weak hidden double @round(double noundef %0) local_unnamed_addr #3 {
  %2 = tail call double @llvm.trunc.f64(double %0)
  %3 = fsub double %0, %2
  %4 = fcmp ult double %3, 5.000000e-01
  br i1 %4, label %7, label %5

5:                                                ; preds = %1
  %6 = fadd double %2, 1.000000e+00
  br label %11

7:                                                ; preds = %1
  %8 = fcmp ugt double %3, -5.000000e-01
  br i1 %8, label %11, label %9

9:                                                ; preds = %7
  %10 = fadd double %2, -1.000000e+00
  br label %11

11:                                               ; preds = %7, %9, %5
  %12 = phi double [ %6, %5 ], [ %10, %9 ], [ %2, %7 ]
  ret double %12
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare double @llvm.trunc.f64(double) #32

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(read, inaccessiblemem: none)
define hidden ptr @fa_table_entry(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #41 {
  %4 = load ptr, ptr %0, align 8, !tbaa !248
  %5 = load ptr, ptr %4, align 8, !tbaa !69
  %6 = lshr i64 %1, 5
  %7 = trunc i64 %6 to i32
  %8 = getelementptr inbounds ptr, ptr %5, i32 %7
  %9 = load ptr, ptr %8, align 4, !tbaa !6
  %10 = and i64 %1, 31
  %11 = mul nsw i64 %10, %2
  %12 = trunc i64 %11 to i32
  %13 = getelementptr inbounds i8, ptr %9, i32 %12
  ret ptr %13
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_table_new(i64 noundef %0, i64 noundef %1) local_unnamed_addr #3 {
  %3 = tail call ptr @fa_alloc(i32 noundef 80, i32 noundef 8) #54
  %4 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 9
  store i64 0, ptr %4, align 8, !tbaa !250
  %5 = tail call i64 @llvm.smax.i64(i64 %0, i64 4)
  %6 = mul nsw i64 %5, %1
  %7 = trunc i64 %6 to i32
  %8 = tail call ptr @fa_alloc(i32 noundef %7, i32 noundef 16) #54
  store ptr %8, ptr %3, align 8, !tbaa !248
  %9 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 1
  store i64 0, ptr %9, align 8, !tbaa !251
  %10 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 2
  store i64 0, ptr %10, align 8, !tbaa !252
  %11 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 3
  store i64 %5, ptr %11, align 8, !tbaa !253
  %12 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 4
  store ptr null, ptr %12, align 8, !tbaa !254
  %13 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 6
  store i64 0, ptr %13, align 8, !tbaa !255
  %14 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 7
  store i64 0, ptr %14, align 8, !tbaa !256
  %15 = shl nuw nsw i64 %5, 1
  br label %16

16:                                               ; preds = %16, %2
  %17 = phi i64 [ 8, %2 ], [ %19, %16 ]
  %18 = icmp slt i64 %17, %15
  %19 = shl nsw i64 %17, 1
  br i1 %18, label %16, label %20, !llvm.loop !257

20:                                               ; preds = %16
  tail call fastcc void @table_index(ptr noundef nonnull %3, i64 noundef %17, i64 noundef %1) #53
  ret ptr %3
}

; Function Attrs: nounwind optsize
define internal fastcc void @table_index(ptr nocapture noundef %0, i64 noundef %1, i64 noundef %2) unnamed_addr #3 {
  %4 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %5 = load ptr, ptr %4, align 8, !tbaa !254
  %6 = icmp eq ptr %5, null
  br i1 %6, label %15, label %7

7:                                                ; preds = %3
  %8 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %9 = load i64, ptr %8, align 8, !tbaa !258
  %10 = trunc i64 %9 to i32
  %11 = add i32 %10, 1
  %12 = icmp ult i64 %9, 2147483648
  %13 = select i1 %12, i32 2, i32 3
  %14 = shl i32 %11, %13
  tail call void @fa_free(ptr noundef nonnull %5, i32 noundef %14, i32 noundef 8) #54
  br label %15

15:                                               ; preds = %7, %3
  %16 = trunc i64 %1 to i32
  %17 = add nsw i64 %1, -1
  %18 = icmp ult i64 %17, 2147483648
  %19 = select i1 %18, i32 2, i32 3
  %20 = shl i32 %16, %19
  %21 = tail call ptr @fa_alloc(i32 noundef %20, i32 noundef 8) #54
  store ptr %21, ptr %4, align 8, !tbaa !254
  tail call void @llvm.memset.p0.i32(ptr align 1 %21, i8 0, i32 %20, i1 false)
  %22 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  store i64 %17, ptr %22, align 8, !tbaa !258
  %23 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 1
  %24 = load i64, ptr %23, align 8, !tbaa !251
  %25 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 8
  store i64 %24, ptr %25, align 8, !tbaa !259
  %26 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 2
  %27 = load i64, ptr %26, align 8, !tbaa !252
  %28 = icmp sgt i64 %27, 0
  br i1 %28, label %29, label %32

29:                                               ; preds = %15
  %30 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 6
  %31 = load ptr, ptr %0, align 8, !tbaa !248
  br label %33

32:                                               ; preds = %81, %15
  ret void

33:                                               ; preds = %29, %81
  %34 = phi i64 [ %27, %29 ], [ %82, %81 ]
  %35 = phi i64 [ 0, %29 ], [ %83, %81 ]
  %36 = load i64, ptr %30, align 8, !tbaa !255
  %37 = icmp eq i64 %36, 0
  br i1 %37, label %45, label %38

38:                                               ; preds = %33
  %39 = load ptr, ptr %31, align 8, !tbaa !69
  %40 = lshr i64 %35, 5
  %41 = trunc i64 %40 to i32
  %42 = getelementptr inbounds ptr, ptr %39, i32 %41
  %43 = load ptr, ptr %42, align 4, !tbaa !6
  %44 = and i64 %35, 31
  br label %45

45:                                               ; preds = %33, %38
  %46 = phi i64 [ %44, %38 ], [ %35, %33 ]
  %47 = phi ptr [ %43, %38 ], [ %31, %33 ]
  %48 = mul nsw i64 %46, %2
  %49 = trunc i64 %48 to i32
  %50 = getelementptr inbounds i8, ptr %47, i32 %49
  %51 = getelementptr inbounds i8, ptr %50, i32 8
  %52 = load i64, ptr %51, align 8, !tbaa !89
  %53 = icmp eq i64 %52, 0
  br i1 %53, label %81, label %54

54:                                               ; preds = %45
  %55 = load i64, ptr %50, align 8, !tbaa !89
  %56 = load i64, ptr %22, align 8, !tbaa !258
  %57 = icmp ult i64 %56, 2147483648
  %58 = load ptr, ptr %4, align 8, !tbaa !254
  br label %59

59:                                               ; preds = %71, %54
  %60 = phi i64 [ %55, %54 ], [ %72, %71 ]
  %61 = and i64 %60, %56
  %62 = trunc i64 %61 to i32
  br i1 %57, label %63, label %67

63:                                               ; preds = %59
  %64 = getelementptr inbounds i32, ptr %58, i32 %62
  %65 = load i32, ptr %64, align 4, !tbaa !23
  %66 = icmp eq i32 %65, 0
  br i1 %66, label %73, label %71

67:                                               ; preds = %59
  %68 = getelementptr inbounds i64, ptr %58, i32 %62
  %69 = load i64, ptr %68, align 8, !tbaa !89
  %70 = icmp eq i64 %69, 0
  br i1 %70, label %76, label %71

71:                                               ; preds = %67, %63
  %72 = add nsw i64 %61, 1
  br label %59, !llvm.loop !260

73:                                               ; preds = %63
  %74 = trunc i64 %35 to i32
  %75 = add i32 %74, 1
  store i32 %75, ptr %64, align 4, !tbaa !23
  br label %78

76:                                               ; preds = %67
  %77 = add nuw nsw i64 %35, 1
  store i64 %77, ptr %68, align 8, !tbaa !89
  br label %78

78:                                               ; preds = %73, %76
  %79 = add nsw i64 %61, 1
  store i64 %79, ptr %51, align 8, !tbaa !89
  %80 = load i64, ptr %26, align 8, !tbaa !252
  br label %81

81:                                               ; preds = %78, %45
  %82 = phi i64 [ %80, %78 ], [ %34, %45 ]
  %83 = add nuw nsw i64 %35, 1
  %84 = icmp slt i64 %83, %82
  br i1 %84, label %33, label %32, !llvm.loop !261
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_table_new_growing(i64 noundef %0, i64 noundef %1) local_unnamed_addr #3 {
  %3 = tail call ptr @fa_alloc(i32 noundef 80, i32 noundef 8) #54
  %4 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 9
  store i64 0, ptr %4, align 8, !tbaa !250
  %5 = tail call i64 @llvm.smax.i64(i64 %0, i64 4)
  %6 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 4) #54
  store ptr null, ptr %6, align 8, !tbaa !69
  %7 = getelementptr inbounds %struct.FaTableEntries, ptr %6, i32 0, i32 1
  store i64 0, ptr %7, align 8, !tbaa !79
  %8 = getelementptr inbounds %struct.FaTableEntries, ptr %6, i32 0, i32 2
  store i64 0, ptr %8, align 8, !tbaa !73
  store ptr %6, ptr %3, align 8, !tbaa !248
  %9 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 1
  store i64 0, ptr %9, align 8, !tbaa !251
  %10 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 2
  store i64 0, ptr %10, align 8, !tbaa !252
  %11 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 3
  store i64 %5, ptr %11, align 8, !tbaa !253
  %12 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 4
  store ptr null, ptr %12, align 8, !tbaa !254
  %13 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 6
  store i64 1, ptr %13, align 8, !tbaa !255
  %14 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 7
  store i64 0, ptr %14, align 8, !tbaa !256
  %15 = shl nuw nsw i64 %5, 1
  br label %16

16:                                               ; preds = %16, %2
  %17 = phi i64 [ 8, %2 ], [ %19, %16 ]
  %18 = icmp slt i64 %17, %15
  %19 = shl nsw i64 %17, 1
  br i1 %18, label %16, label %20, !llvm.loop !257

20:                                               ; preds = %16
  tail call fastcc void @table_index(ptr noundef nonnull %3, i64 noundef %17, i64 noundef %1) #53
  ret ptr %3
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_table_new_map(i64 noundef %0, i64 noundef %1) local_unnamed_addr #3 {
  %3 = tail call ptr @fa_table_new_growing(i64 noundef %0, i64 noundef %1) #53
  %4 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 6
  store i64 2, ptr %4, align 8, !tbaa !255
  ret ptr %3
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(write, argmem: readwrite, inaccessiblemem: none)
define hidden noundef ptr @fa_table_local(ptr noundef returned %0, i64 noundef %1, i64 noundef %2, i32 noundef %3) local_unnamed_addr #42 {
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(80) %0, i8 0, i32 72, i1 false)
  %5 = tail call i64 @llvm.smax.i64(i64 %1, i64 4)
  %6 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 3
  store i64 %5, ptr %6, align 8, !tbaa !253
  %7 = sext i32 %3 to i64
  %8 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 6
  store i64 %7, ptr %8, align 8, !tbaa !255
  %9 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 9
  store i64 3, ptr %9, align 8, !tbaa !250
  %10 = getelementptr inbounds i8, ptr %0, i32 80
  %11 = icmp eq i32 %3, 0
  br i1 %11, label %37, label %12

12:                                               ; preds = %4
  %13 = getelementptr inbounds i8, ptr %0, i32 104
  %14 = add nuw i64 %5, 31
  %15 = lshr i64 %14, 5
  store ptr %13, ptr %10, align 8, !tbaa !69
  %16 = getelementptr inbounds i8, ptr %0, i32 88
  store i64 %15, ptr %16, align 8, !tbaa !79
  %17 = getelementptr inbounds i8, ptr %0, i32 96
  store i64 %15, ptr %17, align 8, !tbaa !73
  %18 = shl nuw nsw i64 %15, 2
  %19 = trunc i64 %18 to i32
  %20 = getelementptr inbounds i8, ptr %13, i32 %19
  %21 = ptrtoint ptr %20 to i32
  %22 = add i32 %21, 15
  %23 = and i32 %22, -16
  %24 = inttoptr i32 %23 to ptr
  %25 = trunc i64 %2 to i32
  %26 = shl i32 %25, 5
  br label %28

27:                                               ; preds = %28
  store ptr %10, ptr %0, align 8, !tbaa !248
  br label %45

28:                                               ; preds = %12, %28
  %29 = phi i64 [ 0, %12 ], [ %35, %28 ]
  %30 = phi ptr [ %24, %12 ], [ %34, %28 ]
  %31 = load ptr, ptr %10, align 8, !tbaa !69
  %32 = trunc i64 %29 to i32
  %33 = getelementptr inbounds ptr, ptr %31, i32 %32
  store ptr %30, ptr %33, align 4, !tbaa !6
  %34 = getelementptr inbounds i8, ptr %30, i32 %26
  %35 = add nuw nsw i64 %29, 1
  %36 = icmp eq i64 %35, %15
  br i1 %36, label %27, label %28, !llvm.loop !262

37:                                               ; preds = %4
  %38 = ptrtoint ptr %10 to i32
  %39 = add i32 %38, 15
  %40 = and i32 %39, -16
  %41 = inttoptr i32 %40 to ptr
  store ptr %41, ptr %0, align 8, !tbaa !248
  %42 = mul nsw i64 %5, %2
  %43 = trunc i64 %42 to i32
  %44 = getelementptr inbounds i8, ptr %41, i32 %43
  br label %45

45:                                               ; preds = %37, %27
  %46 = phi ptr [ %34, %27 ], [ %44, %37 ]
  %47 = ptrtoint ptr %46 to i32
  %48 = add i32 %47, 7
  %49 = and i32 %48, -8
  %50 = inttoptr i32 %49 to ptr
  %51 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  store ptr %50, ptr %51, align 8, !tbaa !254
  %52 = shl nuw nsw i64 %5, 1
  br label %53

53:                                               ; preds = %53, %45
  %54 = phi i64 [ 8, %45 ], [ %56, %53 ]
  %55 = icmp slt i64 %54, %52
  %56 = shl nsw i64 %54, 1
  br i1 %55, label %53, label %57, !llvm.loop !257

57:                                               ; preds = %53
  %58 = add nsw i64 %54, -1
  %59 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  store i64 %58, ptr %59, align 8, !tbaa !258
  %60 = trunc i64 %54 to i32
  %61 = icmp ult i64 %54, 2147483649
  %62 = select i1 %61, i32 2, i32 3
  %63 = shl i32 %60, %62
  tail call void @llvm.memset.p0.i32(ptr align 8 %50, i8 0, i32 %63, i1 false)
  ret ptr %0
}

; Function Attrs: alwaysinline nounwind optsize
define hidden i64 @fa_table_find(ptr nocapture noundef readonly %0, i64 noundef %1, ptr noundef %2, ptr noundef readonly %3, i64 noundef %4) local_unnamed_addr #13 {
  %6 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %7 = load i64, ptr %6, align 8, !tbaa !258
  %8 = icmp ult i64 %7, 2147483648
  %9 = and i64 %7, %1
  %10 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %11 = load ptr, ptr %10, align 8, !tbaa !254
  %12 = trunc i64 %9 to i32
  br i1 %8, label %13, label %50

13:                                               ; preds = %5
  %14 = getelementptr inbounds i32, ptr %11, i32 %12
  %15 = load i32, ptr %14, align 4, !tbaa !23
  %16 = icmp eq i32 %15, 0
  br i1 %16, label %91, label %17

17:                                               ; preds = %13
  %18 = trunc i64 %4 to i32
  %19 = icmp eq ptr %3, null
  br label %20

20:                                               ; preds = %17, %41
  %21 = phi ptr [ %11, %17 ], [ %42, %41 ]
  %22 = phi i64 [ %7, %17 ], [ %43, %41 ]
  %23 = phi i32 [ %15, %17 ], [ %48, %41 ]
  %24 = phi i64 [ %9, %17 ], [ %45, %41 ]
  %25 = icmp sgt i32 %23, 0
  br i1 %25, label %26, label %41

26:                                               ; preds = %20
  %27 = add nsw i32 %23, -1
  %28 = load ptr, ptr %0, align 8, !tbaa !248
  %29 = mul i32 %27, %18
  %30 = getelementptr inbounds i8, ptr %28, i32 %29
  %31 = load i64, ptr %30, align 8, !tbaa !89
  %32 = icmp eq i64 %31, %1
  br i1 %32, label %33, label %41

33:                                               ; preds = %26
  br i1 %19, label %89, label %34

34:                                               ; preds = %33
  %35 = getelementptr inbounds i8, ptr %30, i32 16
  %36 = tail call i32 %3(ptr noundef nonnull %35, ptr noundef %2) #54
  %37 = icmp eq i32 %36, 0
  br i1 %37, label %38, label %87

38:                                               ; preds = %34
  %39 = load i64, ptr %6, align 8, !tbaa !258
  %40 = load ptr, ptr %10, align 8, !tbaa !254
  br label %41

41:                                               ; preds = %38, %26, %20
  %42 = phi ptr [ %40, %38 ], [ %21, %26 ], [ %21, %20 ]
  %43 = phi i64 [ %39, %38 ], [ %22, %26 ], [ %22, %20 ]
  %44 = add nsw i64 %24, 1
  %45 = and i64 %43, %44
  %46 = trunc i64 %45 to i32
  %47 = getelementptr inbounds i32, ptr %42, i32 %46
  %48 = load i32, ptr %47, align 4, !tbaa !23
  %49 = icmp eq i32 %48, 0
  br i1 %49, label %91, label %20

50:                                               ; preds = %5
  %51 = getelementptr inbounds i64, ptr %11, i32 %12
  %52 = load i64, ptr %51, align 8, !tbaa !89
  %53 = icmp eq i64 %52, 0
  br i1 %53, label %91, label %54

54:                                               ; preds = %50
  %55 = icmp eq ptr %3, null
  br label %56

56:                                               ; preds = %54, %78
  %57 = phi ptr [ %11, %54 ], [ %79, %78 ]
  %58 = phi i64 [ %7, %54 ], [ %80, %78 ]
  %59 = phi i64 [ %52, %54 ], [ %85, %78 ]
  %60 = phi i64 [ %9, %54 ], [ %82, %78 ]
  %61 = icmp sgt i64 %59, 0
  br i1 %61, label %62, label %78

62:                                               ; preds = %56
  %63 = add nsw i64 %59, -1
  %64 = load ptr, ptr %0, align 8, !tbaa !248
  %65 = mul nsw i64 %63, %4
  %66 = trunc i64 %65 to i32
  %67 = getelementptr inbounds i8, ptr %64, i32 %66
  %68 = load i64, ptr %67, align 8, !tbaa !89
  %69 = icmp eq i64 %68, %1
  br i1 %69, label %70, label %78

70:                                               ; preds = %62
  br i1 %55, label %91, label %71

71:                                               ; preds = %70
  %72 = getelementptr inbounds i8, ptr %67, i32 16
  %73 = tail call i32 %3(ptr noundef nonnull %72, ptr noundef %2) #54
  %74 = icmp eq i32 %73, 0
  br i1 %74, label %75, label %91

75:                                               ; preds = %71
  %76 = load i64, ptr %6, align 8, !tbaa !258
  %77 = load ptr, ptr %10, align 8, !tbaa !254
  br label %78

78:                                               ; preds = %75, %62, %56
  %79 = phi ptr [ %77, %75 ], [ %57, %62 ], [ %57, %56 ]
  %80 = phi i64 [ %76, %75 ], [ %58, %62 ], [ %58, %56 ]
  %81 = add nsw i64 %60, 1
  %82 = and i64 %80, %81
  %83 = trunc i64 %82 to i32
  %84 = getelementptr inbounds i64, ptr %79, i32 %83
  %85 = load i64, ptr %84, align 8, !tbaa !89
  %86 = icmp eq i64 %85, 0
  br i1 %86, label %91, label %56

87:                                               ; preds = %34
  %88 = zext nneg i32 %27 to i64
  br label %91

89:                                               ; preds = %33
  %90 = zext nneg i32 %27 to i64
  br label %91

91:                                               ; preds = %78, %71, %70, %41, %50, %87, %89, %13
  %92 = phi i64 [ %88, %87 ], [ %90, %89 ], [ -1, %13 ], [ -1, %50 ], [ -1, %41 ], [ -1, %78 ], [ %63, %71 ], [ %63, %70 ]
  ret i64 %92
}

; Function Attrs: alwaysinline nounwind optsize
define hidden i64 @fa_table_find_growing(ptr nocapture noundef readonly %0, i64 noundef %1, ptr noundef %2, ptr noundef readonly %3, i64 noundef %4) local_unnamed_addr #13 {
  %6 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %7 = load i64, ptr %6, align 8, !tbaa !258
  %8 = icmp ult i64 %7, 2147483648
  %9 = and i64 %7, %1
  %10 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %11 = load ptr, ptr %10, align 8, !tbaa !254
  %12 = trunc i64 %9 to i32
  br i1 %8, label %13, label %56

13:                                               ; preds = %5
  %14 = getelementptr inbounds i32, ptr %11, i32 %12
  %15 = load i32, ptr %14, align 4, !tbaa !23
  %16 = icmp eq i32 %15, 0
  br i1 %16, label %99, label %17

17:                                               ; preds = %13
  %18 = icmp eq ptr %3, null
  br label %19

19:                                               ; preds = %17, %47
  %20 = phi ptr [ %11, %17 ], [ %48, %47 ]
  %21 = phi i64 [ %7, %17 ], [ %49, %47 ]
  %22 = phi i32 [ %15, %17 ], [ %54, %47 ]
  %23 = phi i64 [ %9, %17 ], [ %51, %47 ]
  %24 = icmp sgt i32 %22, 0
  br i1 %24, label %25, label %47

25:                                               ; preds = %19
  %26 = add nsw i32 %22, -1
  %27 = zext nneg i32 %26 to i64
  %28 = load ptr, ptr %0, align 8, !tbaa !248
  %29 = load ptr, ptr %28, align 8, !tbaa !69
  %30 = lshr i32 %26, 5
  %31 = getelementptr inbounds ptr, ptr %29, i32 %30
  %32 = load ptr, ptr %31, align 4, !tbaa !6
  %33 = and i64 %27, 31
  %34 = mul nsw i64 %33, %4
  %35 = trunc i64 %34 to i32
  %36 = getelementptr inbounds i8, ptr %32, i32 %35
  %37 = load i64, ptr %36, align 8, !tbaa !89
  %38 = icmp eq i64 %37, %1
  br i1 %38, label %39, label %47

39:                                               ; preds = %25
  br i1 %18, label %99, label %40

40:                                               ; preds = %39
  %41 = getelementptr inbounds i8, ptr %36, i32 16
  %42 = tail call i32 %3(ptr noundef nonnull %41, ptr noundef %2) #54
  %43 = icmp eq i32 %42, 0
  br i1 %43, label %44, label %99

44:                                               ; preds = %40
  %45 = load i64, ptr %6, align 8, !tbaa !258
  %46 = load ptr, ptr %10, align 8, !tbaa !254
  br label %47

47:                                               ; preds = %44, %25, %19
  %48 = phi ptr [ %46, %44 ], [ %20, %25 ], [ %20, %19 ]
  %49 = phi i64 [ %45, %44 ], [ %21, %25 ], [ %21, %19 ]
  %50 = add nsw i64 %23, 1
  %51 = and i64 %49, %50
  %52 = trunc i64 %51 to i32
  %53 = getelementptr inbounds i32, ptr %48, i32 %52
  %54 = load i32, ptr %53, align 4, !tbaa !23
  %55 = icmp eq i32 %54, 0
  br i1 %55, label %99, label %19

56:                                               ; preds = %5
  %57 = getelementptr inbounds i64, ptr %11, i32 %12
  %58 = load i64, ptr %57, align 8, !tbaa !89
  %59 = icmp eq i64 %58, 0
  br i1 %59, label %99, label %60

60:                                               ; preds = %56
  %61 = icmp eq ptr %3, null
  br label %62

62:                                               ; preds = %60, %90
  %63 = phi ptr [ %11, %60 ], [ %91, %90 ]
  %64 = phi i64 [ %7, %60 ], [ %92, %90 ]
  %65 = phi i64 [ %58, %60 ], [ %97, %90 ]
  %66 = phi i64 [ %9, %60 ], [ %94, %90 ]
  %67 = icmp sgt i64 %65, 0
  br i1 %67, label %68, label %90

68:                                               ; preds = %62
  %69 = add nsw i64 %65, -1
  %70 = load ptr, ptr %0, align 8, !tbaa !248
  %71 = load ptr, ptr %70, align 8, !tbaa !69
  %72 = lshr i64 %69, 5
  %73 = trunc i64 %72 to i32
  %74 = getelementptr inbounds ptr, ptr %71, i32 %73
  %75 = load ptr, ptr %74, align 4, !tbaa !6
  %76 = and i64 %69, 31
  %77 = mul nsw i64 %76, %4
  %78 = trunc i64 %77 to i32
  %79 = getelementptr inbounds i8, ptr %75, i32 %78
  %80 = load i64, ptr %79, align 8, !tbaa !89
  %81 = icmp eq i64 %80, %1
  br i1 %81, label %82, label %90

82:                                               ; preds = %68
  br i1 %61, label %99, label %83

83:                                               ; preds = %82
  %84 = getelementptr inbounds i8, ptr %79, i32 16
  %85 = tail call i32 %3(ptr noundef nonnull %84, ptr noundef %2) #54
  %86 = icmp eq i32 %85, 0
  br i1 %86, label %87, label %99

87:                                               ; preds = %83
  %88 = load i64, ptr %6, align 8, !tbaa !258
  %89 = load ptr, ptr %10, align 8, !tbaa !254
  br label %90

90:                                               ; preds = %87, %68, %62
  %91 = phi ptr [ %89, %87 ], [ %63, %68 ], [ %63, %62 ]
  %92 = phi i64 [ %88, %87 ], [ %64, %68 ], [ %64, %62 ]
  %93 = add nsw i64 %66, 1
  %94 = and i64 %92, %93
  %95 = trunc i64 %94 to i32
  %96 = getelementptr inbounds i64, ptr %91, i32 %95
  %97 = load i64, ptr %96, align 8, !tbaa !89
  %98 = icmp eq i64 %97, 0
  br i1 %98, label %99, label %62

99:                                               ; preds = %90, %83, %82, %47, %40, %39, %56, %13
  %100 = phi i64 [ -1, %13 ], [ -1, %56 ], [ -1, %47 ], [ %27, %40 ], [ %27, %39 ], [ -1, %90 ], [ %69, %83 ], [ %69, %82 ]
  ret i64 %100
}

; Function Attrs: alwaysinline nounwind optsize
define hidden i64 @fa_table_probe(ptr nocapture noundef readonly %0, i64 noundef %1, ptr noundef %2, ptr noundef readonly %3, i64 noundef %4) local_unnamed_addr #13 {
  %6 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %7 = load i64, ptr %6, align 8, !tbaa !258
  %8 = icmp ult i64 %7, 2147483648
  %9 = and i64 %7, %1
  %10 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %11 = load ptr, ptr %10, align 8, !tbaa !254
  %12 = trunc i64 %9 to i32
  br i1 %8, label %13, label %62

13:                                               ; preds = %5
  %14 = getelementptr inbounds i32, ptr %11, i32 %12
  %15 = load i32, ptr %14, align 4, !tbaa !23
  %16 = icmp eq i32 %15, 0
  br i1 %16, label %23, label %17

17:                                               ; preds = %13
  %18 = trunc i64 %4 to i32
  %19 = icmp eq ptr %3, null
  br label %26

20:                                               ; preds = %51
  %21 = icmp slt i64 %55, 0
  %22 = select i1 %21, i64 %57, i64 %55
  br label %23

23:                                               ; preds = %20, %13
  %24 = phi i64 [ %9, %13 ], [ %22, %20 ]
  %25 = xor i64 %24, -1
  br label %113

26:                                               ; preds = %17, %51
  %27 = phi ptr [ %11, %17 ], [ %52, %51 ]
  %28 = phi i64 [ %7, %17 ], [ %53, %51 ]
  %29 = phi i32 [ %15, %17 ], [ %60, %51 ]
  %30 = phi i64 [ %9, %17 ], [ %57, %51 ]
  %31 = phi i64 [ -1, %17 ], [ %55, %51 ]
  %32 = icmp sgt i32 %29, 0
  br i1 %32, label %33, label %48

33:                                               ; preds = %26
  %34 = add nsw i32 %29, -1
  %35 = load ptr, ptr %0, align 8, !tbaa !248
  %36 = mul i32 %34, %18
  %37 = getelementptr inbounds i8, ptr %35, i32 %36
  %38 = load i64, ptr %37, align 8, !tbaa !89
  %39 = icmp eq i64 %38, %1
  br i1 %39, label %40, label %51

40:                                               ; preds = %33
  br i1 %19, label %111, label %41

41:                                               ; preds = %40
  %42 = getelementptr inbounds i8, ptr %37, i32 16
  %43 = tail call i32 %3(ptr noundef nonnull %42, ptr noundef %2) #54
  %44 = icmp eq i32 %43, 0
  br i1 %44, label %45, label %111

45:                                               ; preds = %41
  %46 = load i64, ptr %6, align 8, !tbaa !258
  %47 = load ptr, ptr %10, align 8, !tbaa !254
  br label %51

48:                                               ; preds = %26
  %49 = icmp slt i64 %31, 0
  %50 = select i1 %49, i64 %30, i64 %31
  br label %51

51:                                               ; preds = %45, %48, %33
  %52 = phi ptr [ %27, %48 ], [ %27, %33 ], [ %47, %45 ]
  %53 = phi i64 [ %28, %48 ], [ %28, %33 ], [ %46, %45 ]
  %54 = phi i64 [ %50, %48 ], [ %31, %33 ], [ %31, %45 ]
  %55 = freeze i64 %54
  %56 = add nsw i64 %30, 1
  %57 = and i64 %53, %56
  %58 = trunc i64 %57 to i32
  %59 = getelementptr inbounds i32, ptr %52, i32 %58
  %60 = load i32, ptr %59, align 4, !tbaa !23
  %61 = icmp eq i32 %60, 0
  br i1 %61, label %20, label %26

62:                                               ; preds = %5
  %63 = getelementptr inbounds i64, ptr %11, i32 %12
  %64 = load i64, ptr %63, align 8, !tbaa !89
  %65 = icmp eq i64 %64, 0
  br i1 %65, label %71, label %66

66:                                               ; preds = %62
  %67 = icmp eq ptr %3, null
  br label %74

68:                                               ; preds = %100
  %69 = icmp slt i64 %104, 0
  %70 = select i1 %69, i64 %106, i64 %104
  br label %71

71:                                               ; preds = %68, %62
  %72 = phi i64 [ %9, %62 ], [ %70, %68 ]
  %73 = xor i64 %72, -1
  br label %113

74:                                               ; preds = %66, %100
  %75 = phi ptr [ %11, %66 ], [ %101, %100 ]
  %76 = phi i64 [ %7, %66 ], [ %102, %100 ]
  %77 = phi i64 [ %64, %66 ], [ %109, %100 ]
  %78 = phi i64 [ %9, %66 ], [ %106, %100 ]
  %79 = phi i64 [ -1, %66 ], [ %104, %100 ]
  %80 = icmp sgt i64 %77, 0
  br i1 %80, label %81, label %97

81:                                               ; preds = %74
  %82 = add nsw i64 %77, -1
  %83 = load ptr, ptr %0, align 8, !tbaa !248
  %84 = mul nsw i64 %82, %4
  %85 = trunc i64 %84 to i32
  %86 = getelementptr inbounds i8, ptr %83, i32 %85
  %87 = load i64, ptr %86, align 8, !tbaa !89
  %88 = icmp eq i64 %87, %1
  br i1 %88, label %89, label %100

89:                                               ; preds = %81
  br i1 %67, label %113, label %90

90:                                               ; preds = %89
  %91 = getelementptr inbounds i8, ptr %86, i32 16
  %92 = tail call i32 %3(ptr noundef nonnull %91, ptr noundef %2) #54
  %93 = icmp eq i32 %92, 0
  br i1 %93, label %94, label %113

94:                                               ; preds = %90
  %95 = load i64, ptr %6, align 8, !tbaa !258
  %96 = load ptr, ptr %10, align 8, !tbaa !254
  br label %100

97:                                               ; preds = %74
  %98 = icmp slt i64 %79, 0
  %99 = select i1 %98, i64 %78, i64 %79
  br label %100

100:                                              ; preds = %94, %97, %81
  %101 = phi ptr [ %75, %97 ], [ %75, %81 ], [ %96, %94 ]
  %102 = phi i64 [ %76, %97 ], [ %76, %81 ], [ %95, %94 ]
  %103 = phi i64 [ %99, %97 ], [ %79, %81 ], [ %79, %94 ]
  %104 = freeze i64 %103
  %105 = add nsw i64 %78, 1
  %106 = and i64 %102, %105
  %107 = trunc i64 %106 to i32
  %108 = getelementptr inbounds i64, ptr %101, i32 %107
  %109 = load i64, ptr %108, align 8, !tbaa !89
  %110 = icmp eq i64 %109, 0
  br i1 %110, label %68, label %74

111:                                              ; preds = %41, %40
  %112 = zext nneg i32 %34 to i64
  br label %113

113:                                              ; preds = %89, %90, %111, %71, %23
  %114 = phi i64 [ %25, %23 ], [ %73, %71 ], [ %112, %111 ], [ %82, %90 ], [ %82, %89 ]
  ret i64 %114
}

; Function Attrs: alwaysinline nounwind optsize
define hidden i64 @fa_table_probe_growing(ptr nocapture noundef readonly %0, i64 noundef %1, ptr noundef %2, ptr noundef readonly %3, i64 noundef %4) local_unnamed_addr #13 {
  %6 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %7 = load i64, ptr %6, align 8, !tbaa !258
  %8 = icmp ult i64 %7, 2147483648
  %9 = and i64 %7, %1
  %10 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %11 = load ptr, ptr %10, align 8, !tbaa !254
  %12 = trunc i64 %9 to i32
  br i1 %8, label %13, label %68

13:                                               ; preds = %5
  %14 = getelementptr inbounds i32, ptr %11, i32 %12
  %15 = load i32, ptr %14, align 4, !tbaa !23
  %16 = icmp eq i32 %15, 0
  br i1 %16, label %22, label %17

17:                                               ; preds = %13
  %18 = icmp eq ptr %3, null
  br label %25

19:                                               ; preds = %57
  %20 = icmp slt i64 %61, 0
  %21 = select i1 %20, i64 %63, i64 %61
  br label %22

22:                                               ; preds = %19, %13
  %23 = phi i64 [ %9, %13 ], [ %21, %19 ]
  %24 = xor i64 %23, -1
  br label %123

25:                                               ; preds = %17, %57
  %26 = phi ptr [ %11, %17 ], [ %58, %57 ]
  %27 = phi i64 [ %7, %17 ], [ %59, %57 ]
  %28 = phi i32 [ %15, %17 ], [ %66, %57 ]
  %29 = phi i64 [ %9, %17 ], [ %63, %57 ]
  %30 = phi i64 [ -1, %17 ], [ %61, %57 ]
  %31 = icmp sgt i32 %28, 0
  br i1 %31, label %32, label %54

32:                                               ; preds = %25
  %33 = add nsw i32 %28, -1
  %34 = zext nneg i32 %33 to i64
  %35 = load ptr, ptr %0, align 8, !tbaa !248
  %36 = load ptr, ptr %35, align 8, !tbaa !69
  %37 = lshr i32 %33, 5
  %38 = getelementptr inbounds ptr, ptr %36, i32 %37
  %39 = load ptr, ptr %38, align 4, !tbaa !6
  %40 = and i64 %34, 31
  %41 = mul nsw i64 %40, %4
  %42 = trunc i64 %41 to i32
  %43 = getelementptr inbounds i8, ptr %39, i32 %42
  %44 = load i64, ptr %43, align 8, !tbaa !89
  %45 = icmp eq i64 %44, %1
  br i1 %45, label %46, label %57

46:                                               ; preds = %32
  br i1 %18, label %123, label %47

47:                                               ; preds = %46
  %48 = getelementptr inbounds i8, ptr %43, i32 16
  %49 = tail call i32 %3(ptr noundef nonnull %48, ptr noundef %2) #54
  %50 = icmp eq i32 %49, 0
  br i1 %50, label %51, label %123

51:                                               ; preds = %47
  %52 = load i64, ptr %6, align 8, !tbaa !258
  %53 = load ptr, ptr %10, align 8, !tbaa !254
  br label %57

54:                                               ; preds = %25
  %55 = icmp slt i64 %30, 0
  %56 = select i1 %55, i64 %29, i64 %30
  br label %57

57:                                               ; preds = %51, %54, %32
  %58 = phi ptr [ %26, %54 ], [ %26, %32 ], [ %53, %51 ]
  %59 = phi i64 [ %27, %54 ], [ %27, %32 ], [ %52, %51 ]
  %60 = phi i64 [ %56, %54 ], [ %30, %32 ], [ %30, %51 ]
  %61 = freeze i64 %60
  %62 = add nsw i64 %29, 1
  %63 = and i64 %59, %62
  %64 = trunc i64 %63 to i32
  %65 = getelementptr inbounds i32, ptr %58, i32 %64
  %66 = load i32, ptr %65, align 4, !tbaa !23
  %67 = icmp eq i32 %66, 0
  br i1 %67, label %19, label %25

68:                                               ; preds = %5
  %69 = getelementptr inbounds i64, ptr %11, i32 %12
  %70 = load i64, ptr %69, align 8, !tbaa !89
  %71 = icmp eq i64 %70, 0
  br i1 %71, label %77, label %72

72:                                               ; preds = %68
  %73 = icmp eq ptr %3, null
  br label %80

74:                                               ; preds = %112
  %75 = icmp slt i64 %116, 0
  %76 = select i1 %75, i64 %118, i64 %116
  br label %77

77:                                               ; preds = %74, %68
  %78 = phi i64 [ %9, %68 ], [ %76, %74 ]
  %79 = xor i64 %78, -1
  br label %123

80:                                               ; preds = %72, %112
  %81 = phi ptr [ %11, %72 ], [ %113, %112 ]
  %82 = phi i64 [ %7, %72 ], [ %114, %112 ]
  %83 = phi i64 [ %70, %72 ], [ %121, %112 ]
  %84 = phi i64 [ %9, %72 ], [ %118, %112 ]
  %85 = phi i64 [ -1, %72 ], [ %116, %112 ]
  %86 = icmp sgt i64 %83, 0
  br i1 %86, label %87, label %109

87:                                               ; preds = %80
  %88 = add nsw i64 %83, -1
  %89 = load ptr, ptr %0, align 8, !tbaa !248
  %90 = load ptr, ptr %89, align 8, !tbaa !69
  %91 = lshr i64 %88, 5
  %92 = trunc i64 %91 to i32
  %93 = getelementptr inbounds ptr, ptr %90, i32 %92
  %94 = load ptr, ptr %93, align 4, !tbaa !6
  %95 = and i64 %88, 31
  %96 = mul nsw i64 %95, %4
  %97 = trunc i64 %96 to i32
  %98 = getelementptr inbounds i8, ptr %94, i32 %97
  %99 = load i64, ptr %98, align 8, !tbaa !89
  %100 = icmp eq i64 %99, %1
  br i1 %100, label %101, label %112

101:                                              ; preds = %87
  br i1 %73, label %123, label %102

102:                                              ; preds = %101
  %103 = getelementptr inbounds i8, ptr %98, i32 16
  %104 = tail call i32 %3(ptr noundef nonnull %103, ptr noundef %2) #54
  %105 = icmp eq i32 %104, 0
  br i1 %105, label %106, label %123

106:                                              ; preds = %102
  %107 = load i64, ptr %6, align 8, !tbaa !258
  %108 = load ptr, ptr %10, align 8, !tbaa !254
  br label %112

109:                                              ; preds = %80
  %110 = icmp slt i64 %85, 0
  %111 = select i1 %110, i64 %84, i64 %85
  br label %112

112:                                              ; preds = %106, %109, %87
  %113 = phi ptr [ %81, %109 ], [ %81, %87 ], [ %108, %106 ]
  %114 = phi i64 [ %82, %109 ], [ %82, %87 ], [ %107, %106 ]
  %115 = phi i64 [ %111, %109 ], [ %85, %87 ], [ %85, %106 ]
  %116 = freeze i64 %115
  %117 = add nsw i64 %84, 1
  %118 = and i64 %114, %117
  %119 = trunc i64 %118 to i32
  %120 = getelementptr inbounds i64, ptr %113, i32 %119
  %121 = load i64, ptr %120, align 8, !tbaa !89
  %122 = icmp eq i64 %121, 0
  br i1 %122, label %74, label %80

123:                                              ; preds = %101, %102, %46, %47, %77, %22
  %124 = phi i64 [ %24, %22 ], [ %79, %77 ], [ %34, %47 ], [ %34, %46 ], [ %88, %102 ], [ %88, %101 ]
  ret i64 %124
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_table_append(ptr nocapture noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3) local_unnamed_addr #3 {
  %5 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 2
  %6 = load i64, ptr %5, align 8, !tbaa !252
  %7 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 3
  %8 = load i64, ptr %7, align 8, !tbaa !253
  %9 = icmp slt i64 %6, %8
  br i1 %9, label %14, label %10

10:                                               ; preds = %4
  %11 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 7
  %12 = load i64, ptr %11, align 8, !tbaa !256
  %13 = icmp eq i64 %12, 0
  br i1 %13, label %24, label %14

14:                                               ; preds = %10, %4
  %15 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 8
  %16 = load i64, ptr %15, align 8, !tbaa !259
  %17 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %18 = load i64, ptr %17, align 8, !tbaa !258
  %19 = add nsw i64 %18, 1
  %20 = sdiv i64 %19, 4
  %21 = mul nsw i64 %20, 3
  %22 = icmp slt i64 %16, %21
  br i1 %22, label %87, label %23

23:                                               ; preds = %14
  tail call fastcc void @table_index(ptr noundef nonnull %0, i64 noundef %19, i64 noundef %2) #53
  br label %91

24:                                               ; preds = %10
  %25 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 6
  %26 = load i64, ptr %25, align 8, !tbaa !255
  %27 = icmp eq i64 %26, 0
  br i1 %27, label %36, label %28

28:                                               ; preds = %24
  %29 = shl nsw i64 %8, 1
  store i64 %29, ptr %7, align 8, !tbaa !253
  %30 = shl nsw i64 %8, 2
  br label %31

31:                                               ; preds = %31, %28
  %32 = phi i64 [ 8, %28 ], [ %34, %31 ]
  %33 = icmp slt i64 %32, %30
  %34 = shl nsw i64 %32, 1
  br i1 %33, label %31, label %35, !llvm.loop !257

35:                                               ; preds = %31
  tail call fastcc void @table_index(ptr noundef %0, i64 noundef %32, i64 noundef %2) #53
  br label %91

36:                                               ; preds = %24
  %37 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 1
  %38 = load i64, ptr %37, align 8, !tbaa !251
  %39 = shl nsw i64 %38, 1
  %40 = icmp sgt i64 %39, %6
  %41 = zext i1 %40 to i64
  %42 = shl nsw i64 %8, %41
  %43 = mul nsw i64 %42, %2
  %44 = trunc i64 %43 to i32
  %45 = tail call ptr @fa_alloc(i32 noundef %44, i32 noundef 16) #54
  %46 = load i64, ptr %5, align 8, !tbaa !252
  %47 = icmp sgt i64 %46, 0
  br i1 %47, label %48, label %50

48:                                               ; preds = %36
  %49 = trunc i64 %2 to i32
  br label %60

50:                                               ; preds = %77, %36
  %51 = phi i64 [ 0, %36 ], [ %79, %77 ]
  %52 = load ptr, ptr %0, align 8, !tbaa !248
  %53 = load i64, ptr %7, align 8, !tbaa !253
  %54 = mul nsw i64 %53, %2
  %55 = trunc i64 %54 to i32
  tail call void @fa_free(ptr noundef %52, i32 noundef %55, i32 noundef 16) #54
  store ptr %45, ptr %0, align 8, !tbaa !248
  store i64 %51, ptr %5, align 8, !tbaa !252
  store i64 %42, ptr %7, align 8, !tbaa !253
  %56 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %57 = load i64, ptr %56, align 8, !tbaa !258
  %58 = add nsw i64 %57, 1
  %59 = shl nsw i64 %42, 1
  br label %82

60:                                               ; preds = %77, %48
  %61 = phi i64 [ %46, %48 ], [ %78, %77 ]
  %62 = phi i64 [ 0, %48 ], [ %80, %77 ]
  %63 = phi i64 [ 0, %48 ], [ %79, %77 ]
  %64 = load ptr, ptr %0, align 8, !tbaa !248
  %65 = mul nsw i64 %62, %2
  %66 = trunc i64 %65 to i32
  %67 = getelementptr inbounds i8, ptr %64, i32 %66
  %68 = getelementptr inbounds i8, ptr %67, i32 8
  %69 = load i64, ptr %68, align 8, !tbaa !89
  %70 = icmp eq i64 %69, 0
  br i1 %70, label %77, label %71

71:                                               ; preds = %60
  %72 = mul nsw i64 %63, %2
  %73 = trunc i64 %72 to i32
  %74 = getelementptr inbounds i8, ptr %45, i32 %73
  tail call fastcc void @copy_bytes(ptr noundef %74, ptr noundef nonnull %67, i32 noundef %49) #53
  %75 = add nsw i64 %63, 1
  %76 = load i64, ptr %5, align 8, !tbaa !252
  br label %77

77:                                               ; preds = %71, %60
  %78 = phi i64 [ %76, %71 ], [ %61, %60 ]
  %79 = phi i64 [ %75, %71 ], [ %63, %60 ]
  %80 = add nuw nsw i64 %62, 1
  %81 = icmp slt i64 %80, %78
  br i1 %81, label %60, label %50, !llvm.loop !263

82:                                               ; preds = %82, %50
  %83 = phi i64 [ %58, %50 ], [ %85, %82 ]
  %84 = icmp slt i64 %83, %59
  %85 = shl nsw i64 %83, 1
  br i1 %84, label %82, label %86, !llvm.loop !264

86:                                               ; preds = %82
  tail call fastcc void @table_index(ptr noundef nonnull %0, i64 noundef %83, i64 noundef %2) #53
  br label %91

87:                                               ; preds = %14
  %88 = icmp sgt i64 %3, -1
  br i1 %88, label %91, label %89

89:                                               ; preds = %87
  %90 = xor i64 %3, -1
  br label %116

91:                                               ; preds = %86, %35, %23, %87
  %92 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %93 = load i64, ptr %92, align 8, !tbaa !258
  %94 = and i64 %93, %1
  %95 = icmp ult i64 %93, 2147483648
  %96 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %97 = load ptr, ptr %96, align 8, !tbaa !254
  br label %98

98:                                               ; preds = %111, %91
  %99 = phi i64 [ %94, %91 ], [ %113, %111 ]
  %100 = trunc i64 %99 to i32
  br i1 %95, label %101, label %105

101:                                              ; preds = %98
  %102 = getelementptr inbounds i32, ptr %97, i32 %100
  %103 = load i32, ptr %102, align 4, !tbaa !23
  %104 = sext i32 %103 to i64
  br label %108

105:                                              ; preds = %98
  %106 = getelementptr inbounds i64, ptr %97, i32 %100
  %107 = load i64, ptr %106, align 8, !tbaa !89
  br label %108

108:                                              ; preds = %101, %105
  %109 = phi i64 [ %104, %101 ], [ %107, %105 ]
  %110 = icmp sgt i64 %109, 0
  br i1 %110, label %111, label %114

111:                                              ; preds = %108
  %112 = add nsw i64 %99, 1
  %113 = and i64 %112, %93
  br label %98, !llvm.loop !265

114:                                              ; preds = %108
  %115 = load i64, ptr %5, align 8, !tbaa !252
  br label %116

116:                                              ; preds = %114, %89
  %117 = phi i64 [ %6, %89 ], [ %115, %114 ]
  %118 = phi i64 [ %90, %89 ], [ %99, %114 ]
  %119 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 7
  %120 = load i64, ptr %119, align 8, !tbaa !256
  %121 = icmp eq i64 %120, 0
  br i1 %121, label %135, label %122

122:                                              ; preds = %116
  %123 = add nsw i64 %120, -1
  %124 = load ptr, ptr %0, align 8, !tbaa !248
  %125 = load ptr, ptr %124, align 8, !tbaa !69
  %126 = lshr i64 %123, 5
  %127 = trunc i64 %126 to i32
  %128 = getelementptr inbounds ptr, ptr %125, i32 %127
  %129 = load ptr, ptr %128, align 4, !tbaa !6
  %130 = and i64 %123, 31
  %131 = mul nsw i64 %130, %2
  %132 = trunc i64 %131 to i32
  %133 = getelementptr inbounds i8, ptr %129, i32 %132
  %134 = load i64, ptr %133, align 8, !tbaa !89
  store i64 %134, ptr %119, align 8, !tbaa !256
  br label %149

135:                                              ; preds = %116
  %136 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 6
  %137 = load i64, ptr %136, align 8, !tbaa !255
  %138 = icmp eq i64 %137, 0
  br i1 %138, label %143, label %139

139:                                              ; preds = %135
  %140 = tail call fastcc ptr @table_stable_slot(ptr noundef nonnull %0, i64 noundef %117, i64 noundef %2) #53
  %141 = load i64, ptr %5, align 8, !tbaa !252
  %142 = add nsw i64 %141, 1
  store i64 %142, ptr %5, align 8, !tbaa !252
  br label %149

143:                                              ; preds = %135
  %144 = load ptr, ptr %0, align 8, !tbaa !248
  %145 = mul nsw i64 %117, %2
  %146 = trunc i64 %145 to i32
  %147 = getelementptr inbounds i8, ptr %144, i32 %146
  %148 = add nsw i64 %117, 1
  store i64 %148, ptr %5, align 8, !tbaa !252
  br label %149

149:                                              ; preds = %139, %143, %122
  %150 = phi i64 [ %123, %122 ], [ %117, %139 ], [ %117, %143 ]
  %151 = phi ptr [ %133, %122 ], [ %140, %139 ], [ %147, %143 ]
  store i64 %1, ptr %151, align 8, !tbaa !89
  %152 = add nsw i64 %118, 1
  %153 = getelementptr inbounds i8, ptr %151, i32 8
  store i64 %152, ptr %153, align 8, !tbaa !89
  %154 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %155 = load i64, ptr %154, align 8, !tbaa !258
  %156 = icmp ult i64 %155, 2147483648
  %157 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %158 = load ptr, ptr %157, align 8, !tbaa !254
  %159 = trunc i64 %118 to i32
  br i1 %156, label %160, label %164

160:                                              ; preds = %149
  %161 = getelementptr inbounds i32, ptr %158, i32 %159
  %162 = load i32, ptr %161, align 4, !tbaa !23
  %163 = sext i32 %162 to i64
  br label %167

164:                                              ; preds = %149
  %165 = getelementptr inbounds i64, ptr %158, i32 %159
  %166 = load i64, ptr %165, align 8, !tbaa !89
  br label %167

167:                                              ; preds = %160, %164
  %168 = phi i64 [ %163, %160 ], [ %166, %164 ]
  %169 = icmp eq i64 %168, 0
  br i1 %169, label %170, label %174

170:                                              ; preds = %167
  %171 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 8
  %172 = load i64, ptr %171, align 8, !tbaa !259
  %173 = add nsw i64 %172, 1
  store i64 %173, ptr %171, align 8, !tbaa !259
  br label %174

174:                                              ; preds = %170, %167
  %175 = add nsw i64 %150, 1
  br i1 %156, label %176, label %180

176:                                              ; preds = %174
  %177 = trunc i64 %175 to i32
  %178 = trunc i64 %118 to i32
  %179 = getelementptr inbounds i32, ptr %158, i32 %178
  store i32 %177, ptr %179, align 4, !tbaa !23
  br label %183

180:                                              ; preds = %174
  %181 = trunc i64 %118 to i32
  %182 = getelementptr inbounds i64, ptr %158, i32 %181
  store i64 %175, ptr %182, align 8, !tbaa !89
  br label %183

183:                                              ; preds = %176, %180
  %184 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 1
  %185 = load i64, ptr %184, align 8, !tbaa !251
  %186 = add nsw i64 %185, 1
  store i64 %186, ptr %184, align 8, !tbaa !251
  ret ptr %151
}

; Function Attrs: nounwind optsize
define internal fastcc ptr @table_stable_slot(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2) unnamed_addr #3 {
  %4 = load ptr, ptr %0, align 8, !tbaa !248
  %5 = ashr i64 %1, 5
  %6 = getelementptr inbounds %struct.FaTableEntries, ptr %4, i32 0, i32 1
  %7 = load i64, ptr %6, align 8, !tbaa !79
  %8 = icmp eq i64 %5, %7
  br i1 %8, label %11, label %9

9:                                                ; preds = %3
  %10 = trunc i64 %5 to i32
  br label %42

11:                                               ; preds = %3
  %12 = getelementptr inbounds %struct.FaTableEntries, ptr %4, i32 0, i32 2
  %13 = load i64, ptr %12, align 8, !tbaa !73
  %14 = icmp eq i64 %5, %13
  br i1 %14, label %15, label %33

15:                                               ; preds = %11
  %16 = icmp ult i64 %1, 32
  %17 = shl nsw i64 %5, 1
  %18 = select i1 %16, i64 4, i64 %17
  %19 = trunc i64 %18 to i32
  %20 = shl i32 %19, 2
  %21 = tail call ptr @fa_alloc(i32 noundef %20, i32 noundef 4) #54
  %22 = load ptr, ptr %4, align 8, !tbaa !69
  %23 = icmp eq ptr %22, null
  br i1 %23, label %32, label %24

24:                                               ; preds = %15
  %25 = load i64, ptr %6, align 8, !tbaa !79
  %26 = trunc i64 %25 to i32
  %27 = shl i32 %26, 2
  tail call fastcc void @copy_bytes(ptr noundef %21, ptr noundef nonnull %22, i32 noundef %27) #53
  %28 = load ptr, ptr %4, align 8, !tbaa !69
  %29 = load i64, ptr %12, align 8, !tbaa !73
  %30 = trunc i64 %29 to i32
  %31 = shl i32 %30, 2
  tail call void @fa_free(ptr noundef %28, i32 noundef %31, i32 noundef 4) #54
  br label %32

32:                                               ; preds = %24, %15
  store ptr %21, ptr %4, align 8, !tbaa !69
  store i64 %18, ptr %12, align 8, !tbaa !73
  br label %33

33:                                               ; preds = %32, %11
  %34 = trunc i64 %2 to i32
  %35 = shl i32 %34, 5
  %36 = tail call ptr @fa_alloc(i32 noundef %35, i32 noundef 16) #54
  %37 = load ptr, ptr %4, align 8, !tbaa !69
  %38 = trunc i64 %5 to i32
  %39 = getelementptr inbounds ptr, ptr %37, i32 %38
  store ptr %36, ptr %39, align 4, !tbaa !6
  %40 = load i64, ptr %6, align 8, !tbaa !79
  %41 = add nsw i64 %40, 1
  store i64 %41, ptr %6, align 8, !tbaa !79
  br label %42

42:                                               ; preds = %9, %33
  %43 = phi i32 [ %10, %9 ], [ %38, %33 ]
  %44 = load ptr, ptr %4, align 8, !tbaa !69
  %45 = getelementptr inbounds ptr, ptr %44, i32 %43
  %46 = load ptr, ptr %45, align 4, !tbaa !6
  %47 = and i64 %1, 31
  %48 = mul nsw i64 %47, %2
  %49 = trunc i64 %48 to i32
  %50 = getelementptr inbounds i8, ptr %46, i32 %49
  ret ptr %50
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define hidden void @fa_table_kill(ptr nocapture noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #43 {
  %4 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 6
  %5 = load i64, ptr %4, align 8, !tbaa !255
  %6 = icmp eq i64 %5, 0
  %7 = load ptr, ptr %0, align 8, !tbaa !248
  br i1 %6, label %16, label %8

8:                                                ; preds = %3
  %9 = load ptr, ptr %7, align 8, !tbaa !69
  %10 = lshr i64 %1, 5
  %11 = trunc i64 %10 to i32
  %12 = getelementptr inbounds ptr, ptr %9, i32 %11
  %13 = load ptr, ptr %12, align 4, !tbaa !6
  %14 = and i64 %1, 31
  %15 = mul nsw i64 %14, %2
  br label %18

16:                                               ; preds = %3
  %17 = mul nsw i64 %2, %1
  br label %18

18:                                               ; preds = %8, %16
  %19 = phi i64 [ %15, %8 ], [ %17, %16 ]
  %20 = phi ptr [ %13, %8 ], [ %7, %16 ]
  %21 = trunc i64 %19 to i32
  %22 = getelementptr inbounds i8, ptr %20, i32 %21
  %23 = getelementptr inbounds i8, ptr %22, i32 8
  %24 = load i64, ptr %23, align 8, !tbaa !89
  %25 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %26 = load i64, ptr %25, align 8, !tbaa !258
  %27 = icmp ult i64 %26, 2147483648
  %28 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %29 = load ptr, ptr %28, align 8, !tbaa !254
  %30 = trunc i64 %24 to i32
  %31 = add i32 %30, -1
  br i1 %27, label %32, label %34

32:                                               ; preds = %18
  %33 = getelementptr inbounds i32, ptr %29, i32 %31
  store i32 -1, ptr %33, align 4, !tbaa !23
  br label %36

34:                                               ; preds = %18
  %35 = getelementptr inbounds i64, ptr %29, i32 %31
  store i64 -1, ptr %35, align 8, !tbaa !89
  br label %36

36:                                               ; preds = %32, %34
  store i64 0, ptr %23, align 8, !tbaa !89
  %37 = load i64, ptr %4, align 8, !tbaa !255
  %38 = icmp eq i64 %37, 2
  br i1 %38, label %39, label %43

39:                                               ; preds = %36
  %40 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 7
  %41 = load i64, ptr %40, align 8, !tbaa !256
  store i64 %41, ptr %22, align 8, !tbaa !89
  %42 = add nsw i64 %1, 1
  store i64 %42, ptr %40, align 8, !tbaa !256
  br label %43

43:                                               ; preds = %39, %36
  %44 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 1
  %45 = load i64, ptr %44, align 8, !tbaa !251
  %46 = add nsw i64 %45, -1
  store i64 %46, ptr %44, align 8, !tbaa !251
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, argmem: readwrite, inaccessiblemem: none)
define hidden void @fa_table_clear(ptr nocapture noundef %0) local_unnamed_addr #44 {
  %2 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 1
  store i64 0, ptr %2, align 8, !tbaa !251
  %3 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 2
  store i64 0, ptr %3, align 8, !tbaa !252
  %4 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 7
  store i64 0, ptr %4, align 8, !tbaa !256
  %5 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 8
  store i64 0, ptr %5, align 8, !tbaa !259
  %6 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %7 = load ptr, ptr %6, align 8, !tbaa !254
  %8 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %9 = load i64, ptr %8, align 8, !tbaa !258
  %10 = trunc i64 %9 to i32
  %11 = add i32 %10, 1
  %12 = icmp ult i64 %9, 2147483648
  %13 = select i1 %12, i32 2, i32 3
  %14 = shl i32 %11, %13
  tail call void @llvm.memset.p0.i32(ptr align 1 %7, i8 0, i32 %14, i1 false)
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_table_free(ptr noundef %0, i64 noundef %1) local_unnamed_addr #3 {
  %3 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 9
  %4 = load i64, ptr %3, align 8, !tbaa !250
  %5 = icmp eq i64 %4, 0
  br i1 %5, label %6, label %51

6:                                                ; preds = %2
  %7 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 6
  %8 = load i64, ptr %7, align 8, !tbaa !255
  %9 = icmp eq i64 %8, 0
  %10 = load ptr, ptr %0, align 8, !tbaa !248
  br i1 %9, label %36, label %11

11:                                               ; preds = %6
  %12 = getelementptr inbounds %struct.FaTableEntries, ptr %10, i32 0, i32 1
  %13 = load i64, ptr %12, align 8, !tbaa !79
  %14 = icmp sgt i64 %13, 0
  br i1 %14, label %15, label %18

15:                                               ; preds = %11
  %16 = trunc i64 %1 to i32
  %17 = shl i32 %16, 5
  br label %21

18:                                               ; preds = %21, %11
  %19 = load ptr, ptr %10, align 8, !tbaa !69
  %20 = icmp eq ptr %19, null
  br i1 %20, label %35, label %30

21:                                               ; preds = %15, %21
  %22 = phi i64 [ 0, %15 ], [ %27, %21 ]
  %23 = load ptr, ptr %10, align 8, !tbaa !69
  %24 = trunc i64 %22 to i32
  %25 = getelementptr inbounds ptr, ptr %23, i32 %24
  %26 = load ptr, ptr %25, align 4, !tbaa !6
  tail call void @fa_free(ptr noundef %26, i32 noundef %17, i32 noundef 16) #54
  %27 = add nuw nsw i64 %22, 1
  %28 = load i64, ptr %12, align 8, !tbaa !79
  %29 = icmp slt i64 %27, %28
  br i1 %29, label %21, label %18, !llvm.loop !266

30:                                               ; preds = %18
  %31 = getelementptr inbounds %struct.FaTableEntries, ptr %10, i32 0, i32 2
  %32 = load i64, ptr %31, align 8, !tbaa !73
  %33 = trunc i64 %32 to i32
  %34 = shl i32 %33, 2
  tail call void @fa_free(ptr noundef nonnull %19, i32 noundef %34, i32 noundef 4) #54
  br label %35

35:                                               ; preds = %30, %18
  tail call void @fa_free(ptr noundef nonnull %10, i32 noundef 24, i32 noundef 4) #54
  br label %41

36:                                               ; preds = %6
  %37 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 3
  %38 = load i64, ptr %37, align 8, !tbaa !253
  %39 = mul nsw i64 %38, %1
  %40 = trunc i64 %39 to i32
  tail call void @fa_free(ptr noundef %10, i32 noundef %40, i32 noundef 16) #54
  br label %41

41:                                               ; preds = %36, %35
  %42 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %43 = load ptr, ptr %42, align 8, !tbaa !254
  %44 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %45 = load i64, ptr %44, align 8, !tbaa !258
  %46 = trunc i64 %45 to i32
  %47 = add i32 %46, 1
  %48 = icmp ult i64 %45, 2147483648
  %49 = select i1 %48, i32 2, i32 3
  %50 = shl i32 %47, %49
  tail call void @fa_free(ptr noundef %43, i32 noundef %50, i32 noundef 8) #54
  tail call void @fa_free(ptr noundef nonnull %0, i32 noundef 80, i32 noundef 8) #54
  br label %51

51:                                               ; preds = %2, %41
  ret void
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_table_copy(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #3 {
  %3 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 6
  %4 = load i64, ptr %3, align 8, !tbaa !255
  %5 = icmp eq i64 %4, 0
  %6 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 1
  %7 = load i64, ptr %6, align 8, !tbaa !251
  %8 = tail call i64 @llvm.smax.i64(i64 %7, i64 4)
  %9 = icmp eq i64 %4, 2
  br i1 %9, label %10, label %13

10:                                               ; preds = %2
  %11 = tail call ptr @fa_table_new_growing(i64 noundef %8, i64 noundef %1) #53
  %12 = getelementptr inbounds %struct.FaTable, ptr %11, i32 0, i32 6
  store i64 2, ptr %12, align 8, !tbaa !255
  br label %18

13:                                               ; preds = %2
  br i1 %5, label %16, label %14

14:                                               ; preds = %13
  %15 = tail call ptr @fa_table_new_growing(i64 noundef %8, i64 noundef %1) #53
  br label %18

16:                                               ; preds = %13
  %17 = tail call ptr @fa_table_new(i64 noundef %8, i64 noundef %1) #53
  br label %18

18:                                               ; preds = %14, %16, %10
  %19 = phi ptr [ %11, %10 ], [ %15, %14 ], [ %17, %16 ]
  %20 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 2
  %21 = load i64, ptr %20, align 8, !tbaa !252
  %22 = icmp sgt i64 %21, 0
  br i1 %22, label %23, label %25

23:                                               ; preds = %18
  %24 = trunc i64 %1 to i32
  br label %32

25:                                               ; preds = %69, %18
  %26 = phi i64 [ 0, %18 ], [ %71, %69 ]
  %27 = getelementptr inbounds %struct.FaTable, ptr %19, i32 0, i32 2
  store i64 %26, ptr %27, align 8, !tbaa !252
  %28 = getelementptr inbounds %struct.FaTable, ptr %19, i32 0, i32 1
  store i64 %26, ptr %28, align 8, !tbaa !251
  %29 = getelementptr inbounds %struct.FaTable, ptr %19, i32 0, i32 5
  %30 = load i64, ptr %29, align 8, !tbaa !258
  %31 = add nsw i64 %30, 1
  tail call fastcc void @table_index(ptr noundef %19, i64 noundef %31, i64 noundef %1) #53
  ret ptr %19

32:                                               ; preds = %23, %69
  %33 = phi i64 [ %21, %23 ], [ %70, %69 ]
  %34 = phi i64 [ 0, %23 ], [ %71, %69 ]
  %35 = phi i64 [ 0, %23 ], [ %72, %69 ]
  %36 = load ptr, ptr %0, align 8, !tbaa !248
  br i1 %5, label %37, label %44

37:                                               ; preds = %32
  %38 = mul nsw i64 %35, %1
  %39 = trunc i64 %38 to i32
  %40 = getelementptr inbounds i8, ptr %36, i32 %39
  %41 = getelementptr inbounds i8, ptr %40, i32 8
  %42 = load i64, ptr %41, align 8, !tbaa !89
  %43 = icmp eq i64 %42, 0
  br i1 %43, label %69, label %59

44:                                               ; preds = %32
  %45 = load ptr, ptr %36, align 8, !tbaa !69
  %46 = lshr i64 %35, 5
  %47 = trunc i64 %46 to i32
  %48 = getelementptr inbounds ptr, ptr %45, i32 %47
  %49 = load ptr, ptr %48, align 4, !tbaa !6
  %50 = and i64 %35, 31
  %51 = mul nsw i64 %50, %1
  %52 = trunc i64 %51 to i32
  %53 = getelementptr inbounds i8, ptr %49, i32 %52
  %54 = getelementptr inbounds i8, ptr %53, i32 8
  %55 = load i64, ptr %54, align 8, !tbaa !89
  %56 = icmp eq i64 %55, 0
  br i1 %56, label %69, label %57

57:                                               ; preds = %44
  %58 = tail call fastcc ptr @table_stable_slot(ptr noundef %19, i64 noundef %34, i64 noundef %1) #53
  br label %64

59:                                               ; preds = %37
  %60 = load ptr, ptr %19, align 8, !tbaa !248
  %61 = mul nsw i64 %34, %1
  %62 = trunc i64 %61 to i32
  %63 = getelementptr inbounds i8, ptr %60, i32 %62
  br label %64

64:                                               ; preds = %59, %57
  %65 = phi ptr [ %53, %57 ], [ %40, %59 ]
  %66 = phi ptr [ %58, %57 ], [ %63, %59 ]
  tail call fastcc void @copy_bytes(ptr noundef %66, ptr noundef nonnull %65, i32 noundef %24) #53
  %67 = add nsw i64 %34, 1
  %68 = load i64, ptr %20, align 8, !tbaa !252
  br label %69

69:                                               ; preds = %44, %64, %37
  %70 = phi i64 [ %68, %64 ], [ %33, %37 ], [ %33, %44 ]
  %71 = phi i64 [ %67, %64 ], [ %34, %37 ], [ %34, %44 ]
  %72 = add nuw nsw i64 %35, 1
  %73 = icmp slt i64 %72, %70
  br i1 %73, label %32, label %25, !llvm.loop !267
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: read)
define hidden i64 @fa_hash_bytes(ptr nocapture noundef readonly %0, i32 noundef %1) local_unnamed_addr #12 {
  %3 = zext i32 %1 to i64
  %4 = mul i64 %3, -7046029254386353131
  %5 = xor i64 %4, -3750763034362895579
  %6 = icmp ult i32 %1, 8
  br i1 %6, label %19, label %7

7:                                                ; preds = %2, %7
  %8 = phi i32 [ %17, %7 ], [ 8, %2 ]
  %9 = phi i64 [ %16, %7 ], [ %5, %2 ]
  %10 = phi i32 [ %8, %7 ], [ 0, %2 ]
  %11 = getelementptr inbounds i8, ptr %0, i32 %10
  %12 = load i64, ptr %11, align 1
  %13 = xor i64 %12, %9
  %14 = mul i64 %13, -7046029254386353131
  %15 = lshr i64 %14, 29
  %16 = xor i64 %15, %14
  %17 = add nuw i32 %8, 8
  %18 = icmp ugt i32 %17, %1
  br i1 %18, label %19, label %7, !llvm.loop !268

19:                                               ; preds = %7, %2
  %20 = phi i32 [ 0, %2 ], [ %8, %7 ]
  %21 = phi i64 [ %5, %2 ], [ %16, %7 ]
  %22 = icmp ult i32 %20, %1
  br i1 %22, label %23, label %56

23:                                               ; preds = %19
  %24 = sub i32 %1, %20
  %25 = icmp ugt i32 %24, 3
  %26 = getelementptr inbounds i8, ptr %0, i32 %20
  br i1 %25, label %27, label %36

27:                                               ; preds = %23
  %28 = load i32, ptr %26, align 1
  %29 = getelementptr inbounds i8, ptr %0, i32 %1
  %30 = getelementptr inbounds i8, ptr %29, i32 -4
  %31 = load i32, ptr %30, align 1
  %32 = zext i32 %28 to i64
  %33 = zext i32 %31 to i64
  %34 = shl nuw i64 %33, 32
  %35 = or disjoint i64 %34, %32
  br label %52

36:                                               ; preds = %23
  %37 = load i8, ptr %26, align 1, !tbaa !1
  %38 = zext i8 %37 to i64
  %39 = lshr i32 %24, 1
  %40 = or disjoint i32 %39, %20
  %41 = getelementptr inbounds i8, ptr %0, i32 %40
  %42 = load i8, ptr %41, align 1, !tbaa !1
  %43 = zext i8 %42 to i64
  %44 = shl nuw nsw i64 %43, 8
  %45 = or disjoint i64 %44, %38
  %46 = getelementptr i8, ptr %0, i32 %1
  %47 = getelementptr i8, ptr %46, i32 -1
  %48 = load i8, ptr %47, align 1, !tbaa !1
  %49 = zext i8 %48 to i64
  %50 = shl nuw nsw i64 %49, 16
  %51 = or disjoint i64 %45, %50
  br label %52

52:                                               ; preds = %36, %27
  %53 = phi i64 [ %35, %27 ], [ %51, %36 ]
  %54 = xor i64 %53, %21
  %55 = mul i64 %54, -7046029254386353131
  br label %56

56:                                               ; preds = %52, %19
  %57 = phi i64 [ %55, %52 ], [ %21, %19 ]
  %58 = lshr i64 %57, 33
  %59 = xor i64 %58, %57
  %60 = mul i64 %59, -49064778989728563
  %61 = lshr i64 %60, 33
  %62 = xor i64 %61, %60
  ret i64 %62
}

; Function Attrs: alwaysinline nofree norecurse nosync nounwind optsize memory(read, inaccessiblemem: none)
define hidden i64 @fa_hash_string_bits(i32 noundef %0, i32 noundef %1, i32 noundef %2) local_unnamed_addr #45 {
  %4 = and i32 %2, 1073741824
  %5 = icmp eq i32 %4, 0
  br i1 %5, label %6, label %9

6:                                                ; preds = %3
  %7 = inttoptr i32 %0 to ptr
  %8 = tail call i64 @fa_hash_bytes(ptr noundef %7, i32 noundef %1) #53
  br label %73

9:                                                ; preds = %3
  %10 = lshr i32 %2, 24
  %11 = and i32 %10, 31
  %12 = zext nneg i32 %11 to i64
  %13 = mul i64 %12, -7046029254386353131
  %14 = xor i64 %13, -3750763034362895579
  %15 = zext i32 %0 to i64
  %16 = zext i32 %1 to i64
  %17 = shl nuw i64 %16, 32
  %18 = or disjoint i64 %17, %15
  %19 = zext i32 %2 to i64
  %20 = icmp ugt i32 %11, 7
  %21 = xor i64 %14, %18
  %22 = mul i64 %21, -7046029254386353131
  %23 = lshr i64 %22, 29
  %24 = xor i64 %23, %22
  %25 = select i1 %20, i64 %24, i64 %14
  %26 = icmp ugt i32 %11, 15
  %27 = xor i64 %25, %19
  %28 = mul i64 %27, -7046029254386353131
  %29 = lshr i64 %28, 29
  %30 = xor i64 %29, %28
  %31 = select i1 %26, i64 %30, i64 %25
  %32 = and i32 %10, 7
  %33 = icmp eq i32 %32, 0
  br i1 %33, label %66, label %34

34:                                               ; preds = %9
  %35 = select i1 %20, i64 %19, i64 %18
  %36 = select i1 %26, i64 0, i64 %35
  %37 = icmp ugt i32 %32, 3
  br i1 %37, label %38, label %46

38:                                               ; preds = %34
  %39 = and i64 %36, 4294967295
  %40 = shl nuw nsw i32 %32, 3
  %41 = add nsw i32 %40, -32
  %42 = zext nneg i32 %41 to i64
  %43 = lshr i64 %36, %42
  %44 = shl i64 %43, 32
  %45 = or disjoint i64 %44, %39
  br label %62

46:                                               ; preds = %34
  %47 = and i64 %36, 255
  %48 = shl nuw nsw i32 %10, 2
  %49 = and i32 %48, 8
  %50 = zext nneg i32 %49 to i64
  %51 = lshr i64 %36, %50
  %52 = shl i64 %51, 8
  %53 = and i64 %52, 65280
  %54 = or disjoint i64 %53, %47
  %55 = shl nuw nsw i32 %32, 3
  %56 = add nsw i32 %55, -8
  %57 = zext nneg i32 %56 to i64
  %58 = lshr i64 %36, %57
  %59 = shl i64 %58, 16
  %60 = and i64 %59, 16711680
  %61 = or disjoint i64 %54, %60
  br label %62

62:                                               ; preds = %46, %38
  %63 = phi i64 [ %45, %38 ], [ %61, %46 ]
  %64 = xor i64 %63, %31
  %65 = mul i64 %64, -7046029254386353131
  br label %66

66:                                               ; preds = %62, %9
  %67 = phi i64 [ %65, %62 ], [ %31, %9 ]
  %68 = lshr i64 %67, 33
  %69 = xor i64 %68, %67
  %70 = mul i64 %69, -49064778989728563
  %71 = lshr i64 %70, 33
  %72 = xor i64 %71, %70
  br label %73

73:                                               ; preds = %66, %6
  %74 = phi i64 [ %72, %66 ], [ %8, %6 ]
  ret i64 %74
}

; Function Attrs: cold noinline nounwind optsize
define hidden noundef ptr @fa_error_key_not_found() local_unnamed_addr #21 {
  %1 = alloca %struct.FaString, align 4
  %2 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #54
  store i64 2, ptr %2, align 8, !tbaa !241
  %3 = getelementptr inbounds %struct.MessageError, ptr %2, i32 0, i32 1
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %1) #56
  %4 = load ptr, ptr @builder, align 4, !tbaa !6
  %5 = load i32, ptr @builder_length, align 4, !tbaa !4
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %1, ptr noundef %4, i32 noundef %5, i32 noundef 0) #53
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %3, ptr noundef nonnull align 4 dereferenceable(12) %1, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %1) #56
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  ret ptr %2
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(argmem: readwrite)
define hidden noundef ptr @fa_shared_retain(ptr noundef returned %0) local_unnamed_addr #46 {
  %2 = icmp eq ptr %0, null
  br i1 %2, label %5, label %3

3:                                                ; preds = %1
  %4 = atomicrmw add ptr %0, i64 1 monotonic, align 8
  br label %5

5:                                                ; preds = %3, %1
  ret ptr %0
}

; Function Attrs: nounwind optsize
define hidden void @fa_shared_release(ptr noundef %0) local_unnamed_addr #3 {
  %2 = icmp eq ptr %0, null
  br i1 %2, label %15, label %3

3:                                                ; preds = %1
  %4 = atomicrmw sub ptr %0, i64 1 acq_rel, align 8
  %5 = icmp eq i64 %4, 1
  br i1 %5, label %6, label %15

6:                                                ; preds = %3
  %7 = getelementptr inbounds %struct.FaShared, ptr %0, i32 0, i32 2
  %8 = load ptr, ptr %7, align 8, !tbaa !269
  %9 = icmp eq ptr %8, null
  br i1 %9, label %11, label %10

10:                                               ; preds = %6
  tail call void %8(ptr noundef nonnull %0) #54
  br label %11

11:                                               ; preds = %10, %6
  %12 = getelementptr inbounds %struct.FaShared, ptr %0, i32 0, i32 1
  %13 = load i64, ptr %12, align 8, !tbaa !271
  %14 = trunc i64 %13 to i32
  tail call void @fa_free(ptr noundef nonnull %0, i32 noundef %14, i32 noundef 16) #54
  br label %15

15:                                               ; preds = %11, %3, %1
  ret void
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_atomic_new(i64 noundef %0) local_unnamed_addr #3 {
  %2 = tail call fastcc ptr @shared_new(i32 noundef 40, ptr noundef null) #53
  %3 = getelementptr inbounds %struct.FaAtomicBlock, ptr %2, i32 0, i32 1
  store i64 %0, ptr %3, align 8, !tbaa !272
  ret ptr %2
}

; Function Attrs: nounwind optsize
define internal fastcc ptr @shared_new(i32 noundef %0, ptr noundef %1) unnamed_addr #3 {
  %3 = tail call ptr @fa_alloc(i32 noundef %0, i32 noundef 16) #54
  %4 = icmp eq ptr %3, null
  br i1 %4, label %5, label %6

5:                                                ; preds = %2
  tail call fastcc void @out_of_memory() #57
  unreachable

6:                                                ; preds = %2
  tail call void @llvm.memset.p0.i32(ptr nonnull align 8 %3, i8 0, i32 %0, i1 false)
  store i64 1, ptr %3, align 8, !tbaa !274
  %7 = zext i32 %0 to i64
  %8 = getelementptr inbounds %struct.FaShared, ptr %3, i32 0, i32 1
  store i64 %7, ptr %8, align 8, !tbaa !271
  %9 = getelementptr inbounds %struct.FaShared, ptr %3, i32 0, i32 2
  store ptr %1, ptr %9, align 8, !tbaa !269
  ret ptr %3
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(argmem: readwrite)
define hidden void @fa_remote_source_start(ptr nocapture noundef %0) local_unnamed_addr #46 {
  %2 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 9
  %3 = atomicrmw add ptr %2, i64 1 acq_rel, align 8
  ret void
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(argmem: readwrite)
define hidden void @fa_remote_source_end(ptr nocapture noundef %0) local_unnamed_addr #46 {
  %2 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 9
  %3 = atomicrmw sub ptr %2, i64 1 acq_rel, align 8
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_task_spawn(ptr noundef %0, i32 noundef %1) local_unnamed_addr #3 {
  %3 = tail call fastcc ptr @executor() #53
  %4 = getelementptr inbounds i8, ptr %0, i32 16
  %5 = icmp eq i32 %1, 0
  br i1 %5, label %13, label %6

6:                                                ; preds = %2
  %7 = getelementptr inbounds i8, ptr %0, i32 32
  %8 = load i32, ptr %7, align 4, !tbaa !275
  %9 = or i32 %8, 1
  store i32 %9, ptr %7, align 4, !tbaa !275
  %10 = getelementptr inbounds %struct.FaExecutor, ptr %3, i32 0, i32 8
  %11 = load i64, ptr %10, align 8, !tbaa !277
  %12 = add nsw i64 %11, 1
  store i64 %12, ptr %10, align 8, !tbaa !277
  br label %13

13:                                               ; preds = %6, %2
  %14 = getelementptr inbounds i8, ptr %0, i32 20
  store ptr null, ptr %14, align 4, !tbaa !279
  %15 = getelementptr inbounds %struct.FaExecutor, ptr %3, i32 0, i32 1
  %16 = load ptr, ptr %15, align 4, !tbaa !280
  %17 = getelementptr inbounds i8, ptr %0, i32 24
  store ptr %16, ptr %17, align 4, !tbaa !281
  %18 = icmp eq ptr %16, null
  br i1 %18, label %21, label %19

19:                                               ; preds = %13
  %20 = getelementptr inbounds %struct.FaTask, ptr %16, i32 0, i32 1
  store ptr %4, ptr %20, align 4, !tbaa !279
  br label %22

21:                                               ; preds = %13
  store ptr %4, ptr %3, align 8, !tbaa !282
  br label %22

22:                                               ; preds = %19, %21
  store ptr %4, ptr %15, align 4, !tbaa !280
  %23 = getelementptr inbounds %struct.FaExecutor, ptr %3, i32 0, i32 2
  %24 = load i64, ptr %23, align 8, !tbaa !283
  %25 = add nsw i64 %24, 1
  store i64 %25, ptr %23, align 8, !tbaa !283
  %26 = getelementptr inbounds i8, ptr %0, i32 28
  store atomic i32 1, ptr %26 release, align 4
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc ptr @executor() unnamed_addr #3 {
  %1 = load ptr, ptr @current_executor, align 4, !tbaa !6
  %2 = icmp eq ptr %1, null
  br i1 %2, label %3, label %10

3:                                                ; preds = %0
  %4 = tail call ptr @fa_alloc(i32 noundef 288, i32 noundef 16) #54
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(288) %4, i8 0, i32 288, i1 false)
  %5 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 13
  %6 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 11
  store ptr %5, ptr %6, align 8, !tbaa !284
  %7 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 12
  store ptr %5, ptr %7, align 4, !tbaa !285
  %8 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 15
  store i32 -1, ptr %8, align 8, !tbaa !286
  %9 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 16
  store i32 -1, ptr %9, align 4, !tbaa !287
  store i32 128, ptr @fa_coop_budget, align 4, !tbaa !23
  store ptr %4, ptr @current_executor, align 4, !tbaa !6
  br label %10

10:                                               ; preds = %0, %3
  %11 = phi ptr [ %4, %3 ], [ %1, %0 ]
  ret ptr %11
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define hidden void @fa_task_await(ptr nocapture noundef writeonly %0, ptr noundef %1) local_unnamed_addr #47 {
  %3 = getelementptr inbounds i8, ptr %0, i32 16
  store ptr %1, ptr %3, align 4, !tbaa !288
  %4 = getelementptr inbounds i8, ptr %1, i32 28
  store atomic i32 2, ptr %4 release, align 4
  %5 = load ptr, ptr @current_executor, align 4, !tbaa !6
  %6 = icmp eq ptr %5, null
  br i1 %6, label %15, label %7

7:                                                ; preds = %2
  %8 = getelementptr inbounds %struct.FaExecutor, ptr %5, i32 0, i32 7
  %9 = load ptr, ptr %8, align 8, !tbaa !289
  %10 = icmp eq ptr %9, %1
  %11 = icmp eq ptr %9, null
  %12 = or i1 %10, %11
  br i1 %12, label %13, label %15

13:                                               ; preds = %7
  %14 = getelementptr inbounds %struct.FaExecutor, ptr %5, i32 0, i32 20
  store ptr null, ptr %14, align 8, !tbaa !290
  br label %15

15:                                               ; preds = %7, %13, %2
  ret void
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define hidden void @fa_task_finish(ptr noundef %0) local_unnamed_addr #47 {
  %2 = getelementptr inbounds i8, ptr %0, i32 16
  %3 = getelementptr inbounds i8, ptr %0, i32 32
  %4 = load i32, ptr %3, align 4, !tbaa !275
  %5 = or i32 %4, 2
  store i32 %5, ptr %3, align 4, !tbaa !275
  %6 = getelementptr inbounds i8, ptr %0, i32 28
  store atomic i32 3, ptr %6 release, align 4
  %7 = and i32 %4, 1
  %8 = icmp eq i32 %7, 0
  br i1 %8, label %14, label %9

9:                                                ; preds = %1
  %10 = load ptr, ptr @current_executor, align 4, !tbaa !6
  %11 = getelementptr inbounds %struct.FaExecutor, ptr %10, i32 0, i32 8
  %12 = load i64, ptr %11, align 8, !tbaa !277
  %13 = add nsw i64 %12, -1
  store i64 %13, ptr %11, align 8, !tbaa !277
  store ptr %0, ptr @frame_to_free, align 4, !tbaa !6
  br label %40

14:                                               ; preds = %1
  %15 = load ptr, ptr %2, align 4, !tbaa !288
  %16 = icmp eq ptr %15, null
  br i1 %16, label %40, label %17

17:                                               ; preds = %14
  store ptr null, ptr %2, align 4, !tbaa !288
  %18 = ptrtoint ptr %15 to i32
  %19 = and i32 %18, 1
  %20 = icmp eq i32 %19, 0
  br i1 %20, label %39, label %21

21:                                               ; preds = %17
  %22 = and i32 %18, -2
  %23 = inttoptr i32 %22 to ptr
  %24 = getelementptr inbounds i8, ptr %0, i32 20
  store ptr null, ptr %24, align 4, !tbaa !279
  %25 = getelementptr inbounds %struct.FaJoin, ptr %23, i32 0, i32 3
  %26 = load ptr, ptr %25, align 4, !tbaa !291
  %27 = icmp eq ptr %26, null
  br i1 %27, label %30, label %28

28:                                               ; preds = %21
  %29 = getelementptr inbounds %struct.FaTask, ptr %26, i32 0, i32 1
  store ptr %2, ptr %29, align 4, !tbaa !279
  br label %32

30:                                               ; preds = %21
  %31 = getelementptr inbounds %struct.FaJoin, ptr %23, i32 0, i32 2
  store ptr %2, ptr %31, align 8, !tbaa !293
  br label %32

32:                                               ; preds = %30, %28
  store ptr %2, ptr %25, align 4, !tbaa !291
  %33 = getelementptr inbounds %struct.FaJoin, ptr %23, i32 0, i32 1
  %34 = load i64, ptr %33, align 8, !tbaa !294
  %35 = add nsw i64 %34, -1
  store i64 %35, ptr %33, align 8, !tbaa !294
  %36 = load ptr, ptr %23, align 8, !tbaa !295
  %37 = icmp eq ptr %36, null
  br i1 %37, label %40, label %38

38:                                               ; preds = %32
  store ptr %36, ptr @next_frame, align 4, !tbaa !6
  store ptr null, ptr %23, align 8, !tbaa !295
  br label %40

39:                                               ; preds = %17
  store ptr %15, ptr @next_frame, align 4, !tbaa !6
  br label %40

40:                                               ; preds = %38, %32, %39, %14, %9
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_task_drop(ptr noundef %0) local_unnamed_addr #3 {
  %2 = icmp ult ptr %0, inttoptr (i32 2 to ptr)
  br i1 %2, label %32, label %3

3:                                                ; preds = %1
  %4 = load ptr, ptr @current_executor, align 4, !tbaa !6
  %5 = icmp eq ptr %4, null
  br i1 %5, label %31, label %6

6:                                                ; preds = %3
  %7 = getelementptr inbounds i8, ptr %0, i32 28
  %8 = load atomic i32, ptr %7 acquire, align 4
  %9 = icmp eq i32 %8, 5
  br i1 %9, label %10, label %11

10:                                               ; preds = %6
  tail call fastcc void @drain_remote(ptr noundef nonnull %4) #53
  br label %11

11:                                               ; preds = %10, %6
  %12 = load atomic i32, ptr %7 acquire, align 4
  %13 = icmp eq i32 %12, 1
  br i1 %13, label %14, label %31

14:                                               ; preds = %11
  %15 = getelementptr inbounds i8, ptr %0, i32 24
  %16 = load ptr, ptr %15, align 4, !tbaa !281
  %17 = icmp eq ptr %16, null
  %18 = getelementptr inbounds i8, ptr %0, i32 20
  %19 = load ptr, ptr %18, align 4, !tbaa !279
  br i1 %17, label %22, label %20

20:                                               ; preds = %14
  %21 = getelementptr inbounds %struct.FaTask, ptr %16, i32 0, i32 1
  store ptr %19, ptr %21, align 4, !tbaa !279
  br label %23

22:                                               ; preds = %14
  store ptr %19, ptr %4, align 8, !tbaa !282
  br label %23

23:                                               ; preds = %20, %22
  %24 = icmp eq ptr %19, null
  %25 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 1
  %26 = getelementptr inbounds %struct.FaTask, ptr %19, i32 0, i32 2
  %27 = select i1 %24, ptr %25, ptr %26
  store ptr %16, ptr %27, align 4, !tbaa !6
  store ptr null, ptr %18, align 4, !tbaa !279
  store ptr null, ptr %15, align 4, !tbaa !281
  %28 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 2
  %29 = load i64, ptr %28, align 8, !tbaa !283
  %30 = add nsw i64 %29, -1
  store i64 %30, ptr %28, align 8, !tbaa !283
  br label %31

31:                                               ; preds = %11, %23, %3
  tail call void @fa_destroy(ptr noundef %0) #54
  br label %32

32:                                               ; preds = %1, %31
  ret void
}

; Function Attrs: nofree norecurse nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc void @drain_remote(ptr noundef %0) unnamed_addr #33 {
  %2 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 12
  %3 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 13, i32 1
  %4 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 11
  %5 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 13
  %6 = ptrtoint ptr %5 to i32
  %7 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 1
  %8 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 2
  br label %9

9:                                                ; preds = %47, %1
  %10 = load ptr, ptr %4, align 8, !tbaa !284
  %11 = getelementptr inbounds %struct.FaTask, ptr %10, i32 0, i32 1
  %12 = load atomic i32, ptr %11 acquire, align 4
  %13 = inttoptr i32 %12 to ptr
  %14 = icmp eq ptr %10, %5
  br i1 %14, label %15, label %21

15:                                               ; preds = %9
  %16 = icmp eq i32 %12, 0
  br i1 %16, label %51, label %17

17:                                               ; preds = %15
  store ptr %13, ptr %4, align 8, !tbaa !284
  %18 = getelementptr inbounds %struct.FaTask, ptr %13, i32 0, i32 1
  %19 = load atomic i32, ptr %18 acquire, align 4
  %20 = inttoptr i32 %19 to ptr
  br label %21

21:                                               ; preds = %17, %9
  %22 = phi ptr [ %20, %17 ], [ %13, %9 ]
  %23 = phi ptr [ %13, %17 ], [ %10, %9 ]
  %24 = icmp eq ptr %22, null
  br i1 %24, label %25, label %38

25:                                               ; preds = %21
  %26 = load atomic i32, ptr %2 acquire, align 4
  %27 = inttoptr i32 %26 to ptr
  %28 = icmp eq ptr %23, %27
  br i1 %28, label %29, label %51

29:                                               ; preds = %25
  store atomic i32 0, ptr %3 monotonic, align 4
  %30 = atomicrmw xchg ptr %2, i32 %6 acq_rel, align 4
  %31 = inttoptr i32 %30 to ptr
  %32 = getelementptr inbounds %struct.FaTask, ptr %31, i32 0, i32 1
  store atomic i32 %6, ptr %32 release, align 4
  %33 = getelementptr inbounds %struct.FaTask, ptr %23, i32 0, i32 1
  %34 = load atomic i32, ptr %33 acquire, align 4
  %35 = icmp eq i32 %34, 0
  br i1 %35, label %51, label %36

36:                                               ; preds = %29
  %37 = inttoptr i32 %34 to ptr
  br label %38

38:                                               ; preds = %36, %21
  %39 = phi ptr [ %37, %36 ], [ %22, %21 ]
  store ptr %39, ptr %4, align 8, !tbaa !284
  %40 = getelementptr inbounds %struct.FaTask, ptr %23, i32 0, i32 1
  store ptr null, ptr %40, align 4, !tbaa !279
  %41 = load ptr, ptr %7, align 4, !tbaa !280
  %42 = getelementptr inbounds %struct.FaTask, ptr %23, i32 0, i32 2
  store ptr %41, ptr %42, align 4, !tbaa !281
  %43 = icmp eq ptr %41, null
  br i1 %43, label %46, label %44

44:                                               ; preds = %38
  %45 = getelementptr inbounds %struct.FaTask, ptr %41, i32 0, i32 1
  store ptr %23, ptr %45, align 4, !tbaa !279
  br label %47

46:                                               ; preds = %38
  store ptr %23, ptr %0, align 8, !tbaa !282
  br label %47

47:                                               ; preds = %44, %46
  store ptr %23, ptr %7, align 4, !tbaa !280
  %48 = load i64, ptr %8, align 8, !tbaa !283
  %49 = add nsw i64 %48, 1
  store i64 %49, ptr %8, align 8, !tbaa !283
  %50 = getelementptr inbounds %struct.FaTask, ptr %23, i32 0, i32 3
  store atomic i32 1, ptr %50 release, align 4
  br label %9, !llvm.loop !296

51:                                               ; preds = %15, %25, %29
  ret void
}

; Function Attrs: optsize
declare void @fa_destroy(ptr noundef) local_unnamed_addr #4

; Function Attrs: nounwind optsize
define hidden void @fa_task_yield(ptr noundef %0) local_unnamed_addr #3 {
  %2 = tail call fastcc ptr @executor() #53
  %3 = getelementptr inbounds i8, ptr %0, i32 16
  %4 = getelementptr inbounds i8, ptr %0, i32 20
  store ptr null, ptr %4, align 4, !tbaa !279
  %5 = getelementptr inbounds %struct.FaExecutor, ptr %2, i32 0, i32 1
  %6 = load ptr, ptr %5, align 4, !tbaa !280
  %7 = getelementptr inbounds i8, ptr %0, i32 24
  store ptr %6, ptr %7, align 4, !tbaa !281
  %8 = icmp eq ptr %6, null
  br i1 %8, label %11, label %9

9:                                                ; preds = %1
  %10 = getelementptr inbounds %struct.FaTask, ptr %6, i32 0, i32 1
  store ptr %3, ptr %10, align 4, !tbaa !279
  br label %12

11:                                               ; preds = %1
  store ptr %3, ptr %2, align 8, !tbaa !282
  br label %12

12:                                               ; preds = %9, %11
  store ptr %3, ptr %5, align 4, !tbaa !280
  %13 = getelementptr inbounds %struct.FaExecutor, ptr %2, i32 0, i32 2
  %14 = load i64, ptr %13, align 8, !tbaa !283
  %15 = add nsw i64 %14, 1
  store i64 %15, ptr %13, align 8, !tbaa !283
  %16 = getelementptr inbounds i8, ptr %0, i32 28
  store atomic i32 1, ptr %16 release, align 4
  ret void
}

; Function Attrs: nounwind optsize
define hidden i32 @fa_coop_back_edge() local_unnamed_addr #3 {
  %1 = load ptr, ptr @current_executor, align 4, !tbaa !6
  store i32 128, ptr @fa_coop_budget, align 4, !tbaa !23
  %2 = icmp eq ptr %1, null
  br i1 %2, label %13, label %3

3:                                                ; preds = %0
  %4 = tail call i64 @fa_clock() #54
  %5 = getelementptr inbounds %struct.FaExecutor, ptr %1, i32 0, i32 19
  %6 = load i64, ptr %5, align 8, !tbaa !297
  %7 = icmp eq i64 %6, 0
  br i1 %7, label %8, label %9

8:                                                ; preds = %3
  store i64 %4, ptr %5, align 8, !tbaa !297
  br label %13

9:                                                ; preds = %3
  %10 = sub nsw i64 %4, %6
  %11 = icmp sgt i64 %10, 2000000
  %12 = zext i1 %11 to i32
  br label %13

13:                                               ; preds = %8, %9, %0
  %14 = phi i32 [ 0, %0 ], [ 0, %8 ], [ %12, %9 ]
  ret i32 %14
}

; Function Attrs: optsize
declare i64 @fa_clock() local_unnamed_addr #4

; Function Attrs: nounwind optsize
define hidden void @fa_sleep_async(ptr noundef %0, ptr noundef %1, i64 noundef %2) local_unnamed_addr #3 {
  %4 = tail call fastcc ptr @executor() #53
  store ptr %1, ptr %0, align 8, !tbaa !298
  %5 = getelementptr inbounds %struct.FaWait, ptr %0, i32 0, i32 1
  store ptr %4, ptr %5, align 4, !tbaa !300
  %6 = getelementptr inbounds %struct.FaWait, ptr %0, i32 0, i32 8
  store i64 -1, ptr %6, align 8, !tbaa !301
  %7 = icmp slt i64 %2, 1
  br i1 %7, label %8, label %23

8:                                                ; preds = %3
  %9 = getelementptr inbounds i8, ptr %1, i32 16
  %10 = getelementptr inbounds i8, ptr %1, i32 20
  store ptr null, ptr %10, align 4, !tbaa !279
  %11 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 1
  %12 = load ptr, ptr %11, align 4, !tbaa !280
  %13 = getelementptr inbounds i8, ptr %1, i32 24
  store ptr %12, ptr %13, align 4, !tbaa !281
  %14 = icmp eq ptr %12, null
  br i1 %14, label %17, label %15

15:                                               ; preds = %8
  %16 = getelementptr inbounds %struct.FaTask, ptr %12, i32 0, i32 1
  store ptr %9, ptr %16, align 4, !tbaa !279
  br label %18

17:                                               ; preds = %8
  store ptr %9, ptr %4, align 8, !tbaa !282
  br label %18

18:                                               ; preds = %15, %17
  store ptr %9, ptr %11, align 4, !tbaa !280
  %19 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 2
  %20 = load i64, ptr %19, align 8, !tbaa !283
  %21 = add nsw i64 %20, 1
  store i64 %21, ptr %19, align 8, !tbaa !283
  %22 = getelementptr inbounds i8, ptr %1, i32 28
  store atomic i32 1, ptr %22 release, align 4
  br label %82

23:                                               ; preds = %3
  %24 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 4
  %25 = load i64, ptr %24, align 8, !tbaa !302
  %26 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 5
  %27 = load i64, ptr %26, align 8, !tbaa !303
  %28 = icmp eq i64 %25, %27
  br i1 %28, label %29, label %59

29:                                               ; preds = %23
  %30 = icmp slt i64 %25, 16
  %31 = shl nuw nsw i64 %25, 1
  %32 = select i1 %30, i64 16, i64 %31
  %33 = trunc i64 %32 to i32
  %34 = shl i32 %33, 2
  %35 = tail call ptr @fa_alloc(i32 noundef %34, i32 noundef 4) #54
  %36 = load i64, ptr %24, align 8, !tbaa !302
  %37 = icmp sgt i64 %36, 0
  br i1 %37, label %38, label %40

38:                                               ; preds = %29
  %39 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 3
  br label %43

40:                                               ; preds = %43, %29
  %41 = load i64, ptr %26, align 8, !tbaa !303
  %42 = icmp sgt i64 %41, 0
  br i1 %42, label %52, label %57

43:                                               ; preds = %38, %43
  %44 = phi i64 [ 0, %38 ], [ %50, %43 ]
  %45 = load ptr, ptr %39, align 8, !tbaa !304
  %46 = trunc i64 %44 to i32
  %47 = getelementptr inbounds ptr, ptr %45, i32 %46
  %48 = load ptr, ptr %47, align 4, !tbaa !6
  %49 = getelementptr inbounds ptr, ptr %35, i32 %46
  store ptr %48, ptr %49, align 4, !tbaa !6
  %50 = add nuw nsw i64 %44, 1
  %51 = icmp eq i64 %50, %36
  br i1 %51, label %40, label %43, !llvm.loop !305

52:                                               ; preds = %40
  %53 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 3
  %54 = load ptr, ptr %53, align 8, !tbaa !304
  %55 = trunc i64 %41 to i32
  %56 = shl i32 %55, 2
  tail call void @fa_free(ptr noundef %54, i32 noundef %56, i32 noundef 4) #54
  br label %57

57:                                               ; preds = %52, %40
  %58 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 3
  store ptr %35, ptr %58, align 8, !tbaa !304
  store i64 %32, ptr %26, align 8, !tbaa !303
  br label %59

59:                                               ; preds = %57, %23
  %60 = tail call i64 @fa_clock() #54
  %61 = mul nsw i64 %2, 1000000
  %62 = add nsw i64 %60, %61
  %63 = getelementptr inbounds %struct.FaWait, ptr %0, i32 0, i32 6
  store i64 %62, ptr %63, align 8, !tbaa !306
  %64 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 6
  %65 = load i64, ptr %64, align 8, !tbaa !307
  %66 = getelementptr inbounds %struct.FaWait, ptr %0, i32 0, i32 7
  store i64 %65, ptr %66, align 8, !tbaa !308
  %67 = add nsw i64 %65, 1
  store i64 %67, ptr %64, align 8, !tbaa !307
  %68 = load i64, ptr %24, align 8, !tbaa !302
  %69 = add nsw i64 %68, 1
  store i64 %69, ptr %24, align 8, !tbaa !302
  %70 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 3
  %71 = load ptr, ptr %70, align 8, !tbaa !304
  %72 = trunc i64 %68 to i32
  %73 = getelementptr inbounds ptr, ptr %71, i32 %72
  store ptr %0, ptr %73, align 4, !tbaa !6
  store i64 %68, ptr %6, align 8, !tbaa !301
  tail call fastcc void @timer_up(ptr noundef nonnull %4, i64 noundef %68) #53
  %74 = getelementptr inbounds i8, ptr %1, i32 28
  store atomic i32 2, ptr %74 release, align 4
  %75 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 7
  %76 = load ptr, ptr %75, align 8, !tbaa !289
  %77 = icmp eq ptr %76, %1
  %78 = icmp eq ptr %76, null
  %79 = or i1 %77, %78
  br i1 %79, label %80, label %82

80:                                               ; preds = %59
  %81 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 20
  store ptr null, ptr %81, align 8, !tbaa !290
  br label %82

82:                                               ; preds = %80, %59, %18
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc void @timer_up(ptr nocapture noundef readonly %0, i64 noundef %1) unnamed_addr #19 {
  %3 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 3
  %4 = load ptr, ptr %3, align 8, !tbaa !304
  %5 = trunc i64 %1 to i32
  %6 = getelementptr inbounds ptr, ptr %4, i32 %5
  %7 = load ptr, ptr %6, align 4, !tbaa !6
  %8 = getelementptr inbounds %struct.FaWait, ptr %7, i32 0, i32 7
  %9 = icmp sgt i64 %1, 0
  br i1 %9, label %10, label %40

10:                                               ; preds = %2
  %11 = getelementptr inbounds %struct.FaWait, ptr %7, i32 0, i32 6
  %12 = load i64, ptr %11, align 8, !tbaa !306
  br label %13

13:                                               ; preds = %10, %31
  %14 = phi i64 [ %1, %10 ], [ %16, %31 ]
  %15 = add nsw i64 %14, -1
  %16 = lshr i64 %15, 2
  %17 = load ptr, ptr %3, align 8, !tbaa !304
  %18 = trunc i64 %16 to i32
  %19 = getelementptr inbounds ptr, ptr %17, i32 %18
  %20 = load ptr, ptr %19, align 4, !tbaa !6
  %21 = getelementptr inbounds %struct.FaWait, ptr %20, i32 0, i32 6
  %22 = load i64, ptr %21, align 8, !tbaa !306
  %23 = icmp slt i64 %12, %22
  br i1 %23, label %31, label %24

24:                                               ; preds = %13
  %25 = icmp eq i64 %12, %22
  br i1 %25, label %26, label %38

26:                                               ; preds = %24
  %27 = load i64, ptr %8, align 8, !tbaa !308
  %28 = getelementptr inbounds %struct.FaWait, ptr %20, i32 0, i32 7
  %29 = load i64, ptr %28, align 8, !tbaa !308
  %30 = icmp slt i64 %27, %29
  br i1 %30, label %31, label %38

31:                                               ; preds = %13, %26
  %32 = trunc i64 %14 to i32
  %33 = getelementptr inbounds ptr, ptr %17, i32 %32
  store ptr %20, ptr %33, align 4, !tbaa !6
  %34 = getelementptr inbounds %struct.FaWait, ptr %20, i32 0, i32 8
  store i64 %14, ptr %34, align 8, !tbaa !301
  %35 = icmp ult i64 %15, 4
  br i1 %35, label %36, label %13

36:                                               ; preds = %31
  %37 = load ptr, ptr %3, align 8, !tbaa !304
  br label %40

38:                                               ; preds = %26, %24
  %39 = trunc i64 %14 to i32
  br label %40

40:                                               ; preds = %36, %38, %2
  %41 = phi i32 [ %5, %2 ], [ %39, %38 ], [ %18, %36 ]
  %42 = phi ptr [ %4, %2 ], [ %17, %38 ], [ %37, %36 ]
  %43 = phi i64 [ %1, %2 ], [ %14, %38 ], [ %16, %36 ]
  %44 = getelementptr inbounds ptr, ptr %42, i32 %41
  store ptr %7, ptr %44, align 4, !tbaa !6
  %45 = getelementptr inbounds %struct.FaWait, ptr %7, i32 0, i32 8
  store i64 %43, ptr %45, align 8, !tbaa !301
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden void @fa_timer_cancel(ptr nocapture noundef readonly %0) local_unnamed_addr #19 {
  %2 = getelementptr inbounds %struct.FaWait, ptr %0, i32 0, i32 8
  %3 = load i64, ptr %2, align 8, !tbaa !301
  %4 = icmp sgt i64 %3, -1
  br i1 %4, label %5, label %8

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaWait, ptr %0, i32 0, i32 1
  %7 = load ptr, ptr %6, align 4, !tbaa !300
  tail call fastcc void @timer_remove(ptr noundef %7, i64 noundef %3) #53
  br label %8

8:                                                ; preds = %5, %1
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc void @timer_remove(ptr nocapture noundef %0, i64 noundef %1) unnamed_addr #19 {
  %3 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 3
  %4 = load ptr, ptr %3, align 8, !tbaa !304
  %5 = trunc i64 %1 to i32
  %6 = getelementptr inbounds ptr, ptr %4, i32 %5
  %7 = load ptr, ptr %6, align 4, !tbaa !6
  %8 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 4
  %9 = load i64, ptr %8, align 8, !tbaa !302
  %10 = add nsw i64 %9, -1
  store i64 %10, ptr %8, align 8, !tbaa !302
  %11 = icmp sgt i64 %10, %1
  br i1 %11, label %12, label %118

12:                                               ; preds = %2
  %13 = trunc i64 %10 to i32
  %14 = getelementptr inbounds ptr, ptr %4, i32 %13
  %15 = load ptr, ptr %14, align 4, !tbaa !6
  store ptr %15, ptr %6, align 4, !tbaa !6
  %16 = getelementptr inbounds %struct.FaWait, ptr %15, i32 0, i32 8
  store i64 %1, ptr %16, align 8, !tbaa !301
  %17 = icmp sgt i64 %1, 0
  %18 = load ptr, ptr %3, align 8, !tbaa !304
  br i1 %17, label %19, label %39

19:                                               ; preds = %12
  %20 = add nuw i64 %1, 17179869183
  %21 = lshr i64 %20, 2
  %22 = trunc i64 %21 to i32
  %23 = getelementptr inbounds ptr, ptr %18, i32 %22
  %24 = load ptr, ptr %23, align 4, !tbaa !6
  %25 = getelementptr inbounds %struct.FaWait, ptr %15, i32 0, i32 6
  %26 = load i64, ptr %25, align 8, !tbaa !306
  %27 = getelementptr inbounds %struct.FaWait, ptr %24, i32 0, i32 6
  %28 = load i64, ptr %27, align 8, !tbaa !306
  %29 = icmp slt i64 %26, %28
  br i1 %29, label %38, label %30

30:                                               ; preds = %19
  %31 = icmp eq i64 %26, %28
  br i1 %31, label %32, label %39

32:                                               ; preds = %30
  %33 = getelementptr inbounds %struct.FaWait, ptr %15, i32 0, i32 7
  %34 = load i64, ptr %33, align 8, !tbaa !308
  %35 = getelementptr inbounds %struct.FaWait, ptr %24, i32 0, i32 7
  %36 = load i64, ptr %35, align 8, !tbaa !308
  %37 = icmp slt i64 %34, %36
  br i1 %37, label %38, label %39

38:                                               ; preds = %19, %32
  tail call fastcc void @timer_up(ptr noundef nonnull %0, i64 noundef %1) #53
  br label %118

39:                                               ; preds = %30, %32, %12
  %40 = getelementptr inbounds ptr, ptr %18, i32 %5
  %41 = load ptr, ptr %40, align 4, !tbaa !6
  %42 = shl nsw i64 %1, 2
  %43 = or disjoint i64 %42, 1
  %44 = icmp slt i64 %43, %10
  br i1 %44, label %45, label %112

45:                                               ; preds = %39
  %46 = getelementptr inbounds %struct.FaWait, ptr %41, i32 0, i32 6
  %47 = load i64, ptr %46, align 8, !tbaa !306
  %48 = getelementptr inbounds %struct.FaWait, ptr %41, i32 0, i32 7
  br label %49

49:                                               ; preds = %101, %45
  %50 = phi i64 [ %43, %45 ], [ %106, %101 ]
  %51 = phi i64 [ %42, %45 ], [ %105, %101 ]
  %52 = phi i64 [ %1, %45 ], [ %59, %101 ]
  %53 = add i64 %51, 5
  %54 = tail call i64 @llvm.smin.i64(i64 %53, i64 %10)
  %55 = or disjoint i64 %51, 2
  %56 = icmp slt i64 %55, %54
  %57 = load ptr, ptr %3, align 8, !tbaa !304
  br i1 %56, label %73, label %58

58:                                               ; preds = %97, %49
  %59 = phi i64 [ %50, %49 ], [ %98, %97 ]
  %60 = trunc i64 %59 to i32
  %61 = getelementptr inbounds ptr, ptr %57, i32 %60
  %62 = load ptr, ptr %61, align 4, !tbaa !6
  %63 = getelementptr inbounds %struct.FaWait, ptr %62, i32 0, i32 6
  %64 = load i64, ptr %63, align 8, !tbaa !306
  %65 = icmp slt i64 %64, %47
  br i1 %65, label %101, label %66

66:                                               ; preds = %58
  %67 = icmp eq i64 %64, %47
  br i1 %67, label %68, label %110

68:                                               ; preds = %66
  %69 = getelementptr inbounds %struct.FaWait, ptr %62, i32 0, i32 7
  %70 = load i64, ptr %69, align 8, !tbaa !308
  %71 = load i64, ptr %48, align 8, !tbaa !308
  %72 = icmp slt i64 %70, %71
  br i1 %72, label %101, label %110

73:                                               ; preds = %49, %97
  %74 = phi i64 [ %99, %97 ], [ %55, %49 ]
  %75 = phi i64 [ %98, %97 ], [ %50, %49 ]
  %76 = trunc i64 %74 to i32
  %77 = getelementptr inbounds ptr, ptr %57, i32 %76
  %78 = load ptr, ptr %77, align 4, !tbaa !6
  %79 = trunc i64 %75 to i32
  %80 = getelementptr inbounds ptr, ptr %57, i32 %79
  %81 = load ptr, ptr %80, align 4, !tbaa !6
  %82 = getelementptr inbounds %struct.FaWait, ptr %78, i32 0, i32 6
  %83 = load i64, ptr %82, align 8, !tbaa !306
  %84 = getelementptr inbounds %struct.FaWait, ptr %81, i32 0, i32 6
  %85 = load i64, ptr %84, align 8, !tbaa !306
  %86 = icmp slt i64 %83, %85
  br i1 %86, label %97, label %87

87:                                               ; preds = %73
  %88 = icmp eq i64 %83, %85
  br i1 %88, label %89, label %96

89:                                               ; preds = %87
  %90 = getelementptr inbounds %struct.FaWait, ptr %78, i32 0, i32 7
  %91 = load i64, ptr %90, align 8, !tbaa !308
  %92 = getelementptr inbounds %struct.FaWait, ptr %81, i32 0, i32 7
  %93 = load i64, ptr %92, align 8, !tbaa !308
  %94 = icmp sge i64 %91, %93
  %95 = freeze i1 %94
  br i1 %95, label %96, label %97

96:                                               ; preds = %89, %87
  br label %97

97:                                               ; preds = %96, %89, %73
  %98 = phi i64 [ %75, %96 ], [ %74, %89 ], [ %74, %73 ]
  %99 = add nsw i64 %74, 1
  %100 = icmp eq i64 %99, %54
  br i1 %100, label %58, label %73, !llvm.loop !309

101:                                              ; preds = %68, %58
  %102 = trunc i64 %52 to i32
  %103 = getelementptr inbounds ptr, ptr %57, i32 %102
  store ptr %62, ptr %103, align 4, !tbaa !6
  %104 = getelementptr inbounds %struct.FaWait, ptr %62, i32 0, i32 8
  store i64 %52, ptr %104, align 8, !tbaa !301
  %105 = shl nsw i64 %59, 2
  %106 = or disjoint i64 %105, 1
  %107 = icmp slt i64 %106, %10
  br i1 %107, label %49, label %108

108:                                              ; preds = %101
  %109 = load ptr, ptr %3, align 8, !tbaa !304
  br label %112

110:                                              ; preds = %68, %66
  %111 = trunc i64 %52 to i32
  br label %112

112:                                              ; preds = %39, %108, %110
  %113 = phi i32 [ %5, %39 ], [ %111, %110 ], [ %60, %108 ]
  %114 = phi ptr [ %18, %39 ], [ %57, %110 ], [ %109, %108 ]
  %115 = phi i64 [ %1, %39 ], [ %52, %110 ], [ %59, %108 ]
  %116 = getelementptr inbounds ptr, ptr %114, i32 %113
  store ptr %41, ptr %116, align 4, !tbaa !6
  %117 = getelementptr inbounds %struct.FaWait, ptr %41, i32 0, i32 8
  store i64 %115, ptr %117, align 8, !tbaa !301
  br label %118

118:                                              ; preds = %38, %112, %2
  %119 = getelementptr inbounds %struct.FaWait, ptr %7, i32 0, i32 8
  store i64 -1, ptr %119, align 8, !tbaa !301
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_sleep(i64 noundef %0) local_unnamed_addr #3 {
  %2 = icmp sgt i64 %0, 0
  br i1 %2, label %3, label %5

3:                                                ; preds = %1
  %4 = mul nsw i64 %0, 1000000
  tail call void @fa_nanosleep(i64 noundef %4) #54
  br label %5

5:                                                ; preds = %3, %1
  ret void
}

; Function Attrs: optsize
declare void @fa_nanosleep(i64 noundef) local_unnamed_addr #4

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: write)
define hidden void @fa_join_init(ptr nocapture noundef writeonly %0) local_unnamed_addr #23 {
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(24) %0, i8 0, i32 24, i1 false)
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, argmem: readwrite, inaccessiblemem: none)
define hidden void @fa_join_add(ptr noundef %0, ptr noundef %1) local_unnamed_addr #44 {
  %3 = icmp ult ptr %1, inttoptr (i32 2 to ptr)
  br i1 %3, label %25, label %4

4:                                                ; preds = %2
  %5 = getelementptr inbounds i8, ptr %1, i32 16
  %6 = load ptr, ptr %1, align 4, !tbaa !6
  %7 = icmp eq ptr %6, null
  br i1 %7, label %8, label %18

8:                                                ; preds = %4
  %9 = getelementptr inbounds i8, ptr %1, i32 20
  store ptr null, ptr %9, align 4, !tbaa !279
  %10 = getelementptr inbounds %struct.FaJoin, ptr %0, i32 0, i32 3
  %11 = load ptr, ptr %10, align 4, !tbaa !291
  %12 = icmp eq ptr %11, null
  br i1 %12, label %15, label %13

13:                                               ; preds = %8
  %14 = getelementptr inbounds %struct.FaTask, ptr %11, i32 0, i32 1
  store ptr %5, ptr %14, align 4, !tbaa !279
  br label %17

15:                                               ; preds = %8
  %16 = getelementptr inbounds %struct.FaJoin, ptr %0, i32 0, i32 2
  store ptr %5, ptr %16, align 8, !tbaa !293
  br label %17

17:                                               ; preds = %13, %15
  store ptr %5, ptr %10, align 4, !tbaa !291
  br label %25

18:                                               ; preds = %4
  %19 = ptrtoint ptr %0 to i32
  %20 = or i32 %19, 1
  %21 = inttoptr i32 %20 to ptr
  store ptr %21, ptr %5, align 4, !tbaa !288
  %22 = getelementptr inbounds %struct.FaJoin, ptr %0, i32 0, i32 1
  %23 = load i64, ptr %22, align 8, !tbaa !294
  %24 = add nsw i64 %23, 1
  store i64 %24, ptr %22, align 8, !tbaa !294
  br label %25

25:                                               ; preds = %17, %18, %2
  ret void
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define hidden ptr @fa_join_next(ptr nocapture noundef %0, ptr noundef %1) local_unnamed_addr #47 {
  %3 = getelementptr inbounds %struct.FaJoin, ptr %0, i32 0, i32 2
  %4 = load ptr, ptr %3, align 8, !tbaa !293
  %5 = icmp eq ptr %4, null
  br i1 %5, label %14, label %6

6:                                                ; preds = %2
  %7 = getelementptr inbounds %struct.FaTask, ptr %4, i32 0, i32 1
  %8 = load ptr, ptr %7, align 4, !tbaa !279
  store ptr %8, ptr %3, align 8, !tbaa !293
  %9 = icmp eq ptr %8, null
  br i1 %9, label %10, label %12

10:                                               ; preds = %6
  %11 = getelementptr inbounds %struct.FaJoin, ptr %0, i32 0, i32 3
  store ptr null, ptr %11, align 4, !tbaa !291
  br label %12

12:                                               ; preds = %10, %6
  store ptr null, ptr %7, align 4, !tbaa !279
  %13 = getelementptr inbounds i8, ptr %4, i32 -16
  br label %22

14:                                               ; preds = %2
  %15 = getelementptr inbounds %struct.FaJoin, ptr %0, i32 0, i32 1
  %16 = load i64, ptr %15, align 8, !tbaa !294
  %17 = icmp eq i64 %16, 0
  br i1 %17, label %22, label %18

18:                                               ; preds = %14
  %19 = icmp eq ptr %1, null
  br i1 %19, label %22, label %20

20:                                               ; preds = %18
  store ptr %1, ptr %0, align 8, !tbaa !295
  %21 = getelementptr inbounds i8, ptr %1, i32 28
  store atomic i32 2, ptr %21 release, align 4
  br label %22

22:                                               ; preds = %18, %14, %20, %12
  %23 = phi ptr [ %13, %12 ], [ null, %20 ], [ inttoptr (i32 1 to ptr), %14 ], [ null, %18 ]
  ret ptr %23
}

; Function Attrs: cold noinline noreturn nounwind optsize
define hidden void @fa_task_stuck() local_unnamed_addr #38 {
  tail call fastcc void @deadlock(ptr noundef nonnull @.str.30) #59
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define internal fastcc void @deadlock(ptr noundef %0) unnamed_addr #38 {
  tail call void @fa_flush() #53
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.38, i32 noundef 16) #54
  br label %2

2:                                                ; preds = %2, %1
  %3 = phi i32 [ 0, %1 ], [ %7, %2 ]
  %4 = getelementptr inbounds i8, ptr %0, i32 %3
  %5 = load i8, ptr %4, align 1, !tbaa !1
  %6 = icmp eq i8 %5, 0
  %7 = add i32 %3, 1
  br i1 %6, label %8, label %2, !llvm.loop !27

8:                                                ; preds = %2
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull %0, i32 noundef %3) #54
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.22, i32 noundef 1) #54
  tail call void @fa_exit(i32 noundef 101) #55
  unreachable
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(none)
define hidden noalias noundef ptr @fa_io_register(i32 noundef %0) local_unnamed_addr #22 {
  ret ptr null
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define hidden noundef i32 @fa_io_wait(ptr nocapture noundef %0, i32 noundef %1, ptr noundef %2, ptr noundef %3) local_unnamed_addr #47 {
  %5 = icmp eq i32 %1, 0
  %6 = select i1 %5, i32 1, i32 4
  %7 = getelementptr inbounds %struct.FaIo, ptr %0, i32 0, i32 1
  %8 = load i32, ptr %7, align 4, !tbaa !310
  %9 = and i32 %8, %6
  %10 = icmp eq i32 %9, 0
  br i1 %10, label %14, label %11

11:                                               ; preds = %4
  %12 = xor i32 %6, -1
  %13 = and i32 %8, %12
  store i32 %13, ptr %7, align 4, !tbaa !310
  br label %14

14:                                               ; preds = %11, %4
  store ptr %3, ptr %2, align 8, !tbaa !298
  %15 = getelementptr inbounds %struct.FaIo, ptr %0, i32 0, i32 4
  %16 = load ptr, ptr %15, align 4, !tbaa !312
  %17 = getelementptr inbounds %struct.FaWait, ptr %2, i32 0, i32 1
  store ptr %16, ptr %17, align 4, !tbaa !300
  %18 = getelementptr inbounds %struct.FaWait, ptr %2, i32 0, i32 9
  store i32 0, ptr %18, align 8, !tbaa !313
  %19 = getelementptr inbounds %struct.FaWait, ptr %2, i32 0, i32 10
  store i32 0, ptr %19, align 4, !tbaa !314
  %20 = getelementptr inbounds %struct.FaWait, ptr %2, i32 0, i32 8
  store i64 -1, ptr %20, align 8, !tbaa !301
  %21 = getelementptr inbounds %struct.FaIo, ptr %0, i32 0, i32 3
  %22 = getelementptr inbounds %struct.FaIo, ptr %0, i32 0, i32 2
  %23 = select i1 %5, ptr %22, ptr %21
  store ptr %2, ptr %23, align 4, !tbaa !6
  %24 = getelementptr inbounds %struct.FaExecutor, ptr %16, i32 0, i32 10
  %25 = load i64, ptr %24, align 8, !tbaa !315
  %26 = add nsw i64 %25, 1
  store i64 %26, ptr %24, align 8, !tbaa !315
  %27 = getelementptr inbounds i8, ptr %3, i32 28
  store atomic i32 2, ptr %27 release, align 4
  ret i32 1
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define hidden void @fa_io_cancel(ptr nocapture noundef %0, ptr noundef readnone %1) local_unnamed_addr #43 {
  %3 = getelementptr inbounds %struct.FaIo, ptr %0, i32 0, i32 2
  %4 = load ptr, ptr %3, align 4, !tbaa !316
  %5 = icmp eq ptr %4, %1
  br i1 %5, label %6, label %12

6:                                                ; preds = %2
  store ptr null, ptr %3, align 4, !tbaa !316
  %7 = getelementptr inbounds %struct.FaIo, ptr %0, i32 0, i32 4
  %8 = load ptr, ptr %7, align 4, !tbaa !312
  %9 = getelementptr inbounds %struct.FaExecutor, ptr %8, i32 0, i32 10
  %10 = load i64, ptr %9, align 8, !tbaa !315
  %11 = add nsw i64 %10, -1
  store i64 %11, ptr %9, align 8, !tbaa !315
  br label %12

12:                                               ; preds = %6, %2
  %13 = getelementptr inbounds %struct.FaIo, ptr %0, i32 0, i32 3
  %14 = load ptr, ptr %13, align 4, !tbaa !317
  %15 = icmp eq ptr %14, %1
  br i1 %15, label %16, label %22

16:                                               ; preds = %12
  store ptr null, ptr %13, align 4, !tbaa !317
  %17 = getelementptr inbounds %struct.FaIo, ptr %0, i32 0, i32 4
  %18 = load ptr, ptr %17, align 4, !tbaa !312
  %19 = getelementptr inbounds %struct.FaExecutor, ptr %18, i32 0, i32 10
  %20 = load i64, ptr %19, align 8, !tbaa !315
  %21 = add nsw i64 %20, -1
  store i64 %21, ptr %19, align 8, !tbaa !315
  br label %22

22:                                               ; preds = %16, %12
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_io_close(ptr noundef %0) local_unnamed_addr #3 {
  %2 = icmp eq ptr %0, null
  br i1 %2, label %4, label %3

3:                                                ; preds = %1
  tail call void @fa_free(ptr noundef nonnull %0, i32 noundef 20, i32 noundef 16) #54
  br label %4

4:                                                ; preds = %1, %3
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_run_main(ptr noundef %0, i32 noundef %1) local_unnamed_addr #3 {
  %3 = tail call fastcc ptr @executor() #53
  %4 = getelementptr inbounds %struct.FaExecutor, ptr %3, i32 0, i32 7
  store ptr %0, ptr %4, align 8, !tbaa !289
  %5 = getelementptr inbounds %struct.FaExecutor, ptr %3, i32 0, i32 17
  store i32 %1, ptr %5, align 8, !tbaa !318
  %6 = getelementptr inbounds %struct.FaExecutor, ptr %3, i32 0, i32 8
  %7 = getelementptr inbounds %struct.FaExecutor, ptr %3, i32 0, i32 9
  %8 = getelementptr inbounds %struct.FaExecutor, ptr %3, i32 0, i32 2
  %9 = getelementptr inbounds %struct.FaExecutor, ptr %3, i32 0, i32 1
  %10 = getelementptr inbounds %struct.FaExecutor, ptr %3, i32 0, i32 18
  %11 = getelementptr inbounds %struct.FaExecutor, ptr %3, i32 0, i32 4
  %12 = getelementptr inbounds %struct.FaExecutor, ptr %3, i32 0, i32 10
  %13 = getelementptr inbounds %struct.FaExecutor, ptr %3, i32 0, i32 3
  %14 = getelementptr inbounds %struct.FaExecutor, ptr %3, i32 0, i32 15
  br label %15

15:                                               ; preds = %62, %2
  %16 = load ptr, ptr %0, align 4, !tbaa !6
  %17 = icmp eq ptr %16, null
  br i1 %17, label %18, label %24

18:                                               ; preds = %15
  %19 = load i64, ptr %6, align 8, !tbaa !277
  %20 = icmp sgt i64 %19, 0
  br i1 %20, label %24, label %21

21:                                               ; preds = %18
  %22 = load atomic i64, ptr %7 acquire, align 8
  %23 = icmp sgt i64 %22, 0
  br i1 %23, label %24, label %95

24:                                               ; preds = %18, %15, %21
  %25 = load i64, ptr %8, align 8, !tbaa !283
  %26 = icmp sgt i64 %25, 0
  br i1 %26, label %27, label %47

27:                                               ; preds = %24
  %28 = tail call i64 @llvm.smin.i64(i64 %25, i64 61)
  br label %29

29:                                               ; preds = %33, %27
  %30 = phi i64 [ %28, %27 ], [ %45, %33 ]
  %31 = load ptr, ptr %3, align 8, !tbaa !282
  %32 = icmp eq ptr %31, null
  br i1 %32, label %47, label %33

33:                                               ; preds = %29
  %34 = getelementptr inbounds %struct.FaTask, ptr %31, i32 0, i32 1
  %35 = load ptr, ptr %34, align 4, !tbaa !279
  store ptr %35, ptr %3, align 8, !tbaa !282
  %36 = icmp eq ptr %35, null
  %37 = getelementptr inbounds %struct.FaTask, ptr %35, i32 0, i32 2
  %38 = select i1 %36, ptr %9, ptr %37
  store ptr null, ptr %38, align 4, !tbaa !6
  %39 = load i64, ptr %8, align 8, !tbaa !283
  %40 = add nsw i64 %39, -1
  store i64 %40, ptr %8, align 8, !tbaa !283
  store ptr null, ptr %34, align 4, !tbaa !279
  %41 = getelementptr inbounds %struct.FaTask, ptr %31, i32 0, i32 2
  store ptr null, ptr %41, align 4, !tbaa !281
  %42 = getelementptr inbounds i8, ptr %31, i32 -16
  tail call fastcc void @turn(ptr noundef nonnull %3, ptr noundef nonnull %42) #53
  %43 = load i64, ptr %10, align 8, !tbaa !319
  %44 = add nsw i64 %43, 1
  store i64 %44, ptr %10, align 8, !tbaa !319
  %45 = add nsw i64 %30, -1
  %46 = icmp sgt i64 %30, 1
  br i1 %46, label %29, label %47, !llvm.loop !320

47:                                               ; preds = %33, %29, %24
  %48 = load i64, ptr %11, align 8, !tbaa !302
  %49 = icmp sgt i64 %48, 0
  br i1 %49, label %50, label %55

50:                                               ; preds = %47
  %51 = tail call i64 @fa_clock() #54
  %52 = load i64, ptr %11, align 8, !tbaa !302
  %53 = icmp sgt i64 %52, 0
  br i1 %53, label %54, label %55

54:                                               ; preds = %50
  tail call fastcc void @fire_timers(ptr noundef nonnull %3, i64 noundef %51) #53
  br label %55

55:                                               ; preds = %54, %50, %47
  %56 = phi i64 [ %51, %54 ], [ %51, %50 ], [ 0, %47 ]
  tail call fastcc void @drain_remote(ptr noundef nonnull %3) #53
  %57 = load i64, ptr %8, align 8, !tbaa !283
  %58 = icmp sgt i64 %57, 0
  br i1 %58, label %59, label %67

59:                                               ; preds = %55
  %60 = load i32, ptr %14, align 8, !tbaa !286
  %61 = icmp sgt i32 %60, -1
  br i1 %61, label %63, label %62

62:                                               ; preds = %59, %63, %66, %74, %80, %87, %88, %91
  br label %15, !llvm.loop !321

63:                                               ; preds = %59
  %64 = load i64, ptr %10, align 8, !tbaa !319
  %65 = icmp sgt i64 %64, 60
  br i1 %65, label %66, label %62

66:                                               ; preds = %63
  store i64 0, ptr %10, align 8, !tbaa !319
  br label %62

67:                                               ; preds = %55
  store i64 0, ptr %10, align 8, !tbaa !319
  %68 = load ptr, ptr %4, align 8, !tbaa !289
  %69 = load ptr, ptr %68, align 4, !tbaa !6
  %70 = icmp eq ptr %69, null
  br i1 %70, label %71, label %77

71:                                               ; preds = %67
  %72 = load i64, ptr %6, align 8, !tbaa !277
  %73 = icmp eq i64 %72, 0
  br i1 %73, label %74, label %77

74:                                               ; preds = %71
  %75 = load atomic i64, ptr %7 acquire, align 8
  %76 = icmp eq i64 %75, 0
  br i1 %76, label %62, label %77

77:                                               ; preds = %74, %71, %67
  %78 = load i64, ptr %11, align 8, !tbaa !302
  %79 = icmp sgt i64 %78, 0
  br i1 %79, label %80, label %88

80:                                               ; preds = %77
  %81 = load ptr, ptr %13, align 8, !tbaa !304
  %82 = load ptr, ptr %81, align 4, !tbaa !6
  %83 = getelementptr inbounds %struct.FaWait, ptr %82, i32 0, i32 6
  %84 = load i64, ptr %83, align 8, !tbaa !306
  %85 = sub nsw i64 %84, %56
  %86 = icmp slt i64 %85, 1
  br i1 %86, label %62, label %87

87:                                               ; preds = %80
  tail call void @fa_nanosleep(i64 noundef %85) #54
  br label %62

88:                                               ; preds = %77
  %89 = load atomic i64, ptr %7 acquire, align 8
  %90 = icmp sgt i64 %89, 0
  br i1 %90, label %62, label %91

91:                                               ; preds = %88
  %92 = load i64, ptr %12, align 8, !tbaa !315
  %93 = icmp sgt i64 %92, 0
  br i1 %93, label %62, label %94

94:                                               ; preds = %91
  tail call fastcc void @deadlock_stop(ptr noundef nonnull %3) #59
  unreachable

95:                                               ; preds = %21
  tail call void @fa_destroy(ptr noundef nonnull %0) #54
  %96 = getelementptr inbounds %struct.FaExecutor, ptr %3, i32 0, i32 5
  %97 = load i64, ptr %96, align 8, !tbaa !303
  %98 = icmp sgt i64 %97, 0
  br i1 %98, label %99, label %103

99:                                               ; preds = %95
  %100 = load ptr, ptr %13, align 8, !tbaa !304
  %101 = trunc i64 %97 to i32
  %102 = shl i32 %101, 2
  tail call void @fa_free(ptr noundef %100, i32 noundef %102, i32 noundef 4) #54
  br label %103

103:                                              ; preds = %99, %95
  store ptr null, ptr @current_executor, align 4, !tbaa !6
  tail call void @fa_free(ptr noundef nonnull %3, i32 noundef 288, i32 noundef 16) #54
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_wasm_start(ptr noundef %0, i32 noundef %1, ptr noundef %2) local_unnamed_addr #3 {
  %4 = tail call fastcc ptr @executor() #53
  %5 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 7
  store ptr %0, ptr %5, align 8, !tbaa !289
  %6 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 17
  store i32 %1, ptr %6, align 8, !tbaa !318
  store ptr %2, ptr @wasm_finish, align 4, !tbaa !6
  ret void
}

; Function Attrs: nounwind optsize
define hidden noundef i32 @fa_wasm_run() local_unnamed_addr #3 {
  %1 = load ptr, ptr @current_executor, align 4, !tbaa !6
  %2 = icmp eq ptr %1, null
  br i1 %2, label %83, label %3

3:                                                ; preds = %0
  %4 = getelementptr inbounds %struct.FaExecutor, ptr %1, i32 0, i32 4
  %5 = load i64, ptr %4, align 8, !tbaa !302
  %6 = icmp sgt i64 %5, 0
  br i1 %6, label %7, label %9

7:                                                ; preds = %3
  %8 = tail call i64 @fa_clock() #54
  tail call fastcc void @fire_timers(ptr noundef nonnull %1, i64 noundef %8) #53
  br label %9

9:                                                ; preds = %7, %3
  %10 = getelementptr inbounds %struct.FaExecutor, ptr %1, i32 0, i32 2
  %11 = load i64, ptr %10, align 8, !tbaa !283
  %12 = icmp sgt i64 %11, 0
  br i1 %12, label %13, label %32

13:                                               ; preds = %9
  %14 = tail call i64 @llvm.smin.i64(i64 %11, i64 61)
  %15 = getelementptr inbounds %struct.FaExecutor, ptr %1, i32 0, i32 1
  br label %16

16:                                               ; preds = %13, %20
  %17 = phi i64 [ %14, %13 ], [ %30, %20 ]
  %18 = load ptr, ptr %1, align 8, !tbaa !282
  %19 = icmp eq ptr %18, null
  br i1 %19, label %32, label %20

20:                                               ; preds = %16
  %21 = getelementptr inbounds %struct.FaTask, ptr %18, i32 0, i32 1
  %22 = load ptr, ptr %21, align 4, !tbaa !279
  store ptr %22, ptr %1, align 8, !tbaa !282
  %23 = icmp eq ptr %22, null
  %24 = getelementptr inbounds %struct.FaTask, ptr %22, i32 0, i32 2
  %25 = select i1 %23, ptr %15, ptr %24
  store ptr null, ptr %25, align 4, !tbaa !6
  %26 = load i64, ptr %10, align 8, !tbaa !283
  %27 = add nsw i64 %26, -1
  store i64 %27, ptr %10, align 8, !tbaa !283
  store ptr null, ptr %21, align 4, !tbaa !279
  %28 = getelementptr inbounds %struct.FaTask, ptr %18, i32 0, i32 2
  store ptr null, ptr %28, align 4, !tbaa !281
  %29 = getelementptr inbounds i8, ptr %18, i32 -16
  tail call fastcc void @turn(ptr noundef nonnull %1, ptr noundef nonnull %29) #53
  %30 = add nsw i64 %17, -1
  %31 = icmp sgt i64 %17, 1
  br i1 %31, label %16, label %32, !llvm.loop !322

32:                                               ; preds = %20, %16, %9
  %33 = load i64, ptr %4, align 8, !tbaa !302
  %34 = icmp sgt i64 %33, 0
  br i1 %34, label %35, label %40

35:                                               ; preds = %32
  %36 = tail call i64 @fa_clock() #54
  %37 = load i64, ptr %4, align 8, !tbaa !302
  %38 = icmp sgt i64 %37, 0
  br i1 %38, label %39, label %40

39:                                               ; preds = %35
  tail call fastcc void @fire_timers(ptr noundef nonnull %1, i64 noundef %36) #53
  br label %40

40:                                               ; preds = %32, %39, %35
  %41 = phi i64 [ %36, %39 ], [ %36, %35 ], [ 0, %32 ]
  %42 = load i64, ptr %10, align 8, !tbaa !283
  %43 = icmp sgt i64 %42, 0
  br i1 %43, label %83, label %44

44:                                               ; preds = %40
  %45 = getelementptr inbounds %struct.FaExecutor, ptr %1, i32 0, i32 7
  %46 = load ptr, ptr %45, align 8, !tbaa !289
  %47 = load ptr, ptr %46, align 4, !tbaa !6
  %48 = icmp eq ptr %47, null
  br i1 %48, label %49, label %64

49:                                               ; preds = %44
  %50 = getelementptr inbounds %struct.FaExecutor, ptr %1, i32 0, i32 8
  %51 = load i64, ptr %50, align 8, !tbaa !277
  %52 = icmp eq i64 %51, 0
  br i1 %52, label %53, label %64

53:                                               ; preds = %49
  tail call void @fa_destroy(ptr noundef nonnull %46) #54
  %54 = getelementptr inbounds %struct.FaExecutor, ptr %1, i32 0, i32 5
  %55 = load i64, ptr %54, align 8, !tbaa !303
  %56 = icmp sgt i64 %55, 0
  br i1 %56, label %57, label %62

57:                                               ; preds = %53
  %58 = getelementptr inbounds %struct.FaExecutor, ptr %1, i32 0, i32 3
  %59 = load ptr, ptr %58, align 8, !tbaa !304
  %60 = trunc i64 %55 to i32
  %61 = shl i32 %60, 2
  tail call void @fa_free(ptr noundef %59, i32 noundef %61, i32 noundef 4) #54
  br label %62

62:                                               ; preds = %57, %53
  store ptr null, ptr @current_executor, align 4, !tbaa !6
  tail call void @fa_free(ptr noundef nonnull %1, i32 noundef 288, i32 noundef 16) #54
  %63 = load ptr, ptr @wasm_finish, align 4, !tbaa !6
  tail call void %63() #54
  br label %83

64:                                               ; preds = %49, %44
  %65 = load i64, ptr %4, align 8, !tbaa !302
  %66 = icmp sgt i64 %65, 0
  br i1 %66, label %67, label %82

67:                                               ; preds = %64
  %68 = getelementptr inbounds %struct.FaExecutor, ptr %1, i32 0, i32 3
  %69 = load ptr, ptr %68, align 8, !tbaa !304
  %70 = load ptr, ptr %69, align 4, !tbaa !6
  %71 = getelementptr inbounds %struct.FaWait, ptr %70, i32 0, i32 6
  %72 = load i64, ptr %71, align 8, !tbaa !306
  %73 = sub nsw i64 %72, %41
  %74 = load i32, ptr @wasm_timer_token, align 4, !tbaa !23
  %75 = add nsw i32 %74, 1
  store i32 %75, ptr @wasm_timer_token, align 4, !tbaa !23
  %76 = icmp sgt i64 %73, 0
  br i1 %76, label %77, label %80

77:                                               ; preds = %67
  %78 = sitofp i64 %73 to double
  %79 = fdiv double %78, 1.000000e+06
  br label %80

80:                                               ; preds = %67, %77
  %81 = phi double [ %79, %77 ], [ 0.000000e+00, %67 ]
  tail call void @fa_host_timer(i32 noundef %75, double noundef %81) #54
  br label %83

82:                                               ; preds = %64
  tail call fastcc void @deadlock_stop(ptr noundef nonnull %1) #59
  unreachable

83:                                               ; preds = %62, %80, %40, %0
  %84 = phi i32 [ 2, %0 ], [ 2, %62 ], [ 0, %80 ], [ 1, %40 ]
  ret i32 %84
}

; Function Attrs: nofree norecurse nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc void @fire_timers(ptr nocapture noundef %0, i64 noundef %1) unnamed_addr #33 {
  %3 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 3
  %4 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 4
  %5 = load i64, ptr %4, align 8, !tbaa !302
  %6 = icmp sgt i64 %5, 0
  br i1 %6, label %7, label %32

7:                                                ; preds = %2
  %8 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 1
  %9 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 2
  br label %10

10:                                               ; preds = %7, %26
  %11 = load ptr, ptr %3, align 8, !tbaa !304
  %12 = load ptr, ptr %11, align 4, !tbaa !6
  %13 = getelementptr inbounds %struct.FaWait, ptr %12, i32 0, i32 6
  %14 = load i64, ptr %13, align 8, !tbaa !306
  %15 = icmp sgt i64 %14, %1
  br i1 %15, label %32, label %16

16:                                               ; preds = %10
  tail call fastcc void @timer_remove(ptr noundef nonnull %0, i64 noundef 0) #53
  %17 = load ptr, ptr %12, align 8, !tbaa !298
  %18 = getelementptr inbounds i8, ptr %17, i32 16
  %19 = getelementptr inbounds i8, ptr %17, i32 20
  store ptr null, ptr %19, align 4, !tbaa !279
  %20 = load ptr, ptr %8, align 4, !tbaa !280
  %21 = getelementptr inbounds i8, ptr %17, i32 24
  store ptr %20, ptr %21, align 4, !tbaa !281
  %22 = icmp eq ptr %20, null
  br i1 %22, label %25, label %23

23:                                               ; preds = %16
  %24 = getelementptr inbounds %struct.FaTask, ptr %20, i32 0, i32 1
  store ptr %18, ptr %24, align 4, !tbaa !279
  br label %26

25:                                               ; preds = %16
  store ptr %18, ptr %0, align 8, !tbaa !282
  br label %26

26:                                               ; preds = %23, %25
  store ptr %18, ptr %8, align 4, !tbaa !280
  %27 = load i64, ptr %9, align 8, !tbaa !283
  %28 = add nsw i64 %27, 1
  store i64 %28, ptr %9, align 8, !tbaa !283
  %29 = getelementptr inbounds i8, ptr %17, i32 28
  store atomic i32 1, ptr %29 release, align 4
  %30 = load i64, ptr %4, align 8, !tbaa !302
  %31 = icmp sgt i64 %30, 0
  br i1 %31, label %10, label %32, !llvm.loop !323

32:                                               ; preds = %10, %26, %2
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @turn(ptr nocapture noundef %0, ptr noundef %1) unnamed_addr #3 {
  store i32 128, ptr @fa_coop_budget, align 4, !tbaa !23
  %3 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 19
  store i64 0, ptr %3, align 8, !tbaa !297
  %4 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 17
  %5 = load i32, ptr %4, align 8, !tbaa !318
  %6 = icmp eq i32 %5, 0
  br i1 %6, label %9, label %7

7:                                                ; preds = %2
  %8 = tail call i64 @fa_clock() #54
  br label %9

9:                                                ; preds = %2, %7
  %10 = phi i64 [ %8, %7 ], [ 0, %2 ]
  %11 = icmp eq ptr %1, null
  br i1 %11, label %21, label %12

12:                                               ; preds = %9, %18
  %13 = phi ptr [ %19, %18 ], [ %1, %9 ]
  %14 = getelementptr inbounds i8, ptr %13, i32 28
  store atomic i32 4, ptr %14 release, align 4
  tail call void @fa_resume(ptr noundef nonnull %13) #54
  %15 = load ptr, ptr @frame_to_free, align 4, !tbaa !6
  %16 = icmp eq ptr %15, null
  br i1 %16, label %18, label %17

17:                                               ; preds = %12
  store ptr null, ptr @frame_to_free, align 4, !tbaa !6
  tail call void @fa_destroy(ptr noundef nonnull %15) #54
  br label %18

18:                                               ; preds = %17, %12
  %19 = load ptr, ptr @next_frame, align 4, !tbaa !6
  store ptr null, ptr @next_frame, align 4, !tbaa !6
  %20 = icmp eq ptr %19, null
  br i1 %20, label %21, label %12, !llvm.loop !324

21:                                               ; preds = %18, %9
  %22 = load i32, ptr %4, align 8, !tbaa !318
  %23 = icmp eq i32 %22, 0
  br i1 %23, label %54, label %24

24:                                               ; preds = %21
  %25 = tail call i64 @fa_clock() #54
  %26 = sub nsw i64 %25, %10
  %27 = icmp sgt i64 %26, 50000000
  br i1 %27, label %28, label %54

28:                                               ; preds = %24
  %29 = load ptr, ptr @fa_turn_site, align 4, !tbaa !6
  %30 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 23
  %31 = load i32, ptr %30, align 8, !tbaa !325
  %32 = icmp sgt i32 %31, 0
  br i1 %32, label %36, label %43

33:                                               ; preds = %36
  %34 = add nuw nsw i32 %37, 1
  %35 = icmp eq i32 %34, %31
  br i1 %35, label %41, label %36, !llvm.loop !326

36:                                               ; preds = %28, %33
  %37 = phi i32 [ %34, %33 ], [ 0, %28 ]
  %38 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 22, i32 %37
  %39 = load ptr, ptr %38, align 4, !tbaa !6
  %40 = icmp eq ptr %39, %29
  br i1 %40, label %54, label %33

41:                                               ; preds = %33
  %42 = icmp slt i32 %31, 32
  br i1 %42, label %43, label %46

43:                                               ; preds = %41, %28
  %44 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 22, i32 %31
  store ptr %29, ptr %44, align 4, !tbaa !6
  %45 = add nsw i32 %31, 1
  store i32 %45, ptr %30, align 8, !tbaa !325
  br label %46

46:                                               ; preds = %43, %41
  %47 = load i32, ptr @output_length, align 4, !tbaa !4
  %48 = icmp eq i32 %47, 0
  br i1 %48, label %50, label %49

49:                                               ; preds = %46
  tail call void @fa_write(i32 noundef 1, ptr noundef nonnull @output, i32 noundef %47) #54
  store i32 0, ptr @output_length, align 4, !tbaa !4
  br label %50

50:                                               ; preds = %49, %46
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  tail call fastcc void @append_c(ptr noundef nonnull @.str.39) #53
  %51 = udiv i64 %26, 1000000
  tail call fastcc void @append_u64(i64 noundef %51, i32 noundef 0) #53
  tail call fastcc void @append_c(ptr noundef nonnull @.str.40) #53
  %52 = icmp eq ptr %29, null
  %53 = select i1 %52, ptr @.str.41, ptr %29
  tail call fastcc void @append_c(ptr noundef nonnull %53) #53
  tail call fastcc void @append_c(ptr noundef nonnull @.str.42) #53
  tail call void @fa_b_line(i32 noundef 2) #53
  br label %54

54:                                               ; preds = %36, %50, %24, %21
  ret void
}

; Function Attrs: optsize
declare void @fa_host_timer(i32 noundef, double noundef) local_unnamed_addr #48

; Function Attrs: cold noinline noreturn nounwind optsize
define internal fastcc void @deadlock_stop(ptr nocapture noundef readonly %0) unnamed_addr #38 {
  %2 = load i32, ptr @output_length, align 4, !tbaa !4
  %3 = icmp eq i32 %2, 0
  br i1 %3, label %5, label %4

4:                                                ; preds = %1
  tail call void @fa_write(i32 noundef 1, ptr noundef nonnull @output, i32 noundef %2) #54
  store i32 0, ptr @output_length, align 4, !tbaa !4
  br label %5

5:                                                ; preds = %1, %4
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  tail call fastcc void @append_c(ptr noundef nonnull @.str.43) #53
  %6 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 20
  %7 = load ptr, ptr %6, align 8, !tbaa !290
  %8 = icmp eq ptr %7, null
  br i1 %8, label %9, label %13

9:                                                ; preds = %5
  %10 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 21
  %11 = load ptr, ptr %10, align 4, !tbaa !327
  %12 = icmp eq ptr %11, null
  br i1 %12, label %17, label %13

13:                                               ; preds = %9, %5
  %14 = phi ptr [ @.str.44, %5 ], [ @.str.46, %9 ]
  %15 = phi ptr [ %6, %5 ], [ %10, %9 ]
  tail call fastcc void @append_c(ptr noundef nonnull %14) #53
  %16 = load ptr, ptr %15, align 4, !tbaa !6
  tail call fastcc void @append_c(ptr noundef %16) #53
  tail call fastcc void @append_c(ptr noundef nonnull @.str.45) #53
  br label %17

17:                                               ; preds = %13, %9
  tail call void @fa_b_line(i32 noundef 2) #53
  tail call void @fa_exit(i32 noundef 101) #55
  unreachable
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define hidden void @fa_wasm_wake(ptr nocapture noundef %0) local_unnamed_addr #47 {
  %2 = getelementptr inbounds %struct.FaWait, ptr %0, i32 0, i32 9
  store i32 1, ptr %2, align 8, !tbaa !313
  %3 = load ptr, ptr @current_executor, align 4, !tbaa !6
  %4 = load ptr, ptr %0, align 8, !tbaa !298
  %5 = getelementptr inbounds i8, ptr %4, i32 16
  %6 = getelementptr inbounds i8, ptr %4, i32 20
  store ptr null, ptr %6, align 4, !tbaa !279
  %7 = getelementptr inbounds %struct.FaExecutor, ptr %3, i32 0, i32 1
  %8 = load ptr, ptr %7, align 4, !tbaa !280
  %9 = getelementptr inbounds i8, ptr %4, i32 24
  store ptr %8, ptr %9, align 4, !tbaa !281
  %10 = icmp eq ptr %8, null
  br i1 %10, label %13, label %11

11:                                               ; preds = %1
  %12 = getelementptr inbounds %struct.FaTask, ptr %8, i32 0, i32 1
  store ptr %5, ptr %12, align 4, !tbaa !279
  br label %14

13:                                               ; preds = %1
  store ptr %5, ptr %3, align 8, !tbaa !282
  br label %14

14:                                               ; preds = %11, %13
  store ptr %5, ptr %7, align 4, !tbaa !280
  %15 = getelementptr inbounds %struct.FaExecutor, ptr %3, i32 0, i32 2
  %16 = load i64, ptr %15, align 8, !tbaa !283
  %17 = add nsw i64 %16, 1
  store i64 %17, ptr %15, align 8, !tbaa !283
  %18 = getelementptr inbounds i8, ptr %4, i32 28
  store atomic i32 1, ptr %18 release, align 4
  ret void
}

; Function Attrs: nounwind optsize
define hidden noundef i32 @fa_blocking_submit(ptr noundef %0, ptr nocapture noundef writeonly %1, ptr nocapture noundef readnone %2) local_unnamed_addr #3 {
  %4 = load ptr, ptr %0, align 4, !tbaa !328
  tail call void %4(ptr noundef nonnull %0) #54
  %5 = getelementptr inbounds %struct.FaJob, ptr %0, i32 0, i32 5
  store i32 2, ptr %5, align 4, !tbaa !330
  %6 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 9
  store i32 1, ptr %6, align 8, !tbaa !313
  ret i32 0
}

; Function Attrs: nounwind optsize
define hidden void @fa_blocking_cancel(ptr noundef %0) local_unnamed_addr #3 {
  %2 = getelementptr inbounds %struct.FaJob, ptr %0, i32 0, i32 1
  %3 = load ptr, ptr %2, align 4, !tbaa !331
  tail call void %3(ptr noundef %0) #54
  ret void
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_mutex_new(i64 noundef %0, ptr noundef %1) local_unnamed_addr #3 {
  %3 = trunc i64 %0 to i32
  %4 = add i32 %3, 176
  %5 = tail call fastcc ptr @shared_new(i32 noundef %4, ptr noundef %1) #53
  ret ptr %5
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_mutex_lock(ptr noundef returned %0) local_unnamed_addr #3 {
  %2 = icmp eq ptr %0, null
  br i1 %2, label %5, label %3

3:                                                ; preds = %1
  %4 = atomicrmw add ptr %0, i64 1 monotonic, align 8
  br label %5

5:                                                ; preds = %1, %3
  %6 = getelementptr inbounds %struct.FaMutex, ptr %0, i32 0, i32 1
  %7 = atomicrmw xchg ptr %6, i32 1 acquire, align 16
  %8 = icmp eq i32 %7, 0
  br i1 %8, label %10, label %9

9:                                                ; preds = %5
  tail call fastcc void @deadlock(ptr noundef nonnull @.str.31) #59
  unreachable

10:                                               ; preds = %5
  ret ptr %0
}

; Function Attrs: nounwind optsize
define hidden noundef i32 @fa_mutex_lock_async(ptr noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef %3) local_unnamed_addr #3 {
  %5 = icmp eq ptr %0, null
  br i1 %5, label %8, label %6

6:                                                ; preds = %4
  %7 = atomicrmw add ptr %0, i64 1 monotonic, align 8
  br label %8

8:                                                ; preds = %4, %6
  %9 = getelementptr inbounds %struct.FaMutex, ptr %0, i32 0, i32 3
  %10 = load ptr, ptr %9, align 8, !tbaa !6
  %11 = icmp eq ptr %10, null
  br i1 %11, label %12, label %23

12:                                               ; preds = %8
  %13 = getelementptr inbounds %struct.FaMutex, ptr %0, i32 0, i32 1
  %14 = atomicrmw xchg ptr %13, i32 1 acquire, align 16
  %15 = icmp eq i32 %14, 0
  br i1 %15, label %16, label %23

16:                                               ; preds = %12
  %17 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 9
  store i32 1, ptr %17, align 8, !tbaa !313
  %18 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 10
  store i32 0, ptr %18, align 4, !tbaa !314
  %19 = load i32, ptr @fa_coop_budget, align 4, !tbaa !23
  %20 = add nsw i32 %19, -1
  store i32 %20, ptr @fa_coop_budget, align 4, !tbaa !23
  %21 = icmp sgt i32 %19, 1
  br i1 %21, label %24, label %22

22:                                               ; preds = %16
  tail call void @fa_task_yield(ptr noundef %2) #53
  br label %24

23:                                               ; preds = %12, %8
  tail call fastcc void @line_push(ptr noundef nonnull %9, ptr noundef %1, ptr noundef %2, ptr noundef %3) #53
  br label %24

24:                                               ; preds = %16, %23, %22
  %25 = phi i32 [ 2, %22 ], [ 1, %23 ], [ 0, %16 ]
  ret i32 %25
}

; Function Attrs: nounwind optsize
define internal fastcc void @line_push(ptr nocapture noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef %3) unnamed_addr #3 {
  %5 = tail call fastcc ptr @executor() #53
  store ptr %2, ptr %1, align 8, !tbaa !298
  %6 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 1
  store ptr %5, ptr %6, align 4, !tbaa !300
  %7 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 5
  store ptr %3, ptr %7, align 4, !tbaa !332
  %8 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 9
  store i32 0, ptr %8, align 8, !tbaa !313
  %9 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 10
  store i32 0, ptr %9, align 4, !tbaa !314
  %10 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 8
  store i64 -1, ptr %10, align 8, !tbaa !301
  %11 = load ptr, ptr %0, align 4, !tbaa !6
  %12 = icmp eq ptr %11, null
  %13 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 2
  br i1 %12, label %14, label %16

14:                                               ; preds = %4
  store ptr %1, ptr %13, align 8, !tbaa !333
  %15 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 3
  store ptr %1, ptr %15, align 4, !tbaa !334
  br label %21

16:                                               ; preds = %4
  store ptr %11, ptr %13, align 8, !tbaa !333
  %17 = getelementptr inbounds %struct.FaWait, ptr %11, i32 0, i32 3
  %18 = load ptr, ptr %17, align 4, !tbaa !334
  %19 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 3
  store ptr %18, ptr %19, align 4, !tbaa !334
  %20 = getelementptr inbounds %struct.FaWait, ptr %18, i32 0, i32 2
  store ptr %1, ptr %20, align 8, !tbaa !333
  br label %21

21:                                               ; preds = %16, %14
  %22 = phi ptr [ %17, %16 ], [ %0, %14 ]
  store ptr %1, ptr %22, align 4, !tbaa !6
  %23 = getelementptr inbounds i8, ptr %2, i32 28
  store atomic i32 2, ptr %23 release, align 4
  %24 = getelementptr inbounds %struct.FaExecutor, ptr %5, i32 0, i32 21
  store ptr %3, ptr %24, align 4, !tbaa !327
  %25 = getelementptr inbounds %struct.FaExecutor, ptr %5, i32 0, i32 7
  %26 = load ptr, ptr %25, align 8, !tbaa !289
  %27 = icmp eq ptr %26, %2
  %28 = icmp eq ptr %26, null
  %29 = or i1 %27, %28
  br i1 %29, label %30, label %32

30:                                               ; preds = %21
  %31 = getelementptr inbounds %struct.FaExecutor, ptr %5, i32 0, i32 20
  store ptr %3, ptr %31, align 8, !tbaa !290
  br label %32

32:                                               ; preds = %21, %30
  ret void
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(argmem: readwrite)
define hidden noundef ptr @fa_mutex_try_lock(ptr noundef %0) local_unnamed_addr #46 {
  %2 = getelementptr inbounds %struct.FaMutex, ptr %0, i32 0, i32 1
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
define hidden void @fa_mutex_unlock(ptr noundef %0) local_unnamed_addr #3 {
  %2 = icmp eq ptr %0, null
  br i1 %2, label %27, label %3

3:                                                ; preds = %1
  %4 = getelementptr inbounds %struct.FaMutex, ptr %0, i32 0, i32 3
  %5 = load ptr, ptr %4, align 4, !tbaa !6
  %6 = icmp eq ptr %5, null
  br i1 %6, label %24, label %7

7:                                                ; preds = %3
  %8 = getelementptr inbounds %struct.FaWait, ptr %5, i32 0, i32 2
  %9 = load ptr, ptr %8, align 8, !tbaa !333
  %10 = icmp eq ptr %9, %5
  br i1 %10, label %19, label %11

11:                                               ; preds = %7
  %12 = getelementptr inbounds %struct.FaWait, ptr %5, i32 0, i32 3
  %13 = load ptr, ptr %12, align 4, !tbaa !334
  %14 = getelementptr inbounds %struct.FaWait, ptr %13, i32 0, i32 2
  store ptr %9, ptr %14, align 8, !tbaa !333
  %15 = load ptr, ptr %8, align 8, !tbaa !333
  %16 = getelementptr inbounds %struct.FaWait, ptr %15, i32 0, i32 3
  store ptr %13, ptr %16, align 4, !tbaa !334
  %17 = load ptr, ptr %4, align 4, !tbaa !6
  %18 = icmp eq ptr %17, %5
  br i1 %18, label %19, label %21

19:                                               ; preds = %11, %7
  %20 = phi ptr [ null, %7 ], [ %15, %11 ]
  store ptr %20, ptr %4, align 4, !tbaa !6
  br label %21

21:                                               ; preds = %19, %11
  store ptr null, ptr %8, align 8, !tbaa !333
  %22 = getelementptr inbounds %struct.FaWait, ptr %5, i32 0, i32 3
  store ptr null, ptr %22, align 4, !tbaa !334
  %23 = getelementptr inbounds %struct.FaWait, ptr %5, i32 0, i32 9
  store i32 1, ptr %23, align 8, !tbaa !313
  tail call fastcc void @wake(ptr noundef nonnull %5) #53
  br label %26

24:                                               ; preds = %3
  %25 = getelementptr inbounds %struct.FaMutex, ptr %0, i32 0, i32 1
  store atomic i32 0, ptr %25 release, align 16
  br label %26

26:                                               ; preds = %24, %21
  tail call void @fa_shared_release(ptr noundef nonnull %0) #53
  br label %27

27:                                               ; preds = %1, %26
  ret void
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define internal fastcc void @wake(ptr nocapture noundef %0) unnamed_addr #47 {
  %2 = load ptr, ptr %0, align 8, !tbaa !298
  %3 = getelementptr inbounds i8, ptr %2, i32 16
  %4 = getelementptr inbounds %struct.FaWait, ptr %0, i32 0, i32 1
  %5 = load ptr, ptr %4, align 4, !tbaa !300
  %6 = load ptr, ptr @current_executor, align 4, !tbaa !6
  %7 = icmp eq ptr %5, %6
  br i1 %7, label %8, label %22

8:                                                ; preds = %1
  %9 = getelementptr inbounds i8, ptr %2, i32 20
  store ptr null, ptr %9, align 4, !tbaa !279
  %10 = getelementptr inbounds %struct.FaExecutor, ptr %5, i32 0, i32 1
  %11 = load ptr, ptr %10, align 4, !tbaa !280
  %12 = getelementptr inbounds i8, ptr %2, i32 24
  store ptr %11, ptr %12, align 4, !tbaa !281
  %13 = icmp eq ptr %11, null
  br i1 %13, label %16, label %14

14:                                               ; preds = %8
  %15 = getelementptr inbounds %struct.FaTask, ptr %11, i32 0, i32 1
  store ptr %3, ptr %15, align 4, !tbaa !279
  br label %17

16:                                               ; preds = %8
  store ptr %3, ptr %5, align 8, !tbaa !282
  br label %17

17:                                               ; preds = %14, %16
  store ptr %3, ptr %10, align 4, !tbaa !280
  %18 = getelementptr inbounds %struct.FaExecutor, ptr %5, i32 0, i32 2
  %19 = load i64, ptr %18, align 8, !tbaa !283
  %20 = add nsw i64 %19, 1
  store i64 %20, ptr %18, align 8, !tbaa !283
  %21 = getelementptr inbounds i8, ptr %2, i32 28
  store atomic i32 1, ptr %21 release, align 4
  br label %31

22:                                               ; preds = %1
  %23 = getelementptr inbounds %struct.FaWait, ptr %0, i32 0, i32 10
  store i32 1, ptr %23, align 4, !tbaa !314
  %24 = getelementptr inbounds i8, ptr %2, i32 28
  store atomic i32 5, ptr %24 release, align 4
  %25 = getelementptr inbounds i8, ptr %2, i32 20
  store atomic i32 0, ptr %25 monotonic, align 4
  %26 = getelementptr inbounds %struct.FaExecutor, ptr %5, i32 0, i32 12
  %27 = ptrtoint ptr %3 to i32
  %28 = atomicrmw xchg ptr %26, i32 %27 acq_rel, align 4
  %29 = inttoptr i32 %28 to ptr
  %30 = getelementptr inbounds %struct.FaTask, ptr %29, i32 0, i32 1
  store atomic i32 %27, ptr %30 release, align 4
  br label %31

31:                                               ; preds = %22, %17
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_mutex_cancel(ptr noundef %0, ptr noundef %1) local_unnamed_addr #3 {
  %3 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 9
  %4 = load i32, ptr %3, align 8, !tbaa !313
  %5 = icmp eq i32 %4, 0
  br i1 %5, label %6, label %23

6:                                                ; preds = %2
  %7 = getelementptr inbounds %struct.FaMutex, ptr %0, i32 0, i32 3
  %8 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 2
  %9 = load ptr, ptr %8, align 8, !tbaa !333
  %10 = icmp eq ptr %9, %1
  br i1 %10, label %19, label %11

11:                                               ; preds = %6
  %12 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 3
  %13 = load ptr, ptr %12, align 4, !tbaa !334
  %14 = getelementptr inbounds %struct.FaWait, ptr %13, i32 0, i32 2
  store ptr %9, ptr %14, align 8, !tbaa !333
  %15 = load ptr, ptr %8, align 8, !tbaa !333
  %16 = getelementptr inbounds %struct.FaWait, ptr %15, i32 0, i32 3
  store ptr %13, ptr %16, align 4, !tbaa !334
  %17 = load ptr, ptr %7, align 4, !tbaa !6
  %18 = icmp eq ptr %17, %1
  br i1 %18, label %19, label %21

19:                                               ; preds = %11, %6
  %20 = phi ptr [ null, %6 ], [ %15, %11 ]
  store ptr %20, ptr %7, align 4, !tbaa !6
  br label %21

21:                                               ; preds = %11, %19
  store ptr null, ptr %8, align 8, !tbaa !333
  %22 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 3
  store ptr null, ptr %22, align 4, !tbaa !334
  tail call void @fa_shared_release(ptr noundef nonnull %0) #53
  br label %24

23:                                               ; preds = %2
  tail call fastcc void @forget_remote_wake(ptr noundef nonnull %1) #53
  tail call void @fa_mutex_unlock(ptr noundef %0) #53
  br label %24

24:                                               ; preds = %23, %21
  ret void
}

; Function Attrs: nofree norecurse nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc void @forget_remote_wake(ptr nocapture noundef readonly %0) unnamed_addr #33 {
  %2 = getelementptr inbounds %struct.FaWait, ptr %0, i32 0, i32 10
  %3 = load i32, ptr %2, align 4, !tbaa !314
  %4 = icmp eq i32 %3, 0
  br i1 %4, label %28, label %5

5:                                                ; preds = %1
  %6 = load ptr, ptr @current_executor, align 4, !tbaa !6
  tail call fastcc void @drain_remote(ptr noundef %6) #53
  %7 = load ptr, ptr %0, align 8, !tbaa !298
  %8 = getelementptr inbounds i8, ptr %7, i32 28
  %9 = load atomic i32, ptr %8 acquire, align 4
  %10 = icmp eq i32 %9, 1
  br i1 %10, label %11, label %28

11:                                               ; preds = %5
  %12 = getelementptr inbounds i8, ptr %7, i32 24
  %13 = load ptr, ptr %12, align 4, !tbaa !281
  %14 = icmp eq ptr %13, null
  %15 = getelementptr inbounds i8, ptr %7, i32 20
  %16 = load ptr, ptr %15, align 4, !tbaa !279
  br i1 %14, label %19, label %17

17:                                               ; preds = %11
  %18 = getelementptr inbounds %struct.FaTask, ptr %13, i32 0, i32 1
  store ptr %16, ptr %18, align 4, !tbaa !279
  br label %20

19:                                               ; preds = %11
  store ptr %16, ptr %6, align 8, !tbaa !282
  br label %20

20:                                               ; preds = %17, %19
  %21 = icmp eq ptr %16, null
  %22 = getelementptr inbounds %struct.FaExecutor, ptr %6, i32 0, i32 1
  %23 = getelementptr inbounds %struct.FaTask, ptr %16, i32 0, i32 2
  %24 = select i1 %21, ptr %22, ptr %23
  store ptr %13, ptr %24, align 4, !tbaa !6
  store ptr null, ptr %15, align 4, !tbaa !279
  store ptr null, ptr %12, align 4, !tbaa !281
  %25 = getelementptr inbounds %struct.FaExecutor, ptr %6, i32 0, i32 2
  %26 = load i64, ptr %25, align 8, !tbaa !283
  %27 = add nsw i64 %26, -1
  store i64 %27, ptr %25, align 8, !tbaa !283
  br label %28

28:                                               ; preds = %5, %20, %1
  ret void
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_channel_new(i64 noundef %0, i64 noundef %1, ptr noundef %2) local_unnamed_addr #3 {
  %4 = tail call fastcc ptr @shared_new(i32 noundef 240, ptr noundef nonnull @channel_drop) #53
  %5 = tail call i64 @llvm.umin.i64(i64 %0, i64 9223372036854775807)
  %6 = getelementptr inbounds %struct.FaChannel, ptr %4, i32 0, i32 3
  store i64 %5, ptr %6, align 16, !tbaa !335
  %7 = tail call i64 @llvm.smax.i64(i64 %1, i64 1)
  %8 = getelementptr inbounds %struct.FaChannel, ptr %4, i32 0, i32 4
  store i64 %7, ptr %8, align 8, !tbaa !338
  %9 = getelementptr inbounds %struct.FaChannel, ptr %4, i32 0, i32 11
  store ptr %2, ptr %9, align 8, !tbaa !339
  ret ptr %4
}

; Function Attrs: nounwind optsize
define internal void @channel_drop(ptr nocapture noundef readonly %0) #3 {
  %2 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 7
  %3 = load i64, ptr %2, align 16, !tbaa !340
  %4 = icmp sgt i64 %3, 0
  br i1 %4, label %5, label %11

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 11
  %7 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 6
  %8 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 8
  %9 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 5
  %10 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 4
  br label %15

11:                                               ; preds = %31, %1
  %12 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 8
  %13 = load i64, ptr %12, align 8, !tbaa !341
  %14 = icmp sgt i64 %13, 0
  br i1 %14, label %35, label %42

15:                                               ; preds = %5, %31
  %16 = phi i64 [ %3, %5 ], [ %32, %31 ]
  %17 = phi i64 [ 0, %5 ], [ %33, %31 ]
  %18 = load ptr, ptr %6, align 8, !tbaa !339
  %19 = icmp eq ptr %18, null
  br i1 %19, label %31, label %20

20:                                               ; preds = %15
  %21 = load i64, ptr %7, align 8, !tbaa !342
  %22 = add nsw i64 %21, %17
  %23 = load i64, ptr %8, align 8, !tbaa !341
  %24 = srem i64 %22, %23
  %25 = load ptr, ptr %9, align 16, !tbaa !343
  %26 = load i64, ptr %10, align 8, !tbaa !338
  %27 = mul nsw i64 %26, %24
  %28 = trunc i64 %27 to i32
  %29 = getelementptr inbounds i8, ptr %25, i32 %28
  tail call void %18(ptr noundef %29) #54
  %30 = load i64, ptr %2, align 16, !tbaa !340
  br label %31

31:                                               ; preds = %20, %15
  %32 = phi i64 [ %30, %20 ], [ %16, %15 ]
  %33 = add nuw nsw i64 %17, 1
  %34 = icmp slt i64 %33, %32
  br i1 %34, label %15, label %11, !llvm.loop !344

35:                                               ; preds = %11
  %36 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 5
  %37 = load ptr, ptr %36, align 16, !tbaa !343
  %38 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 4
  %39 = load i64, ptr %38, align 8, !tbaa !338
  %40 = mul nsw i64 %39, %13
  %41 = trunc i64 %40 to i32
  tail call void @fa_free(ptr noundef %37, i32 noundef %41, i32 noundef 16) #54
  br label %42

42:                                               ; preds = %35, %11
  ret void
}

; Function Attrs: nounwind optsize
define hidden noundef i32 @fa_channel_send(ptr nocapture noundef %0, ptr nocapture noundef readonly %1, i32 noundef %2) local_unnamed_addr #3 {
  %4 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 9
  %5 = load i32, ptr %4, align 16, !tbaa !345
  %6 = icmp eq i32 %5, 0
  br i1 %6, label %7, label %13

7:                                                ; preds = %3
  %8 = tail call fastcc i32 @channel_send_now(ptr noundef nonnull %0, ptr noundef %1) #53, !range !53
  %9 = icmp eq i32 %8, 0
  br i1 %9, label %10, label %13

10:                                               ; preds = %7
  %11 = icmp eq i32 %2, 0
  br i1 %11, label %13, label %12

12:                                               ; preds = %10
  tail call fastcc void @deadlock(ptr noundef nonnull @.str.32) #59
  unreachable

13:                                               ; preds = %10, %7, %3
  %14 = phi i32 [ 0, %3 ], [ 1, %7 ], [ 0, %10 ]
  ret i32 %14
}

; Function Attrs: nounwind optsize
define internal fastcc noundef i32 @channel_send_now(ptr nocapture noundef %0, ptr nocapture noundef readonly %1) unnamed_addr #3 {
  %3 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 13
  %4 = load ptr, ptr %3, align 4, !tbaa !6
  %5 = icmp eq ptr %4, null
  br i1 %5, label %28, label %6

6:                                                ; preds = %2
  %7 = getelementptr inbounds %struct.FaWait, ptr %4, i32 0, i32 2
  %8 = load ptr, ptr %7, align 8, !tbaa !333
  %9 = icmp eq ptr %8, %4
  br i1 %9, label %18, label %10

10:                                               ; preds = %6
  %11 = getelementptr inbounds %struct.FaWait, ptr %4, i32 0, i32 3
  %12 = load ptr, ptr %11, align 4, !tbaa !334
  %13 = getelementptr inbounds %struct.FaWait, ptr %12, i32 0, i32 2
  store ptr %8, ptr %13, align 8, !tbaa !333
  %14 = load ptr, ptr %7, align 8, !tbaa !333
  %15 = getelementptr inbounds %struct.FaWait, ptr %14, i32 0, i32 3
  store ptr %12, ptr %15, align 4, !tbaa !334
  %16 = load ptr, ptr %3, align 4, !tbaa !6
  %17 = icmp eq ptr %16, %4
  br i1 %17, label %18, label %20

18:                                               ; preds = %10, %6
  %19 = phi ptr [ null, %6 ], [ %14, %10 ]
  store ptr %19, ptr %3, align 4, !tbaa !6
  br label %20

20:                                               ; preds = %18, %10
  store ptr null, ptr %7, align 8, !tbaa !333
  %21 = getelementptr inbounds %struct.FaWait, ptr %4, i32 0, i32 3
  store ptr null, ptr %21, align 4, !tbaa !334
  %22 = getelementptr inbounds %struct.FaWait, ptr %4, i32 0, i32 4
  %23 = load ptr, ptr %22, align 8, !tbaa !346
  %24 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 4
  %25 = load i64, ptr %24, align 8, !tbaa !338
  %26 = trunc i64 %25 to i32
  tail call fastcc void @copy_bytes(ptr noundef %23, ptr noundef %1, i32 noundef %26) #53
  %27 = getelementptr inbounds %struct.FaWait, ptr %4, i32 0, i32 9
  store i32 1, ptr %27, align 8, !tbaa !313
  tail call fastcc void @wake(ptr noundef nonnull %4) #53
  br label %35

28:                                               ; preds = %2
  %29 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 7
  %30 = load i64, ptr %29, align 16, !tbaa !340
  %31 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 3
  %32 = load i64, ptr %31, align 16, !tbaa !335
  %33 = icmp slt i64 %30, %32
  br i1 %33, label %34, label %35

34:                                               ; preds = %28
  tail call fastcc void @channel_push(ptr noundef nonnull %0, ptr noundef %1) #53
  br label %35

35:                                               ; preds = %28, %34, %20
  %36 = phi i32 [ 1, %20 ], [ 1, %34 ], [ 0, %28 ]
  ret i32 %36
}

; Function Attrs: nounwind optsize
define hidden noundef i32 @fa_channel_send_async(ptr nocapture noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef %3, ptr noundef %4) local_unnamed_addr #3 {
  %6 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 9
  %7 = load i32, ptr %6, align 16, !tbaa !345
  %8 = icmp eq i32 %7, 0
  br i1 %8, label %9, label %22

9:                                                ; preds = %5
  %10 = tail call fastcc i32 @channel_send_now(ptr noundef nonnull %0, ptr noundef %1) #53, !range !53
  %11 = icmp eq i32 %10, 0
  br i1 %11, label %19, label %12

12:                                               ; preds = %9
  %13 = getelementptr inbounds %struct.FaWait, ptr %2, i32 0, i32 9
  store i32 1, ptr %13, align 8, !tbaa !313
  %14 = getelementptr inbounds %struct.FaWait, ptr %2, i32 0, i32 10
  store i32 0, ptr %14, align 4, !tbaa !314
  %15 = load i32, ptr @fa_coop_budget, align 4, !tbaa !23
  %16 = add nsw i32 %15, -1
  store i32 %16, ptr @fa_coop_budget, align 4, !tbaa !23
  %17 = icmp sgt i32 %15, 1
  br i1 %17, label %22, label %18

18:                                               ; preds = %12
  tail call void @fa_task_yield(ptr noundef %3) #53
  br label %22

19:                                               ; preds = %9
  %20 = getelementptr inbounds %struct.FaWait, ptr %2, i32 0, i32 4
  store ptr %1, ptr %20, align 8, !tbaa !346
  %21 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 12
  tail call fastcc void @line_push(ptr noundef nonnull %21, ptr noundef %2, ptr noundef %3, ptr noundef %4) #53
  br label %22

22:                                               ; preds = %5, %12, %19, %18
  %23 = phi i32 [ 2, %18 ], [ 1, %19 ], [ 0, %12 ], [ 3, %5 ]
  ret i32 %23
}

; Function Attrs: nounwind optsize
define hidden noundef i32 @fa_channel_receive(ptr nocapture noundef %0, ptr nocapture noundef writeonly %1, i32 noundef %2) local_unnamed_addr #3 {
  %4 = tail call fastcc i32 @channel_receive_now(ptr noundef %0, ptr noundef %1) #53, !range !53
  %5 = icmp eq i32 %4, 0
  br i1 %5, label %6, label %13

6:                                                ; preds = %3
  %7 = icmp eq i32 %2, 0
  br i1 %7, label %13, label %8

8:                                                ; preds = %6
  %9 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 9
  %10 = load i32, ptr %9, align 16, !tbaa !345
  %11 = icmp eq i32 %10, 0
  br i1 %11, label %12, label %13

12:                                               ; preds = %8
  tail call fastcc void @deadlock(ptr noundef nonnull @.str.33) #59
  unreachable

13:                                               ; preds = %6, %8, %3
  %14 = phi i32 [ 1, %3 ], [ 0, %8 ], [ 0, %6 ]
  ret i32 %14
}

; Function Attrs: nounwind optsize
define internal fastcc noundef i32 @channel_receive_now(ptr nocapture noundef %0, ptr nocapture noundef writeonly %1) unnamed_addr #3 {
  %3 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 7
  %4 = load i64, ptr %3, align 16, !tbaa !340
  %5 = icmp sgt i64 %4, 0
  br i1 %5, label %6, label %45

6:                                                ; preds = %2
  %7 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 5
  %8 = load ptr, ptr %7, align 16, !tbaa !343
  %9 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 6
  %10 = load i64, ptr %9, align 8, !tbaa !342
  %11 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 4
  %12 = load i64, ptr %11, align 8, !tbaa !338
  %13 = mul nsw i64 %12, %10
  %14 = trunc i64 %13 to i32
  %15 = getelementptr inbounds i8, ptr %8, i32 %14
  %16 = trunc i64 %12 to i32
  tail call fastcc void @copy_bytes(ptr noundef %1, ptr noundef %15, i32 noundef %16) #53
  %17 = load i64, ptr %9, align 8, !tbaa !342
  %18 = add nsw i64 %17, 1
  %19 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 8
  %20 = load i64, ptr %19, align 8, !tbaa !341
  %21 = srem i64 %18, %20
  store i64 %21, ptr %9, align 8, !tbaa !342
  %22 = load i64, ptr %3, align 16, !tbaa !340
  %23 = add nsw i64 %22, -1
  store i64 %23, ptr %3, align 16, !tbaa !340
  %24 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 12
  %25 = load ptr, ptr %24, align 4, !tbaa !6
  %26 = icmp eq ptr %25, null
  br i1 %26, label %73, label %27

27:                                               ; preds = %6
  %28 = getelementptr inbounds %struct.FaWait, ptr %25, i32 0, i32 2
  %29 = load ptr, ptr %28, align 8, !tbaa !333
  %30 = icmp eq ptr %29, %25
  br i1 %30, label %39, label %31

31:                                               ; preds = %27
  %32 = getelementptr inbounds %struct.FaWait, ptr %25, i32 0, i32 3
  %33 = load ptr, ptr %32, align 4, !tbaa !334
  %34 = getelementptr inbounds %struct.FaWait, ptr %33, i32 0, i32 2
  store ptr %29, ptr %34, align 8, !tbaa !333
  %35 = load ptr, ptr %28, align 8, !tbaa !333
  %36 = getelementptr inbounds %struct.FaWait, ptr %35, i32 0, i32 3
  store ptr %33, ptr %36, align 4, !tbaa !334
  %37 = load ptr, ptr %24, align 4, !tbaa !6
  %38 = icmp eq ptr %37, %25
  br i1 %38, label %39, label %41

39:                                               ; preds = %31, %27
  %40 = phi ptr [ null, %27 ], [ %35, %31 ]
  store ptr %40, ptr %24, align 4, !tbaa !6
  br label %41

41:                                               ; preds = %39, %31
  store ptr null, ptr %28, align 8, !tbaa !333
  %42 = getelementptr inbounds %struct.FaWait, ptr %25, i32 0, i32 3
  store ptr null, ptr %42, align 4, !tbaa !334
  %43 = getelementptr inbounds %struct.FaWait, ptr %25, i32 0, i32 4
  %44 = load ptr, ptr %43, align 8, !tbaa !346
  tail call fastcc void @channel_push(ptr noundef nonnull %0, ptr noundef %44) #53
  br label %70

45:                                               ; preds = %2
  %46 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 12
  %47 = load ptr, ptr %46, align 4, !tbaa !6
  %48 = icmp eq ptr %47, null
  br i1 %48, label %73, label %49

49:                                               ; preds = %45
  %50 = getelementptr inbounds %struct.FaWait, ptr %47, i32 0, i32 2
  %51 = load ptr, ptr %50, align 8, !tbaa !333
  %52 = icmp eq ptr %51, %47
  br i1 %52, label %61, label %53

53:                                               ; preds = %49
  %54 = getelementptr inbounds %struct.FaWait, ptr %47, i32 0, i32 3
  %55 = load ptr, ptr %54, align 4, !tbaa !334
  %56 = getelementptr inbounds %struct.FaWait, ptr %55, i32 0, i32 2
  store ptr %51, ptr %56, align 8, !tbaa !333
  %57 = load ptr, ptr %50, align 8, !tbaa !333
  %58 = getelementptr inbounds %struct.FaWait, ptr %57, i32 0, i32 3
  store ptr %55, ptr %58, align 4, !tbaa !334
  %59 = load ptr, ptr %46, align 4, !tbaa !6
  %60 = icmp eq ptr %59, %47
  br i1 %60, label %61, label %63

61:                                               ; preds = %53, %49
  %62 = phi ptr [ null, %49 ], [ %57, %53 ]
  store ptr %62, ptr %46, align 4, !tbaa !6
  br label %63

63:                                               ; preds = %61, %53
  store ptr null, ptr %50, align 8, !tbaa !333
  %64 = getelementptr inbounds %struct.FaWait, ptr %47, i32 0, i32 3
  store ptr null, ptr %64, align 4, !tbaa !334
  %65 = getelementptr inbounds %struct.FaWait, ptr %47, i32 0, i32 4
  %66 = load ptr, ptr %65, align 8, !tbaa !346
  %67 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 4
  %68 = load i64, ptr %67, align 8, !tbaa !338
  %69 = trunc i64 %68 to i32
  tail call fastcc void @copy_bytes(ptr noundef %1, ptr noundef %66, i32 noundef %69) #53
  br label %70

70:                                               ; preds = %63, %41
  %71 = phi ptr [ %25, %41 ], [ %47, %63 ]
  %72 = getelementptr inbounds %struct.FaWait, ptr %71, i32 0, i32 9
  store i32 1, ptr %72, align 8, !tbaa !313
  tail call fastcc void @wake(ptr noundef nonnull %71) #53
  br label %73

73:                                               ; preds = %70, %45, %6
  %74 = phi i32 [ 1, %6 ], [ 0, %45 ], [ 1, %70 ]
  ret i32 %74
}

; Function Attrs: nounwind optsize
define hidden noundef i32 @fa_channel_receive_async(ptr nocapture noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef %3, ptr noundef %4) local_unnamed_addr #3 {
  %6 = tail call fastcc i32 @channel_receive_now(ptr noundef %0, ptr noundef %1) #53, !range !53
  %7 = icmp eq i32 %6, 0
  br i1 %7, label %15, label %8

8:                                                ; preds = %5
  %9 = getelementptr inbounds %struct.FaWait, ptr %2, i32 0, i32 9
  store i32 1, ptr %9, align 8, !tbaa !313
  %10 = getelementptr inbounds %struct.FaWait, ptr %2, i32 0, i32 10
  store i32 0, ptr %10, align 4, !tbaa !314
  %11 = load i32, ptr @fa_coop_budget, align 4, !tbaa !23
  %12 = add nsw i32 %11, -1
  store i32 %12, ptr @fa_coop_budget, align 4, !tbaa !23
  %13 = icmp sgt i32 %11, 1
  br i1 %13, label %22, label %14

14:                                               ; preds = %8
  tail call void @fa_task_yield(ptr noundef %3) #53
  br label %22

15:                                               ; preds = %5
  %16 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 9
  %17 = load i32, ptr %16, align 16, !tbaa !345
  %18 = icmp eq i32 %17, 0
  br i1 %18, label %19, label %22

19:                                               ; preds = %15
  %20 = getelementptr inbounds %struct.FaWait, ptr %2, i32 0, i32 4
  store ptr %1, ptr %20, align 8, !tbaa !346
  %21 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 13
  tail call fastcc void @line_push(ptr noundef nonnull %21, ptr noundef %2, ptr noundef %3, ptr noundef %4) #53
  br label %22

22:                                               ; preds = %15, %8, %19, %14
  %23 = phi i32 [ 2, %14 ], [ 1, %19 ], [ 0, %8 ], [ 3, %15 ]
  ret i32 %23
}

; Function Attrs: nofree norecurse nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden i32 @fa_channel_cancel(ptr nocapture noundef %0, ptr noundef %1) local_unnamed_addr #33 {
  %3 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 9
  %4 = load i32, ptr %3, align 8, !tbaa !313
  %5 = icmp eq i32 %4, 0
  br i1 %5, label %6, label %36

6:                                                ; preds = %2
  %7 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 12
  %8 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 13
  %9 = load ptr, ptr %8, align 16, !tbaa !347
  br label %10

10:                                               ; preds = %15, %6
  %11 = phi ptr [ %9, %6 ], [ %17, %15 ]
  %12 = icmp eq ptr %11, null
  br i1 %12, label %19, label %13

13:                                               ; preds = %10
  %14 = icmp eq ptr %11, %1
  br i1 %14, label %19, label %15

15:                                               ; preds = %13
  %16 = getelementptr inbounds %struct.FaWait, ptr %11, i32 0, i32 2
  %17 = load ptr, ptr %16, align 8, !tbaa !333
  %18 = icmp eq ptr %17, %9
  br i1 %18, label %19, label %10, !llvm.loop !348

19:                                               ; preds = %15, %13, %10
  %20 = phi ptr [ %7, %10 ], [ %8, %13 ], [ %7, %15 ]
  %21 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 2
  %22 = load ptr, ptr %21, align 8, !tbaa !333
  %23 = icmp eq ptr %22, %1
  br i1 %23, label %32, label %24

24:                                               ; preds = %19
  %25 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 3
  %26 = load ptr, ptr %25, align 4, !tbaa !334
  %27 = getelementptr inbounds %struct.FaWait, ptr %26, i32 0, i32 2
  store ptr %22, ptr %27, align 8, !tbaa !333
  %28 = load ptr, ptr %21, align 8, !tbaa !333
  %29 = getelementptr inbounds %struct.FaWait, ptr %28, i32 0, i32 3
  store ptr %26, ptr %29, align 4, !tbaa !334
  %30 = load ptr, ptr %20, align 4, !tbaa !6
  %31 = icmp eq ptr %30, %1
  br i1 %31, label %32, label %34

32:                                               ; preds = %24, %19
  %33 = phi ptr [ null, %19 ], [ %28, %24 ]
  store ptr %33, ptr %20, align 4, !tbaa !6
  br label %34

34:                                               ; preds = %32, %24
  store ptr null, ptr %21, align 8, !tbaa !333
  %35 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 3
  store ptr null, ptr %35, align 4, !tbaa !334
  br label %37

36:                                               ; preds = %2
  tail call fastcc void @forget_remote_wake(ptr noundef nonnull %1) #53
  br label %37

37:                                               ; preds = %34, %36
  ret i32 %4
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(argmem: readwrite)
define hidden i32 @fa_wait_status(ptr nocapture noundef readonly %0) local_unnamed_addr #46 {
  %2 = getelementptr inbounds %struct.FaWait, ptr %0, i32 0, i32 9
  %3 = load atomic i32, ptr %2 acquire, align 8
  ret i32 %3
}

; Function Attrs: nofree norecurse nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden void @fa_channel_close(ptr nocapture noundef %0) local_unnamed_addr #33 {
  %2 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 9
  store i32 1, ptr %2, align 16, !tbaa !345
  %3 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 13
  %4 = load ptr, ptr %3, align 4, !tbaa !6
  %5 = icmp eq ptr %4, null
  br i1 %5, label %6, label %10

6:                                                ; preds = %25, %1
  %7 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 12
  %8 = load ptr, ptr %7, align 4, !tbaa !6
  %9 = icmp eq ptr %8, null
  br i1 %9, label %50, label %30

10:                                               ; preds = %1, %25
  %11 = phi ptr [ %28, %25 ], [ %4, %1 ]
  %12 = getelementptr inbounds %struct.FaWait, ptr %11, i32 0, i32 2
  %13 = load ptr, ptr %12, align 8, !tbaa !333
  %14 = icmp eq ptr %13, %11
  br i1 %14, label %23, label %15

15:                                               ; preds = %10
  %16 = getelementptr inbounds %struct.FaWait, ptr %11, i32 0, i32 3
  %17 = load ptr, ptr %16, align 4, !tbaa !334
  %18 = getelementptr inbounds %struct.FaWait, ptr %17, i32 0, i32 2
  store ptr %13, ptr %18, align 8, !tbaa !333
  %19 = load ptr, ptr %12, align 8, !tbaa !333
  %20 = getelementptr inbounds %struct.FaWait, ptr %19, i32 0, i32 3
  store ptr %17, ptr %20, align 4, !tbaa !334
  %21 = load ptr, ptr %3, align 4, !tbaa !6
  %22 = icmp eq ptr %21, %11
  br i1 %22, label %23, label %25

23:                                               ; preds = %15, %10
  %24 = phi ptr [ null, %10 ], [ %19, %15 ]
  store ptr %24, ptr %3, align 4, !tbaa !6
  br label %25

25:                                               ; preds = %15, %23
  store ptr null, ptr %12, align 8, !tbaa !333
  %26 = getelementptr inbounds %struct.FaWait, ptr %11, i32 0, i32 3
  store ptr null, ptr %26, align 4, !tbaa !334
  %27 = getelementptr inbounds %struct.FaWait, ptr %11, i32 0, i32 9
  store i32 2, ptr %27, align 8, !tbaa !313
  tail call fastcc void @wake(ptr noundef nonnull %11) #53
  %28 = load ptr, ptr %3, align 4, !tbaa !6
  %29 = icmp eq ptr %28, null
  br i1 %29, label %6, label %10, !llvm.loop !349

30:                                               ; preds = %6, %45
  %31 = phi ptr [ %48, %45 ], [ %8, %6 ]
  %32 = getelementptr inbounds %struct.FaWait, ptr %31, i32 0, i32 2
  %33 = load ptr, ptr %32, align 8, !tbaa !333
  %34 = icmp eq ptr %33, %31
  br i1 %34, label %43, label %35

35:                                               ; preds = %30
  %36 = getelementptr inbounds %struct.FaWait, ptr %31, i32 0, i32 3
  %37 = load ptr, ptr %36, align 4, !tbaa !334
  %38 = getelementptr inbounds %struct.FaWait, ptr %37, i32 0, i32 2
  store ptr %33, ptr %38, align 8, !tbaa !333
  %39 = load ptr, ptr %32, align 8, !tbaa !333
  %40 = getelementptr inbounds %struct.FaWait, ptr %39, i32 0, i32 3
  store ptr %37, ptr %40, align 4, !tbaa !334
  %41 = load ptr, ptr %7, align 4, !tbaa !6
  %42 = icmp eq ptr %41, %31
  br i1 %42, label %43, label %45

43:                                               ; preds = %35, %30
  %44 = phi ptr [ null, %30 ], [ %39, %35 ]
  store ptr %44, ptr %7, align 4, !tbaa !6
  br label %45

45:                                               ; preds = %43, %35
  store ptr null, ptr %32, align 8, !tbaa !333
  %46 = getelementptr inbounds %struct.FaWait, ptr %31, i32 0, i32 3
  store ptr null, ptr %46, align 4, !tbaa !334
  %47 = getelementptr inbounds %struct.FaWait, ptr %31, i32 0, i32 9
  store i32 2, ptr %47, align 8, !tbaa !313
  tail call fastcc void @wake(ptr noundef nonnull %31) #53
  %48 = load ptr, ptr %7, align 4, !tbaa !6
  %49 = icmp eq ptr %48, null
  br i1 %49, label %50, label %30, !llvm.loop !350

50:                                               ; preds = %45, %6
  ret void
}

; Function Attrs: nounwind optsize
define weak hidden i128 @__multi3(i128 noundef %0, i128 noundef %1) local_unnamed_addr #3 {
  %3 = trunc i128 %0 to i64
  %4 = lshr i128 %0, 64
  %5 = trunc i128 %4 to i64
  %6 = trunc i128 %1 to i64
  %7 = lshr i128 %1, 64
  %8 = trunc i128 %7 to i64
  %9 = and i64 %3, 4294967295
  %10 = lshr i64 %3, 32
  %11 = and i64 %6, 4294967295
  %12 = lshr i64 %6, 32
  %13 = mul nuw i64 %11, %9
  %14 = mul nuw i64 %11, %10
  %15 = mul nuw i64 %12, %9
  %16 = mul nuw i64 %12, %10
  %17 = lshr i64 %13, 32
  %18 = and i64 %14, 4294967295
  %19 = add nuw nsw i64 %17, %18
  %20 = and i64 %15, 4294967295
  %21 = add nuw nsw i64 %19, %20
  %22 = shl i64 %21, 32
  %23 = and i64 %13, 4294967295
  %24 = or disjoint i64 %22, %23
  %25 = lshr i64 %14, 32
  %26 = lshr i64 %15, 32
  %27 = lshr i64 %21, 32
  %28 = mul i64 %8, %3
  %29 = mul i64 %5, %6
  %30 = add i64 %16, %29
  %31 = add i64 %30, %28
  %32 = add i64 %31, %25
  %33 = add i64 %32, %26
  %34 = add i64 %33, %27
  %35 = zext i64 %34 to i128
  %36 = shl nuw i128 %35, 64
  %37 = zext i64 %24 to i128
  %38 = or disjoint i128 %36, %37
  ret i128 %38
}

; Function Attrs: nounwind optsize
define weak hidden i128 @__udivti3(i128 noundef %0, i128 noundef %1) local_unnamed_addr #3 {
  %3 = alloca i128, align 16
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %3) #56
  %4 = call fastcc i128 @divide_u128(i128 noundef %0, i128 noundef %1, ptr noundef nonnull %3) #53
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %3) #56
  ret i128 %4
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: write)
define internal fastcc i128 @divide_u128(i128 noundef %0, i128 noundef %1, ptr nocapture noundef writeonly %2) unnamed_addr #10 {
  %4 = trunc i128 %1 to i64
  %5 = lshr i128 %1, 64
  %6 = trunc i128 %5 to i64
  %7 = trunc i128 %0 to i64
  %8 = lshr i128 %0, 64
  %9 = trunc i128 %8 to i64
  br label %19

10:                                               ; preds = %52
  %11 = zext i64 %53 to i128
  %12 = shl nuw i128 %11, 64
  %13 = zext i64 %54 to i128
  %14 = or disjoint i128 %12, %13
  store i128 %14, ptr %2, align 16, !tbaa !351
  %15 = zext i64 %55 to i128
  %16 = shl nuw i128 %15, 64
  %17 = zext i64 %56 to i128
  %18 = or disjoint i128 %16, %17
  ret i128 %18

19:                                               ; preds = %3, %52
  %20 = phi i64 [ 127, %3 ], [ %57, %52 ]
  %21 = phi i64 [ 0, %3 ], [ %56, %52 ]
  %22 = phi i64 [ 0, %3 ], [ %55, %52 ]
  %23 = phi i64 [ 0, %3 ], [ %54, %52 ]
  %24 = phi i64 [ 0, %3 ], [ %53, %52 ]
  %25 = tail call i64 @llvm.fshl.i64(i64 %24, i64 %23, i64 1)
  %26 = icmp ugt i64 %20, 63
  %27 = add nsw i64 %20, -64
  %28 = lshr i64 %9, %27
  %29 = lshr i64 %7, %20
  %30 = select i1 %26, i64 %28, i64 %29
  %31 = and i64 %30, 1
  %32 = shl i64 %23, 1
  %33 = or disjoint i64 %31, %32
  %34 = icmp ugt i64 %25, %6
  br i1 %34, label %39, label %35

35:                                               ; preds = %19
  %36 = icmp ne i64 %25, %6
  %37 = icmp ult i64 %33, %4
  %38 = select i1 %36, i1 true, i1 %37
  br i1 %38, label %52, label %39

39:                                               ; preds = %35, %19
  %40 = icmp ult i64 %33, %4
  %41 = sext i1 %40 to i64
  %42 = sub i64 %33, %4
  %43 = sub i64 %25, %6
  %44 = add i64 %43, %41
  br i1 %26, label %45, label %49

45:                                               ; preds = %39
  %46 = add nsw i64 %20, -64
  %47 = shl nuw i64 1, %46
  %48 = or i64 %47, %22
  br label %52

49:                                               ; preds = %39
  %50 = shl nuw i64 1, %20
  %51 = or i64 %21, %50
  br label %52

52:                                               ; preds = %45, %49, %35
  %53 = phi i64 [ %25, %35 ], [ %44, %49 ], [ %44, %45 ]
  %54 = phi i64 [ %33, %35 ], [ %42, %49 ], [ %42, %45 ]
  %55 = phi i64 [ %22, %35 ], [ %22, %49 ], [ %48, %45 ]
  %56 = phi i64 [ %21, %35 ], [ %51, %49 ], [ %21, %45 ]
  %57 = add nsw i64 %20, -1
  %58 = icmp eq i64 %20, 0
  br i1 %58, label %10, label %19, !llvm.loop !353
}

; Function Attrs: nounwind optsize
define weak hidden i128 @__umodti3(i128 noundef %0, i128 noundef %1) local_unnamed_addr #3 {
  %3 = alloca i128, align 16
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %3) #56
  %4 = call fastcc i128 @divide_u128(i128 noundef %0, i128 noundef %1, ptr noundef nonnull %3) #53
  %5 = load i128, ptr %3, align 16, !tbaa !351
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %3) #56
  ret i128 %5
}

; Function Attrs: nounwind optsize
define weak hidden i128 @__divti3(i128 noundef %0, i128 noundef %1) local_unnamed_addr #3 {
  %3 = alloca i128, align 16
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %3) #56
  %4 = tail call noundef i128 @llvm.abs.i128(i128 %0, i1 false)
  %5 = tail call noundef i128 @llvm.abs.i128(i128 %1, i1 false)
  %6 = call fastcc i128 @divide_u128(i128 noundef %4, i128 noundef %5, ptr noundef nonnull %3) #53
  %7 = lshr i128 %0, 127
  %8 = trunc i128 %7 to i32
  %9 = lshr i128 %1, 127
  %10 = trunc i128 %9 to i32
  %11 = icmp eq i32 %8, %10
  %12 = sub i128 0, %6
  %13 = select i1 %11, i128 %6, i128 %12
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %3) #56
  ret i128 %13
}

; Function Attrs: nounwind optsize
define weak hidden i128 @__modti3(i128 noundef %0, i128 noundef %1) local_unnamed_addr #3 {
  %3 = alloca i128, align 16
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %3) #56
  %4 = tail call noundef i128 @llvm.abs.i128(i128 %0, i1 false)
  %5 = tail call noundef i128 @llvm.abs.i128(i128 %1, i1 false)
  %6 = call fastcc i128 @divide_u128(i128 noundef %4, i128 noundef %5, ptr noundef nonnull %3) #53
  %7 = icmp slt i128 %0, 0
  %8 = load i128, ptr %3, align 16, !tbaa !351
  %9 = sub i128 0, %8
  %10 = select i1 %7, i128 %9, i128 %8
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %3) #56
  ret i128 %10
}

; Function Attrs: optsize
declare i32 @fa_host_format_float(double noundef, i32 noundef, ptr noundef) local_unnamed_addr #49

; Function Attrs: noinline nounwind optsize
define internal fastcc void @string_grow(ptr nocapture noundef %0, ptr nocapture noundef readonly %1, i32 noundef %2, i32 noundef %3) unnamed_addr #15 {
  %5 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 2
  %6 = load i32, ptr %5, align 4, !tbaa !19
  %7 = and i32 %6, 1073741824
  %8 = icmp eq i32 %7, 0
  br i1 %8, label %12, label %9

9:                                                ; preds = %4
  %10 = lshr i32 %6, 24
  %11 = and i32 %10, 31
  br label %15

12:                                               ; preds = %4
  %13 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 1
  %14 = load i32, ptr %13, align 4, !tbaa !18
  br label %15

15:                                               ; preds = %9, %12
  %16 = phi i32 [ %11, %9 ], [ %14, %12 ]
  %17 = add i32 %16, %2
  %18 = icmp ult i32 %17, %16
  %19 = icmp ugt i32 %17, 1073741822
  %20 = or i1 %18, %19
  br i1 %20, label %21, label %22

21:                                               ; preds = %15
  tail call fastcc void @out_of_memory() #57
  unreachable

22:                                               ; preds = %15
  %23 = and i32 %6, 1073741823
  %24 = icmp ne i32 %23, 0
  %25 = icmp ne i32 %23, 1073741823
  %26 = and i1 %24, %25
  %27 = and i1 %8, %26
  %28 = shl nuw nsw i32 %23, 1
  %29 = tail call i32 @llvm.umax.i32(i32 %28, i32 %17)
  %30 = select i1 %27, i32 %29, i32 %17
  %31 = tail call i32 @llvm.umin.i32(i32 %30, i32 1073741822)
  %32 = tail call i32 @llvm.umax.i32(i32 %31, i32 16)
  %33 = tail call ptr @fa_alloc(i32 noundef %32, i32 noundef 1) #54
  %34 = load i32, ptr %5, align 4, !tbaa !19
  %35 = and i32 %34, 1073741824
  %36 = icmp eq i32 %35, 0
  br i1 %36, label %37, label %39

37:                                               ; preds = %22
  %38 = load ptr, ptr %0, align 4, !tbaa !13
  br label %39

39:                                               ; preds = %22, %37
  %40 = phi ptr [ %38, %37 ], [ %0, %22 ]
  tail call fastcc void @copy_bytes(ptr noundef %33, ptr noundef %40, i32 noundef %16) #53
  %41 = getelementptr inbounds i8, ptr %33, i32 %16
  tail call fastcc void @copy_bytes(ptr noundef %41, ptr noundef %1, i32 noundef %2) #53
  br i1 %8, label %42, label %45

42:                                               ; preds = %39
  switch i32 %23, label %43 [
    i32 1073741823, label %45
    i32 0, label %45
  ]

43:                                               ; preds = %42
  %44 = load ptr, ptr %0, align 4, !tbaa !13
  tail call void @fa_free(ptr noundef %44, i32 noundef %23, i32 noundef 1) #54
  br label %45

45:                                               ; preds = %42, %42, %39, %43
  store ptr %33, ptr %0, align 4, !tbaa !13
  %46 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 1
  store i32 %17, ptr %46, align 4, !tbaa !18
  %47 = or i32 %32, %3
  store i32 %47, ptr %5, align 4, !tbaa !19
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(none)
define internal fastcc i32 @break_kind(i32 noundef %0) unnamed_addr #50 {
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
  %50 = phi i32 [ %65, %61 ], [ 0, %41 ]
  %51 = phi i32 [ %53, %61 ], [ 153, %41 ]
  br label %52

52:                                               ; preds = %55, %49
  %53 = phi i32 [ %57, %55 ], [ %51, %49 ]
  %54 = icmp ult i32 %50, %53
  br i1 %54, label %55, label %66

55:                                               ; preds = %52
  %56 = add i32 %53, %50
  %57 = lshr i32 %56, 1
  %58 = getelementptr inbounds %struct.Range, ptr @EXTEND, i32 %57
  %59 = load i32, ptr %58, align 8, !tbaa !127
  %60 = icmp ugt i32 %59, %0
  br i1 %60, label %52, label %61, !llvm.loop !354

61:                                               ; preds = %55
  %62 = getelementptr inbounds %struct.Range, ptr @EXTEND, i32 %57, i32 1
  %63 = load i32, ptr %62, align 4, !tbaa !129
  %64 = icmp ult i32 %63, %0
  %65 = add nuw i32 %57, 1
  br i1 %64, label %49, label %83, !llvm.loop !354

66:                                               ; preds = %52, %78
  %67 = phi i32 [ %82, %78 ], [ 0, %52 ]
  %68 = phi i32 [ %70, %78 ], [ 22, %52 ]
  br label %69

69:                                               ; preds = %72, %66
  %70 = phi i32 [ %74, %72 ], [ %68, %66 ]
  %71 = icmp ult i32 %67, %70
  br i1 %71, label %72, label %83

72:                                               ; preds = %69
  %73 = add i32 %70, %67
  %74 = lshr i32 %73, 1
  %75 = getelementptr inbounds %struct.Range, ptr @PICTOGRAPHIC, i32 %74
  %76 = load i32, ptr %75, align 8, !tbaa !127
  %77 = icmp ugt i32 %76, %0
  br i1 %77, label %69, label %78, !llvm.loop !354

78:                                               ; preds = %72
  %79 = getelementptr inbounds %struct.Range, ptr @PICTOGRAPHIC, i32 %74, i32 1
  %80 = load i32, ptr %79, align 4, !tbaa !129
  %81 = icmp ult i32 %80, %0
  %82 = add nuw i32 %74, 1
  br i1 %81, label %66, label %83, !llvm.loop !354

83:                                               ; preds = %61, %78, %69, %35, %29, %23, %20, %18, %3, %5, %1, %44, %13, %2
  %84 = phi i32 [ 2, %2 ], [ %17, %13 ], [ %48, %44 ], [ 1, %1 ], [ 3, %5 ], [ 3, %3 ], [ 5, %18 ], [ 6, %20 ], [ 8, %23 ], [ 9, %29 ], [ 10, %35 ], [ 0, %69 ], [ 7, %78 ], [ 4, %61 ]
  ret i32 %84
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc i32 @at_boundary(ptr nocapture noundef %0, i32 noundef %1) unnamed_addr #19 {
  %3 = load ptr, ptr %0, align 4, !tbaa !48
  %4 = getelementptr inbounds %struct.Boundaries, ptr %0, i32 0, i32 3
  %5 = load i32, ptr %4, align 4, !tbaa !52
  %6 = icmp ne i32 %5, 0
  %7 = icmp eq i32 %1, 0
  %8 = or i1 %7, %6
  br i1 %8, label %37, label %9

9:                                                ; preds = %2
  %10 = getelementptr inbounds %struct.Boundaries, ptr %0, i32 0, i32 1
  %11 = load i32, ptr %10, align 4, !tbaa !50
  %12 = icmp ugt i32 %11, %1
  br i1 %12, label %13, label %37

13:                                               ; preds = %9
  %14 = getelementptr inbounds i8, ptr %3, i32 %1
  %15 = load i8, ptr %14, align 1, !tbaa !1
  %16 = icmp sgt i8 %15, -1
  br i1 %16, label %17, label %25

17:                                               ; preds = %13
  %18 = getelementptr i8, ptr %14, i32 -1
  %19 = load i8, ptr %18, align 1, !tbaa !1
  %20 = icmp sgt i8 %19, -1
  br i1 %20, label %21, label %25

21:                                               ; preds = %17
  %22 = icmp eq i8 %19, 13
  %23 = icmp eq i8 %15, 10
  %24 = and i1 %23, %22
  br i1 %24, label %25, label %37

25:                                               ; preds = %21, %17, %13
  %26 = getelementptr inbounds %struct.Boundaries, ptr %0, i32 0, i32 2
  %27 = load i32, ptr %26, align 4, !tbaa !51
  %28 = icmp ult i32 %27, %1
  br i1 %28, label %29, label %33

29:                                               ; preds = %25, %29
  %30 = phi i32 [ %31, %29 ], [ %27, %25 ]
  %31 = tail call fastcc i32 @next_boundary(ptr noundef %3, i32 noundef %11, i32 noundef %30) #53
  store i32 %31, ptr %26, align 4, !tbaa !51
  %32 = icmp ult i32 %31, %1
  br i1 %32, label %29, label %33, !llvm.loop !355

33:                                               ; preds = %29, %25
  %34 = phi i32 [ %27, %25 ], [ %31, %29 ]
  %35 = icmp eq i32 %34, %1
  %36 = zext i1 %35 to i32
  br label %37

37:                                               ; preds = %2, %9, %21, %33
  %38 = phi i32 [ %36, %33 ], [ 1, %21 ], [ 1, %9 ], [ 1, %2 ]
  ret i32 %38
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.ctlz.i64(i64, i1 immarg) #32

; Function Attrs: noinline nounwind optsize
define internal fastcc ptr @take_pages(i32 noundef %0) unnamed_addr #15 {
  br label %2

2:                                                ; preds = %6, %1
  %3 = phi ptr [ @free_spans, %1 ], [ %4, %6 ]
  %4 = load ptr, ptr %3, align 4, !tbaa !6
  %5 = icmp eq ptr %4, null
  br i1 %5, label %22, label %6

6:                                                ; preds = %2
  %7 = getelementptr inbounds %struct.FreeSpan, ptr %4, i32 0, i32 1
  %8 = load i32, ptr %7, align 4, !tbaa !131
  %9 = icmp ult i32 %8, %0
  br i1 %9, label %2, label %10

10:                                               ; preds = %6
  %11 = icmp eq i32 %8, %0
  br i1 %11, label %12, label %14

12:                                               ; preds = %10
  %13 = load ptr, ptr %4, align 4, !tbaa !134
  br label %20

14:                                               ; preds = %10
  %15 = shl i32 %0, 16
  %16 = getelementptr inbounds i8, ptr %4, i32 %15
  %17 = load ptr, ptr %4, align 4, !tbaa !134
  store ptr %17, ptr %16, align 4, !tbaa !134
  %18 = sub i32 %8, %0
  %19 = getelementptr inbounds %struct.FreeSpan, ptr %16, i32 0, i32 1
  store i32 %18, ptr %19, align 4, !tbaa !131
  br label %20

20:                                               ; preds = %14, %12
  %21 = phi ptr [ %16, %14 ], [ %13, %12 ]
  store ptr %21, ptr %3, align 4, !tbaa !6
  br label %40

22:                                               ; preds = %2
  %23 = tail call i32 @llvm.umax.i32(i32 %0, i32 16)
  %24 = tail call ptr @fa_pages(i32 noundef %23) #54
  %25 = icmp eq ptr %24, null
  br i1 %25, label %40, label %26

26:                                               ; preds = %22
  %27 = icmp ult i32 %0, 16
  br i1 %27, label %28, label %40

28:                                               ; preds = %26
  %29 = shl nuw nsw i32 %0, 16
  %30 = getelementptr inbounds i8, ptr %24, i32 %29
  %31 = sub nuw nsw i32 %23, %0
  %32 = getelementptr inbounds %struct.FreeSpan, ptr %30, i32 0, i32 1
  store i32 %31, ptr %32, align 4, !tbaa !131
  store ptr null, ptr %30, align 4, !tbaa !134
  br label %33

33:                                               ; preds = %33, %28
  %34 = phi ptr [ @free_spans, %28 ], [ %35, %33 ]
  %35 = load ptr, ptr %34, align 4, !tbaa !6
  %36 = icmp ne ptr %35, null
  %37 = icmp ult ptr %35, %30
  %38 = and i1 %36, %37
  br i1 %38, label %33, label %39, !llvm.loop !356

39:                                               ; preds = %33
  store ptr %35, ptr %30, align 4, !tbaa !134
  store ptr %30, ptr %34, align 4, !tbaa !6
  br label %40

40:                                               ; preds = %20, %22, %26, %39
  %41 = phi ptr [ null, %22 ], [ %24, %39 ], [ %24, %26 ], [ %4, %20 ]
  ret ptr %41
}

; Function Attrs: optsize
declare ptr @fa_pages(i32 noundef) local_unnamed_addr #4

; Function Attrs: nounwind optsize
define internal fastcc void @mark(ptr noundef %0) unnamed_addr #3 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 17
  %3 = load i32, ptr %2, align 8, !tbaa !226
  %4 = icmp eq i32 %3, 0
  br i1 %4, label %21, label %5

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 14
  %7 = load i32, ptr %6, align 4, !tbaa !234
  %8 = icmp eq i32 %7, 0
  br i1 %8, label %9, label %40

9:                                                ; preds = %5
  %10 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 15
  %11 = load i32, ptr %10, align 8, !tbaa !228
  %12 = icmp eq i32 %11, 0
  br i1 %12, label %13, label %40

13:                                               ; preds = %9
  store i32 1, ptr %6, align 4, !tbaa !234
  %14 = load ptr, ptr @reactive_pending, align 4, !tbaa !6
  %15 = load i64, ptr @reactive_pending_count, align 8, !tbaa !89
  %16 = tail call fastcc ptr @grow_items(ptr noundef %14, i64 noundef %15, ptr noundef nonnull @reactive_pending_capacity, i32 noundef 4) #53
  store ptr %16, ptr @reactive_pending, align 4, !tbaa !6
  %17 = load i64, ptr @reactive_pending_count, align 8, !tbaa !89
  %18 = trunc i64 %17 to i32
  %19 = getelementptr inbounds ptr, ptr %16, i32 %18
  store ptr %0, ptr %19, align 4, !tbaa !6
  %20 = add nsw i64 %17, 1
  store i64 %20, ptr @reactive_pending_count, align 8, !tbaa !89
  br label %40

21:                                               ; preds = %1
  %22 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 11
  %23 = load i32, ptr %22, align 8, !tbaa !215
  %24 = icmp eq i32 %23, 0
  br i1 %24, label %25, label %40

25:                                               ; preds = %21
  store i32 1, ptr %22, align 8, !tbaa !215
  %26 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 3
  %27 = load i64, ptr %26, align 8, !tbaa !357
  %28 = icmp sgt i64 %27, 0
  br i1 %28, label %29, label %40

29:                                               ; preds = %25
  %30 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 2
  br label %31

31:                                               ; preds = %29, %31
  %32 = phi i64 [ 0, %29 ], [ %37, %31 ]
  %33 = load ptr, ptr %30, align 8, !tbaa !358
  %34 = trunc i64 %32 to i32
  %35 = getelementptr inbounds ptr, ptr %33, i32 %34
  %36 = load ptr, ptr %35, align 4, !tbaa !6
  tail call fastcc void @mark(ptr noundef %36) #53
  %37 = add nuw nsw i64 %32, 1
  %38 = load i64, ptr %26, align 8, !tbaa !357
  %39 = icmp slt i64 %37, %38
  br i1 %39, label %31, label %40, !llvm.loop !359

40:                                               ; preds = %31, %25, %21, %5, %9, %13
  ret void
}

; Function Attrs: optsize
declare void @fa_resume(ptr noundef) local_unnamed_addr #4

; Function Attrs: nounwind optsize
define internal fastcc void @channel_push(ptr nocapture noundef %0, ptr nocapture noundef readonly %1) unnamed_addr #3 {
  %3 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 7
  %4 = load i64, ptr %3, align 16, !tbaa !340
  %5 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 8
  %6 = load i64, ptr %5, align 8, !tbaa !341
  %7 = icmp eq i64 %4, %6
  br i1 %7, label %13, label %8

8:                                                ; preds = %2
  %9 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 6
  %10 = load i64, ptr %9, align 8, !tbaa !342
  %11 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 5
  %12 = load ptr, ptr %11, align 16, !tbaa !343
  br label %63

13:                                               ; preds = %2
  %14 = icmp slt i64 %4, 4
  %15 = shl nuw nsw i64 %4, 1
  %16 = select i1 %14, i64 4, i64 %15
  %17 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 4
  %18 = load i64, ptr %17, align 8, !tbaa !338
  %19 = mul nsw i64 %18, %16
  %20 = trunc i64 %19 to i32
  %21 = tail call ptr @fa_alloc(i32 noundef %20, i32 noundef 16) #54
  %22 = icmp eq ptr %21, null
  br i1 %22, label %29, label %23

23:                                               ; preds = %13
  %24 = load i64, ptr %3, align 16, !tbaa !340
  %25 = icmp sgt i64 %24, 0
  br i1 %25, label %26, label %30

26:                                               ; preds = %23
  %27 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 6
  %28 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 5
  br label %34

29:                                               ; preds = %13
  tail call fastcc void @out_of_memory() #57
  unreachable

30:                                               ; preds = %34, %23
  %31 = phi i64 [ %24, %23 ], [ %50, %34 ]
  %32 = load i64, ptr %5, align 8, !tbaa !341
  %33 = icmp sgt i64 %32, 0
  br i1 %33, label %52, label %59

34:                                               ; preds = %26, %34
  %35 = phi i64 [ 0, %26 ], [ %49, %34 ]
  %36 = load i64, ptr %27, align 8, !tbaa !342
  %37 = add nsw i64 %36, %35
  %38 = load i64, ptr %5, align 8, !tbaa !341
  %39 = srem i64 %37, %38
  %40 = load i64, ptr %17, align 8, !tbaa !338
  %41 = mul nsw i64 %40, %35
  %42 = trunc i64 %41 to i32
  %43 = getelementptr inbounds i8, ptr %21, i32 %42
  %44 = load ptr, ptr %28, align 16, !tbaa !343
  %45 = mul nsw i64 %40, %39
  %46 = trunc i64 %45 to i32
  %47 = getelementptr inbounds i8, ptr %44, i32 %46
  %48 = trunc i64 %40 to i32
  tail call fastcc void @copy_bytes(ptr noundef nonnull %43, ptr noundef %47, i32 noundef %48) #53
  %49 = add nuw nsw i64 %35, 1
  %50 = load i64, ptr %3, align 16, !tbaa !340
  %51 = icmp slt i64 %49, %50
  br i1 %51, label %34, label %30, !llvm.loop !360

52:                                               ; preds = %30
  %53 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 5
  %54 = load ptr, ptr %53, align 16, !tbaa !343
  %55 = load i64, ptr %17, align 8, !tbaa !338
  %56 = mul nsw i64 %55, %32
  %57 = trunc i64 %56 to i32
  tail call void @fa_free(ptr noundef %54, i32 noundef %57, i32 noundef 16) #54
  %58 = load i64, ptr %3, align 16, !tbaa !340
  br label %59

59:                                               ; preds = %52, %30
  %60 = phi i64 [ %58, %52 ], [ %31, %30 ]
  %61 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 5
  store ptr %21, ptr %61, align 16, !tbaa !343
  %62 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 6
  store i64 0, ptr %62, align 8, !tbaa !342
  store i64 %16, ptr %5, align 8, !tbaa !341
  br label %63

63:                                               ; preds = %8, %59
  %64 = phi ptr [ %12, %8 ], [ %21, %59 ]
  %65 = phi i64 [ %6, %8 ], [ %16, %59 ]
  %66 = phi i64 [ %4, %8 ], [ %60, %59 ]
  %67 = phi i64 [ %10, %8 ], [ 0, %59 ]
  %68 = add nsw i64 %66, %67
  %69 = srem i64 %68, %65
  %70 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 4
  %71 = load i64, ptr %70, align 8, !tbaa !338
  %72 = mul nsw i64 %71, %69
  %73 = trunc i64 %72 to i32
  %74 = getelementptr inbounds i8, ptr %64, i32 %73
  %75 = trunc i64 %71 to i32
  tail call fastcc void @copy_bytes(ptr noundef %74, ptr noundef %1, i32 noundef %75) #53
  %76 = load i64, ptr %3, align 16, !tbaa !340
  %77 = add nsw i64 %76, 1
  store i64 %77, ptr %3, align 16, !tbaa !340
  ret void
}

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.abs.i64(i64, i1 immarg) #51

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.umin.i32(i32, i32) #51

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.umax.i32(i32, i32) #51

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.smin.i64(i64, i64) #51

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.smax.i64(i64, i64) #51

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.umin.i64(i64, i64) #51

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.fshl.i64(i64, i64, i64) #51

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.usub.sat.i64(i64, i64) #51

; Function Attrs: nocallback nofree nounwind willreturn memory(argmem: write)
declare void @llvm.memset.p0.i64(ptr nocapture writeonly, i8, i64, i1 immarg) #52

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i128 @llvm.abs.i128(i128, i1 immarg) #51

attributes #0 = { alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: readwrite) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #1 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: readwrite) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #2 = { nounwind optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" "wasm-export-name"="flush" }
attributes #3 = { nounwind optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #4 = { optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #5 = { noreturn nounwind optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #6 = { noreturn optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #7 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, argmem: none, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #8 = { mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite) }
attributes #9 = { mustprogress nocallback nofree nounwind willreturn memory(argmem: readwrite) }
attributes #10 = { nofree norecurse nosync nounwind optsize memory(argmem: write) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #11 = { mustprogress nocallback nofree nounwind willreturn memory(argmem: write) }
attributes #12 = { nofree norecurse nosync nounwind optsize memory(argmem: read) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #13 = { alwaysinline nounwind optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #14 = { nofree norecurse nosync nounwind optsize memory(read, argmem: readwrite, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #15 = { noinline nounwind optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #16 = { alwaysinline nofree norecurse nosync nounwind optsize memory(argmem: read) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #17 = { nofree noinline norecurse nosync nounwind optsize memory(argmem: read) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #18 = { nofree norecurse nosync nounwind optsize memory(argmem: readwrite) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #19 = { nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #20 = { nofree noinline norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #21 = { cold noinline nounwind optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #22 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #23 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: write) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #24 = { optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" "wasm-import-module"="fa" "wasm-import-name"="argumentsSize" }
attributes #25 = { optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" "wasm-import-module"="fa" "wasm-import-name"="argumentsCopy" }
attributes #26 = { alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #27 = { alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(readwrite, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #28 = { mustprogress nofree noinline norecurse nosync nounwind optsize willreturn memory(none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #29 = { mustprogress nofree noinline norecurse nosync nounwind optsize willreturn memory(argmem: readwrite) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #30 = { alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(readwrite, argmem: none, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #31 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(readwrite, argmem: read, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #32 = { mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #33 = { nofree norecurse nounwind optsize memory(readwrite, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #34 = { nofree norecurse nosync nounwind optsize memory(read, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #35 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(read, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #36 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(read, argmem: none, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #37 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #38 = { cold noinline noreturn nounwind optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #39 = { cold mustprogress nofree noinline norecurse nosync nounwind optsize willreturn memory(none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #40 = { optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" "wasm-import-module"="fa" "wasm-import-name"="pow" }
attributes #41 = { alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(read, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #42 = { nofree norecurse nosync nounwind optsize memory(write, argmem: readwrite, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #43 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(readwrite, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #44 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, argmem: readwrite, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #45 = { alwaysinline nofree norecurse nosync nounwind optsize memory(read, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #46 = { mustprogress nofree norecurse nounwind optsize willreturn memory(argmem: readwrite) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #47 = { mustprogress nofree norecurse nounwind optsize willreturn memory(readwrite, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #48 = { optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" "wasm-import-module"="fa" "wasm-import-name"="timer" }
attributes #49 = { optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" "wasm-import-module"="fa" "wasm-import-name"="formatFloat" }
attributes #50 = { nofree norecurse nosync nounwind optsize memory(none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #51 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #52 = { nocallback nofree nounwind willreturn memory(argmem: write) }
attributes #53 = { optsize }
attributes #54 = { nounwind optsize }
attributes #55 = { noreturn nounwind optsize }
attributes #56 = { nounwind }
attributes #57 = { noreturn optsize }
attributes #58 = { cold optsize }
attributes #59 = { cold noreturn optsize }

!llvm.module.flags = !{!0}

!0 = !{i32 1, !"wchar_size", i32 4}
!1 = !{!2, !2, i64 0}
!2 = !{!"omnipotent char", !3, i64 0}
!3 = !{!"Simple C/C++ TBAA"}
!4 = !{!5, !5, i64 0}
!5 = !{!"long", !2, i64 0}
!6 = !{!7, !7, i64 0}
!7 = !{!"any pointer", !2, i64 0}
!8 = distinct !{!8, !9}
!9 = !{!"llvm.loop.mustprogress"}
!10 = !{i64 0, i64 4, !6, i64 4, i64 4, !4, i64 8, i64 4, !4}
!11 = !{i64 0, i64 4, !4, i64 4, i64 4, !4}
!12 = !{i64 0, i64 4, !4}
!13 = !{!14, !7, i64 0}
!14 = !{!"", !7, i64 0, !5, i64 4, !5, i64 8}
!15 = !{!16}
!16 = distinct !{!16, !17, !"ascii_string: argument 0"}
!17 = distinct !{!17, !"ascii_string"}
!18 = !{!14, !5, i64 4}
!19 = !{!14, !5, i64 8}
!20 = !{i32 0, i32 -2147483647}
!21 = distinct !{!21, !9}
!22 = distinct !{!22, !9}
!23 = !{!24, !24, i64 0}
!24 = !{!"int", !2, i64 0}
!25 = distinct !{!25, !9}
!26 = distinct !{!26, !9}
!27 = distinct !{!27, !9}
!28 = distinct !{!28, !9}
!29 = distinct !{!29, !9}
!30 = distinct !{!30, !9}
!31 = distinct !{!31, !9}
!32 = distinct !{!32, !9}
!33 = distinct !{!33, !9}
!34 = !{!"branch_weights", i32 4000000, i32 4001}
!35 = !{!"branch_weights", i32 1, i32 2000}
!36 = distinct !{!36, !9}
!37 = distinct !{!37, !9}
!38 = !{i32 0, i32 13}
!39 = distinct !{!39, !9}
!40 = distinct !{!40, !9}
!41 = !{i64 0, i64 65}
!42 = distinct !{!42, !9}
!43 = distinct !{!43, !9}
!44 = !{i64 7, i64 65}
!45 = distinct !{!45, !9}
!46 = distinct !{!46, !9}
!47 = distinct !{!47, !9}
!48 = !{!49, !7, i64 0}
!49 = !{!"", !7, i64 0, !5, i64 4, !5, i64 8, !5, i64 12}
!50 = !{!49, !5, i64 4}
!51 = !{!49, !5, i64 8}
!52 = !{!49, !5, i64 12}
!53 = !{i32 0, i32 2}
!54 = distinct !{!54, !9}
!55 = distinct !{!55, !9}
!56 = distinct !{!56, !9}
!57 = distinct !{!57, !9}
!58 = !{!59, !7, i64 0}
!59 = !{!"", !7, i64 0, !5, i64 4, !5, i64 8, !7, i64 12, !5, i64 16, !24, i64 20, !24, i64 24}
!60 = !{!59, !5, i64 4}
!61 = !{!59, !5, i64 8}
!62 = !{!59, !7, i64 12}
!63 = !{!59, !5, i64 16}
!64 = !{!59, !24, i64 20}
!65 = !{!59, !24, i64 24}
!66 = !{ptr @replace_one, ptr @split_one}
!67 = distinct !{!67, !9}
!68 = distinct !{!68, !9}
!69 = !{!70, !7, i64 0}
!70 = !{!"", !7, i64 0, !71, i64 8, !71, i64 16}
!71 = !{!"long long", !2, i64 0}
!72 = distinct !{!72, !9}
!73 = !{!70, !71, i64 16}
!74 = distinct !{!74, !9}
!75 = !{!76, !7, i64 0}
!76 = !{!"", !7, i64 0, !5, i64 4, !5, i64 8, !7, i64 12, !5, i64 16}
!77 = !{!76, !5, i64 4}
!78 = !{!76, !5, i64 8}
!79 = !{!70, !71, i64 8}
!80 = !{!76, !7, i64 12}
!81 = !{!76, !5, i64 16}
!82 = !{!"branch_weights", i32 2000, i32 1}
!83 = distinct !{!83, !9}
!84 = distinct !{!84, !9}
!85 = distinct !{!85, !9}
!86 = distinct !{!86, !9}
!87 = distinct !{!87, !9}
!88 = distinct !{!88, !9}
!89 = !{!71, !71, i64 0}
!90 = distinct !{!90, !9}
!91 = distinct !{!91, !9}
!92 = distinct !{!92, !9}
!93 = !{!94, !71, i64 0}
!94 = !{!"", !71, i64 0, !71, i64 8, !71, i64 16}
!95 = !{!94, !71, i64 8}
!96 = !{!94, !71, i64 16}
!97 = distinct !{!97, !9}
!98 = distinct !{!98, !9}
!99 = distinct !{!99, !9}
!100 = distinct !{!100, !9}
!101 = distinct !{!101, !9}
!102 = distinct !{!102, !9}
!103 = distinct !{!103, !9}
!104 = distinct !{!104, !9}
!105 = !{i64 0, i64 1, !4}
!106 = distinct !{!106, !9}
!107 = distinct !{!107, !9}
!108 = distinct !{!108, !9}
!109 = distinct !{!109, !9}
!110 = distinct !{!110, !9}
!111 = distinct !{!111, !9}
!112 = distinct !{!112, !9}
!113 = !{!114, !71, i64 216}
!114 = !{!"", !71, i64 0, !71, i64 8, !7, i64 16, !7, i64 20, !2, i64 24, !71, i64 216}
!115 = !{!114, !71, i64 0}
!116 = distinct !{!116, !9}
!117 = !{!114, !71, i64 8}
!118 = !{!114, !7, i64 16}
!119 = !{!114, !7, i64 20}
!120 = distinct !{!120, !9}
!121 = distinct !{!121, !9}
!122 = distinct !{!122, !9}
!123 = distinct !{!123, !9}
!124 = distinct !{!124, !9}
!125 = !{!126, !7, i64 0}
!126 = !{!"FreeBlock", !7, i64 0}
!127 = !{!128, !24, i64 0}
!128 = !{!"", !24, i64 0, !24, i64 4}
!129 = !{!128, !24, i64 4}
!130 = distinct !{!130, !9}
!131 = !{!132, !5, i64 4}
!132 = !{!"FreeSpan", !7, i64 0, !5, i64 4}
!133 = distinct !{!133, !9}
!134 = !{!132, !7, i64 0}
!135 = !{!136, !7, i64 0}
!136 = !{!"FaArena", !7, i64 0, !7, i64 4, !7, i64 8, !7, i64 12, !7, i64 16}
!137 = !{!138, !7, i64 4}
!138 = !{!"FaArenaCleanup", !7, i64 0, !7, i64 4, !7, i64 8}
!139 = !{!138, !7, i64 8}
!140 = distinct !{!140, !9}
!141 = !{!136, !7, i64 4}
!142 = distinct !{!142, !9}
!143 = !{!136, !7, i64 8}
!144 = !{!136, !7, i64 12}
!145 = !{!136, !7, i64 16}
!146 = !{!138, !7, i64 0}
!147 = !{!148, !71, i64 48}
!148 = !{!"", !71, i64 0, !7, i64 8, !7, i64 12, !7, i64 16, !24, i64 20, !24, i64 24, !24, i64 28, !24, i64 32, !24, i64 36, !24, i64 40, !71, i64 48}
!149 = !{!148, !71, i64 0}
!150 = !{!148, !24, i64 28}
!151 = !{!148, !24, i64 32}
!152 = !{!148, !24, i64 20}
!153 = !{!148, !24, i64 24}
!154 = !{!148, !7, i64 8}
!155 = !{!148, !7, i64 12}
!156 = distinct !{!156, !9}
!157 = distinct !{!157, !9}
!158 = !{!148, !24, i64 36}
!159 = !{!148, !24, i64 40}
!160 = distinct !{!160, !9}
!161 = !{!148, !7, i64 16}
!162 = distinct !{!162, !9}
!163 = distinct !{!163, !9}
!164 = distinct !{!164, !9}
!165 = distinct !{!165, !9}
!166 = !{!167, !7, i64 32}
!167 = !{!"", !71, i64 0, !7, i64 8, !71, i64 16, !71, i64 24, !7, i64 32, !71, i64 40}
!168 = !{!167, !71, i64 16}
!169 = !{!167, !7, i64 8}
!170 = distinct !{!170, !9}
!171 = !{!167, !71, i64 40}
!172 = !{!167, !71, i64 24}
!173 = !{!167, !71, i64 0}
!174 = distinct !{!174, !9}
!175 = distinct !{!175, !9}
!176 = distinct !{!176, !9}
!177 = distinct !{!177, !9}
!178 = distinct !{!178, !9}
!179 = distinct !{!179, !9}
!180 = distinct !{!180, !9}
!181 = distinct !{!181, !9}
!182 = !{!183, !71, i64 8}
!183 = !{!"", !7, i64 0, !7, i64 4, !71, i64 8}
!184 = !{!183, !7, i64 0}
!185 = !{!183, !7, i64 4}
!186 = distinct !{!186, !9}
!187 = distinct !{!187, !9}
!188 = !{!189, !71, i64 48}
!189 = !{!"FaSource", !71, i64 0, !71, i64 8, !7, i64 16, !71, i64 24, !71, i64 32, !7, i64 40, !7, i64 44, !71, i64 48, !71, i64 56}
!190 = !{!189, !7, i64 44}
!191 = !{!192, !71, i64 0}
!192 = !{!"", !71, i64 0, !71, i64 8, !7, i64 16}
!193 = !{!192, !71, i64 8}
!194 = distinct !{!194, !9}
!195 = !{!192, !7, i64 16}
!196 = !{!189, !7, i64 40}
!197 = !{i64 0, i64 8, !89, i64 8, i64 8, !89, i64 16, i64 4, !6}
!198 = !{i64 0, i64 8, !89, i64 8, i64 4, !6}
!199 = !{i64 0, i64 4, !6}
!200 = distinct !{!200, !9}
!201 = !{!202, !71, i64 96}
!202 = !{!"FaObserver", !189, i64 0, !7, i64 64, !7, i64 68, !7, i64 72, !7, i64 76, !7, i64 80, !7, i64 84, !7, i64 88, !71, i64 96, !71, i64 104, !71, i64 112, !24, i64 120, !24, i64 124, !24, i64 128, !24, i64 132, !24, i64 136, !24, i64 140, !24, i64 144, !24, i64 148, !71, i64 152, !2, i64 160}
!203 = !{!202, !7, i64 84}
!204 = distinct !{!204, !9}
!205 = !{!202, !71, i64 104}
!206 = !{!202, !7, i64 88}
!207 = !{!189, !71, i64 8}
!208 = !{!189, !71, i64 24}
!209 = !{!189, !7, i64 16}
!210 = distinct !{!210, !9}
!211 = distinct !{!211, !9}
!212 = distinct !{!212, !9}
!213 = distinct !{!213, !9}
!214 = !{!202, !71, i64 0}
!215 = !{!202, !24, i64 120}
!216 = !{!202, !7, i64 72}
!217 = !{!202, !7, i64 76}
!218 = !{!202, !7, i64 80}
!219 = !{!202, !7, i64 68}
!220 = !{!202, !7, i64 64}
!221 = !{!202, !71, i64 152}
!222 = !{!202, !24, i64 124}
!223 = !{!202, !24, i64 128}
!224 = !{!202, !24, i64 140}
!225 = !{!202, !71, i64 8}
!226 = !{!202, !24, i64 144}
!227 = !{!202, !71, i64 112}
!228 = !{!202, !24, i64 136}
!229 = distinct !{!229, !9}
!230 = distinct !{!230, !9}
!231 = distinct !{!231, !9}
!232 = distinct !{!232, !9}
!233 = distinct !{!233, !9}
!234 = !{!202, !24, i64 132}
!235 = distinct !{!235, !9}
!236 = distinct !{!236, !9}
!237 = !{!189, !71, i64 0}
!238 = !{!189, !71, i64 32}
!239 = !{!189, !71, i64 56}
!240 = distinct !{!240, !9}
!241 = !{!242, !71, i64 0}
!242 = !{!"", !71, i64 0, !14, i64 8}
!243 = distinct !{!243, !9}
!244 = distinct !{!244, !9}
!245 = distinct !{!245, !9}
!246 = !{!247, !71, i64 0}
!247 = !{!"", !71, i64 0, !14, i64 8, !14, i64 20}
!248 = !{!249, !7, i64 0}
!249 = !{!"", !7, i64 0, !71, i64 8, !71, i64 16, !71, i64 24, !7, i64 32, !71, i64 40, !71, i64 48, !71, i64 56, !71, i64 64, !71, i64 72}
!250 = !{!249, !71, i64 72}
!251 = !{!249, !71, i64 8}
!252 = !{!249, !71, i64 16}
!253 = !{!249, !71, i64 24}
!254 = !{!249, !7, i64 32}
!255 = !{!249, !71, i64 48}
!256 = !{!249, !71, i64 56}
!257 = distinct !{!257, !9}
!258 = !{!249, !71, i64 40}
!259 = !{!249, !71, i64 64}
!260 = distinct !{!260, !9}
!261 = distinct !{!261, !9}
!262 = distinct !{!262, !9}
!263 = distinct !{!263, !9}
!264 = distinct !{!264, !9}
!265 = distinct !{!265, !9}
!266 = distinct !{!266, !9}
!267 = distinct !{!267, !9}
!268 = distinct !{!268, !9}
!269 = !{!270, !7, i64 16}
!270 = !{!"", !71, i64 0, !71, i64 8, !7, i64 16, !71, i64 24}
!271 = !{!270, !71, i64 8}
!272 = !{!273, !71, i64 32}
!273 = !{!"", !270, i64 0, !71, i64 32}
!274 = !{!270, !71, i64 0}
!275 = !{!276, !24, i64 16}
!276 = !{!"FaTask", !7, i64 0, !7, i64 4, !7, i64 8, !24, i64 12, !24, i64 16}
!277 = !{!278, !71, i64 56}
!278 = !{!"FaExecutor", !7, i64 0, !7, i64 4, !71, i64 8, !7, i64 16, !71, i64 24, !71, i64 32, !71, i64 40, !7, i64 48, !71, i64 56, !71, i64 64, !71, i64 72, !7, i64 80, !7, i64 84, !276, i64 88, !24, i64 108, !24, i64 112, !24, i64 116, !24, i64 120, !71, i64 128, !71, i64 136, !7, i64 144, !7, i64 148, !2, i64 152, !24, i64 280}
!279 = !{!276, !7, i64 4}
!280 = !{!278, !7, i64 4}
!281 = !{!276, !7, i64 8}
!282 = !{!278, !7, i64 0}
!283 = !{!278, !71, i64 8}
!284 = !{!278, !7, i64 80}
!285 = !{!278, !7, i64 84}
!286 = !{!278, !24, i64 112}
!287 = !{!278, !24, i64 116}
!288 = !{!276, !7, i64 0}
!289 = !{!278, !7, i64 48}
!290 = !{!278, !7, i64 144}
!291 = !{!292, !7, i64 20}
!292 = !{!"", !7, i64 0, !71, i64 8, !7, i64 16, !7, i64 20}
!293 = !{!292, !7, i64 16}
!294 = !{!292, !71, i64 8}
!295 = !{!292, !7, i64 0}
!296 = distinct !{!296, !9}
!297 = !{!278, !71, i64 136}
!298 = !{!299, !7, i64 0}
!299 = !{!"FaWait", !7, i64 0, !7, i64 4, !7, i64 8, !7, i64 12, !7, i64 16, !7, i64 20, !71, i64 24, !71, i64 32, !71, i64 40, !24, i64 48, !24, i64 52}
!300 = !{!299, !7, i64 4}
!301 = !{!299, !71, i64 40}
!302 = !{!278, !71, i64 24}
!303 = !{!278, !71, i64 32}
!304 = !{!278, !7, i64 16}
!305 = distinct !{!305, !9}
!306 = !{!299, !71, i64 24}
!307 = !{!278, !71, i64 40}
!308 = !{!299, !71, i64 32}
!309 = distinct !{!309, !9}
!310 = !{!311, !24, i64 4}
!311 = !{!"FaIo", !24, i64 0, !24, i64 4, !7, i64 8, !7, i64 12, !7, i64 16}
!312 = !{!311, !7, i64 16}
!313 = !{!299, !24, i64 48}
!314 = !{!299, !24, i64 52}
!315 = !{!278, !71, i64 72}
!316 = !{!311, !7, i64 8}
!317 = !{!311, !7, i64 12}
!318 = !{!278, !24, i64 120}
!319 = !{!278, !71, i64 128}
!320 = distinct !{!320, !9}
!321 = distinct !{!321, !9}
!322 = distinct !{!322, !9}
!323 = distinct !{!323, !9}
!324 = distinct !{!324, !9}
!325 = !{!278, !24, i64 280}
!326 = distinct !{!326, !9}
!327 = !{!278, !7, i64 148}
!328 = !{!329, !7, i64 0}
!329 = !{!"FaJob", !7, i64 0, !7, i64 4, !7, i64 8, !7, i64 12, !7, i64 16, !24, i64 20}
!330 = !{!329, !24, i64 20}
!331 = !{!329, !7, i64 4}
!332 = !{!299, !7, i64 20}
!333 = !{!299, !7, i64 8}
!334 = !{!299, !7, i64 12}
!335 = !{!336, !71, i64 160}
!336 = !{!"", !270, i64 0, !337, i64 32, !337, i64 96, !71, i64 160, !71, i64 168, !7, i64 176, !71, i64 184, !71, i64 192, !71, i64 200, !24, i64 208, !24, i64 212, !7, i64 216, !7, i64 220, !7, i64 224}
!337 = !{!"", !2, i64 0}
!338 = !{!336, !71, i64 168}
!339 = !{!336, !7, i64 216}
!340 = !{!336, !71, i64 192}
!341 = !{!336, !71, i64 200}
!342 = !{!336, !71, i64 184}
!343 = !{!336, !7, i64 176}
!344 = distinct !{!344, !9}
!345 = !{!336, !24, i64 208}
!346 = !{!299, !7, i64 16}
!347 = !{!336, !7, i64 224}
!348 = distinct !{!348, !9}
!349 = distinct !{!349, !9}
!350 = distinct !{!350, !9}
!351 = !{!352, !352, i64 0}
!352 = !{!"__int128", !2, i64 0}
!353 = distinct !{!353, !9}
!354 = distinct !{!354, !9}
!355 = distinct !{!355, !9}
!356 = distinct !{!356, !9}
!357 = !{!202, !71, i64 24}
!358 = !{!202, !7, i64 16}
!359 = distinct !{!359, !9}
!360 = distinct !{!360, !9}
