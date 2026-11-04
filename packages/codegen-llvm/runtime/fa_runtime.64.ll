source_filename = "fa_runtime.c"

%struct.AlternateStack = type { ptr, i32, i64 }
%struct.SignalAction = type { ptr, [16 x i64], i32, ptr }
%struct.BareError = type { i64 }
%struct.anon = type { %struct.FaSystemLock, %struct.FaSystemLock, ptr, ptr, i32, i32, [8 x i8] }
%struct.FaSystemLock = type { [64 x i8] }
%struct.anon.0 = type { %struct.FaSystemLock, %struct.FaSystemLock, ptr, i64, i32, i32, i32 }
%struct.Range = type { i32, i32 }
%struct.FaEpollEvent = type <{ i32, i64 }>
%struct.FaString = type { ptr, i64, i64 }
%struct.Boundaries = type { ptr, i64, i64, i64 }
%struct.Replacing = type { ptr, i64, i64, ptr, i64, i32, i32 }
%struct.FaArrayHeader = type { ptr, i64, i64 }
%struct.Splitting = type { ptr, i64, i64, ptr, i64 }
%struct.IndexError = type { i64, i64, i64 }
%struct.FaSegmented = type { i64, i64, [48 x ptr] }
%struct.PageHeader = type { i32, i32 }
%struct.FreeSpan = type { ptr, i64 }
%struct.FaArena = type { ptr, ptr, ptr, ptr, ptr }
%struct.FaArenaCleanup = type { ptr, ptr, ptr }
%struct.FaPool = type { i64, ptr, ptr, ptr, i32, i32, i32, i32, i32, i32 }
%struct.PoolChunk = type { ptr, i32, i32, i32, i32, [0 x i64] }
%struct.FaLinks = type { i64, ptr, i64, i64, ptr, i64 }
%struct.FaAddressMap = type { ptr, ptr, i64 }
%struct.FaSource = type { i64, i64, ptr, i64, i64, ptr, ptr, i64, i64 }
%struct.FaChild = type { i64, i64, ptr }
%struct.FaObserver = type { %struct.FaSource, ptr, ptr, ptr, ptr, ptr, ptr, ptr, i64, i64, i64, i32, i32, i32, i32, i32, i32, i32, i32, i64, [2 x ptr] }
%struct.MessageError = type { i64, %struct.FaString }
%struct.ParseError = type { i64, %struct.FaString, %struct.FaString }
%struct.FaTable = type { ptr, i64, i64, i64, ptr, i64, i64, i64, i64 }
%struct.FaTableEntries = type { ptr, i64, i64 }
%struct.FaShared = type { i64, i64, ptr, i64 }
%struct.FaAtomicBlock = type { %struct.FaShared, i64 }
%struct.FaExecutor = type { ptr, ptr, ptr, ptr, i64, i64, ptr, i64, i64, i64, ptr, i64, i64, i64, i64, ptr, ptr, %struct.FaTask, i32, i32, i32, i32, i64, i32, i64, ptr, ptr, [32 x ptr], i32 }
%struct.FaTask = type { ptr, ptr, ptr, i32, i32 }
%struct.FaJoin = type { ptr, i64, ptr, ptr }
%struct.FaWait = type { ptr, ptr, ptr, ptr, ptr, ptr, i64, i64, i64, i32, i32, ptr }
%struct.FaTimer = type { i64, ptr }
%struct.FaIo = type { i32, i32, ptr, ptr, ptr }
%struct.FaJob = type { ptr, ptr, ptr, ptr, ptr, i32 }
%struct.FaTimespec = type { i64, i64 }
%struct.FaSocket = type { %struct.FaShared, i32, i32, i64, ptr }
%struct.FaSocketAddress = type { i16, i16, i32, [8 x i8] }
%struct.FaPollFd = type { i32, i16, i16 }
%struct.FaFileJob = type { %struct.FaJob, i32, i32, ptr, i64, ptr, i64, i64 }
%struct.FileErrorBox = type { i64, %struct.FaString, %struct.FaString }
%struct.FaWorker = type { %struct.FaShared, %struct.FaSystemLock, %struct.FaSystemLock, ptr, ptr, ptr, ptr, i64, i32, i32, i32, i32, ptr }
%struct.FaCall = type { ptr, ptr, ptr, ptr, ptr, ptr, ptr, i64, i32, i32, ptr, i64, ptr, i64, ptr, ptr, ptr }
%struct.FaWorkerPool = type { %struct.FaShared, %struct.FaSystemLock, %struct.FaSystemLock, %struct.FaSystemLock, ptr, i64, ptr, ptr, i64, i64, i64, ptr, ptr, ptr, i32, [12 x i8] }
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
@fa_compact_characters = local_unnamed_addr global i32 1, align 4
@.str.12 = private unnamed_addr constant [3 x i8] c"SS\00", align 1
@argument_count = internal unnamed_addr global i64 0, align 8
@argument_items = internal unnamed_addr global ptr null, align 8
@free_blocks = internal thread_local(initialexec) global [40 x ptr] zeroinitializer, align 16
@class_next = internal thread_local(initialexec) global [40 x ptr] zeroinitializer, align 16
@class_end = internal thread_local(initialexec) global [40 x ptr] zeroinitializer, align 16
@tree_arena_depth = internal thread_local(initialexec) global i64 0, align 8
@tree_arena = internal thread_local(initialexec) global ptr null, align 8
@builder_capacity = internal thread_local(initialexec) global i64 0, align 8
@fa_live_blocks = global i64 0, align 8
@live_tree_arenas = internal global i64 0, align 8
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
@blocking = internal global %struct.anon zeroinitializer, align 16
@workers_alive = internal global i64 0, align 8
@current_worker = internal thread_local(initialexec) global ptr null, align 8
@fa_blocking_depth = thread_local(initialexec) global i32 0, align 4
@.str.31 = private unnamed_addr constant [38 x i8] c"WorkerPool.submit (the queue is full)\00", align 1
@worker_timeout = internal constant %struct.BareError { i64 8 }, align 8
@.str.32 = private unnamed_addr constant [38 x i8] c"`lock()` on a mutex this thread holds\00", align 1
@.str.33 = private unnamed_addr constant [60 x i8] c"`send` on a full channel that no other thread receives from\00", align 1
@.str.34 = private unnamed_addr constant [60 x i8] c"`receive` on an empty channel that no other thread sends to\00", align 1
@in_parallel = internal thread_local(initialexec) global i32 0, align 4
@pool = internal global %struct.anon.0 zeroinitializer, align 16
@fa_turn_site = thread_local(initialexec) global ptr null, align 8
@.str.35 = private unnamed_addr constant [1 x i8] zeroinitializer, align 1
@decimal_u64.pairs = internal unnamed_addr constant [201 x i8] c"00010203040506070809101112131415161718192021222324252627282930313233343536373839404142434445464748495051525354555657585960616263646566676869707172737475767778798081828384858687888990919293949596979899\00", align 16
@.str.36 = private unnamed_addr constant [4 x i8] c"NaN\00", align 1
@.str.37 = private unnamed_addr constant [9 x i8] c"Infinity\00", align 1
@.str.38 = private unnamed_addr constant [3 x i8] c"0.\00", align 1
@.str.39 = private unnamed_addr constant [5 x i8] c"%.*e\00", align 1
@.str.40 = private unnamed_addr constant [8 x i8] c"%llde%d\00", align 1
@.str.41 = private unnamed_addr constant [6 x i8] c"%.14e\00", align 1
@out_of_memory.message = internal constant [21 x i8] c"stop: out of memory\0A\00", align 16
@EXTEND = internal unnamed_addr constant [153 x %struct.Range] [%struct.Range { i32 768, i32 879 }, %struct.Range { i32 1155, i32 1161 }, %struct.Range { i32 1425, i32 1469 }, %struct.Range { i32 1471, i32 1471 }, %struct.Range { i32 1473, i32 1474 }, %struct.Range { i32 1476, i32 1477 }, %struct.Range { i32 1479, i32 1479 }, %struct.Range { i32 1552, i32 1562 }, %struct.Range { i32 1611, i32 1631 }, %struct.Range { i32 1648, i32 1648 }, %struct.Range { i32 1750, i32 1756 }, %struct.Range { i32 1759, i32 1764 }, %struct.Range { i32 1767, i32 1768 }, %struct.Range { i32 1770, i32 1773 }, %struct.Range { i32 1809, i32 1809 }, %struct.Range { i32 1840, i32 1866 }, %struct.Range { i32 1958, i32 1968 }, %struct.Range { i32 2027, i32 2035 }, %struct.Range { i32 2070, i32 2093 }, %struct.Range { i32 2137, i32 2139 }, %struct.Range { i32 2200, i32 2207 }, %struct.Range { i32 2250, i32 2273 }, %struct.Range { i32 2275, i32 2307 }, %struct.Range { i32 2362, i32 2364 }, %struct.Range { i32 2366, i32 2383 }, %struct.Range { i32 2385, i32 2391 }, %struct.Range { i32 2402, i32 2403 }, %struct.Range { i32 2433, i32 2435 }, %struct.Range { i32 2492, i32 2492 }, %struct.Range { i32 2494, i32 2509 }, %struct.Range { i32 2519, i32 2519 }, %struct.Range { i32 2530, i32 2531 }, %struct.Range { i32 2561, i32 2563 }, %struct.Range { i32 2620, i32 2641 }, %struct.Range { i32 2672, i32 2673 }, %struct.Range { i32 2677, i32 2677 }, %struct.Range { i32 2689, i32 2691 }, %struct.Range { i32 2748, i32 2748 }, %struct.Range { i32 2750, i32 2765 }, %struct.Range { i32 2786, i32 2787 }, %struct.Range { i32 2817, i32 2819 }, %struct.Range { i32 2876, i32 2876 }, %struct.Range { i32 2878, i32 2903 }, %struct.Range { i32 2914, i32 2915 }, %struct.Range { i32 2946, i32 2946 }, %struct.Range { i32 3006, i32 3021 }, %struct.Range { i32 3031, i32 3031 }, %struct.Range { i32 3072, i32 3076 }, %struct.Range { i32 3132, i32 3132 }, %struct.Range { i32 3134, i32 3158 }, %struct.Range { i32 3170, i32 3171 }, %struct.Range { i32 3201, i32 3203 }, %struct.Range { i32 3260, i32 3260 }, %struct.Range { i32 3262, i32 3286 }, %struct.Range { i32 3298, i32 3299 }, %struct.Range { i32 3328, i32 3331 }, %struct.Range { i32 3387, i32 3388 }, %struct.Range { i32 3390, i32 3405 }, %struct.Range { i32 3415, i32 3415 }, %struct.Range { i32 3426, i32 3427 }, %struct.Range { i32 3457, i32 3459 }, %struct.Range { i32 3530, i32 3551 }, %struct.Range { i32 3570, i32 3571 }, %struct.Range { i32 3633, i32 3633 }, %struct.Range { i32 3635, i32 3642 }, %struct.Range { i32 3655, i32 3662 }, %struct.Range { i32 3761, i32 3761 }, %struct.Range { i32 3763, i32 3772 }, %struct.Range { i32 3784, i32 3790 }, %struct.Range { i32 3864, i32 3865 }, %struct.Range { i32 3893, i32 3893 }, %struct.Range { i32 3895, i32 3895 }, %struct.Range { i32 3897, i32 3897 }, %struct.Range { i32 3902, i32 3903 }, %struct.Range { i32 3953, i32 3972 }, %struct.Range { i32 3974, i32 3975 }, %struct.Range { i32 3981, i32 4028 }, %struct.Range { i32 4038, i32 4038 }, %struct.Range { i32 4139, i32 4158 }, %struct.Range { i32 4182, i32 4185 }, %struct.Range { i32 4190, i32 4192 }, %struct.Range { i32 4194, i32 4196 }, %struct.Range { i32 4199, i32 4205 }, %struct.Range { i32 4209, i32 4212 }, %struct.Range { i32 4226, i32 4237 }, %struct.Range { i32 4239, i32 4239 }, %struct.Range { i32 4250, i32 4253 }, %struct.Range { i32 4957, i32 4959 }, %struct.Range { i32 5906, i32 5909 }, %struct.Range { i32 5938, i32 5940 }, %struct.Range { i32 5970, i32 5971 }, %struct.Range { i32 6002, i32 6003 }, %struct.Range { i32 6068, i32 6099 }, %struct.Range { i32 6109, i32 6109 }, %struct.Range { i32 6155, i32 6157 }, %struct.Range { i32 6159, i32 6159 }, %struct.Range { i32 6277, i32 6278 }, %struct.Range { i32 6313, i32 6313 }, %struct.Range { i32 6432, i32 6459 }, %struct.Range { i32 6679, i32 6683 }, %struct.Range { i32 6741, i32 6783 }, %struct.Range { i32 6832, i32 6911 }, %struct.Range { i32 6912, i32 6916 }, %struct.Range { i32 6964, i32 6980 }, %struct.Range { i32 7019, i32 7027 }, %struct.Range { i32 7040, i32 7042 }, %struct.Range { i32 7073, i32 7085 }, %struct.Range { i32 7142, i32 7155 }, %struct.Range { i32 7204, i32 7223 }, %struct.Range { i32 7376, i32 7378 }, %struct.Range { i32 7380, i32 7400 }, %struct.Range { i32 7405, i32 7405 }, %struct.Range { i32 7412, i32 7412 }, %struct.Range { i32 7415, i32 7417 }, %struct.Range { i32 7616, i32 7679 }, %struct.Range { i32 8204, i32 8204 }, %struct.Range { i32 8400, i32 8432 }, %struct.Range { i32 11503, i32 11505 }, %struct.Range { i32 11647, i32 11647 }, %struct.Range { i32 11744, i32 11775 }, %struct.Range { i32 12330, i32 12335 }, %struct.Range { i32 12441, i32 12442 }, %struct.Range { i32 42607, i32 42610 }, %struct.Range { i32 42612, i32 42621 }, %struct.Range { i32 42654, i32 42655 }, %struct.Range { i32 42736, i32 42737 }, %struct.Range { i32 43010, i32 43010 }, %struct.Range { i32 43014, i32 43014 }, %struct.Range { i32 43019, i32 43019 }, %struct.Range { i32 43043, i32 43047 }, %struct.Range { i32 43052, i32 43052 }, %struct.Range { i32 43136, i32 43137 }, %struct.Range { i32 43188, i32 43205 }, %struct.Range { i32 43232, i32 43249 }, %struct.Range { i32 43263, i32 43263 }, %struct.Range { i32 43302, i32 43309 }, %struct.Range { i32 43335, i32 43347 }, %struct.Range { i32 43392, i32 43395 }, %struct.Range { i32 43443, i32 43456 }, %struct.Range { i32 43561, i32 43574 }, %struct.Range { i32 43587, i32 43587 }, %struct.Range { i32 43596, i32 43597 }, %struct.Range { i32 43755, i32 43759 }, %struct.Range { i32 43765, i32 43766 }, %struct.Range { i32 44003, i32 44010 }, %struct.Range { i32 44012, i32 44013 }, %struct.Range { i32 64286, i32 64286 }, %struct.Range { i32 65024, i32 65039 }, %struct.Range { i32 65056, i32 65071 }, %struct.Range { i32 65438, i32 65439 }, %struct.Range { i32 127995, i32 127999 }, %struct.Range { i32 917536, i32 917631 }, %struct.Range { i32 917760, i32 917999 }], align 16
@PICTOGRAPHIC = internal unnamed_addr constant [22 x %struct.Range] [%struct.Range { i32 169, i32 169 }, %struct.Range { i32 174, i32 174 }, %struct.Range { i32 8252, i32 8252 }, %struct.Range { i32 8265, i32 8265 }, %struct.Range { i32 8482, i32 8482 }, %struct.Range { i32 8505, i32 8505 }, %struct.Range { i32 8596, i32 8618 }, %struct.Range { i32 8986, i32 9215 }, %struct.Range { i32 9410, i32 9410 }, %struct.Range { i32 9642, i32 9726 }, %struct.Range { i32 9728, i32 10175 }, %struct.Range { i32 10548, i32 10549 }, %struct.Range { i32 11013, i32 11093 }, %struct.Range { i32 12336, i32 12336 }, %struct.Range { i32 12349, i32 12349 }, %struct.Range { i32 12951, i32 12951 }, %struct.Range { i32 12953, i32 12953 }, %struct.Range { i32 126976, i32 127231 }, %struct.Range { i32 127245, i32 127405 }, %struct.Range { i32 127489, i32 127994 }, %struct.Range { i32 128000, i32 129791 }, %struct.Range { i32 130048, i32 131069 }], align 16
@ascii_table = internal constant [128 x i8] c"\00\01\02\03\04\05\06\07\08\09\0A\0B\0C\0D\0E\0F\10\11\12\13\14\15\16\17\18\19\1A\1B\1C\1D\1E\1F !\22#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\7F", align 16
@spans_lock = internal global i32 0, align 4
@free_spans = internal unnamed_addr global ptr null, align 8
@arena_refill.message = internal constant [21 x i8] c"stop: out of memory\0A\00", align 16
@.str.42 = private unnamed_addr constant [34 x i8] c"a derived value depends on itself\00", align 1
@.str.43 = private unnamed_addr constant [7 x i8] c"stop: \00", align 1
@.str.44 = private unnamed_addr constant [7 x i8] c"\0A  at \00", align 1
@on_fault.message = internal constant [22 x i8] c"stop: stack overflow\0A\00", align 16
@__const.executor.event = private unnamed_addr constant %struct.FaEpollEvent <{ i32 1, i64 0 }>, align 1
@.str.45 = private unnamed_addr constant [17 x i8] c"stop: deadlock: \00", align 1
@.str.46 = private unnamed_addr constant [37 x i8] c"warning: a task kept the thread for \00", align 1
@.str.47 = private unnamed_addr constant [6 x i8] c" ms (\00", align 1
@.str.48 = private unnamed_addr constant [7 x i8] c"a task\00", align 1
@.str.49 = private unnamed_addr constant [51 x i8] c"): await Task.yield() or move the work to a worker\00", align 1
@.str.50 = private unnamed_addr constant [33 x i8] c"stop: deadlock: every task waits\00", align 1
@.str.51 = private unnamed_addr constant [17 x i8] c" (main waits on \00", align 1
@.str.52 = private unnamed_addr constant [2 x i8] c")\00", align 1
@.str.53 = private unnamed_addr constant [19 x i8] c" (a task waits on \00", align 1
@parse_host.local = internal unnamed_addr constant [10 x i8] c"localhost\00", align 1
@.str.54 = private unnamed_addr constant [13 x i8] c"can't listen\00", align 1
@.str.55 = private unnamed_addr constant [7 x i8] c"accept\00", align 1
@.str.56 = private unnamed_addr constant [3 x i8] c": \00", align 1
@.str.57 = private unnamed_addr constant [44 x i8] c"unknown host (an IPv4 address or localhost)\00", align 1
@.str.58 = private unnamed_addr constant [26 x i8] c"no network on this target\00", align 1
@.str.59 = private unnamed_addr constant [21 x i8] c"the socket is closed\00", align 1
@.str.60 = private unnamed_addr constant [7 x i8] c"error \00", align 1
@.str.61 = private unnamed_addr constant [8 x i8] c"connect\00", align 1
@.str.62 = private unnamed_addr constant [5 x i8] c"read\00", align 1
@.str.63 = private unnamed_addr constant [6 x i8] c"write\00", align 1

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: readwrite)
define void @fa_copy_bytes(ptr nocapture noundef writeonly %0, ptr nocapture noundef readonly %1, i64 noundef %2) local_unnamed_addr #0 {
  tail call fastcc void @copy_bytes(ptr noundef %0, ptr noundef %1, i64 noundef %2) #44
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: readwrite)
define internal fastcc void @copy_bytes(ptr nocapture noundef writeonly %0, ptr nocapture noundef readonly %1, i64 noundef %2) unnamed_addr #1 {
  %4 = icmp ult i64 %2, 17
  br i1 %4, label %5, label %35

5:                                                ; preds = %3
  %6 = icmp ugt i64 %2, 7
  br i1 %6, label %7, label %14

7:                                                ; preds = %5
  %8 = load i64, ptr %1, align 1
  %9 = getelementptr inbounds i8, ptr %1, i64 %2
  %10 = getelementptr inbounds i8, ptr %9, i64 -8
  %11 = load i64, ptr %10, align 1
  store i64 %8, ptr %0, align 1
  %12 = getelementptr inbounds i8, ptr %0, i64 %2
  %13 = getelementptr inbounds i8, ptr %12, i64 -8
  store i64 %11, ptr %13, align 1
  br label %36

14:                                               ; preds = %5
  %15 = icmp ugt i64 %2, 3
  br i1 %15, label %16, label %23

16:                                               ; preds = %14
  %17 = load i32, ptr %1, align 1
  %18 = getelementptr inbounds i8, ptr %1, i64 %2
  %19 = getelementptr inbounds i8, ptr %18, i64 -4
  %20 = load i32, ptr %19, align 1
  store i32 %17, ptr %0, align 1
  %21 = getelementptr inbounds i8, ptr %0, i64 %2
  %22 = getelementptr inbounds i8, ptr %21, i64 -4
  store i32 %20, ptr %22, align 1
  br label %36

23:                                               ; preds = %14
  %24 = icmp eq i64 %2, 0
  br i1 %24, label %36, label %25

25:                                               ; preds = %23
  %26 = load i8, ptr %1, align 1, !tbaa !2
  %27 = lshr i64 %2, 1
  %28 = getelementptr inbounds i8, ptr %1, i64 %27
  %29 = load i8, ptr %28, align 1, !tbaa !2
  %30 = add nsw i64 %2, -1
  %31 = getelementptr inbounds i8, ptr %1, i64 %30
  %32 = load i8, ptr %31, align 1, !tbaa !2
  store i8 %26, ptr %0, align 1, !tbaa !2
  %33 = getelementptr inbounds i8, ptr %0, i64 %27
  store i8 %29, ptr %33, align 1, !tbaa !2
  %34 = getelementptr inbounds i8, ptr %0, i64 %30
  store i8 %32, ptr %34, align 1, !tbaa !2
  br label %36

35:                                               ; preds = %3
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %0, ptr align 1 %1, i64 %2, i1 false)
  br label %36

36:                                               ; preds = %7, %23, %25, %16, %35
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_flush() local_unnamed_addr #2 {
  %1 = atomicrmw xchg ptr @output_lock, i32 1 acquire, align 4
  %2 = icmp eq i32 %1, 0
  br i1 %2, label %12, label %6

3:                                                ; preds = %9, %6
  %4 = atomicrmw xchg ptr @output_lock, i32 1 acquire, align 4
  %5 = icmp eq i32 %4, 0
  br i1 %5, label %12, label %6, !llvm.loop !5

6:                                                ; preds = %0, %3
  %7 = load atomic i32, ptr @output_lock monotonic, align 4
  %8 = icmp eq i32 %7, 0
  br i1 %8, label %3, label %9

9:                                                ; preds = %6, %9
  tail call void @llvm.x86.sse2.pause()
  %10 = load atomic i32, ptr @output_lock monotonic, align 4
  %11 = icmp eq i32 %10, 0
  br i1 %11, label %3, label %9, !llvm.loop !7

12:                                               ; preds = %3, %0
  %13 = load i64, ptr @output_length, align 8, !tbaa !8
  %14 = icmp eq i64 %13, 0
  br i1 %14, label %16, label %15

15:                                               ; preds = %12
  tail call void @fa_write(i32 noundef 1, ptr noundef nonnull @output, i64 noundef %13) #45
  store i64 0, ptr @output_length, align 8, !tbaa !8
  br label %16

16:                                               ; preds = %12, %15
  store atomic i32 0, ptr @output_lock release, align 4
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_out(i32 noundef %0, ptr noundef %1, i64 noundef %2) local_unnamed_addr #2 {
  %4 = atomicrmw xchg ptr @output_lock, i32 1 acquire, align 4
  %5 = icmp eq i32 %4, 0
  br i1 %5, label %15, label %9

6:                                                ; preds = %12, %9
  %7 = atomicrmw xchg ptr @output_lock, i32 1 acquire, align 4
  %8 = icmp eq i32 %7, 0
  br i1 %8, label %15, label %9, !llvm.loop !5

9:                                                ; preds = %3, %6
  %10 = load atomic i32, ptr @output_lock monotonic, align 4
  %11 = icmp eq i32 %10, 0
  br i1 %11, label %6, label %12

12:                                               ; preds = %9, %12
  tail call void @llvm.x86.sse2.pause()
  %13 = load atomic i32, ptr @output_lock monotonic, align 4
  %14 = icmp eq i32 %13, 0
  br i1 %14, label %6, label %12, !llvm.loop !7

15:                                               ; preds = %6, %3
  %16 = icmp eq i32 %0, 1
  %17 = load i64, ptr @output_length, align 8, !tbaa !8
  br i1 %16, label %22, label %18

18:                                               ; preds = %15
  %19 = icmp eq i64 %17, 0
  br i1 %19, label %21, label %20

20:                                               ; preds = %18
  tail call void @fa_write(i32 noundef 1, ptr noundef nonnull @output, i64 noundef %17) #45
  store i64 0, ptr @output_length, align 8, !tbaa !8
  br label %21

21:                                               ; preds = %18, %20
  tail call void @fa_write(i32 noundef %0, ptr noundef %1, i64 noundef %2) #45
  br label %35

22:                                               ; preds = %15
  %23 = add i64 %17, %2
  %24 = icmp ugt i64 %23, 8192
  br i1 %24, label %25, label %31

25:                                               ; preds = %22
  %26 = icmp eq i64 %17, 0
  br i1 %26, label %28, label %27

27:                                               ; preds = %25
  tail call void @fa_write(i32 noundef 1, ptr noundef nonnull @output, i64 noundef %17) #45
  store i64 0, ptr @output_length, align 8, !tbaa !8
  br label %28

28:                                               ; preds = %25, %27
  %29 = icmp ugt i64 %2, 8192
  br i1 %29, label %30, label %31

30:                                               ; preds = %28
  tail call void @fa_write(i32 noundef 1, ptr noundef %1, i64 noundef %2) #45
  br label %35

31:                                               ; preds = %28, %22
  %32 = phi i64 [ %17, %22 ], [ 0, %28 ]
  %33 = getelementptr inbounds i8, ptr @output, i64 %32
  tail call fastcc void @copy_bytes(ptr noundef nonnull %33, ptr noundef %1, i64 noundef %2) #44
  %34 = add i64 %32, %2
  store i64 %34, ptr @output_length, align 8, !tbaa !8
  br label %35

35:                                               ; preds = %31, %30, %21
  store atomic i32 0, ptr @output_lock release, align 4
  ret void
}

; Function Attrs: optsize
declare void @fa_write(i32 noundef, ptr noundef, i64 noundef) local_unnamed_addr #3

; Function Attrs: noreturn nounwind optsize
define void @fa_terminate(i32 noundef %0) local_unnamed_addr #4 {
  tail call void @fa_flush() #44
  tail call void @fa_exit(i32 noundef %0) #46
  unreachable
}

; Function Attrs: noreturn optsize
declare void @fa_exit(i32 noundef) local_unnamed_addr #5

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, inaccessiblemem: none)
define void @fa_b_reset() local_unnamed_addr #6 {
  %1 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %1, align 8, !tbaa !8
  ret void
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare nonnull ptr @llvm.threadlocal.address.p0(ptr nonnull) #7

; Function Attrs: nounwind optsize
define void @fa_b_bytes(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #2 {
  tail call fastcc void @reserve(i64 noundef %1) #44
  %3 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %4 = load ptr, ptr %3, align 8, !tbaa !10
  %5 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %6 = load i64, ptr %5, align 8, !tbaa !8
  %7 = getelementptr inbounds i8, ptr %4, i64 %6
  tail call fastcc void @copy_bytes(ptr noundef %7, ptr noundef %0, i64 noundef %1) #44
  %8 = load i64, ptr %5, align 8, !tbaa !8
  %9 = add i64 %8, %1
  store i64 %9, ptr %5, align 8, !tbaa !8
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @reserve(i64 noundef %0) unnamed_addr #2 {
  %2 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %3 = load i64, ptr %2, align 8, !tbaa !8
  %4 = add i64 %3, %0
  %5 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_capacity)
  %6 = load i64, ptr %5, align 8, !tbaa !8
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
  br i1 %14, label %12, label %16, !llvm.loop !12

16:                                               ; preds = %12
  %17 = tail call ptr @fa_alloc(i64 noundef %13, i64 noundef 1) #45
  %18 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %19 = load ptr, ptr %18, align 8, !tbaa !10
  %20 = load i64, ptr %2, align 8, !tbaa !8
  tail call fastcc void @copy_bytes(ptr noundef %17, ptr noundef %19, i64 noundef %20) #44
  %21 = load ptr, ptr %18, align 8, !tbaa !10
  %22 = icmp eq ptr %21, null
  br i1 %22, label %25, label %23

23:                                               ; preds = %16
  %24 = load i64, ptr %5, align 8, !tbaa !8
  tail call void @fa_free(ptr noundef nonnull %21, i64 noundef %24, i64 noundef 1) #45
  br label %25

25:                                               ; preds = %23, %16
  store ptr %17, ptr %18, align 8, !tbaa !10
  store i64 %13, ptr %5, align 8, !tbaa !8
  br label %26

26:                                               ; preds = %1, %25
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_b_line(i32 noundef %0) local_unnamed_addr #2 {
  tail call fastcc void @reserve(i64 noundef 1) #44
  %2 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %3 = load ptr, ptr %2, align 8, !tbaa !10
  %4 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %5 = load i64, ptr %4, align 8, !tbaa !8
  %6 = getelementptr inbounds i8, ptr %3, i64 %5
  store i8 10, ptr %6, align 1, !tbaa !2
  %7 = load i64, ptr %4, align 8, !tbaa !8
  %8 = add i64 %7, 1
  store i64 %8, ptr %4, align 8, !tbaa !8
  %9 = load ptr, ptr %2, align 8, !tbaa !10
  tail call void @fa_out(i32 noundef %0, ptr noundef %9, i64 noundef %8) #44
  store i64 0, ptr %4, align 8, !tbaa !8
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_string_own(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2, ptr nocapture noundef writeonly %3) local_unnamed_addr #2 {
  %5 = alloca %struct.FaString, align 8
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %5) #40
  %6 = and i64 %2, -9223372036854775808
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 8 %5, ptr noundef %0, i64 noundef %1, i64 noundef %6) #44
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %3, ptr noundef nonnull align 8 dereferenceable(24) %5, i64 24, i1 false), !tbaa.struct !13
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %5) #40
  ret void
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.start.p0(i64 immarg, ptr nocapture) #8

; Function Attrs: nounwind optsize
define internal fastcc void @owned_copy_with(ptr dead_on_unwind noalias nocapture writable writeonly sret(%struct.FaString) align 8 %0, ptr nocapture noundef readonly %1, i64 noundef %2, i64 noundef %3) unnamed_addr #2 {
  %5 = icmp ugt i64 %2, 4611686018427387902
  br i1 %5, label %6, label %7

6:                                                ; preds = %4
  tail call fastcc void @out_of_memory() #47
  unreachable

7:                                                ; preds = %4
  switch i64 %2, label %21 [
    i64 0, label %8
    i64 1, label %11
  ]

8:                                                ; preds = %7
  store ptr @.str.35, ptr %0, align 8, !tbaa.struct !13
  %9 = getelementptr inbounds i8, ptr %0, i64 8
  store i64 0, ptr %9, align 8, !tbaa.struct !14
  %10 = getelementptr inbounds i8, ptr %0, i64 16
  store i64 -9223372036854775808, ptr %10, align 8, !tbaa.struct !15
  br label %44

11:                                               ; preds = %7
  %12 = load i8, ptr %1, align 1, !tbaa !2
  %13 = icmp sgt i8 %12, -1
  br i1 %13, label %14, label %23

14:                                               ; preds = %11
  %15 = zext nneg i8 %12 to i64
  %16 = getelementptr inbounds i8, ptr @ascii_table, i64 %15
  store ptr %16, ptr %0, align 8, !tbaa !16, !alias.scope !18
  %17 = getelementptr inbounds %struct.FaString, ptr %0, i64 0, i32 1
  store i64 1, ptr %17, align 8, !tbaa !21, !alias.scope !18
  %18 = icmp eq i8 %12, 13
  %19 = select i1 %18, i64 0, i64 -9223372036854775808
  %20 = getelementptr inbounds %struct.FaString, ptr %0, i64 0, i32 2
  store i64 %19, ptr %20, align 8, !tbaa !22, !alias.scope !18
  br label %44

21:                                               ; preds = %7
  %22 = icmp ult i64 %2, 24
  br i1 %22, label %23, label %34

23:                                               ; preds = %11, %21
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %0, i8 0, i64 24, i1 false)
  tail call fastcc void @copy_bytes(ptr noundef %0, ptr noundef %1, i64 noundef %2) #44
  %24 = icmp eq i64 %3, 0
  br i1 %24, label %25, label %29

25:                                               ; preds = %23
  %26 = tail call fastcc i64 @simple_flag(ptr noundef %1, i64 noundef %2) #44, !range !23
  %27 = icmp eq i64 %26, 0
  %28 = select i1 %27, i64 64, i64 192
  br label %29

29:                                               ; preds = %23, %25
  %30 = phi i64 [ %28, %25 ], [ 192, %23 ]
  %31 = or i64 %30, %2
  %32 = trunc i64 %31 to i8
  %33 = getelementptr inbounds i8, ptr %0, i64 23
  store i8 %32, ptr %33, align 1, !tbaa !2
  br label %44

34:                                               ; preds = %21
  %35 = tail call ptr @fa_alloc(i64 noundef %2, i64 noundef 1) #45
  tail call fastcc void @copy_bytes(ptr noundef %35, ptr noundef %1, i64 noundef %2) #44
  %36 = icmp eq i64 %3, 0
  br i1 %36, label %37, label %39

37:                                               ; preds = %34
  %38 = tail call fastcc i64 @simple_flag(ptr noundef %35, i64 noundef %2) #44, !range !23
  br label %39

39:                                               ; preds = %34, %37
  %40 = phi i64 [ %38, %37 ], [ %3, %34 ]
  %41 = or i64 %40, %2
  store ptr %35, ptr %0, align 8, !tbaa.struct !13
  %42 = getelementptr inbounds i8, ptr %0, i64 8
  store i64 %2, ptr %42, align 8, !tbaa.struct !14
  %43 = getelementptr inbounds i8, ptr %0, i64 16
  store i64 %41, ptr %43, align 8, !tbaa.struct !15
  br label %44

44:                                               ; preds = %39, %29, %14, %8
  ret void
}

; Function Attrs: mustprogress nocallback nofree nounwind willreturn memory(argmem: readwrite)
declare void @llvm.memcpy.p0.p0.i64(ptr noalias nocapture writeonly, ptr noalias nocapture readonly, i64, i1 immarg) #9

; Function Attrs: mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.end.p0(i64 immarg, ptr nocapture) #8

; Function Attrs: nounwind optsize
define void @fa_b_take(ptr nocapture noundef writeonly %0) local_unnamed_addr #2 {
  %2 = alloca %struct.FaString, align 8
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %2) #40
  %3 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %4 = load ptr, ptr %3, align 8, !tbaa !10
  %5 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %6 = load i64, ptr %5, align 8, !tbaa !8
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 8 %2, ptr noundef %4, i64 noundef %6, i64 noundef 0) #44
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %0, ptr noundef nonnull align 8 dereferenceable(24) %2, i64 24, i1 false), !tbaa.struct !13
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %2) #40
  store i64 0, ptr %5, align 8, !tbaa !8
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define void @fa_string_i64(i64 noundef %0, ptr nocapture noundef writeonly %1) local_unnamed_addr #10 {
  %3 = tail call i64 @llvm.abs.i64(i64 %0, i1 false)
  %4 = lshr i64 %0, 63
  %5 = trunc i64 %4 to i32
  tail call fastcc void @string_u64(i64 noundef %3, i32 noundef %5, ptr noundef %1) #44
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define internal fastcc void @string_u64(i64 noundef %0, i32 noundef %1, ptr nocapture noundef writeonly %2) unnamed_addr #10 {
  %4 = alloca [24 x i8], align 16
  %5 = alloca %struct.FaString, align 8
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %4) #40
  %6 = icmp ugt i64 %0, 99
  br i1 %6, label %7, label %18

7:                                                ; preds = %3, %7
  %8 = phi i64 [ %11, %7 ], [ %0, %3 ]
  %9 = phi i64 [ %12, %7 ], [ 24, %3 ]
  %10 = urem i64 %8, 100
  %11 = udiv i64 %8, 100
  %12 = add i64 %9, -2
  %13 = getelementptr inbounds i8, ptr %4, i64 %12
  %14 = shl nuw nsw i64 %10, 1
  %15 = getelementptr inbounds i8, ptr @decimal_u64.pairs, i64 %14
  %16 = load i16, ptr %15, align 2
  store i16 %16, ptr %13, align 2
  %17 = icmp ugt i64 %8, 9999
  br i1 %17, label %7, label %18, !llvm.loop !24

18:                                               ; preds = %7, %3
  %19 = phi i64 [ 24, %3 ], [ %12, %7 ]
  %20 = phi i64 [ %0, %3 ], [ %11, %7 ]
  %21 = icmp ugt i64 %20, 9
  br i1 %21, label %22, label %28

22:                                               ; preds = %18
  %23 = add i64 %19, -2
  %24 = getelementptr inbounds i8, ptr %4, i64 %23
  %25 = shl nuw nsw i64 %20, 1
  %26 = getelementptr inbounds i8, ptr @decimal_u64.pairs, i64 %25
  %27 = load i16, ptr %26, align 2
  store i16 %27, ptr %24, align 1
  br label %33

28:                                               ; preds = %18
  %29 = trunc i64 %20 to i8
  %30 = or disjoint i8 %29, 48
  %31 = add i64 %19, -1
  %32 = getelementptr inbounds i8, ptr %4, i64 %31
  store i8 %30, ptr %32, align 1, !tbaa !2
  br label %33

33:                                               ; preds = %28, %22
  %34 = phi i64 [ %23, %22 ], [ %31, %28 ]
  %35 = icmp eq i32 %1, 0
  br i1 %35, label %39, label %36

36:                                               ; preds = %33
  %37 = add i64 %34, -1
  %38 = getelementptr inbounds i8, ptr %4, i64 %37
  store i8 45, ptr %38, align 1, !tbaa !2
  br label %39

39:                                               ; preds = %33, %36
  %40 = phi i64 [ %37, %36 ], [ %34, %33 ]
  %41 = sub i64 24, %40
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %5) #40
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %5, i8 0, i64 24, i1 false)
  %42 = getelementptr inbounds i8, ptr %4, i64 %40
  call fastcc void @copy_bytes(ptr noundef nonnull %5, ptr noundef nonnull %42, i64 noundef %41) #44
  %43 = trunc i64 %41 to i8
  %44 = or i8 %43, -64
  %45 = getelementptr inbounds i8, ptr %5, i64 23
  store i8 %44, ptr %45, align 1, !tbaa !2
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %2, ptr noundef nonnull align 8 dereferenceable(24) %5, i64 24, i1 false), !tbaa.struct !13
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %5) #40
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %4) #40
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define void @fa_string_u64(i64 noundef %0, ptr nocapture noundef writeonly %1) local_unnamed_addr #10 {
  tail call fastcc void @string_u64(i64 noundef %0, i32 noundef 0, ptr noundef %1) #44
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_b_i64(i64 noundef %0) local_unnamed_addr #2 {
  %2 = tail call i64 @llvm.abs.i64(i64 %0, i1 false)
  %3 = lshr i64 %0, 63
  %4 = trunc i64 %3 to i32
  tail call fastcc void @append_u64(i64 noundef %2, i32 noundef %4) #44
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @append_u64(i64 noundef %0, i32 noundef %1) unnamed_addr #2 {
  %3 = alloca [24 x i8], align 16
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %3) #40
  %4 = icmp ugt i64 %0, 99
  br i1 %4, label %5, label %16

5:                                                ; preds = %2, %5
  %6 = phi i64 [ %9, %5 ], [ %0, %2 ]
  %7 = phi i64 [ %10, %5 ], [ 24, %2 ]
  %8 = urem i64 %6, 100
  %9 = udiv i64 %6, 100
  %10 = add i64 %7, -2
  %11 = getelementptr inbounds i8, ptr %3, i64 %10
  %12 = shl nuw nsw i64 %8, 1
  %13 = getelementptr inbounds i8, ptr @decimal_u64.pairs, i64 %12
  %14 = load i16, ptr %13, align 2
  store i16 %14, ptr %11, align 2
  %15 = icmp ugt i64 %6, 9999
  br i1 %15, label %5, label %16, !llvm.loop !24

16:                                               ; preds = %5, %2
  %17 = phi i64 [ 24, %2 ], [ %10, %5 ]
  %18 = phi i64 [ %0, %2 ], [ %9, %5 ]
  %19 = icmp ugt i64 %18, 9
  br i1 %19, label %20, label %26

20:                                               ; preds = %16
  %21 = add i64 %17, -2
  %22 = getelementptr inbounds i8, ptr %3, i64 %21
  %23 = shl nuw nsw i64 %18, 1
  %24 = getelementptr inbounds i8, ptr @decimal_u64.pairs, i64 %23
  %25 = load i16, ptr %24, align 2
  store i16 %25, ptr %22, align 1
  br label %31

26:                                               ; preds = %16
  %27 = trunc i64 %18 to i8
  %28 = or disjoint i8 %27, 48
  %29 = add i64 %17, -1
  %30 = getelementptr inbounds i8, ptr %3, i64 %29
  store i8 %28, ptr %30, align 1, !tbaa !2
  br label %31

31:                                               ; preds = %26, %20
  %32 = phi i64 [ %21, %20 ], [ %29, %26 ]
  %33 = icmp eq i32 %1, 0
  br i1 %33, label %37, label %34

34:                                               ; preds = %31
  %35 = add i64 %32, -1
  %36 = getelementptr inbounds i8, ptr %3, i64 %35
  store i8 45, ptr %36, align 1, !tbaa !2
  br label %37

37:                                               ; preds = %31, %34
  %38 = phi i64 [ %35, %34 ], [ %32, %31 ]
  %39 = getelementptr inbounds i8, ptr %3, i64 %38
  %40 = sub i64 24, %38
  tail call fastcc void @reserve(i64 noundef %40) #44
  %41 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %42 = load ptr, ptr %41, align 8, !tbaa !10
  %43 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %44 = load i64, ptr %43, align 8, !tbaa !8
  %45 = getelementptr inbounds i8, ptr %42, i64 %44
  call fastcc void @copy_bytes(ptr noundef %45, ptr noundef nonnull %39, i64 noundef %40) #44
  %46 = load i64, ptr %43, align 8, !tbaa !8
  %47 = add i64 %46, %40
  store i64 %47, ptr %43, align 8, !tbaa !8
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %3) #40
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_b_u64(i64 noundef %0) local_unnamed_addr #2 {
  tail call fastcc void @append_u64(i64 noundef %0, i32 noundef 0) #44
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_b_i128(i64 noundef %0, i64 noundef %1) local_unnamed_addr #2 {
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
  tail call fastcc void @append_u128(i64 noundef %12, i64 noundef %14, i32 noundef %9) #44
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @append_u128(i64 noundef %0, i64 noundef %1, i32 noundef %2) unnamed_addr #2 {
  %4 = alloca [4 x i32], align 16
  %5 = alloca [48 x i8], align 16
  %6 = icmp eq i64 %1, 0
  br i1 %6, label %7, label %8

7:                                                ; preds = %3
  tail call fastcc void @append_u64(i64 noundef %0, i32 noundef %2) #44
  br label %73

8:                                                ; preds = %3
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %4) #40
  %9 = lshr i64 %1, 32
  %10 = trunc i64 %9 to i32
  store i32 %10, ptr %4, align 16, !tbaa !25
  %11 = getelementptr inbounds i32, ptr %4, i64 1
  %12 = trunc i64 %1 to i32
  store i32 %12, ptr %11, align 4, !tbaa !25
  %13 = getelementptr inbounds i32, ptr %4, i64 2
  %14 = lshr i64 %0, 32
  %15 = trunc i64 %14 to i32
  store i32 %15, ptr %13, align 8, !tbaa !25
  %16 = getelementptr inbounds i32, ptr %4, i64 3
  %17 = trunc i64 %0 to i32
  store i32 %17, ptr %16, align 4, !tbaa !25
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %5) #40
  br label %18

18:                                               ; preds = %40, %8
  %19 = phi i64 [ 48, %8 ], [ %45, %40 ]
  br label %20

20:                                               ; preds = %20, %18
  %21 = phi i64 [ 0, %18 ], [ %31, %20 ]
  %22 = phi i64 [ 0, %18 ], [ %30, %20 ]
  %23 = shl nuw nsw i64 %22, 32
  %24 = getelementptr inbounds i32, ptr %4, i64 %21
  %25 = load i32, ptr %24, align 4, !tbaa !25
  %26 = zext i32 %25 to i64
  %27 = or disjoint i64 %23, %26
  %28 = udiv i64 %27, 1000000000
  %29 = trunc i64 %28 to i32
  store i32 %29, ptr %24, align 4, !tbaa !25
  %30 = urem i64 %27, 1000000000
  %31 = add nuw nsw i64 %21, 1
  %32 = icmp eq i64 %31, 4
  br i1 %32, label %33, label %20, !llvm.loop !27

33:                                               ; preds = %20
  %34 = trunc i64 %30 to i32
  %35 = load <4 x i32>, ptr %4, align 16, !tbaa !25
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
  store i8 %48, ptr %49, align 1, !tbaa !2
  %50 = udiv i32 %43, 10
  %51 = add nuw nsw i32 %42, 1
  %52 = icmp ult i32 %42, 8
  %53 = icmp ugt i32 %43, 9
  %54 = select i1 %37, i1 true, i1 %53
  %55 = select i1 %52, i1 %54, i1 false
  br i1 %55, label %41, label %40, !llvm.loop !28

56:                                               ; preds = %33, %40
  %57 = phi i64 [ %45, %40 ], [ %19, %33 ]
  %58 = icmp eq i32 %2, 0
  br i1 %58, label %62, label %59

59:                                               ; preds = %56
  %60 = add i64 %57, -1
  %61 = getelementptr inbounds [48 x i8], ptr %5, i64 0, i64 %60
  store i8 45, ptr %61, align 1, !tbaa !2
  br label %62

62:                                               ; preds = %59, %56
  %63 = phi i64 [ %60, %59 ], [ %57, %56 ]
  %64 = getelementptr inbounds i8, ptr %5, i64 %63
  %65 = sub i64 48, %63
  tail call fastcc void @reserve(i64 noundef %65) #44
  %66 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %67 = load ptr, ptr %66, align 8, !tbaa !10
  %68 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %69 = load i64, ptr %68, align 8, !tbaa !8
  %70 = getelementptr inbounds i8, ptr %67, i64 %69
  call fastcc void @copy_bytes(ptr noundef %70, ptr noundef nonnull %64, i64 noundef %65) #44
  %71 = load i64, ptr %68, align 8, !tbaa !8
  %72 = add i64 %71, %65
  store i64 %72, ptr %68, align 8, !tbaa !8
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %5) #40
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %4) #40
  br label %73

73:                                               ; preds = %62, %7
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_b_u128(i64 noundef %0, i64 noundef %1) local_unnamed_addr #2 {
  tail call fastcc void @append_u128(i64 noundef %0, i64 noundef %1, i32 noundef 0) #44
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_b_bool(i32 noundef %0) local_unnamed_addr #2 {
  %2 = icmp eq i32 %0, 0
  %3 = select i1 %2, ptr @.str.1, ptr @.str
  tail call fastcc void @append_c(ptr noundef nonnull %3) #44
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @append_c(ptr nocapture noundef readonly %0) unnamed_addr #2 {
  br label %2

2:                                                ; preds = %2, %1
  %3 = phi i64 [ 0, %1 ], [ %7, %2 ]
  %4 = getelementptr inbounds i8, ptr %0, i64 %3
  %5 = load i8, ptr %4, align 1, !tbaa !2
  %6 = icmp eq i8 %5, 0
  %7 = add i64 %3, 1
  br i1 %6, label %8, label %2, !llvm.loop !29

8:                                                ; preds = %2
  tail call fastcc void @reserve(i64 noundef %3) #44
  %9 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %10 = load ptr, ptr %9, align 8, !tbaa !10
  %11 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %12 = load i64, ptr %11, align 8, !tbaa !8
  %13 = getelementptr inbounds i8, ptr %10, i64 %12
  tail call fastcc void @copy_bytes(ptr noundef %13, ptr noundef nonnull %0, i64 noundef %3) #44
  %14 = load i64, ptr %11, align 8, !tbaa !8
  %15 = add i64 %14, %3
  store i64 %15, ptr %11, align 8, !tbaa !8
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_b_f64(double noundef %0) local_unnamed_addr #2 {
  tail call fastcc void @append_double(double noundef %0) #44
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @append_double(double noundef %0) unnamed_addr #2 {
  %2 = alloca [40 x i8], align 16
  %3 = alloca [24 x i8], align 16
  %4 = fcmp ord double %0, 0.000000e+00
  br i1 %4, label %6, label %5

5:                                                ; preds = %1
  tail call fastcc void @append_c(ptr noundef nonnull @.str.36) #44
  br label %187

6:                                                ; preds = %1
  %7 = fcmp oeq double %0, 0.000000e+00
  br i1 %7, label %8, label %16

8:                                                ; preds = %6
  tail call fastcc void @reserve(i64 noundef 1) #44
  %9 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %10 = load ptr, ptr %9, align 8, !tbaa !10
  %11 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %12 = load i64, ptr %11, align 8, !tbaa !8
  %13 = getelementptr inbounds i8, ptr %10, i64 %12
  store i8 48, ptr %13, align 1, !tbaa !2
  %14 = load i64, ptr %11, align 8, !tbaa !8
  %15 = add i64 %14, 1
  store i64 %15, ptr %11, align 8, !tbaa !8
  br label %187

16:                                               ; preds = %6
  %17 = fcmp olt double %0, 0.000000e+00
  br i1 %17, label %18, label %27

18:                                               ; preds = %16
  tail call fastcc void @reserve(i64 noundef 1) #44
  %19 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %20 = load ptr, ptr %19, align 8, !tbaa !10
  %21 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %22 = load i64, ptr %21, align 8, !tbaa !8
  %23 = getelementptr inbounds i8, ptr %20, i64 %22
  store i8 45, ptr %23, align 1, !tbaa !2
  %24 = load i64, ptr %21, align 8, !tbaa !8
  %25 = add i64 %24, 1
  store i64 %25, ptr %21, align 8, !tbaa !8
  %26 = fneg double %0
  br label %27

27:                                               ; preds = %18, %16
  %28 = phi double [ %26, %18 ], [ %0, %16 ]
  %29 = fcmp ogt double %28, 0x7FEFFFFFFFFFFFFF
  br i1 %29, label %30, label %31

30:                                               ; preds = %27
  tail call fastcc void @append_c(ptr noundef nonnull @.str.37) #44
  br label %187

31:                                               ; preds = %27
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %3) #40
  call void @llvm.lifetime.start.p0(i64 40, ptr nonnull %2) #40
  br label %32

32:                                               ; preds = %32, %31
  %33 = phi i32 [ 1, %31 ], [ %38, %32 ]
  %34 = add nsw i32 %33, -1
  %35 = call i32 (ptr, i64, ptr, ...) @snprintf(ptr noundef nonnull %2, i64 noundef 40, ptr noundef nonnull @.str.39, i32 noundef %34, double noundef %28) #45
  %36 = call double @strtod(ptr noundef nonnull %2, ptr noundef null) #45
  %37 = fcmp oeq double %36, %28
  %38 = add nuw nsw i32 %33, 1
  %39 = icmp eq i32 %38, 18
  %40 = select i1 %37, i1 true, i1 %39
  br i1 %40, label %41, label %32, !llvm.loop !30

41:                                               ; preds = %32
  %42 = load i8, ptr %2, align 16, !tbaa !2
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
  store i8 %45, ptr %52, align 1, !tbaa !2
  %53 = add nsw i32 %47, 1
  br label %54

54:                                               ; preds = %50, %44
  %55 = phi i32 [ %53, %50 ], [ %47, %44 ]
  %56 = getelementptr inbounds i8, ptr %46, i64 1
  %57 = load i8, ptr %56, align 1, !tbaa !2
  %58 = icmp eq i8 %57, 101
  br i1 %58, label %59, label %44, !llvm.loop !31

59:                                               ; preds = %54, %41
  %60 = phi i32 [ 0, %41 ], [ %55, %54 ]
  %61 = phi ptr [ %2, %41 ], [ %56, %54 ]
  %62 = getelementptr inbounds i8, ptr %61, i64 1
  %63 = load i8, ptr %62, align 1, !tbaa !2
  %64 = getelementptr inbounds i8, ptr %61, i64 2
  %65 = load i8, ptr %64, align 1, !tbaa !2
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
  %80 = load i8, ptr %79, align 1, !tbaa !2
  %81 = icmp eq i8 %80, 0
  br i1 %81, label %67, label %71, !llvm.loop !32

82:                                               ; preds = %86, %67
  %83 = phi i64 [ %69, %67 ], [ %87, %86 ]
  %84 = trunc i64 %83 to i32
  %85 = icmp sgt i32 %84, 1
  br i1 %85, label %86, label %91

86:                                               ; preds = %82
  %87 = add nsw i64 %83, -1
  %88 = getelementptr inbounds i8, ptr %3, i64 %87
  %89 = load i8, ptr %88, align 1, !tbaa !2
  %90 = icmp eq i8 %89, 48
  br i1 %90, label %82, label %91, !llvm.loop !33

91:                                               ; preds = %82, %86
  %92 = phi i32 [ %70, %82 ], [ %84, %86 ]
  %93 = icmp eq i8 %63, 45
  %94 = sub nsw i32 0, %68
  %95 = select i1 %93, i32 %94, i32 %68
  %96 = add nsw i32 %95, 1
  call void @llvm.lifetime.end.p0(i64 40, ptr nonnull %2) #40
  %97 = icmp sle i32 %92, %96
  %98 = icmp slt i32 %95, 21
  %99 = and i1 %98, %97
  br i1 %99, label %100, label %119

100:                                              ; preds = %91
  %101 = sext i32 %92 to i64
  call fastcc void @reserve(i64 noundef %101) #44
  %102 = call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %103 = load ptr, ptr %102, align 8, !tbaa !10
  %104 = call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %105 = load i64, ptr %104, align 8, !tbaa !8
  %106 = getelementptr inbounds i8, ptr %103, i64 %105
  call fastcc void @copy_bytes(ptr noundef %106, ptr noundef nonnull %3, i64 noundef %101) #44
  %107 = load i64, ptr %104, align 8, !tbaa !8
  %108 = add i64 %107, %101
  store i64 %108, ptr %104, align 8, !tbaa !8
  %109 = icmp sgt i32 %92, %95
  br i1 %109, label %186, label %110

110:                                              ; preds = %100, %110
  %111 = phi i32 [ %117, %110 ], [ %92, %100 ]
  call fastcc void @reserve(i64 noundef 1) #44
  %112 = load ptr, ptr %102, align 8, !tbaa !10
  %113 = load i64, ptr %104, align 8, !tbaa !8
  %114 = getelementptr inbounds i8, ptr %112, i64 %113
  store i8 48, ptr %114, align 1, !tbaa !2
  %115 = load i64, ptr %104, align 8, !tbaa !8
  %116 = add i64 %115, 1
  store i64 %116, ptr %104, align 8, !tbaa !8
  %117 = add i32 %111, 1
  %118 = icmp eq i32 %111, %95
  br i1 %118, label %186, label %110, !llvm.loop !34

119:                                              ; preds = %91
  %120 = icmp ult i32 %95, 21
  br i1 %120, label %121, label %133

121:                                              ; preds = %119
  %122 = zext nneg i32 %96 to i64
  call void @fa_b_bytes(ptr noundef nonnull %3, i64 noundef %122) #44
  call fastcc void @reserve(i64 noundef 1) #44
  %123 = call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %124 = load ptr, ptr %123, align 8, !tbaa !10
  %125 = call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %126 = load i64, ptr %125, align 8, !tbaa !8
  %127 = getelementptr inbounds i8, ptr %124, i64 %126
  store i8 46, ptr %127, align 1, !tbaa !2
  %128 = load i64, ptr %125, align 8, !tbaa !8
  %129 = add i64 %128, 1
  store i64 %129, ptr %125, align 8, !tbaa !8
  %130 = getelementptr inbounds i8, ptr %3, i64 %122
  %131 = sub nsw i32 %92, %96
  %132 = sext i32 %131 to i64
  call void @fa_b_bytes(ptr noundef nonnull %130, i64 noundef %132) #44
  br label %186

133:                                              ; preds = %119
  %134 = icmp ugt i32 %95, -7
  br i1 %134, label %135, label %152

135:                                              ; preds = %133
  call fastcc void @append_c(ptr noundef nonnull @.str.38) #44
  %136 = icmp eq i32 %95, -1
  br i1 %136, label %140, label %137

137:                                              ; preds = %135
  %138 = call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %139 = call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  br label %142

140:                                              ; preds = %142, %135
  %141 = sext i32 %92 to i64
  call void @fa_b_bytes(ptr noundef nonnull %3, i64 noundef %141) #44
  br label %186

142:                                              ; preds = %137, %142
  %143 = phi i32 [ 0, %137 ], [ %149, %142 ]
  call fastcc void @reserve(i64 noundef 1) #44
  %144 = load ptr, ptr %138, align 8, !tbaa !10
  %145 = load i64, ptr %139, align 8, !tbaa !8
  %146 = getelementptr inbounds i8, ptr %144, i64 %145
  store i8 48, ptr %146, align 1, !tbaa !2
  %147 = load i64, ptr %139, align 8, !tbaa !8
  %148 = add i64 %147, 1
  store i64 %148, ptr %139, align 8, !tbaa !8
  %149 = add nuw nsw i32 %143, 1
  %150 = xor i32 %95, %149
  %151 = icmp eq i32 %150, -1
  br i1 %151, label %140, label %142, !llvm.loop !35

152:                                              ; preds = %133
  %153 = load i8, ptr %3, align 16, !tbaa !2
  call fastcc void @reserve(i64 noundef 1) #44
  %154 = call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %155 = load ptr, ptr %154, align 8, !tbaa !10
  %156 = call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %157 = load i64, ptr %156, align 8, !tbaa !8
  %158 = getelementptr inbounds i8, ptr %155, i64 %157
  store i8 %153, ptr %158, align 1, !tbaa !2
  %159 = load i64, ptr %156, align 8, !tbaa !8
  %160 = add i64 %159, 1
  store i64 %160, ptr %156, align 8, !tbaa !8
  %161 = icmp sgt i32 %92, 1
  br i1 %161, label %162, label %171

162:                                              ; preds = %152
  call fastcc void @reserve(i64 noundef 1) #44
  %163 = load ptr, ptr %154, align 8, !tbaa !10
  %164 = load i64, ptr %156, align 8, !tbaa !8
  %165 = getelementptr inbounds i8, ptr %163, i64 %164
  store i8 46, ptr %165, align 1, !tbaa !2
  %166 = load i64, ptr %156, align 8, !tbaa !8
  %167 = add i64 %166, 1
  store i64 %167, ptr %156, align 8, !tbaa !8
  %168 = getelementptr inbounds i8, ptr %3, i64 1
  %169 = add nsw i32 %92, -1
  %170 = zext nneg i32 %169 to i64
  call void @fa_b_bytes(ptr noundef nonnull %168, i64 noundef %170) #44
  br label %171

171:                                              ; preds = %162, %152
  call fastcc void @reserve(i64 noundef 1) #44
  %172 = load ptr, ptr %154, align 8, !tbaa !10
  %173 = load i64, ptr %156, align 8, !tbaa !8
  %174 = getelementptr inbounds i8, ptr %172, i64 %173
  store i8 101, ptr %174, align 1, !tbaa !2
  %175 = load i64, ptr %156, align 8, !tbaa !8
  %176 = add i64 %175, 1
  store i64 %176, ptr %156, align 8, !tbaa !8
  %177 = icmp slt i32 %95, 0
  %178 = select i1 %177, i8 45, i8 43
  call fastcc void @reserve(i64 noundef 1) #44
  %179 = load ptr, ptr %154, align 8, !tbaa !10
  %180 = load i64, ptr %156, align 8, !tbaa !8
  %181 = getelementptr inbounds i8, ptr %179, i64 %180
  store i8 %178, ptr %181, align 1, !tbaa !2
  %182 = load i64, ptr %156, align 8, !tbaa !8
  %183 = add i64 %182, 1
  store i64 %183, ptr %156, align 8, !tbaa !8
  %184 = call i32 @llvm.abs.i32(i32 %68, i1 true)
  %185 = zext nneg i32 %184 to i64
  call fastcc void @append_u64(i64 noundef %185, i32 noundef 0) #44
  br label %186

186:                                              ; preds = %110, %100, %121, %171, %140
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %3) #40
  br label %187

187:                                              ; preds = %186, %30, %8, %5
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_b_f32(float noundef %0) local_unnamed_addr #2 {
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
  tail call fastcc void @append_double(double noundef %9) #44
  br label %107

11:                                               ; preds = %1
  call void @llvm.lifetime.start.p0(i64 40, ptr nonnull %3) #40
  %12 = getelementptr inbounds i64, ptr %4, i64 1
  %13 = getelementptr inbounds i64, ptr %4, i64 2
  %14 = insertelement <2 x double> poison, double %9, i64 0
  %15 = shufflevector <2 x double> %14, <2 x double> poison, <2 x i32> zeroinitializer
  br label %16

16:                                               ; preds = %102, %11
  %17 = phi i32 [ 1, %11 ], [ %103, %102 ]
  %18 = add nsw i32 %17, -1
  %19 = call i32 (ptr, i64, ptr, ...) @snprintf(ptr noundef nonnull %3, i64 noundef 40, ptr noundef nonnull @.str.39, i32 noundef %18, double noundef %9) #45
  %20 = load i8, ptr %3, align 16, !tbaa !2
  %21 = icmp eq i8 %20, 45
  %22 = zext i1 %21 to i64
  %23 = getelementptr inbounds i8, ptr %3, i64 %22
  %24 = load i8, ptr %23, align 1, !tbaa !2
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
  %37 = load i8, ptr %36, align 1, !tbaa !2
  %38 = icmp eq i8 %37, 101
  br i1 %38, label %39, label %26, !llvm.loop !36

39:                                               ; preds = %26, %16
  %40 = phi ptr [ %23, %16 ], [ %36, %26 ]
  %41 = phi i64 [ 0, %16 ], [ %35, %26 ]
  %42 = getelementptr inbounds i8, ptr %40, i64 1
  %43 = load i8, ptr %42, align 1, !tbaa !2
  %44 = icmp eq i8 %43, 45
  %45 = getelementptr inbounds i8, ptr %40, i64 2
  %46 = load i8, ptr %45, align 1, !tbaa !2
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
  %57 = load i8, ptr %56, align 1, !tbaa !2
  %58 = icmp eq i8 %57, 0
  br i1 %58, label %59, label %48, !llvm.loop !37

59:                                               ; preds = %48, %39
  %60 = phi i32 [ 0, %39 ], [ %55, %48 ]
  %61 = sub nsw i32 0, %60
  %62 = select i1 %44, i32 %61, i32 %60
  %63 = sub nsw i32 %62, %18
  %64 = sub nsw i64 0, %41
  %65 = select i1 %21, i64 %64, i64 %41
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %4) #40
  store i64 %65, ptr %4, align 16, !tbaa !8
  %66 = add nsw i64 %65, -1
  store i64 %66, ptr %12, align 8, !tbaa !8
  %67 = add nsw i64 %65, 1
  store i64 %67, ptr %13, align 16, !tbaa !8
  br label %71

68:                                               ; preds = %94
  %69 = add nuw nsw i64 %72, 1
  %70 = icmp eq i64 %69, 3
  br i1 %70, label %100, label %71, !llvm.loop !38

71:                                               ; preds = %68, %59
  %72 = phi i64 [ 0, %59 ], [ %69, %68 ]
  %73 = phi double [ 0.000000e+00, %59 ], [ %96, %68 ]
  %74 = phi i32 [ 0, %59 ], [ %95, %68 ]
  %75 = getelementptr inbounds [3 x i64], ptr %4, i64 0, i64 %72
  %76 = load i64, ptr %75, align 8, !tbaa !8
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %2) #40
  %77 = call i32 (ptr, i64, ptr, ...) @snprintf(ptr noundef nonnull %2, i64 noundef 48, ptr noundef nonnull @.str.40, i64 noundef %76, i32 noundef %63) #45
  %78 = call double @strtod(ptr noundef nonnull %2, ptr noundef null) #45
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %2) #40
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
  call fastcc void @append_double(double noundef %96) #44
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %4) #40
  br label %106

102:                                              ; preds = %100
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %4) #40
  %103 = add nuw nsw i32 %17, 1
  %104 = icmp eq i32 %103, 10
  br i1 %104, label %105, label %16, !llvm.loop !39

105:                                              ; preds = %102
  call fastcc void @append_double(double noundef %9) #44
  br label %106

106:                                              ; preds = %105, %101
  call void @llvm.lifetime.end.p0(i64 40, ptr nonnull %3) #40
  br label %107

107:                                              ; preds = %10, %106
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_b_percentage(double noundef %0) local_unnamed_addr #2 {
  %2 = alloca [40 x i8], align 16
  %3 = fmul double %0, 1.000000e+02
  %4 = fcmp ueq double %3, 0.000000e+00
  %5 = fsub double %3, %3
  %6 = fcmp une double %5, 0.000000e+00
  %7 = or i1 %4, %6
  br i1 %7, label %8, label %9

8:                                                ; preds = %1
  tail call fastcc void @append_double(double noundef %3) #44
  br label %12

9:                                                ; preds = %1
  call void @llvm.lifetime.start.p0(i64 40, ptr nonnull %2) #40
  %10 = call i32 (ptr, i64, ptr, ...) @snprintf(ptr noundef nonnull %2, i64 noundef 40, ptr noundef nonnull @.str.41, double noundef %3) #45
  %11 = call double @strtod(ptr noundef nonnull %2, ptr noundef null) #45
  call fastcc void @append_double(double noundef %11) #44
  call void @llvm.lifetime.end.p0(i64 40, ptr nonnull %2) #40
  br label %12

12:                                               ; preds = %8, %9
  call fastcc void @reserve(i64 noundef 1) #44
  %13 = call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %14 = load ptr, ptr %13, align 8, !tbaa !10
  %15 = call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %16 = load i64, ptr %15, align 8, !tbaa !8
  %17 = getelementptr inbounds i8, ptr %14, i64 %16
  store i8 37, ptr %17, align 1, !tbaa !2
  %18 = load i64, ptr %15, align 8, !tbaa !8
  %19 = add i64 %18, 1
  store i64 %19, ptr %15, align 8, !tbaa !8
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_b_quoted(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #2 {
  tail call fastcc void @reserve(i64 noundef 1) #44
  %3 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %4 = load ptr, ptr %3, align 8, !tbaa !10
  %5 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %6 = load i64, ptr %5, align 8, !tbaa !8
  %7 = getelementptr inbounds i8, ptr %4, i64 %6
  store i8 34, ptr %7, align 1, !tbaa !2
  %8 = load i64, ptr %5, align 8, !tbaa !8
  %9 = add i64 %8, 1
  store i64 %9, ptr %5, align 8, !tbaa !8
  %10 = icmp eq i64 %1, 0
  br i1 %10, label %11, label %17

11:                                               ; preds = %59, %2
  tail call fastcc void @reserve(i64 noundef 1) #44
  %12 = load ptr, ptr %3, align 8, !tbaa !10
  %13 = load i64, ptr %5, align 8, !tbaa !8
  %14 = getelementptr inbounds i8, ptr %12, i64 %13
  store i8 34, ptr %14, align 1, !tbaa !2
  %15 = load i64, ptr %5, align 8, !tbaa !8
  %16 = add i64 %15, 1
  store i64 %16, ptr %5, align 8, !tbaa !8
  ret void

17:                                               ; preds = %2, %59
  %18 = phi i64 [ %60, %59 ], [ 0, %2 ]
  %19 = getelementptr inbounds i8, ptr %0, i64 %18
  %20 = load i8, ptr %19, align 1, !tbaa !2
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
  tail call fastcc void @append_c(ptr noundef nonnull @.str.2) #44
  br label %59

23:                                               ; preds = %17
  tail call fastcc void @append_c(ptr noundef nonnull @.str.3) #44
  br label %59

24:                                               ; preds = %17
  tail call fastcc void @append_c(ptr noundef nonnull @.str.4) #44
  br label %59

25:                                               ; preds = %17
  tail call fastcc void @append_c(ptr noundef nonnull @.str.5) #44
  br label %59

26:                                               ; preds = %17
  tail call fastcc void @append_c(ptr noundef nonnull @.str.6) #44
  br label %59

27:                                               ; preds = %17
  tail call fastcc void @append_c(ptr noundef nonnull @.str.7) #44
  br label %59

28:                                               ; preds = %17
  tail call fastcc void @append_c(ptr noundef nonnull @.str.8) #44
  br label %59

29:                                               ; preds = %17
  tail call fastcc void @append_c(ptr noundef nonnull @.str.9) #44
  br label %59

30:                                               ; preds = %17
  %31 = icmp ult i8 %20, 32
  %32 = icmp eq i8 %20, 127
  %33 = or i1 %31, %32
  br i1 %33, label %34, label %53

34:                                               ; preds = %30
  tail call fastcc void @append_c(ptr noundef nonnull @.str.10) #44
  %35 = lshr i32 %21, 4
  %36 = zext nneg i32 %35 to i64
  %37 = getelementptr inbounds [17 x i8], ptr @fa_b_quoted.hex, i64 0, i64 %36
  %38 = load i8, ptr %37, align 1, !tbaa !2
  tail call fastcc void @reserve(i64 noundef 1) #44
  %39 = load ptr, ptr %3, align 8, !tbaa !10
  %40 = load i64, ptr %5, align 8, !tbaa !8
  %41 = getelementptr inbounds i8, ptr %39, i64 %40
  store i8 %38, ptr %41, align 1, !tbaa !2
  %42 = load i64, ptr %5, align 8, !tbaa !8
  %43 = add i64 %42, 1
  store i64 %43, ptr %5, align 8, !tbaa !8
  %44 = and i32 %21, 15
  %45 = zext nneg i32 %44 to i64
  %46 = getelementptr inbounds [17 x i8], ptr @fa_b_quoted.hex, i64 0, i64 %45
  %47 = load i8, ptr %46, align 1, !tbaa !2
  tail call fastcc void @reserve(i64 noundef 1) #44
  %48 = load ptr, ptr %3, align 8, !tbaa !10
  %49 = load i64, ptr %5, align 8, !tbaa !8
  %50 = getelementptr inbounds i8, ptr %48, i64 %49
  store i8 %47, ptr %50, align 1, !tbaa !2
  %51 = load i64, ptr %5, align 8, !tbaa !8
  %52 = add i64 %51, 1
  store i64 %52, ptr %5, align 8, !tbaa !8
  br label %59

53:                                               ; preds = %30
  tail call fastcc void @reserve(i64 noundef 1) #44
  %54 = load ptr, ptr %3, align 8, !tbaa !10
  %55 = load i64, ptr %5, align 8, !tbaa !8
  %56 = getelementptr inbounds i8, ptr %54, i64 %55
  store i8 %20, ptr %56, align 1, !tbaa !2
  %57 = load i64, ptr %5, align 8, !tbaa !8
  %58 = add i64 %57, 1
  store i64 %58, ptr %5, align 8, !tbaa !8
  br label %59

59:                                               ; preds = %34, %53, %29, %28, %27, %26, %25, %24, %23, %22
  %60 = add nuw i64 %18, 1
  %61 = icmp eq i64 %60, %1
  br i1 %61, label %11, label %17, !llvm.loop !40
}

; Function Attrs: nounwind optsize
define void @fa_b_string_array(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #2 {
  tail call fastcc void @reserve(i64 noundef 1) #44
  %3 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %4 = load ptr, ptr %3, align 8, !tbaa !10
  %5 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %6 = load i64, ptr %5, align 8, !tbaa !8
  %7 = getelementptr inbounds i8, ptr %4, i64 %6
  store i8 91, ptr %7, align 1, !tbaa !2
  %8 = load i64, ptr %5, align 8, !tbaa !8
  %9 = add i64 %8, 1
  store i64 %9, ptr %5, align 8, !tbaa !8
  %10 = icmp eq i64 %1, 0
  br i1 %10, label %11, label %17

11:                                               ; preds = %34, %2
  tail call fastcc void @reserve(i64 noundef 1) #44
  %12 = load ptr, ptr %3, align 8, !tbaa !10
  %13 = load i64, ptr %5, align 8, !tbaa !8
  %14 = getelementptr inbounds i8, ptr %12, i64 %13
  store i8 93, ptr %14, align 1, !tbaa !2
  %15 = load i64, ptr %5, align 8, !tbaa !8
  %16 = add i64 %15, 1
  store i64 %16, ptr %5, align 8, !tbaa !8
  ret void

17:                                               ; preds = %2, %34
  %18 = phi i64 [ %37, %34 ], [ 0, %2 ]
  %19 = icmp eq i64 %18, 0
  br i1 %19, label %21, label %20

20:                                               ; preds = %17
  tail call fastcc void @append_c(ptr noundef nonnull @.str.11) #44
  br label %21

21:                                               ; preds = %20, %17
  %22 = getelementptr inbounds %struct.FaString, ptr %0, i64 %18
  %23 = getelementptr inbounds %struct.FaString, ptr %0, i64 %18, i32 2
  %24 = load i64, ptr %23, align 8, !tbaa !22
  %25 = and i64 %24, 4611686018427387904
  %26 = icmp eq i64 %25, 0
  br i1 %26, label %30, label %27

27:                                               ; preds = %21
  %28 = lshr i64 %24, 56
  %29 = and i64 %28, 31
  br label %34

30:                                               ; preds = %21
  %31 = load ptr, ptr %22, align 8, !tbaa !16
  %32 = getelementptr inbounds %struct.FaString, ptr %0, i64 %18, i32 1
  %33 = load i64, ptr %32, align 8, !tbaa !21
  br label %34

34:                                               ; preds = %27, %30
  %35 = phi ptr [ %22, %27 ], [ %31, %30 ]
  %36 = phi i64 [ %29, %27 ], [ %33, %30 ]
  tail call void @fa_b_quoted(ptr noundef %35, i64 noundef %36) #44
  %37 = add nuw i64 %18, 1
  %38 = icmp eq i64 %37, %1
  br i1 %38, label %11, label %17, !llvm.loop !41
}

; Function Attrs: nounwind optsize
define void @fa_concat(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef readonly %2, i64 noundef %3, ptr nocapture noundef writeonly %4) local_unnamed_addr #2 {
  %6 = alloca %struct.FaString, align 8
  %7 = alloca %struct.FaString, align 8
  %8 = add i64 %3, %1
  %9 = icmp ult i64 %8, %1
  %10 = icmp ugt i64 %8, 4611686018427387902
  %11 = or i1 %9, %10
  br i1 %11, label %12, label %13

12:                                               ; preds = %5
  tail call fastcc void @out_of_memory() #47
  unreachable

13:                                               ; preds = %5
  %14 = icmp ult i64 %8, 2
  br i1 %14, label %15, label %18

15:                                               ; preds = %13
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %6) #40
  %16 = icmp eq i64 %1, 0
  %17 = select i1 %16, ptr %2, ptr %0
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 8 %6, ptr noundef %17, i64 noundef %8, i64 noundef 0) #44
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %4, ptr noundef nonnull align 8 dereferenceable(24) %6, i64 24, i1 false), !tbaa.struct !13
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %6) #40
  br label %35

18:                                               ; preds = %13
  %19 = icmp ult i64 %8, 24
  br i1 %19, label %20, label %28

20:                                               ; preds = %18
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %7) #40
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %7, i8 0, i64 24, i1 false)
  call fastcc void @copy_bytes(ptr noundef nonnull %7, ptr noundef %0, i64 noundef %1) #44
  %21 = getelementptr inbounds i8, ptr %7, i64 %1
  call fastcc void @copy_bytes(ptr noundef nonnull %21, ptr noundef %2, i64 noundef %3) #44
  %22 = call fastcc i64 @simple_flag(ptr noundef nonnull %7, i64 noundef %8) #44, !range !23
  %23 = icmp eq i64 %22, 0
  %24 = select i1 %23, i64 64, i64 192
  %25 = or disjoint i64 %24, %8
  %26 = trunc i64 %25 to i8
  %27 = getelementptr inbounds i8, ptr %7, i64 23
  store i8 %26, ptr %27, align 1, !tbaa !2
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %4, ptr noundef nonnull align 8 dereferenceable(24) %7, i64 24, i1 false), !tbaa.struct !13
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %7) #40
  br label %35

28:                                               ; preds = %18
  %29 = tail call ptr @fa_alloc(i64 noundef %8, i64 noundef 1) #45
  tail call fastcc void @copy_bytes(ptr noundef %29, ptr noundef %0, i64 noundef %1) #44
  %30 = getelementptr inbounds i8, ptr %29, i64 %1
  tail call fastcc void @copy_bytes(ptr noundef %30, ptr noundef %2, i64 noundef %3) #44
  store ptr %29, ptr %4, align 8, !tbaa !16
  %31 = getelementptr inbounds %struct.FaString, ptr %4, i64 0, i32 1
  store i64 %8, ptr %31, align 8, !tbaa !21
  %32 = tail call fastcc i64 @simple_flag(ptr noundef %29, i64 noundef %8) #44, !range !23
  %33 = or i64 %32, %8
  %34 = getelementptr inbounds %struct.FaString, ptr %4, i64 0, i32 2
  store i64 %33, ptr %34, align 8, !tbaa !22
  br label %35

35:                                               ; preds = %28, %20, %15
  ret void
}

; Function Attrs: noreturn nounwind optsize
define internal fastcc void @out_of_memory() unnamed_addr #4 {
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @out_of_memory.message, i64 noundef 20) #45
  tail call void @fa_exit(i32 noundef 101) #46
  unreachable
}

; Function Attrs: mustprogress nocallback nofree nounwind willreturn memory(argmem: write)
declare void @llvm.memset.p0.i64(ptr nocapture writeonly, i8, i64, i1 immarg) #11

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: read)
define internal fastcc i64 @simple_flag(ptr nocapture noundef readonly %0, i64 noundef %1) unnamed_addr #12 {
  %3 = icmp ult i64 %1, 8
  br i1 %3, label %4, label %8

4:                                                ; preds = %8, %2
  %5 = phi i64 [ 0, %2 ], [ %9, %8 ]
  %6 = phi i64 [ 0, %2 ], [ %19, %8 ]
  %7 = icmp ult i64 %5, %1
  br i1 %7, label %22, label %33

8:                                                ; preds = %2, %8
  %9 = phi i64 [ %20, %8 ], [ 8, %2 ]
  %10 = phi i64 [ %19, %8 ], [ 0, %2 ]
  %11 = phi i64 [ %9, %8 ], [ 0, %2 ]
  %12 = getelementptr inbounds i8, ptr %0, i64 %11
  %13 = load i64, ptr %12, align 1
  %14 = xor i64 %13, 940422246894996749
  %15 = add i64 %14, -72340172838076673
  %16 = xor i64 %13, -940422246894996750
  %17 = and i64 %15, %16
  %18 = or i64 %10, %17
  %19 = or i64 %18, %13
  %20 = add nuw i64 %9, 8
  %21 = icmp ugt i64 %20, %1
  br i1 %21, label %4, label %8, !llvm.loop !42

22:                                               ; preds = %4, %22
  %23 = phi i64 [ %30, %22 ], [ %6, %4 ]
  %24 = phi i64 [ %31, %22 ], [ %5, %4 ]
  %25 = getelementptr inbounds i8, ptr %0, i64 %24
  %26 = load i8, ptr %25, align 1, !tbaa !2
  %27 = icmp eq i8 %26, 13
  %28 = select i1 %27, i8 -128, i8 %26
  %29 = zext i8 %28 to i64
  %30 = or i64 %23, %29
  %31 = add nuw i64 %24, 1
  %32 = icmp eq i64 %31, %1
  br i1 %32, label %33, label %22, !llvm.loop !43

33:                                               ; preds = %22, %4
  %34 = phi i64 [ %6, %4 ], [ %30, %22 ]
  %35 = and i64 %34, -9187201950435737472
  %36 = icmp eq i64 %35, 0
  %37 = select i1 %36, i64 -9223372036854775808, i64 0
  ret i64 %37
}

; Function Attrs: optsize
declare ptr @fa_alloc(i64 noundef, i64 noundef) local_unnamed_addr #3

; Function Attrs: alwaysinline nounwind optsize
define void @fa_strings_join(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef writeonly %2) local_unnamed_addr #13 {
  %4 = alloca %struct.FaString, align 8
  %5 = icmp eq i64 %1, 0
  br i1 %5, label %36, label %8

6:                                                ; preds = %27
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %4) #40
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %4, i8 0, i64 24, i1 false)
  %7 = icmp ult i64 %28, 24
  br i1 %7, label %37, label %34

8:                                                ; preds = %3, %27
  %9 = phi i64 [ %28, %27 ], [ 0, %3 ]
  %10 = phi i64 [ %31, %27 ], [ -9223372036854775808, %3 ]
  %11 = phi i64 [ %32, %27 ], [ 0, %3 ]
  %12 = getelementptr inbounds %struct.FaString, ptr %0, i64 %11, i32 2
  %13 = load i64, ptr %12, align 8, !tbaa !22
  %14 = and i64 %13, 4611686018427387904
  %15 = icmp eq i64 %14, 0
  br i1 %15, label %19, label %16

16:                                               ; preds = %8
  %17 = lshr i64 %13, 56
  %18 = and i64 %17, 31
  br label %22

19:                                               ; preds = %8
  %20 = getelementptr inbounds %struct.FaString, ptr %0, i64 %11, i32 1
  %21 = load i64, ptr %20, align 8, !tbaa !21
  br label %22

22:                                               ; preds = %16, %19
  %23 = phi i64 [ %18, %16 ], [ %21, %19 ]
  %24 = sub i64 4611686018427387903, %9
  %25 = icmp ult i64 %23, %24
  br i1 %25, label %27, label %26

26:                                               ; preds = %22
  tail call fastcc void @out_of_memory() #47
  unreachable

27:                                               ; preds = %22
  %28 = add i64 %23, %9
  %29 = icmp eq i64 %23, 0
  %30 = select i1 %29, i64 -1, i64 %13
  %31 = and i64 %10, %30
  %32 = add nuw i64 %11, 1
  %33 = icmp eq i64 %32, %1
  br i1 %33, label %6, label %8, !llvm.loop !44

34:                                               ; preds = %6
  %35 = tail call ptr @fa_alloc(i64 noundef %28, i64 noundef 1) #45
  br label %37

36:                                               ; preds = %3
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %4) #40
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %4, i8 0, i64 24, i1 false)
  br label %70

37:                                               ; preds = %34, %6
  %38 = phi ptr [ %4, %6 ], [ %35, %34 ]
  br label %41

39:                                               ; preds = %56
  %40 = icmp eq i64 %31, 0
  br i1 %40, label %63, label %66

41:                                               ; preds = %37, %56
  %42 = phi i64 [ %61, %56 ], [ 0, %37 ]
  %43 = phi i64 [ %60, %56 ], [ 0, %37 ]
  %44 = getelementptr inbounds %struct.FaString, ptr %0, i64 %42
  %45 = getelementptr inbounds %struct.FaString, ptr %0, i64 %42, i32 2
  %46 = load i64, ptr %45, align 8, !tbaa !22
  %47 = and i64 %46, 4611686018427387904
  %48 = icmp eq i64 %47, 0
  br i1 %48, label %52, label %49

49:                                               ; preds = %41
  %50 = lshr i64 %46, 56
  %51 = and i64 %50, 31
  br label %56

52:                                               ; preds = %41
  %53 = getelementptr inbounds %struct.FaString, ptr %0, i64 %42, i32 1
  %54 = load i64, ptr %53, align 8, !tbaa !21
  %55 = load ptr, ptr %44, align 8, !tbaa !16
  br label %56

56:                                               ; preds = %49, %52
  %57 = phi i64 [ %54, %52 ], [ %51, %49 ]
  %58 = phi ptr [ %55, %52 ], [ %44, %49 ]
  %59 = getelementptr inbounds i8, ptr %38, i64 %43
  call fastcc void @copy_bytes(ptr noundef %59, ptr noundef %58, i64 noundef %57) #44
  %60 = add i64 %57, %43
  %61 = add nuw i64 %42, 1
  %62 = icmp eq i64 %61, %1
  br i1 %62, label %39, label %41, !llvm.loop !45

63:                                               ; preds = %39
  %64 = call fastcc i64 @simple_flag(ptr noundef %38, i64 noundef %28) #44, !range !23
  %65 = freeze i64 %64
  br i1 %7, label %67, label %76

66:                                               ; preds = %39
  br i1 %7, label %70, label %76

67:                                               ; preds = %63
  %68 = icmp eq i64 %65, 0
  %69 = select i1 %68, i64 64, i64 192
  br label %70

70:                                               ; preds = %67, %66, %36
  %71 = phi i64 [ %28, %66 ], [ 0, %36 ], [ %28, %67 ]
  %72 = phi i64 [ 192, %66 ], [ 192, %36 ], [ %69, %67 ]
  %73 = or disjoint i64 %72, %71
  %74 = trunc i64 %73 to i8
  %75 = getelementptr inbounds i8, ptr %4, i64 23
  store i8 %74, ptr %75, align 1, !tbaa !2
  br label %81

76:                                               ; preds = %63, %66
  %77 = phi i64 [ %65, %63 ], [ %31, %66 ]
  store ptr %38, ptr %4, align 8, !tbaa !16
  %78 = getelementptr inbounds %struct.FaString, ptr %4, i64 0, i32 1
  store i64 %28, ptr %78, align 8, !tbaa !21
  %79 = or i64 %77, %28
  %80 = getelementptr inbounds %struct.FaString, ptr %4, i64 0, i32 2
  store i64 %79, ptr %80, align 8, !tbaa !22
  br label %81

81:                                               ; preds = %70, %76
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %2, ptr noundef nonnull align 8 dereferenceable(24) %4, i64 24, i1 false), !tbaa.struct !13
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %4) #40
  ret void
}

; Function Attrs: alwaysinline nounwind optsize
define void @fa_string_append(ptr nocapture noundef %0, ptr nocapture noundef readonly %1, i64 noundef %2, i64 noundef %3) local_unnamed_addr #13 {
  %5 = alloca %struct.FaString, align 8
  %6 = getelementptr inbounds %struct.FaString, ptr %0, i64 0, i32 2
  %7 = load i64, ptr %6, align 8, !tbaa !22
  %8 = and i64 %7, 9223372036854775807
  %9 = and i64 %3, -9223372036854775808
  %10 = icmp ne i64 %9, 0
  %11 = icmp ult i64 %8, 4611686018427387903
  %12 = select i1 %10, i1 %11, i1 false
  br i1 %12, label %13, label %23, !prof !46

13:                                               ; preds = %4
  %14 = getelementptr inbounds %struct.FaString, ptr %0, i64 0, i32 1
  %15 = load i64, ptr %14, align 8, !tbaa !21
  %16 = add i64 %15, %2
  %17 = icmp ugt i64 %16, %8
  br i1 %17, label %23, label %18, !prof !47

18:                                               ; preds = %13
  %19 = load ptr, ptr %0, align 8, !tbaa !16
  %20 = getelementptr inbounds i8, ptr %19, i64 %15
  tail call fastcc void @copy_bytes(ptr noundef %20, ptr noundef %1, i64 noundef %2) #44
  %21 = load i64, ptr %14, align 8, !tbaa !21
  %22 = add i64 %21, %2
  store i64 %22, ptr %14, align 8, !tbaa !21
  br label %63

23:                                               ; preds = %4, %13
  %24 = icmp eq i64 %2, 0
  br i1 %24, label %63, label %25

25:                                               ; preds = %23
  %26 = and i64 %7, 4611686018427387904
  %27 = icmp eq i64 %26, 0
  br i1 %27, label %28, label %33

28:                                               ; preds = %25
  %29 = getelementptr inbounds %struct.FaString, ptr %0, i64 0, i32 1
  %30 = load i64, ptr %29, align 8, !tbaa !21
  %31 = add i64 %30, %2
  %32 = icmp ult i64 %31, 24
  br i1 %32, label %48, label %62

33:                                               ; preds = %25
  %34 = lshr i64 %7, 56
  %35 = and i64 %34, 31
  %36 = add i64 %35, %2
  %37 = icmp ult i64 %36, 24
  br i1 %37, label %38, label %62

38:                                               ; preds = %33
  %39 = icmp eq i64 %35, 0
  %40 = select i1 %39, i64 -9223372036854775808, i64 %7
  %41 = and i64 %40, %9
  %42 = getelementptr inbounds i8, ptr %0, i64 %35
  tail call fastcc void @copy_bytes(ptr noundef %42, ptr noundef %1, i64 noundef %2) #44
  %43 = icmp eq i64 %41, 0
  %44 = select i1 %43, i64 64, i64 192
  %45 = or disjoint i64 %44, %36
  %46 = trunc i64 %45 to i8
  %47 = getelementptr inbounds i8, ptr %0, i64 23
  store i8 %46, ptr %47, align 1, !tbaa !2
  br label %63

48:                                               ; preds = %28
  %49 = icmp eq i64 %30, 0
  %50 = select i1 %49, i64 -9223372036854775808, i64 %7
  %51 = and i64 %50, %9
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %5) #40
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %5, i8 0, i64 24, i1 false)
  %52 = load ptr, ptr %0, align 8, !tbaa !16
  call fastcc void @copy_bytes(ptr noundef nonnull %5, ptr noundef %52, i64 noundef %30) #44
  %53 = getelementptr inbounds i8, ptr %5, i64 %30
  call fastcc void @copy_bytes(ptr noundef nonnull %53, ptr noundef %1, i64 noundef %2) #44
  %54 = icmp eq i64 %51, 0
  %55 = select i1 %54, i64 64, i64 192
  %56 = or disjoint i64 %55, %31
  %57 = trunc i64 %56 to i8
  %58 = getelementptr inbounds i8, ptr %5, i64 23
  store i8 %57, ptr %58, align 1, !tbaa !2
  %59 = and i64 %7, 4611686018427387903
  switch i64 %59, label %60 [
    i64 4611686018427387903, label %61
    i64 0, label %61
  ]

60:                                               ; preds = %48
  tail call void @fa_free(ptr noundef %52, i64 noundef %59, i64 noundef 1) #45
  br label %61

61:                                               ; preds = %48, %48, %60
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %0, ptr noundef nonnull align 8 dereferenceable(24) %5, i64 24, i1 false), !tbaa.struct !13
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %5) #40
  br label %63

62:                                               ; preds = %33, %28
  tail call fastcc void @string_append_slow(ptr noundef nonnull %0, ptr noundef %1, i64 noundef %2, i64 noundef %3) #44
  br label %63

63:                                               ; preds = %62, %61, %38, %23, %18
  ret void
}

; Function Attrs: optsize
declare void @fa_free(ptr noundef, i64 noundef, i64 noundef) local_unnamed_addr #3

; Function Attrs: noinline nounwind optsize
define internal fastcc void @string_append_slow(ptr nocapture noundef %0, ptr nocapture noundef readonly %1, i64 noundef %2, i64 noundef %3) unnamed_addr #14 {
  %5 = alloca %struct.FaString, align 8
  %6 = getelementptr inbounds %struct.FaString, ptr %0, i64 0, i32 2
  %7 = load i64, ptr %6, align 8, !tbaa !22
  %8 = and i64 %7, 4611686018427387904
  %9 = icmp eq i64 %8, 0
  br i1 %9, label %13, label %10

10:                                               ; preds = %4
  %11 = lshr i64 %7, 56
  %12 = and i64 %11, 31
  br label %16

13:                                               ; preds = %4
  %14 = getelementptr inbounds %struct.FaString, ptr %0, i64 0, i32 1
  %15 = load i64, ptr %14, align 8, !tbaa !21
  br label %16

16:                                               ; preds = %10, %13
  %17 = phi i64 [ %12, %10 ], [ %15, %13 ]
  %18 = add i64 %17, %2
  %19 = icmp sgt i64 %3, -1
  br i1 %19, label %20, label %30

20:                                               ; preds = %16
  %21 = icmp eq i64 %2, 1
  br i1 %21, label %22, label %28

22:                                               ; preds = %20
  %23 = load i8, ptr %1, align 1, !tbaa !2
  %24 = icmp sgt i8 %23, -1
  %25 = icmp eq i8 %23, 13
  %26 = select i1 %25, i64 0, i64 -9223372036854775808
  %27 = select i1 %24, i64 %26, i64 0
  br label %30

28:                                               ; preds = %20
  %29 = tail call fastcc i64 @simple_flag(ptr noundef %1, i64 noundef %2) #44, !range !23
  br label %30

30:                                               ; preds = %22, %28, %16
  %31 = phi i64 [ -9223372036854775808, %16 ], [ %27, %22 ], [ %29, %28 ]
  %32 = icmp eq i64 %17, 0
  %33 = select i1 %32, i64 -1, i64 %7
  %34 = and i64 %31, %33
  %35 = icmp ult i64 %18, 24
  br i1 %35, label %36, label %55

36:                                               ; preds = %30
  br i1 %9, label %44, label %37

37:                                               ; preds = %36
  %38 = getelementptr inbounds i8, ptr %0, i64 %17
  tail call fastcc void @copy_bytes(ptr noundef %38, ptr noundef %1, i64 noundef %2) #44
  %39 = icmp eq i64 %34, 0
  %40 = select i1 %39, i64 64, i64 192
  %41 = or disjoint i64 %40, %18
  %42 = trunc i64 %41 to i8
  %43 = getelementptr inbounds i8, ptr %0, i64 23
  store i8 %42, ptr %43, align 1, !tbaa !2
  br label %69

44:                                               ; preds = %36
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %5) #40
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %5, i8 0, i64 24, i1 false)
  %45 = load ptr, ptr %0, align 8, !tbaa !16
  call fastcc void @copy_bytes(ptr noundef nonnull %5, ptr noundef %45, i64 noundef %17) #44
  %46 = getelementptr inbounds i8, ptr %5, i64 %17
  call fastcc void @copy_bytes(ptr noundef nonnull %46, ptr noundef %1, i64 noundef %2) #44
  %47 = icmp eq i64 %34, 0
  %48 = select i1 %47, i64 64, i64 192
  %49 = or disjoint i64 %48, %18
  %50 = trunc i64 %49 to i8
  %51 = getelementptr inbounds i8, ptr %5, i64 23
  store i8 %50, ptr %51, align 1, !tbaa !2
  %52 = and i64 %7, 4611686018427387903
  switch i64 %52, label %53 [
    i64 4611686018427387903, label %54
    i64 0, label %54
  ]

53:                                               ; preds = %44
  tail call void @fa_free(ptr noundef %45, i64 noundef %52, i64 noundef 1) #45
  br label %54

54:                                               ; preds = %44, %44, %53
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %0, ptr noundef nonnull align 8 dereferenceable(24) %5, i64 24, i1 false), !tbaa.struct !13
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %5) #40
  br label %69

55:                                               ; preds = %30
  %56 = and i64 %7, 4611686018427387903
  %57 = icmp ne i64 %56, 0
  %58 = icmp ne i64 %56, 4611686018427387903
  %59 = and i1 %57, %58
  %60 = and i1 %9, %59
  %61 = icmp ule i64 %18, %56
  %62 = select i1 %60, i1 %61, i1 false
  br i1 %62, label %63, label %68

63:                                               ; preds = %55
  %64 = load ptr, ptr %0, align 8, !tbaa !16
  %65 = getelementptr inbounds i8, ptr %64, i64 %17
  tail call fastcc void @copy_bytes(ptr noundef %65, ptr noundef %1, i64 noundef %2) #44
  %66 = getelementptr inbounds %struct.FaString, ptr %0, i64 0, i32 1
  store i64 %18, ptr %66, align 8, !tbaa !21
  %67 = or i64 %34, %56
  store i64 %67, ptr %6, align 8, !tbaa !22
  br label %69

68:                                               ; preds = %55
  tail call fastcc void @string_grow(ptr noundef nonnull %0, ptr noundef %1, i64 noundef %2, i64 noundef %34) #44
  br label %69

69:                                               ; preds = %63, %68, %54, %37
  ret void
}

; Function Attrs: alwaysinline nofree norecurse nosync nounwind optsize memory(argmem: read)
define i32 @fa_string_equal(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef readonly %2, i64 noundef %3) local_unnamed_addr #15 {
  %5 = icmp eq i64 %1, %3
  br i1 %5, label %6, label %14

6:                                                ; preds = %4
  switch i64 %1, label %12 [
    i64 0, label %14
    i64 1, label %7
  ]

7:                                                ; preds = %6
  %8 = load i8, ptr %0, align 1, !tbaa !2
  %9 = load i8, ptr %2, align 1, !tbaa !2
  %10 = icmp eq i8 %8, %9
  %11 = zext i1 %10 to i32
  br label %14

12:                                               ; preds = %6
  %13 = tail call fastcc i32 @string_equal_bytes(ptr noundef %0, ptr noundef %2, i64 noundef %1) #44
  br label %14

14:                                               ; preds = %6, %4, %12, %7
  %15 = phi i32 [ %11, %7 ], [ %13, %12 ], [ 0, %4 ], [ 1, %6 ]
  ret i32 %15
}

; Function Attrs: nofree noinline norecurse nosync nounwind optsize memory(argmem: read)
define internal fastcc noundef i32 @string_equal_bytes(ptr nocapture noundef readonly %0, ptr nocapture noundef readonly %1, i64 noundef %2) unnamed_addr #16 {
  %4 = icmp ugt i64 %2, 7
  br i1 %4, label %5, label %16

5:                                                ; preds = %3, %12
  %6 = phi i64 [ %13, %12 ], [ 0, %3 ]
  %7 = getelementptr inbounds i8, ptr %0, i64 %6
  %8 = load i64, ptr %7, align 1
  %9 = getelementptr inbounds i8, ptr %1, i64 %6
  %10 = load i64, ptr %9, align 1
  %11 = icmp eq i64 %8, %10
  br i1 %11, label %12, label %48

12:                                               ; preds = %5
  %13 = add i64 %6, 8
  %14 = sub i64 %2, %13
  %15 = icmp ugt i64 %14, 7
  br i1 %15, label %5, label %16, !llvm.loop !48

16:                                               ; preds = %12, %3
  %17 = phi i64 [ 0, %3 ], [ %13, %12 ]
  %18 = phi i64 [ %2, %3 ], [ %14, %12 ]
  %19 = icmp ugt i64 %18, 3
  br i1 %19, label %20, label %27

20:                                               ; preds = %16
  %21 = getelementptr inbounds i8, ptr %0, i64 %17
  %22 = load i32, ptr %21, align 1
  %23 = getelementptr inbounds i8, ptr %1, i64 %17
  %24 = load i32, ptr %23, align 1
  %25 = icmp eq i32 %22, %24
  %26 = or disjoint i64 %17, 4
  br i1 %25, label %27, label %48

27:                                               ; preds = %20, %16
  %28 = phi i64 [ %26, %20 ], [ %17, %16 ]
  %29 = sub i64 %2, %28
  %30 = icmp ugt i64 %29, 1
  br i1 %30, label %31, label %38

31:                                               ; preds = %27
  %32 = getelementptr inbounds i8, ptr %0, i64 %28
  %33 = load i16, ptr %32, align 1
  %34 = getelementptr inbounds i8, ptr %1, i64 %28
  %35 = load i16, ptr %34, align 1
  %36 = icmp eq i16 %33, %35
  %37 = add i64 %28, 2
  br i1 %36, label %38, label %48

38:                                               ; preds = %31, %27
  %39 = phi i64 [ %37, %31 ], [ %28, %27 ]
  %40 = icmp ult i64 %39, %2
  br i1 %40, label %41, label %47

41:                                               ; preds = %38
  %42 = getelementptr inbounds i8, ptr %0, i64 %39
  %43 = load i8, ptr %42, align 1, !tbaa !2
  %44 = getelementptr inbounds i8, ptr %1, i64 %39
  %45 = load i8, ptr %44, align 1, !tbaa !2
  %46 = icmp eq i8 %43, %45
  br i1 %46, label %47, label %48

47:                                               ; preds = %41, %38
  br label %48

48:                                               ; preds = %5, %41, %20, %31, %47
  %49 = phi i32 [ 1, %47 ], [ 0, %31 ], [ 0, %20 ], [ 0, %41 ], [ 0, %5 ]
  ret i32 %49
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: read)
define i32 @fa_string_compare(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef readonly %2, i64 noundef %3) local_unnamed_addr #12 {
  %5 = tail call i64 @llvm.umin.i64(i64 %1, i64 %3)
  %6 = icmp eq i64 %5, 0
  br i1 %6, label %11, label %7

7:                                                ; preds = %4
  %8 = load i8, ptr %0, align 1, !tbaa !2
  %9 = load i8, ptr %2, align 1, !tbaa !2
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
  br i1 %19, label %39, label %20, !llvm.loop !49

20:                                               ; preds = %16
  %21 = getelementptr inbounds i8, ptr %0, i64 %18
  %22 = load i8, ptr %21, align 1, !tbaa !2
  %23 = getelementptr inbounds i8, ptr %2, i64 %18
  %24 = load i8, ptr %23, align 1, !tbaa !2
  %25 = icmp eq i8 %22, %24
  br i1 %25, label %16, label %26, !llvm.loop !49

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
define i64 @fa_next_grapheme(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #10 {
  %4 = tail call fastcc i64 @next_boundary(ptr noundef %0, i64 noundef %1, i64 noundef %2) #44
  ret i64 %4
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define internal fastcc i64 @next_boundary(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2) unnamed_addr #10 {
  %4 = alloca i64, align 8
  %5 = alloca i64, align 8
  store i64 %2, ptr %4, align 8, !tbaa !8
  %6 = getelementptr inbounds i8, ptr %0, i64 %2
  %7 = load i8, ptr %6, align 1, !tbaa !2
  %8 = icmp sgt i8 %7, -1
  br i1 %8, label %9, label %24

9:                                                ; preds = %3
  %10 = add i64 %2, 1
  %11 = icmp ult i64 %10, %1
  br i1 %11, label %12, label %21

12:                                               ; preds = %9
  %13 = getelementptr inbounds i8, ptr %0, i64 %10
  %14 = load i8, ptr %13, align 1, !tbaa !2
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
  %25 = call fastcc i32 @decode(ptr noundef nonnull %0, i64 noundef %1, ptr noundef nonnull %4) #44
  %26 = tail call fastcc i32 @break_kind(i32 noundef %25) #44, !range !50
  %27 = load i64, ptr %4, align 8, !tbaa !8
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
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %5) #40
  store i64 %38, ptr %5, align 8, !tbaa !8
  %39 = call fastcc i32 @decode(ptr noundef %0, i64 noundef %1, ptr noundef nonnull %5) #44
  %40 = tail call fastcc i32 @break_kind(i32 noundef %39) #44, !range !50
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
  %75 = load i64, ptr %5, align 8, !tbaa !8
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
  %84 = load i64, ptr %5, align 8, !tbaa !8
  %85 = add nuw nsw i32 %36, 1
  br label %94

86:                                               ; preds = %53, %52, %52, %52, %52, %34
  %87 = load i64, ptr %5, align 8, !tbaa !8
  br label %94

88:                                               ; preds = %59
  %89 = load i64, ptr %5, align 8, !tbaa !8
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
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %5) #40
  br label %106

101:                                              ; preds = %74, %94, %88
  %102 = phi i64 [ %89, %88 ], [ %95, %94 ], [ %75, %74 ]
  %103 = phi i32 [ %92, %88 ], [ %96, %94 ], [ 0, %74 ]
  %104 = phi i32 [ 1, %88 ], [ %99, %94 ], [ 1, %74 ]
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %5) #40
  %105 = icmp ult i64 %102, %1
  br i1 %105, label %34, label %106

106:                                              ; preds = %101, %24, %100, %21
  %107 = phi i64 [ %23, %21 ], [ %38, %100 ], [ %27, %24 ], [ %102, %101 ]
  ret i64 %107
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define i64 @fa_graphemes(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #10 {
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
  %11 = load i8, ptr %10, align 1, !tbaa !2
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
  %23 = tail call fastcc i64 @next_boundary(ptr noundef %0, i64 noundef %1, i64 noundef %6) #44
  %24 = add i64 %5, 1
  br label %25

25:                                               ; preds = %22, %13
  %26 = phi i64 [ %7, %13 ], [ %23, %22 ]
  %27 = phi i64 [ %21, %13 ], [ %24, %22 ]
  %28 = icmp ult i64 %26, %1
  br i1 %28, label %4, label %29, !llvm.loop !51

29:                                               ; preds = %25, %2
  %30 = phi i64 [ 0, %2 ], [ %27, %25 ]
  ret i64 %30
}

; Function Attrs: nounwind optsize
define void @fa_b_append_into(ptr nocapture noundef %0) local_unnamed_addr #2 {
  %2 = alloca %struct.FaString, align 8
  %3 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %4 = load ptr, ptr %3, align 8, !tbaa !10
  %5 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %6 = load i64, ptr %5, align 8, !tbaa !8
  %7 = tail call fastcc i64 @simple_flag(ptr noundef %4, i64 noundef %6) #44, !range !23
  %8 = getelementptr inbounds %struct.FaString, ptr %0, i64 0, i32 2
  %9 = load i64, ptr %8, align 8, !tbaa !22
  %10 = and i64 %9, 9223372036854775807
  %11 = and i64 %7, -9223372036854775808
  %12 = icmp ne i64 %11, 0
  %13 = icmp ult i64 %10, 4611686018427387903
  %14 = select i1 %12, i1 %13, i1 false
  br i1 %14, label %15, label %25, !prof !46

15:                                               ; preds = %1
  %16 = getelementptr inbounds %struct.FaString, ptr %0, i64 0, i32 1
  %17 = load i64, ptr %16, align 8, !tbaa !21
  %18 = add i64 %17, %6
  %19 = icmp ugt i64 %18, %10
  br i1 %19, label %25, label %20, !prof !47

20:                                               ; preds = %15
  %21 = load ptr, ptr %0, align 8, !tbaa !16
  %22 = getelementptr inbounds i8, ptr %21, i64 %17
  tail call fastcc void @copy_bytes(ptr noundef %22, ptr noundef %4, i64 noundef %6) #44
  %23 = load i64, ptr %16, align 8, !tbaa !21
  %24 = add i64 %23, %6
  store i64 %24, ptr %16, align 8, !tbaa !21
  br label %65

25:                                               ; preds = %15, %1
  %26 = icmp eq i64 %6, 0
  br i1 %26, label %65, label %27

27:                                               ; preds = %25
  %28 = and i64 %9, 4611686018427387904
  %29 = icmp eq i64 %28, 0
  br i1 %29, label %30, label %35

30:                                               ; preds = %27
  %31 = getelementptr inbounds %struct.FaString, ptr %0, i64 0, i32 1
  %32 = load i64, ptr %31, align 8, !tbaa !21
  %33 = add i64 %32, %6
  %34 = icmp ult i64 %33, 24
  br i1 %34, label %50, label %64

35:                                               ; preds = %27
  %36 = lshr i64 %9, 56
  %37 = and i64 %36, 31
  %38 = add i64 %37, %6
  %39 = icmp ult i64 %38, 24
  br i1 %39, label %40, label %64

40:                                               ; preds = %35
  %41 = icmp eq i64 %37, 0
  %42 = select i1 %41, i64 -9223372036854775808, i64 %9
  %43 = and i64 %42, %11
  %44 = getelementptr inbounds i8, ptr %0, i64 %37
  tail call fastcc void @copy_bytes(ptr noundef %44, ptr noundef %4, i64 noundef %6) #44
  %45 = icmp eq i64 %43, 0
  %46 = select i1 %45, i64 64, i64 192
  %47 = or disjoint i64 %46, %38
  %48 = trunc i64 %47 to i8
  %49 = getelementptr inbounds i8, ptr %0, i64 23
  store i8 %48, ptr %49, align 1, !tbaa !2
  br label %65

50:                                               ; preds = %30
  %51 = icmp eq i64 %32, 0
  %52 = select i1 %51, i64 -9223372036854775808, i64 %9
  %53 = and i64 %52, %11
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %2) #40
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %2, i8 0, i64 24, i1 false)
  %54 = load ptr, ptr %0, align 8, !tbaa !16
  call fastcc void @copy_bytes(ptr noundef nonnull %2, ptr noundef %54, i64 noundef %32) #44
  %55 = getelementptr inbounds i8, ptr %2, i64 %32
  call fastcc void @copy_bytes(ptr noundef nonnull %55, ptr noundef %4, i64 noundef %6) #44
  %56 = icmp eq i64 %53, 0
  %57 = select i1 %56, i64 64, i64 192
  %58 = or disjoint i64 %57, %33
  %59 = trunc i64 %58 to i8
  %60 = getelementptr inbounds i8, ptr %2, i64 23
  store i8 %59, ptr %60, align 1, !tbaa !2
  %61 = and i64 %9, 4611686018427387903
  switch i64 %61, label %62 [
    i64 4611686018427387903, label %63
    i64 0, label %63
  ]

62:                                               ; preds = %50
  tail call void @fa_free(ptr noundef %54, i64 noundef %61, i64 noundef 1) #45
  br label %63

63:                                               ; preds = %50, %50, %62
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %0, ptr noundef nonnull align 8 dereferenceable(24) %2, i64 24, i1 false), !tbaa.struct !13
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %2) #40
  br label %65

64:                                               ; preds = %35, %30
  tail call fastcc void @string_append_slow(ptr noundef nonnull %0, ptr noundef %4, i64 noundef %6, i64 noundef %7) #44
  br label %65

65:                                               ; preds = %20, %25, %40, %63, %64
  store i64 0, ptr %5, align 8, !tbaa !8
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define i32 @fa_string_includes(ptr noundef %0, i64 noundef %1, i64 noundef %2, ptr nocapture noundef readonly %3, i64 noundef %4) local_unnamed_addr #17 {
  %6 = and i64 %2, -9223372036854775808
  %7 = tail call fastcc i64 @find(ptr noundef %0, i64 noundef %1, i64 noundef %6, ptr noundef %3, i64 noundef %4, i32 noundef 0) #44
  %8 = icmp sgt i64 %7, -1
  %9 = zext i1 %8 to i32
  ret i32 %9
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc i64 @find(ptr noundef %0, i64 noundef %1, i64 noundef %2, ptr nocapture noundef readonly %3, i64 noundef %4, i32 noundef %5) unnamed_addr #17 {
  %7 = alloca %struct.Boundaries, align 8
  %8 = alloca %struct.Boundaries, align 8
  %9 = icmp ugt i64 %4, %1
  br i1 %9, label %116, label %10

10:                                               ; preds = %6
  %11 = icmp eq i64 %2, 0
  %12 = icmp eq i64 %4, 0
  %13 = or i1 %11, %12
  %14 = icmp ne i32 %5, 0
  %15 = or i1 %13, %14
  br i1 %15, label %43, label %16

16:                                               ; preds = %10
  %17 = load i8, ptr %3, align 1, !tbaa !2
  %18 = tail call fastcc i64 @next_byte(ptr noundef %0, i64 noundef %1, i64 noundef 0, i8 noundef zeroext %17) #44
  %19 = add i64 %18, %4
  %20 = icmp ugt i64 %19, %1
  br i1 %20, label %116, label %21

21:                                               ; preds = %16, %38
  %22 = phi i64 [ %40, %38 ], [ %18, %16 ]
  %23 = getelementptr i8, ptr %0, i64 %22
  %24 = load i8, ptr %23, align 1, !tbaa !2
  %25 = icmp eq i8 %24, %17
  br i1 %25, label %26, label %38

26:                                               ; preds = %21, %30
  %27 = phi i64 [ %28, %30 ], [ 0, %21 ]
  %28 = add nuw i64 %27, 1
  %29 = icmp eq i64 %28, %4
  br i1 %29, label %36, label %30, !llvm.loop !52

30:                                               ; preds = %26
  %31 = getelementptr i8, ptr %23, i64 %28
  %32 = load i8, ptr %31, align 1, !tbaa !2
  %33 = getelementptr inbounds i8, ptr %3, i64 %28
  %34 = load i8, ptr %33, align 1, !tbaa !2
  %35 = icmp eq i8 %32, %34
  br i1 %35, label %26, label %36, !llvm.loop !52

36:                                               ; preds = %30, %26
  %37 = icmp ult i64 %28, %4
  br i1 %37, label %38, label %116

38:                                               ; preds = %21, %36
  %39 = add i64 %22, 1
  %40 = tail call fastcc i64 @next_byte(ptr noundef nonnull %0, i64 noundef %1, i64 noundef %39, i8 noundef zeroext %17) #44
  %41 = add i64 %40, %4
  %42 = icmp ugt i64 %41, %1
  br i1 %42, label %116, label %21, !llvm.loop !53

43:                                               ; preds = %10
  br i1 %13, label %73, label %44

44:                                               ; preds = %43
  %45 = load i8, ptr %3, align 1, !tbaa !2
  %46 = sub i64 %1, %4
  %47 = icmp eq i64 %46, -1
  br i1 %47, label %116, label %48

48:                                               ; preds = %44, %70
  %49 = phi i64 [ %71, %70 ], [ 0, %44 ]
  %50 = sub i64 %46, %49
  %51 = select i1 %14, i64 %50, i64 %49
  %52 = getelementptr inbounds i8, ptr %0, i64 %51
  %53 = load i8, ptr %52, align 1, !tbaa !2
  %54 = icmp ne i8 %53, %45
  %55 = add i64 %51, %4
  %56 = icmp ugt i64 %55, %1
  %57 = or i1 %56, %54
  br i1 %57, label %70, label %58

58:                                               ; preds = %48, %62
  %59 = phi i64 [ %60, %62 ], [ 0, %48 ]
  %60 = add nuw i64 %59, 1
  %61 = icmp eq i64 %60, %4
  br i1 %61, label %68, label %62, !llvm.loop !52

62:                                               ; preds = %58
  %63 = getelementptr i8, ptr %52, i64 %60
  %64 = load i8, ptr %63, align 1, !tbaa !2
  %65 = getelementptr inbounds i8, ptr %3, i64 %60
  %66 = load i8, ptr %65, align 1, !tbaa !2
  %67 = icmp eq i8 %64, %66
  br i1 %67, label %58, label %68, !llvm.loop !52

68:                                               ; preds = %62, %58
  %69 = icmp ult i64 %60, %4
  br i1 %69, label %70, label %116

70:                                               ; preds = %48, %68
  %71 = add nuw i64 %49, 1
  %72 = icmp eq i64 %49, %46
  br i1 %72, label %116, label %48, !llvm.loop !54

73:                                               ; preds = %43
  call void @llvm.lifetime.start.p0(i64 32, ptr nonnull %7) #40
  store ptr %0, ptr %7, align 8, !tbaa !55
  %74 = getelementptr inbounds %struct.Boundaries, ptr %7, i64 0, i32 1
  store i64 %1, ptr %74, align 8, !tbaa !57
  %75 = getelementptr inbounds %struct.Boundaries, ptr %7, i64 0, i32 2
  store i64 0, ptr %75, align 8, !tbaa !58
  %76 = getelementptr inbounds %struct.Boundaries, ptr %7, i64 0, i32 3
  store i64 %2, ptr %76, align 8, !tbaa !59
  call void @llvm.lifetime.start.p0(i64 32, ptr nonnull %8) #40
  store ptr %0, ptr %8, align 8, !tbaa !55
  %77 = getelementptr inbounds %struct.Boundaries, ptr %8, i64 0, i32 1
  store i64 %1, ptr %77, align 8, !tbaa !57
  %78 = getelementptr inbounds %struct.Boundaries, ptr %8, i64 0, i32 2
  store i64 0, ptr %78, align 8, !tbaa !58
  %79 = getelementptr inbounds %struct.Boundaries, ptr %8, i64 0, i32 3
  store i64 %2, ptr %79, align 8, !tbaa !59
  br label %80

80:                                               ; preds = %73, %109
  %81 = phi i64 [ %112, %109 ], [ %4, %73 ]
  %82 = phi i64 [ %111, %109 ], [ 0, %73 ]
  %83 = phi i64 [ %110, %109 ], [ -1, %73 ]
  %84 = getelementptr i8, ptr %0, i64 %82
  br i1 %12, label %101, label %85

85:                                               ; preds = %80
  %86 = load i8, ptr %84, align 1, !tbaa !2
  %87 = load i8, ptr %3, align 1, !tbaa !2
  %88 = icmp eq i8 %86, %87
  br i1 %88, label %89, label %109

89:                                               ; preds = %85, %93
  %90 = phi i64 [ %91, %93 ], [ 0, %85 ]
  %91 = add nuw i64 %90, 1
  %92 = icmp eq i64 %91, %4
  br i1 %92, label %99, label %93, !llvm.loop !52

93:                                               ; preds = %89
  %94 = getelementptr i8, ptr %84, i64 %91
  %95 = load i8, ptr %94, align 1, !tbaa !2
  %96 = getelementptr inbounds i8, ptr %3, i64 %91
  %97 = load i8, ptr %96, align 1, !tbaa !2
  %98 = icmp eq i8 %95, %97
  br i1 %98, label %89, label %99, !llvm.loop !52

99:                                               ; preds = %93, %89
  %100 = icmp ult i64 %91, %4
  br i1 %100, label %109, label %101

101:                                              ; preds = %80, %99
  %102 = call fastcc i32 @at_boundary(ptr noundef nonnull %7, i64 noundef %82) #44, !range !60
  %103 = icmp eq i32 %102, 0
  br i1 %103, label %109, label %104

104:                                              ; preds = %101
  %105 = call fastcc i32 @at_boundary(ptr noundef nonnull %8, i64 noundef %81) #44, !range !60
  %106 = icmp eq i32 %105, 0
  %107 = or i1 %14, %106
  %108 = select i1 %106, i64 %83, i64 %82
  br i1 %107, label %109, label %114

109:                                              ; preds = %85, %104, %99, %101
  %110 = phi i64 [ %108, %104 ], [ %83, %101 ], [ %83, %99 ], [ %83, %85 ]
  %111 = add i64 %82, 1
  %112 = add i64 %111, %4
  %113 = icmp ugt i64 %112, %1
  br i1 %113, label %114, label %80, !llvm.loop !61

114:                                              ; preds = %109, %104
  %115 = phi i64 [ %110, %109 ], [ %82, %104 ]
  call void @llvm.lifetime.end.p0(i64 32, ptr nonnull %8) #40
  call void @llvm.lifetime.end.p0(i64 32, ptr nonnull %7) #40
  br label %116

116:                                              ; preds = %38, %36, %68, %70, %16, %44, %6, %114
  %117 = phi i64 [ %115, %114 ], [ -1, %6 ], [ -1, %44 ], [ -1, %16 ], [ %51, %68 ], [ -1, %70 ], [ -1, %38 ], [ %22, %36 ]
  ret i64 %117
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define i32 @fa_string_starts_with(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2, ptr nocapture noundef readonly %3, i64 noundef %4) local_unnamed_addr #10 {
  %6 = icmp ugt i64 %4, %1
  br i1 %6, label %37, label %7

7:                                                ; preds = %5
  %8 = icmp eq i64 %4, 0
  br i1 %8, label %27, label %9

9:                                                ; preds = %7
  %10 = load i8, ptr %0, align 1, !tbaa !2
  %11 = load i8, ptr %3, align 1, !tbaa !2
  %12 = icmp eq i8 %10, %11
  br i1 %12, label %13, label %37

13:                                               ; preds = %9, %17
  %14 = phi i64 [ %15, %17 ], [ 0, %9 ]
  %15 = add nuw i64 %14, 1
  %16 = icmp eq i64 %15, %4
  br i1 %16, label %23, label %17, !llvm.loop !52

17:                                               ; preds = %13
  %18 = getelementptr i8, ptr %0, i64 %15
  %19 = load i8, ptr %18, align 1, !tbaa !2
  %20 = getelementptr inbounds i8, ptr %3, i64 %15
  %21 = load i8, ptr %20, align 1, !tbaa !2
  %22 = icmp eq i8 %19, %21
  br i1 %22, label %13, label %23, !llvm.loop !52

23:                                               ; preds = %17, %13
  %24 = icmp ult i64 %15, %4
  br i1 %24, label %37, label %25

25:                                               ; preds = %23
  %26 = icmp sgt i64 %2, -1
  br i1 %26, label %29, label %37

27:                                               ; preds = %7
  %28 = icmp sgt i64 %2, -1
  br i1 %28, label %33, label %37

29:                                               ; preds = %25, %29
  %30 = phi i64 [ %31, %29 ], [ 0, %25 ]
  %31 = tail call fastcc i64 @next_boundary(ptr noundef nonnull %0, i64 noundef %1, i64 noundef %30) #44
  %32 = icmp ult i64 %31, %4
  br i1 %32, label %29, label %33, !llvm.loop !62

33:                                               ; preds = %29, %27
  %34 = phi i64 [ 0, %27 ], [ %31, %29 ]
  %35 = icmp eq i64 %34, %4
  %36 = zext i1 %35 to i32
  br label %37

37:                                               ; preds = %9, %5, %27, %25, %33, %23
  %38 = phi i32 [ 0, %23 ], [ 1, %25 ], [ %36, %33 ], [ 1, %27 ], [ 0, %5 ], [ 0, %9 ]
  ret i32 %38
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define i32 @fa_string_ends_with(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2, ptr nocapture noundef readonly %3, i64 noundef %4) local_unnamed_addr #10 {
  %6 = icmp ugt i64 %4, %1
  br i1 %6, label %39, label %7

7:                                                ; preds = %5
  %8 = sub i64 %1, %4
  %9 = getelementptr i8, ptr %0, i64 %8
  %10 = icmp eq i64 %4, 0
  br i1 %10, label %27, label %11

11:                                               ; preds = %7
  %12 = load i8, ptr %9, align 1, !tbaa !2
  %13 = load i8, ptr %3, align 1, !tbaa !2
  %14 = icmp eq i8 %12, %13
  br i1 %14, label %15, label %39

15:                                               ; preds = %11, %19
  %16 = phi i64 [ %17, %19 ], [ 0, %11 ]
  %17 = add nuw i64 %16, 1
  %18 = icmp eq i64 %17, %4
  br i1 %18, label %25, label %19, !llvm.loop !52

19:                                               ; preds = %15
  %20 = getelementptr i8, ptr %9, i64 %17
  %21 = load i8, ptr %20, align 1, !tbaa !2
  %22 = getelementptr inbounds i8, ptr %3, i64 %17
  %23 = load i8, ptr %22, align 1, !tbaa !2
  %24 = icmp eq i8 %21, %23
  br i1 %24, label %15, label %25, !llvm.loop !52

25:                                               ; preds = %19, %15
  %26 = icmp ult i64 %17, %4
  br i1 %26, label %39, label %27

27:                                               ; preds = %7, %25
  %28 = icmp sgt i64 %2, -1
  br i1 %28, label %29, label %39

29:                                               ; preds = %27
  %30 = icmp eq i64 %1, %4
  br i1 %30, label %35, label %31

31:                                               ; preds = %29, %31
  %32 = phi i64 [ %33, %31 ], [ 0, %29 ]
  %33 = tail call fastcc i64 @next_boundary(ptr noundef %0, i64 noundef %1, i64 noundef %32) #44
  %34 = icmp ult i64 %33, %8
  br i1 %34, label %31, label %35, !llvm.loop !62

35:                                               ; preds = %31, %29
  %36 = phi i64 [ 0, %29 ], [ %33, %31 ]
  %37 = icmp eq i64 %36, %8
  %38 = zext i1 %37 to i32
  br label %39

39:                                               ; preds = %11, %27, %35, %25, %5
  %40 = phi i32 [ 0, %25 ], [ 0, %5 ], [ 1, %27 ], [ %38, %35 ], [ 0, %11 ]
  ret i32 %40
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define i64 @fa_string_index_of(ptr noundef %0, i64 noundef %1, i64 noundef %2, ptr nocapture noundef readonly %3, i64 noundef %4, i32 noundef %5) local_unnamed_addr #17 {
  %7 = icmp sgt i64 %2, -1
  %8 = icmp ne i64 %4, 1
  %9 = or i1 %7, %8
  %10 = icmp ne i32 %5, 0
  %11 = or i1 %9, %10
  %12 = icmp ugt i64 %1, 32
  %13 = or i1 %12, %11
  br i1 %13, label %26, label %14

14:                                               ; preds = %6
  %15 = icmp eq i64 %1, 0
  br i1 %15, label %28, label %16

16:                                               ; preds = %14
  %17 = load i8, ptr %3, align 1, !tbaa !2
  br label %18

18:                                               ; preds = %16, %23
  %19 = phi i64 [ 0, %16 ], [ %24, %23 ]
  %20 = getelementptr inbounds i8, ptr %0, i64 %19
  %21 = load i8, ptr %20, align 1, !tbaa !2
  %22 = icmp eq i8 %21, %17
  br i1 %22, label %28, label %23

23:                                               ; preds = %18
  %24 = add nuw i64 %19, 1
  %25 = icmp eq i64 %24, %1
  br i1 %25, label %28, label %18, !llvm.loop !63

26:                                               ; preds = %6
  %27 = tail call fastcc i64 @string_index_of(ptr noundef %0, i64 noundef %1, i64 noundef %2, ptr noundef %3, i64 noundef %4, i32 noundef %5) #44
  br label %28

28:                                               ; preds = %18, %23, %14, %26
  %29 = phi i64 [ %27, %26 ], [ -1, %14 ], [ %19, %18 ], [ -1, %23 ]
  ret i64 %29
}

; Function Attrs: nofree noinline norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc i64 @string_index_of(ptr noundef %0, i64 noundef %1, i64 noundef %2, ptr nocapture noundef readonly %3, i64 noundef %4, i32 noundef %5) unnamed_addr #18 {
  %7 = and i64 %2, -9223372036854775808
  %8 = tail call fastcc i64 @find(ptr noundef %0, i64 noundef %1, i64 noundef %7, ptr noundef %3, i64 noundef %4, i32 noundef %5) #44
  %9 = or i64 %8, %2
  %10 = icmp sgt i64 %9, -1
  br i1 %10, label %11, label %19

11:                                               ; preds = %6
  %12 = icmp eq i64 %8, 0
  br i1 %12, label %19, label %13

13:                                               ; preds = %11, %13
  %14 = phi i64 [ %17, %13 ], [ 0, %11 ]
  %15 = phi i64 [ %16, %13 ], [ 0, %11 ]
  %16 = add i64 %15, 1
  %17 = tail call fastcc i64 @next_boundary(ptr noundef %0, i64 noundef %1, i64 noundef %14) #44
  %18 = icmp ult i64 %17, %8
  br i1 %18, label %13, label %19, !llvm.loop !64

19:                                               ; preds = %13, %11, %6
  %20 = phi i64 [ %8, %6 ], [ 0, %11 ], [ %16, %13 ]
  ret i64 %20
}

; Function Attrs: nounwind optsize
define void @fa_string_replace(ptr noundef %0, i64 noundef %1, i64 noundef %2, ptr nocapture noundef readonly %3, i64 noundef %4, ptr noundef %5, i64 noundef %6, i32 noundef %7, ptr nocapture noundef writeonly %8) local_unnamed_addr #2 {
  %10 = alloca %struct.FaString, align 8
  %11 = alloca %struct.Replacing, align 8
  %12 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %12, align 8, !tbaa !8
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %11) #40
  store ptr %0, ptr %11, align 8, !tbaa !65
  %13 = getelementptr inbounds %struct.Replacing, ptr %11, i64 0, i32 1
  store i64 0, ptr %13, align 8, !tbaa !67
  %14 = getelementptr inbounds %struct.Replacing, ptr %11, i64 0, i32 2
  store i64 %4, ptr %14, align 8, !tbaa !68
  %15 = getelementptr inbounds %struct.Replacing, ptr %11, i64 0, i32 3
  store ptr %5, ptr %15, align 8, !tbaa !69
  %16 = getelementptr inbounds %struct.Replacing, ptr %11, i64 0, i32 4
  store i64 %6, ptr %16, align 8, !tbaa !70
  %17 = getelementptr inbounds %struct.Replacing, ptr %11, i64 0, i32 5
  store i32 %7, ptr %17, align 8, !tbaa !71
  %18 = getelementptr inbounds %struct.Replacing, ptr %11, i64 0, i32 6
  store i32 0, ptr %18, align 4, !tbaa !72
  %19 = and i64 %2, -9223372036854775808
  call fastcc void @each_match(ptr noundef %0, i64 noundef %1, i64 noundef %19, ptr noundef %3, i64 noundef %4, ptr noundef nonnull @replace_one, ptr noundef nonnull %11) #44
  %20 = load i64, ptr %13, align 8, !tbaa !67
  %21 = getelementptr inbounds i8, ptr %0, i64 %20
  %22 = sub i64 %1, %20
  call fastcc void @reserve(i64 noundef %22) #44
  %23 = call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %24 = load ptr, ptr %23, align 8, !tbaa !10
  %25 = load i64, ptr %12, align 8, !tbaa !8
  %26 = getelementptr inbounds i8, ptr %24, i64 %25
  call fastcc void @copy_bytes(ptr noundef %26, ptr noundef %21, i64 noundef %22) #44
  %27 = load i64, ptr %12, align 8, !tbaa !8
  %28 = add i64 %27, %22
  store i64 %28, ptr %12, align 8, !tbaa !8
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %10) #40
  %29 = load ptr, ptr %23, align 8, !tbaa !10
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 8 %10, ptr noundef %29, i64 noundef %28, i64 noundef 0) #44
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %8, ptr noundef nonnull align 8 dereferenceable(24) %10, i64 24, i1 false), !tbaa.struct !13
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %10) #40
  store i64 0, ptr %12, align 8, !tbaa !8
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %11) #40
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @each_match(ptr noundef %0, i64 noundef %1, i64 noundef %2, ptr nocapture noundef readonly %3, i64 noundef %4, ptr nocapture noundef readonly %5, ptr noundef %6) unnamed_addr #2 {
  %8 = alloca %struct.Boundaries, align 8
  %9 = alloca %struct.Boundaries, align 8
  %10 = icmp ne i64 %2, 0
  %11 = icmp ne i64 %4, 0
  %12 = and i1 %10, %11
  br i1 %12, label %13, label %47

13:                                               ; preds = %7
  %14 = load i8, ptr %3, align 1, !tbaa !2
  %15 = tail call fastcc i64 @next_byte(ptr noundef %0, i64 noundef %1, i64 noundef 0, i8 noundef zeroext %14) #44
  %16 = add i64 %15, %4
  %17 = icmp ugt i64 %16, %1
  br i1 %17, label %91, label %18

18:                                               ; preds = %13, %41
  %19 = phi i8 [ %42, %41 ], [ %14, %13 ]
  %20 = phi i64 [ %45, %41 ], [ %16, %13 ]
  %21 = phi i64 [ %44, %41 ], [ %15, %13 ]
  %22 = getelementptr i8, ptr %0, i64 %21
  %23 = load i8, ptr %22, align 1, !tbaa !2
  %24 = icmp eq i8 %23, %19
  br i1 %24, label %25, label %39

25:                                               ; preds = %18, %29
  %26 = phi i64 [ %27, %29 ], [ 0, %18 ]
  %27 = add nuw i64 %26, 1
  %28 = icmp eq i64 %27, %4
  br i1 %28, label %35, label %29, !llvm.loop !52

29:                                               ; preds = %25
  %30 = getelementptr i8, ptr %22, i64 %27
  %31 = load i8, ptr %30, align 1, !tbaa !2
  %32 = getelementptr inbounds i8, ptr %3, i64 %27
  %33 = load i8, ptr %32, align 1, !tbaa !2
  %34 = icmp eq i8 %31, %33
  br i1 %34, label %25, label %35, !llvm.loop !52

35:                                               ; preds = %29, %25
  %36 = icmp ult i64 %27, %4
  br i1 %36, label %39, label %37

37:                                               ; preds = %35
  tail call void %5(ptr noundef %6, i64 noundef %21) #45, !callees !73
  %38 = load i8, ptr %3, align 1, !tbaa !2
  br label %41

39:                                               ; preds = %18, %35
  %40 = add i64 %21, 1
  br label %41

41:                                               ; preds = %39, %37
  %42 = phi i8 [ %38, %37 ], [ %19, %39 ]
  %43 = phi i64 [ %20, %37 ], [ %40, %39 ]
  %44 = tail call fastcc i64 @next_byte(ptr noundef nonnull %0, i64 noundef %1, i64 noundef %43, i8 noundef zeroext %42) #44
  %45 = add i64 %44, %4
  %46 = icmp ugt i64 %45, %1
  br i1 %46, label %91, label %18, !llvm.loop !74

47:                                               ; preds = %7
  call void @llvm.lifetime.start.p0(i64 32, ptr nonnull %8) #40
  store ptr %0, ptr %8, align 8, !tbaa !55
  %48 = getelementptr inbounds %struct.Boundaries, ptr %8, i64 0, i32 1
  store i64 %1, ptr %48, align 8, !tbaa !57
  %49 = getelementptr inbounds %struct.Boundaries, ptr %8, i64 0, i32 2
  store i64 0, ptr %49, align 8, !tbaa !58
  %50 = getelementptr inbounds %struct.Boundaries, ptr %8, i64 0, i32 3
  store i64 %2, ptr %50, align 8, !tbaa !59
  call void @llvm.lifetime.start.p0(i64 32, ptr nonnull %9) #40
  store ptr %0, ptr %9, align 8, !tbaa !55
  %51 = getelementptr inbounds %struct.Boundaries, ptr %9, i64 0, i32 1
  store i64 %1, ptr %51, align 8, !tbaa !57
  %52 = getelementptr inbounds %struct.Boundaries, ptr %9, i64 0, i32 2
  store i64 0, ptr %52, align 8, !tbaa !58
  %53 = getelementptr inbounds %struct.Boundaries, ptr %9, i64 0, i32 3
  store i64 %2, ptr %53, align 8, !tbaa !59
  %54 = icmp ugt i64 %4, %1
  br i1 %54, label %90, label %55

55:                                               ; preds = %47
  %56 = icmp eq i64 %4, 0
  %57 = tail call i64 @llvm.umax.i64(i64 %4, i64 1)
  br label %58

58:                                               ; preds = %55, %85
  %59 = phi i64 [ %4, %55 ], [ %88, %85 ]
  %60 = phi i64 [ 0, %55 ], [ %87, %85 ]
  %61 = getelementptr i8, ptr %0, i64 %60
  br i1 %56, label %78, label %62

62:                                               ; preds = %58
  %63 = load i8, ptr %61, align 1, !tbaa !2
  %64 = load i8, ptr %3, align 1, !tbaa !2
  %65 = icmp eq i8 %63, %64
  br i1 %65, label %66, label %85

66:                                               ; preds = %62, %70
  %67 = phi i64 [ %68, %70 ], [ 0, %62 ]
  %68 = add nuw i64 %67, 1
  %69 = icmp eq i64 %68, %4
  br i1 %69, label %76, label %70, !llvm.loop !52

70:                                               ; preds = %66
  %71 = getelementptr i8, ptr %61, i64 %68
  %72 = load i8, ptr %71, align 1, !tbaa !2
  %73 = getelementptr inbounds i8, ptr %3, i64 %68
  %74 = load i8, ptr %73, align 1, !tbaa !2
  %75 = icmp eq i8 %72, %74
  br i1 %75, label %66, label %76, !llvm.loop !52

76:                                               ; preds = %70, %66
  %77 = icmp ult i64 %68, %4
  br i1 %77, label %85, label %78

78:                                               ; preds = %58, %76
  %79 = call fastcc i32 @at_boundary(ptr noundef nonnull %8, i64 noundef %60) #44, !range !60
  %80 = icmp eq i32 %79, 0
  br i1 %80, label %85, label %81

81:                                               ; preds = %78
  %82 = call fastcc i32 @at_boundary(ptr noundef nonnull %9, i64 noundef %59) #44, !range !60
  %83 = icmp eq i32 %82, 0
  br i1 %83, label %85, label %84

84:                                               ; preds = %81
  tail call void %5(ptr noundef %6, i64 noundef %60) #45, !callees !73
  br label %85

85:                                               ; preds = %62, %76, %78, %81, %84
  %86 = phi i64 [ %57, %84 ], [ 1, %81 ], [ 1, %78 ], [ 1, %76 ], [ 1, %62 ]
  %87 = add i64 %86, %60
  %88 = add i64 %87, %4
  %89 = icmp ugt i64 %88, %1
  br i1 %89, label %90, label %58, !llvm.loop !75

90:                                               ; preds = %85, %47
  call void @llvm.lifetime.end.p0(i64 32, ptr nonnull %9) #40
  call void @llvm.lifetime.end.p0(i64 32, ptr nonnull %8) #40
  br label %91

91:                                               ; preds = %41, %13, %90
  ret void
}

; Function Attrs: nounwind optsize
define internal void @replace_one(ptr nocapture noundef %0, i64 noundef %1) #2 {
  %3 = getelementptr inbounds %struct.Replacing, ptr %0, i64 0, i32 6
  %4 = load i32, ptr %3, align 4, !tbaa !72
  %5 = icmp eq i32 %4, 0
  br i1 %5, label %6, label %35

6:                                                ; preds = %2
  %7 = load ptr, ptr %0, align 8, !tbaa !65
  %8 = getelementptr inbounds %struct.Replacing, ptr %0, i64 0, i32 1
  %9 = load i64, ptr %8, align 8, !tbaa !67
  %10 = getelementptr inbounds i8, ptr %7, i64 %9
  %11 = sub i64 %1, %9
  tail call fastcc void @reserve(i64 noundef %11) #44
  %12 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %13 = load ptr, ptr %12, align 8, !tbaa !10
  %14 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %15 = load i64, ptr %14, align 8, !tbaa !8
  %16 = getelementptr inbounds i8, ptr %13, i64 %15
  tail call fastcc void @copy_bytes(ptr noundef %16, ptr noundef %10, i64 noundef %11) #44
  %17 = load i64, ptr %14, align 8, !tbaa !8
  %18 = add i64 %17, %11
  store i64 %18, ptr %14, align 8, !tbaa !8
  %19 = getelementptr inbounds %struct.Replacing, ptr %0, i64 0, i32 3
  %20 = load ptr, ptr %19, align 8, !tbaa !69
  %21 = getelementptr inbounds %struct.Replacing, ptr %0, i64 0, i32 4
  %22 = load i64, ptr %21, align 8, !tbaa !70
  tail call fastcc void @reserve(i64 noundef %22) #44
  %23 = load ptr, ptr %12, align 8, !tbaa !10
  %24 = load i64, ptr %14, align 8, !tbaa !8
  %25 = getelementptr inbounds i8, ptr %23, i64 %24
  tail call fastcc void @copy_bytes(ptr noundef %25, ptr noundef %20, i64 noundef %22) #44
  %26 = load i64, ptr %14, align 8, !tbaa !8
  %27 = add i64 %26, %22
  store i64 %27, ptr %14, align 8, !tbaa !8
  %28 = getelementptr inbounds %struct.Replacing, ptr %0, i64 0, i32 2
  %29 = load i64, ptr %28, align 8, !tbaa !68
  %30 = add i64 %29, %1
  store i64 %30, ptr %8, align 8, !tbaa !67
  %31 = getelementptr inbounds %struct.Replacing, ptr %0, i64 0, i32 5
  %32 = load i32, ptr %31, align 8, !tbaa !71
  %33 = icmp eq i32 %32, 0
  %34 = zext i1 %33 to i32
  store i32 %34, ptr %3, align 4, !tbaa !72
  br label %35

35:                                               ; preds = %2, %6
  ret void
}

; Function Attrs: nounwind optsize
define ptr @fa_string_characters(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #2 {
  %4 = tail call fastcc ptr @characters_of(ptr noundef %0, i64 noundef %1, i64 noundef %2) #44
  ret ptr %4
}

; Function Attrs: nounwind optsize
define internal fastcc ptr @characters_of(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2) unnamed_addr #2 {
  %4 = alloca %struct.FaString, align 8
  %5 = icmp sgt i64 %2, -1
  br i1 %5, label %28, label %6

6:                                                ; preds = %3
  %7 = load i32, ptr @fa_compact_characters, align 4, !tbaa !25
  %8 = icmp eq i32 %7, 0
  br i1 %8, label %9, label %24

9:                                                ; preds = %6
  %10 = tail call ptr @fa_array_new(i64 noundef %1, i64 noundef %1, i64 noundef 24) #44
  %11 = load ptr, ptr %10, align 8, !tbaa !16
  %12 = icmp eq i64 %1, 0
  br i1 %12, label %52, label %13

13:                                               ; preds = %9, %13
  %14 = phi i64 [ %22, %13 ], [ 0, %9 ]
  %15 = getelementptr inbounds %struct.FaString, ptr %11, i64 %14
  %16 = getelementptr inbounds i8, ptr %0, i64 %14
  %17 = load i8, ptr %16, align 1, !tbaa !2
  %18 = zext i8 %17 to i64
  %19 = getelementptr inbounds i8, ptr @ascii_table, i64 %18
  store ptr %19, ptr %15, align 8, !tbaa.struct !13
  %20 = getelementptr inbounds i8, ptr %15, i64 8
  store i64 1, ptr %20, align 8, !tbaa.struct !14
  %21 = getelementptr inbounds %struct.FaString, ptr %11, i64 %14, i32 2
  store i64 -9223372036854775808, ptr %21, align 8, !tbaa !22
  %22 = add nuw i64 %14, 1
  %23 = icmp eq i64 %22, %1
  br i1 %23, label %52, label %13, !llvm.loop !76

24:                                               ; preds = %6
  %25 = tail call ptr @fa_array_new(i64 noundef %1, i64 noundef %1, i64 noundef 1) #44
  %26 = load ptr, ptr %25, align 8, !tbaa !16
  tail call fastcc void @copy_bytes(ptr noundef %26, ptr noundef %0, i64 noundef %1) #44
  %27 = getelementptr inbounds %struct.FaArrayHeader, ptr %25, i64 0, i32 2
  store i64 -1, ptr %27, align 8, !tbaa !22
  br label %52

28:                                               ; preds = %3
  %29 = tail call i64 @fa_graphemes(ptr noundef %0, i64 noundef %1) #44
  %30 = tail call ptr @fa_array_new(i64 noundef 0, i64 noundef %29, i64 noundef 24) #44
  %31 = icmp eq i64 %1, 0
  br i1 %31, label %52, label %32

32:                                               ; preds = %28
  %33 = getelementptr inbounds %struct.FaArrayHeader, ptr %30, i64 0, i32 1
  %34 = getelementptr inbounds %struct.FaArrayHeader, ptr %30, i64 0, i32 2
  br label %35

35:                                               ; preds = %32, %43
  %36 = phi i64 [ 0, %32 ], [ %37, %43 ]
  %37 = tail call fastcc i64 @next_boundary(ptr noundef %0, i64 noundef %1, i64 noundef %36) #44
  %38 = load i64, ptr %33, align 8, !tbaa !21
  %39 = load i64, ptr %34, align 8, !tbaa !22
  %40 = icmp slt i64 %38, %39
  br i1 %40, label %43, label %41, !prof !77

41:                                               ; preds = %35
  tail call fastcc void @array_grow(ptr noundef nonnull %30, i64 noundef 24) #44
  %42 = load i64, ptr %33, align 8, !tbaa !21
  br label %43

43:                                               ; preds = %35, %41
  %44 = phi i64 [ %42, %41 ], [ %38, %35 ]
  %45 = load ptr, ptr %30, align 8, !tbaa !16
  %46 = mul nsw i64 %44, 24
  %47 = getelementptr inbounds i8, ptr %45, i64 %46
  %48 = add nsw i64 %44, 1
  store i64 %48, ptr %33, align 8, !tbaa !21
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %4) #40
  %49 = getelementptr inbounds i8, ptr %0, i64 %36
  %50 = sub i64 %37, %36
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 8 %4, ptr noundef %49, i64 noundef %50, i64 noundef 0) #44
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %47, ptr noundef nonnull align 8 dereferenceable(24) %4, i64 24, i1 false), !tbaa.struct !13
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %4) #40
  %51 = icmp ult i64 %37, %1
  br i1 %51, label %35, label %52, !llvm.loop !78

52:                                               ; preds = %13, %43, %9, %28, %24
  %53 = phi ptr [ %25, %24 ], [ %30, %28 ], [ %10, %9 ], [ %30, %43 ], [ %10, %13 ]
  ret ptr %53
}

; Function Attrs: nounwind optsize
define ptr @fa_string_split(ptr noundef %0, i64 noundef %1, i64 noundef %2, ptr nocapture noundef readonly %3, i64 noundef %4) local_unnamed_addr #2 {
  %6 = alloca %struct.Splitting, align 8
  %7 = alloca %struct.FaString, align 8
  %8 = icmp eq i64 %4, 0
  br i1 %8, label %9, label %11

9:                                                ; preds = %5
  %10 = tail call fastcc ptr @characters_of(ptr noundef %0, i64 noundef %1, i64 noundef %2) #44
  br label %40

11:                                               ; preds = %5
  call void @llvm.lifetime.start.p0(i64 40, ptr nonnull %6) #40
  store ptr %0, ptr %6, align 8, !tbaa !79
  %12 = getelementptr inbounds %struct.Splitting, ptr %6, i64 0, i32 1
  store i64 0, ptr %12, align 8, !tbaa !81
  %13 = getelementptr inbounds %struct.Splitting, ptr %6, i64 0, i32 2
  store i64 %4, ptr %13, align 8, !tbaa !82
  %14 = getelementptr inbounds %struct.Splitting, ptr %6, i64 0, i32 3
  %15 = tail call ptr @fa_alloc(i64 noundef 128, i64 noundef 16) #45
  %16 = getelementptr inbounds i8, ptr %15, i64 24
  store i64 1, ptr %16, align 1
  %17 = getelementptr inbounds i8, ptr %15, i64 32
  store ptr %17, ptr %15, align 8, !tbaa !16
  %18 = getelementptr inbounds %struct.FaArrayHeader, ptr %15, i64 0, i32 1
  store i64 0, ptr %18, align 8, !tbaa !21
  %19 = getelementptr inbounds %struct.FaArrayHeader, ptr %15, i64 0, i32 2
  store i64 4, ptr %19, align 8, !tbaa !22
  store ptr %15, ptr %14, align 8, !tbaa !83
  %20 = getelementptr inbounds %struct.Splitting, ptr %6, i64 0, i32 4
  %21 = and i64 %2, -9223372036854775808
  store i64 %21, ptr %20, align 8, !tbaa !84
  call fastcc void @each_match(ptr noundef %0, i64 noundef %1, i64 noundef %21, ptr noundef %3, i64 noundef %4, ptr noundef nonnull @split_one, ptr noundef nonnull %6) #44
  %22 = load ptr, ptr %14, align 8, !tbaa !83
  %23 = getelementptr inbounds %struct.FaArrayHeader, ptr %22, i64 0, i32 1
  %24 = load i64, ptr %23, align 8, !tbaa !21
  %25 = getelementptr inbounds %struct.FaArrayHeader, ptr %22, i64 0, i32 2
  %26 = load i64, ptr %25, align 8, !tbaa !22
  %27 = icmp slt i64 %24, %26
  br i1 %27, label %30, label %28, !prof !77

28:                                               ; preds = %11
  call fastcc void @array_grow(ptr noundef nonnull %22, i64 noundef 24) #44
  %29 = load i64, ptr %23, align 8, !tbaa !21
  br label %30

30:                                               ; preds = %11, %28
  %31 = phi i64 [ %29, %28 ], [ %24, %11 ]
  %32 = load ptr, ptr %22, align 8, !tbaa !16
  %33 = mul nsw i64 %31, 24
  %34 = getelementptr inbounds i8, ptr %32, i64 %33
  %35 = add nsw i64 %31, 1
  store i64 %35, ptr %23, align 8, !tbaa !21
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %7) #40
  %36 = load i64, ptr %12, align 8, !tbaa !81
  %37 = getelementptr inbounds i8, ptr %0, i64 %36
  %38 = sub i64 %1, %36
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 8 %7, ptr noundef %37, i64 noundef %38, i64 noundef %21) #44
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %34, ptr noundef nonnull align 8 dereferenceable(24) %7, i64 24, i1 false), !tbaa.struct !13
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %7) #40
  %39 = load ptr, ptr %14, align 8, !tbaa !83
  call void @llvm.lifetime.end.p0(i64 40, ptr nonnull %6) #40
  br label %40

40:                                               ; preds = %30, %9
  %41 = phi ptr [ %10, %9 ], [ %39, %30 ]
  ret ptr %41
}

; Function Attrs: nounwind optsize
define ptr @fa_array_new(i64 noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #2 {
  %4 = tail call i64 @llvm.smax.i64(i64 %1, i64 %0)
  %5 = mul nsw i64 %4, %2
  %6 = icmp ult i64 %5, 97
  br i1 %6, label %7, label %12

7:                                                ; preds = %3
  %8 = add nuw nsw i64 %5, 32
  %9 = tail call ptr @fa_alloc(i64 noundef %8, i64 noundef 16) #45
  %10 = getelementptr inbounds i8, ptr %9, i64 24
  store i64 1, ptr %10, align 1
  %11 = getelementptr inbounds i8, ptr %9, i64 32
  store ptr %11, ptr %9, align 8, !tbaa !16
  br label %16

12:                                               ; preds = %3
  %13 = tail call ptr @fa_alloc(i64 noundef 32, i64 noundef 8) #45
  %14 = getelementptr inbounds i8, ptr %13, i64 24
  store i64 0, ptr %14, align 1
  %15 = tail call ptr @fa_alloc(i64 noundef %5, i64 noundef 16) #45
  store ptr %15, ptr %13, align 8, !tbaa !16
  br label %16

16:                                               ; preds = %12, %7
  %17 = phi ptr [ %13, %12 ], [ %9, %7 ]
  %18 = getelementptr inbounds %struct.FaArrayHeader, ptr %17, i64 0, i32 1
  store i64 %0, ptr %18, align 8, !tbaa !21
  %19 = getelementptr inbounds %struct.FaArrayHeader, ptr %17, i64 0, i32 2
  store i64 %4, ptr %19, align 8, !tbaa !22
  ret ptr %17
}

; Function Attrs: nounwind optsize
define internal void @split_one(ptr nocapture noundef %0, i64 noundef %1) #2 {
  %3 = alloca %struct.FaString, align 8
  %4 = getelementptr inbounds %struct.Splitting, ptr %0, i64 0, i32 3
  %5 = load ptr, ptr %4, align 8, !tbaa !83
  %6 = getelementptr inbounds %struct.FaArrayHeader, ptr %5, i64 0, i32 1
  %7 = load i64, ptr %6, align 8, !tbaa !21
  %8 = getelementptr inbounds %struct.FaArrayHeader, ptr %5, i64 0, i32 2
  %9 = load i64, ptr %8, align 8, !tbaa !22
  %10 = icmp slt i64 %7, %9
  br i1 %10, label %13, label %11, !prof !77

11:                                               ; preds = %2
  tail call fastcc void @array_grow(ptr noundef nonnull %5, i64 noundef 24) #44
  %12 = load i64, ptr %6, align 8, !tbaa !21
  br label %13

13:                                               ; preds = %2, %11
  %14 = phi i64 [ %12, %11 ], [ %7, %2 ]
  %15 = load ptr, ptr %5, align 8, !tbaa !16
  %16 = mul nsw i64 %14, 24
  %17 = getelementptr inbounds i8, ptr %15, i64 %16
  %18 = add nsw i64 %14, 1
  store i64 %18, ptr %6, align 8, !tbaa !21
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %3) #40
  %19 = load ptr, ptr %0, align 8, !tbaa !79
  %20 = getelementptr inbounds %struct.Splitting, ptr %0, i64 0, i32 1
  %21 = load i64, ptr %20, align 8, !tbaa !81
  %22 = getelementptr inbounds i8, ptr %19, i64 %21
  %23 = sub i64 %1, %21
  %24 = getelementptr inbounds %struct.Splitting, ptr %0, i64 0, i32 4
  %25 = load i64, ptr %24, align 8, !tbaa !84
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 8 %3, ptr noundef %22, i64 noundef %23, i64 noundef %25) #44
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %17, ptr noundef nonnull align 8 dereferenceable(24) %3, i64 24, i1 false), !tbaa.struct !13
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %3) #40
  %26 = getelementptr inbounds %struct.Splitting, ptr %0, i64 0, i32 2
  %27 = load i64, ptr %26, align 8, !tbaa !82
  %28 = add i64 %27, %1
  store i64 %28, ptr %20, align 8, !tbaa !81
  ret void
}

; Function Attrs: nounwind optsize
define ptr @fa_array_push_slot(ptr noundef %0, i64 noundef %1) local_unnamed_addr #2 {
  %3 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %4 = load i64, ptr %3, align 8, !tbaa !21
  %5 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 2
  %6 = load i64, ptr %5, align 8, !tbaa !22
  %7 = icmp slt i64 %4, %6
  br i1 %7, label %10, label %8, !prof !77

8:                                                ; preds = %2
  tail call fastcc void @array_grow(ptr noundef nonnull %0, i64 noundef %1) #44
  %9 = load i64, ptr %3, align 8, !tbaa !21
  br label %10

10:                                               ; preds = %8, %2
  %11 = phi i64 [ %9, %8 ], [ %4, %2 ]
  %12 = load ptr, ptr %0, align 8, !tbaa !16
  %13 = mul nsw i64 %11, %1
  %14 = getelementptr inbounds i8, ptr %12, i64 %13
  %15 = add nsw i64 %11, 1
  store i64 %15, ptr %3, align 8, !tbaa !21
  ret ptr %14
}

; Function Attrs: nounwind optsize
define noundef ptr @fa_string_code_points(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #2 {
  %4 = alloca i64, align 8
  %5 = icmp sgt i64 %2, -1
  br i1 %5, label %9, label %6

6:                                                ; preds = %3
  %7 = tail call ptr @fa_array_new(i64 noundef %1, i64 noundef %1, i64 noundef 4) #44
  %8 = load ptr, ptr %7, align 8, !tbaa !16
  br label %27

9:                                                ; preds = %3
  %10 = icmp eq i64 %1, 0
  br i1 %10, label %22, label %11

11:                                               ; preds = %9, %11
  %12 = phi i64 [ %19, %11 ], [ 0, %9 ]
  %13 = phi i64 [ %20, %11 ], [ 0, %9 ]
  %14 = getelementptr inbounds i8, ptr %0, i64 %13
  %15 = load i8, ptr %14, align 1, !tbaa !2
  %16 = and i8 %15, -64
  %17 = icmp ne i8 %16, -128
  %18 = zext i1 %17 to i64
  %19 = add i64 %12, %18
  %20 = add nuw i64 %13, 1
  %21 = icmp eq i64 %20, %1
  br i1 %21, label %22, label %11, !llvm.loop !85

22:                                               ; preds = %11, %9
  %23 = phi i64 [ 0, %9 ], [ %19, %11 ]
  %24 = tail call ptr @fa_array_new(i64 noundef %23, i64 noundef %23, i64 noundef 4) #44
  %25 = load ptr, ptr %24, align 8, !tbaa !16
  %26 = icmp eq i64 %23, %1
  br i1 %26, label %27, label %39

27:                                               ; preds = %6, %22
  %28 = phi ptr [ %8, %6 ], [ %25, %22 ]
  %29 = phi ptr [ %7, %6 ], [ %24, %22 ]
  %30 = icmp eq i64 %1, 0
  br i1 %30, label %49, label %31

31:                                               ; preds = %27, %31
  %32 = phi i64 [ %37, %31 ], [ 0, %27 ]
  %33 = getelementptr inbounds i8, ptr %0, i64 %32
  %34 = load i8, ptr %33, align 1, !tbaa !2
  %35 = zext i8 %34 to i32
  %36 = getelementptr inbounds i32, ptr %28, i64 %32
  store i32 %35, ptr %36, align 4, !tbaa !25
  %37 = add nuw i64 %32, 1
  %38 = icmp eq i64 %37, %1
  br i1 %38, label %49, label %31, !llvm.loop !86

39:                                               ; preds = %22
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %4) #40
  store i64 0, ptr %4, align 8, !tbaa !8
  %40 = icmp eq i64 %1, 0
  br i1 %40, label %41, label %42

41:                                               ; preds = %42, %39
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %4) #40
  br label %49

42:                                               ; preds = %39, %42
  %43 = phi i64 [ %46, %42 ], [ 0, %39 ]
  %44 = call fastcc i32 @decode(ptr noundef %0, i64 noundef %1, ptr noundef nonnull %4) #44
  %45 = getelementptr inbounds i32, ptr %25, i64 %43
  store i32 %44, ptr %45, align 4, !tbaa !25
  %46 = add i64 %43, 1
  %47 = load i64, ptr %4, align 8, !tbaa !8
  %48 = icmp ult i64 %47, %1
  br i1 %48, label %42, label %41, !llvm.loop !87

49:                                               ; preds = %31, %27, %41
  %50 = phi ptr [ %29, %27 ], [ %24, %41 ], [ %29, %31 ]
  ret ptr %50
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define internal fastcc i32 @decode(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef %2) unnamed_addr #10 {
  %4 = load i64, ptr %2, align 8, !tbaa !8
  %5 = getelementptr inbounds i8, ptr %0, i64 %4
  %6 = load i8, ptr %5, align 1, !tbaa !2
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
  store i64 %25, ptr %2, align 8, !tbaa !8
  ret i32 %24

26:                                               ; preds = %17
  %27 = shl i32 %19, 6
  %28 = getelementptr inbounds i8, ptr %0, i64 %20
  %29 = load i8, ptr %28, align 1, !tbaa !2
  %30 = and i8 %29, 63
  %31 = zext nneg i8 %30 to i32
  %32 = or disjoint i32 %27, %31
  %33 = add nuw nsw i64 %18, 1
  %34 = icmp eq i64 %33, %14
  br i1 %34, label %22, label %17, !llvm.loop !88
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define i64 @fa_string_code_point_at(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2, i64 noundef %3) local_unnamed_addr #10 {
  %5 = alloca i64, align 8
  %6 = icmp sgt i64 %2, -1
  %7 = icmp slt i64 %3, 0
  br i1 %6, label %16, label %8

8:                                                ; preds = %4
  %9 = select i1 %7, i64 %1, i64 0
  %10 = add nsw i64 %9, %3
  %11 = icmp ult i64 %10, %1
  br i1 %11, label %12, label %49

12:                                               ; preds = %8
  %13 = getelementptr inbounds i8, ptr %0, i64 %10
  %14 = load i8, ptr %13, align 1, !tbaa !2
  %15 = zext i8 %14 to i64
  br label %49

16:                                               ; preds = %4
  br i1 %7, label %17, label %34

17:                                               ; preds = %16
  %18 = icmp eq i64 %1, 0
  br i1 %18, label %30, label %19

19:                                               ; preds = %17, %19
  %20 = phi i64 [ %28, %19 ], [ 0, %17 ]
  %21 = phi i64 [ %27, %19 ], [ 0, %17 ]
  %22 = getelementptr inbounds i8, ptr %0, i64 %20
  %23 = load i8, ptr %22, align 1, !tbaa !2
  %24 = and i8 %23, -64
  %25 = icmp ne i8 %24, -128
  %26 = zext i1 %25 to i64
  %27 = add nuw nsw i64 %21, %26
  %28 = add nuw i64 %20, 1
  %29 = icmp eq i64 %28, %1
  br i1 %29, label %30, label %19, !llvm.loop !89

30:                                               ; preds = %19, %17
  %31 = phi i64 [ 0, %17 ], [ %27, %19 ]
  %32 = add nsw i64 %31, %3
  %33 = icmp slt i64 %32, 0
  br i1 %33, label %49, label %34

34:                                               ; preds = %30, %16
  %35 = phi i64 [ %32, %30 ], [ %3, %16 ]
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %5) #40
  store i64 0, ptr %5, align 8, !tbaa !8
  %36 = icmp eq i64 %1, 0
  br i1 %36, label %47, label %41

37:                                               ; preds = %41
  %38 = add nsw i64 %42, -1
  %39 = load i64, ptr %5, align 8, !tbaa !8
  %40 = icmp ult i64 %39, %1
  br i1 %40, label %41, label %47, !llvm.loop !90

41:                                               ; preds = %34, %37
  %42 = phi i64 [ %38, %37 ], [ %35, %34 ]
  %43 = call fastcc i32 @decode(ptr noundef %0, i64 noundef %1, ptr noundef nonnull %5) #44
  %44 = icmp eq i64 %42, 0
  br i1 %44, label %45, label %37

45:                                               ; preds = %41
  %46 = zext i32 %43 to i64
  br label %47

47:                                               ; preds = %37, %34, %45
  %48 = phi i64 [ %46, %45 ], [ -1, %34 ], [ -1, %37 ]
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %5) #40
  br label %49

49:                                               ; preds = %47, %30, %12, %8
  %50 = phi i64 [ %15, %12 ], [ -1, %8 ], [ -1, %30 ], [ %48, %47 ]
  ret i64 %50
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: read)
define i64 @fa_string_code_point_count(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #12 {
  %4 = icmp sgt i64 %2, -1
  br i1 %4, label %5, label %18

5:                                                ; preds = %3
  %6 = icmp eq i64 %1, 0
  br i1 %6, label %18, label %7

7:                                                ; preds = %5, %7
  %8 = phi i64 [ %16, %7 ], [ 0, %5 ]
  %9 = phi i64 [ %15, %7 ], [ 0, %5 ]
  %10 = getelementptr inbounds i8, ptr %0, i64 %8
  %11 = load i8, ptr %10, align 1, !tbaa !2
  %12 = and i8 %11, -64
  %13 = icmp ne i8 %12, -128
  %14 = zext i1 %13 to i64
  %15 = add nuw nsw i64 %9, %14
  %16 = add nuw i64 %8, 1
  %17 = icmp eq i64 %16, %1
  br i1 %17, label %18, label %7, !llvm.loop !89

18:                                               ; preds = %7, %5, %3
  %19 = phi i64 [ %1, %3 ], [ 0, %5 ], [ %15, %7 ]
  ret i64 %19
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define i32 @fa_next_code_point(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef %2) local_unnamed_addr #10 {
  %4 = alloca i64, align 8
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %4) #40
  %5 = load i64, ptr %2, align 8, !tbaa !8
  store i64 %5, ptr %4, align 8, !tbaa !8
  %6 = getelementptr inbounds i8, ptr %0, i64 %5
  %7 = load i8, ptr %6, align 1, !tbaa !2
  %8 = icmp sgt i8 %7, -1
  br i1 %8, label %9, label %12

9:                                                ; preds = %3
  %10 = zext nneg i8 %7 to i32
  %11 = add nsw i64 %5, 1
  br label %15

12:                                               ; preds = %3
  %13 = call fastcc i32 @decode(ptr noundef nonnull %0, i64 noundef %1, ptr noundef nonnull %4) #44
  %14 = load i64, ptr %4, align 8, !tbaa !8
  br label %15

15:                                               ; preds = %12, %9
  %16 = phi i64 [ %14, %12 ], [ %11, %9 ]
  %17 = phi i32 [ %13, %12 ], [ %10, %9 ]
  store i64 %16, ptr %2, align 8, !tbaa !8
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %4) #40
  ret i32 %17
}

; Function Attrs: nounwind optsize
define noundef i32 @fa_string_get(ptr noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3, ptr nocapture noundef writeonly %4) local_unnamed_addr #2 {
  %6 = icmp sgt i64 %2, -1
  br i1 %6, label %19, label %7

7:                                                ; preds = %5
  %8 = icmp slt i64 %3, 0
  %9 = select i1 %8, i64 %1, i64 0
  %10 = add nsw i64 %9, %3
  %11 = icmp ult i64 %10, %1
  br i1 %11, label %12, label %21

12:                                               ; preds = %7
  %13 = getelementptr inbounds i8, ptr %0, i64 %10
  %14 = load i8, ptr %13, align 1, !tbaa !2
  %15 = zext i8 %14 to i64
  %16 = getelementptr inbounds i8, ptr @ascii_table, i64 %15
  store ptr %16, ptr %4, align 8, !tbaa.struct !13
  %17 = getelementptr inbounds i8, ptr %4, i64 8
  store i64 1, ptr %17, align 8, !tbaa.struct !14
  %18 = getelementptr inbounds i8, ptr %4, i64 16
  store i64 -9223372036854775808, ptr %18, align 8, !tbaa !22
  br label %21

19:                                               ; preds = %5
  %20 = tail call fastcc i32 @string_get_slow(ptr noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3, ptr noundef %4) #44
  br label %21

21:                                               ; preds = %7, %19, %12
  %22 = phi i32 [ 1, %12 ], [ %20, %19 ], [ 0, %7 ]
  ret i32 %22
}

; Function Attrs: noinline nounwind optsize
define internal fastcc noundef i32 @string_get_slow(ptr noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3, ptr nocapture noundef writeonly %4) unnamed_addr #14 {
  %6 = alloca %struct.FaString, align 8
  %7 = icmp slt i64 %3, 0
  br i1 %7, label %8, label %12

8:                                                ; preds = %5
  %9 = tail call i64 @fa_graphemes(ptr noundef %0, i64 noundef %1) #44
  %10 = add nsw i64 %9, %3
  %11 = icmp slt i64 %10, 0
  br i1 %11, label %35, label %12

12:                                               ; preds = %5, %8
  %13 = phi i64 [ %10, %8 ], [ %3, %5 ]
  %14 = icmp eq i64 %1, 0
  br i1 %14, label %35, label %18

15:                                               ; preds = %18
  %16 = add nuw nsw i64 %19, 1
  %17 = icmp ult i64 %21, %1
  br i1 %17, label %18, label %35, !llvm.loop !91

18:                                               ; preds = %12, %15
  %19 = phi i64 [ %16, %15 ], [ 0, %12 ]
  %20 = phi i64 [ %21, %15 ], [ 0, %12 ]
  %21 = tail call fastcc i64 @next_boundary(ptr noundef %0, i64 noundef %1, i64 noundef %20) #44
  %22 = icmp eq i64 %19, %13
  br i1 %22, label %23, label %15

23:                                               ; preds = %18
  %24 = and i64 %2, 9223372036854775807
  %25 = icmp eq i64 %24, 0
  %26 = sub i64 %21, %20
  %27 = icmp ugt i64 %26, 1
  %28 = select i1 %25, i1 %27, i1 false
  br i1 %28, label %29, label %33

29:                                               ; preds = %23
  %30 = getelementptr inbounds i8, ptr %0, i64 %20
  store ptr %30, ptr %4, align 8, !tbaa !16
  %31 = getelementptr inbounds %struct.FaString, ptr %4, i64 0, i32 1
  store i64 %26, ptr %31, align 8, !tbaa !21
  %32 = getelementptr inbounds %struct.FaString, ptr %4, i64 0, i32 2
  store i64 0, ptr %32, align 8, !tbaa !22
  br label %35

33:                                               ; preds = %23
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %6) #40
  %34 = getelementptr inbounds i8, ptr %0, i64 %20
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 8 %6, ptr noundef %34, i64 noundef %26, i64 noundef 0) #44
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %4, ptr noundef nonnull align 8 dereferenceable(24) %6, i64 24, i1 false), !tbaa.struct !13
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %6) #40
  br label %35

35:                                               ; preds = %15, %29, %33, %12, %8
  %36 = phi i32 [ 0, %8 ], [ 1, %33 ], [ 1, %29 ], [ 0, %12 ], [ 0, %15 ]
  ret i32 %36
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define i64 @fa_string_length(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #10 {
  %4 = icmp sgt i64 %2, -1
  br i1 %4, label %5, label %7

5:                                                ; preds = %3
  %6 = tail call i64 @fa_graphemes(ptr noundef %0, i64 noundef %1) #44
  br label %7

7:                                                ; preds = %3, %5
  %8 = phi i64 [ %6, %5 ], [ %1, %3 ]
  ret i64 %8
}

; Function Attrs: nounwind optsize
define noundef ptr @fa_string_slice(ptr noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3, i64 noundef %4, i8 noundef signext %5, ptr nocapture noundef writeonly %6) local_unnamed_addr #2 {
  %8 = alloca %struct.FaString, align 8
  %9 = and i64 %2, -9223372036854775808
  %10 = icmp eq i64 %9, 0
  br i1 %10, label %11, label %13

11:                                               ; preds = %7
  %12 = tail call i64 @fa_graphemes(ptr noundef %0, i64 noundef %1) #44
  br label %13

13:                                               ; preds = %7, %11
  %14 = phi i64 [ %12, %11 ], [ %1, %7 ]
  %15 = zext i8 %5 to i32
  %16 = and i32 %15, 1
  %17 = icmp eq i32 %16, 0
  br i1 %17, label %29, label %18

18:                                               ; preds = %13
  %19 = sub nsw i64 0, %14
  %20 = icmp sgt i64 %19, %3
  %21 = icmp slt i64 %14, %3
  %22 = or i1 %20, %21
  br i1 %22, label %23, label %25

23:                                               ; preds = %18
  %24 = tail call ptr @fa_error_index(i64 noundef %3, i64 noundef %14) #44
  br label %86

25:                                               ; preds = %18
  %26 = icmp slt i64 %3, 0
  %27 = select i1 %26, i64 %14, i64 0
  %28 = add nsw i64 %27, %3
  br label %29

29:                                               ; preds = %25, %13
  %30 = phi i64 [ %28, %25 ], [ 0, %13 ]
  %31 = and i32 %15, 2
  %32 = icmp eq i32 %31, 0
  br i1 %32, label %52, label %33

33:                                               ; preds = %29
  %34 = and i32 %15, 4
  %35 = lshr exact i32 %34, 2
  %36 = sub nsw i64 0, %14
  %37 = icmp sgt i64 %36, %4
  br i1 %37, label %50, label %38

38:                                               ; preds = %33
  %39 = shl nuw i32 %34, 29
  %40 = ashr exact i32 %39, 31
  %41 = sext i32 %40 to i64
  %42 = add nsw i64 %14, %41
  %43 = icmp slt i64 %42, %4
  br i1 %43, label %50, label %44

44:                                               ; preds = %38
  %45 = icmp slt i64 %4, 0
  %46 = select i1 %45, i64 %14, i64 0
  %47 = zext nneg i32 %35 to i64
  %48 = add i64 %47, %4
  %49 = add i64 %48, %46
  br label %52

50:                                               ; preds = %33, %38
  %51 = tail call ptr @fa_error_index(i64 noundef %4, i64 noundef %14) #44
  br label %86

52:                                               ; preds = %44, %29
  %53 = phi i64 [ %14, %29 ], [ %49, %44 ]
  %54 = tail call i64 @llvm.smax.i64(i64 %53, i64 %30)
  br i1 %10, label %55, label %72

55:                                               ; preds = %52
  %56 = icmp eq i64 %30, 0
  br i1 %56, label %57, label %60

57:                                               ; preds = %60, %55
  %58 = phi i64 [ 0, %55 ], [ %63, %60 ]
  %59 = icmp ult i64 %30, %54
  br i1 %59, label %66, label %72

60:                                               ; preds = %55, %60
  %61 = phi i64 [ %64, %60 ], [ 0, %55 ]
  %62 = phi i64 [ %63, %60 ], [ 0, %55 ]
  %63 = tail call fastcc i64 @next_boundary(ptr noundef %0, i64 noundef %1, i64 noundef %62) #44
  %64 = add nuw i64 %61, 1
  %65 = icmp eq i64 %64, %30
  br i1 %65, label %57, label %60, !llvm.loop !92

66:                                               ; preds = %57, %66
  %67 = phi i64 [ %70, %66 ], [ %30, %57 ]
  %68 = phi i64 [ %69, %66 ], [ %58, %57 ]
  %69 = tail call fastcc i64 @next_boundary(ptr noundef %0, i64 noundef %1, i64 noundef %68) #44
  %70 = add i64 %67, 1
  %71 = icmp eq i64 %70, %54
  br i1 %71, label %72, label %66, !llvm.loop !93

72:                                               ; preds = %66, %57, %52
  %73 = phi i64 [ %30, %52 ], [ %58, %57 ], [ %58, %66 ]
  %74 = phi i64 [ %54, %52 ], [ %58, %57 ], [ %69, %66 ]
  %75 = and i64 %2, 9223372036854775807
  %76 = icmp eq i64 %75, 0
  %77 = sub i64 %74, %73
  %78 = icmp ugt i64 %77, 1
  %79 = select i1 %76, i1 %78, i1 false
  br i1 %79, label %80, label %84

80:                                               ; preds = %72
  %81 = getelementptr inbounds i8, ptr %0, i64 %73
  store ptr %81, ptr %6, align 8, !tbaa !16
  %82 = getelementptr inbounds %struct.FaString, ptr %6, i64 0, i32 1
  store i64 %77, ptr %82, align 8, !tbaa !21
  %83 = getelementptr inbounds %struct.FaString, ptr %6, i64 0, i32 2
  store i64 %9, ptr %83, align 8, !tbaa !22
  br label %86

84:                                               ; preds = %72
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %8) #40
  %85 = getelementptr inbounds i8, ptr %0, i64 %73
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 8 %8, ptr noundef %85, i64 noundef %77, i64 noundef 0) #44
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %6, ptr noundef nonnull align 8 dereferenceable(24) %8, i64 24, i1 false), !tbaa.struct !13
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %8) #40
  br label %86

86:                                               ; preds = %50, %80, %84, %23
  %87 = phi ptr [ %24, %23 ], [ %51, %50 ], [ null, %84 ], [ null, %80 ]
  ret ptr %87
}

; Function Attrs: cold noinline nounwind optsize
define noundef ptr @fa_error_index(i64 noundef %0, i64 noundef %1) local_unnamed_addr #19 {
  %3 = tail call ptr @fa_alloc(i64 noundef 24, i64 noundef 8) #45
  store i64 1, ptr %3, align 8, !tbaa !94
  %4 = getelementptr inbounds %struct.IndexError, ptr %3, i64 0, i32 1
  store i64 %0, ptr %4, align 8, !tbaa !96
  %5 = getelementptr inbounds %struct.IndexError, ptr %3, i64 0, i32 2
  store i64 %1, ptr %5, align 8, !tbaa !97
  ret ptr %3
}

; Function Attrs: nounwind optsize
define void @fa_string_case(ptr nocapture noundef readonly %0, i64 noundef %1, i32 noundef %2, ptr nocapture noundef writeonly %3) local_unnamed_addr #2 {
  %5 = alloca %struct.FaString, align 8
  %6 = alloca i64, align 8
  %7 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %7, align 8, !tbaa !8
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %6) #40
  store i64 0, ptr %6, align 8, !tbaa !8
  %8 = icmp eq i64 %1, 0
  br i1 %8, label %9, label %11

9:                                                ; preds = %4
  %10 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  br label %150

11:                                               ; preds = %4
  %12 = icmp eq i32 %2, 0
  %13 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  br label %14

14:                                               ; preds = %11, %145
  %15 = call fastcc i32 @decode(ptr noundef %0, i64 noundef %1, ptr noundef nonnull %6) #44
  br i1 %12, label %43, label %16

16:                                               ; preds = %14
  %17 = add i32 %15, -97
  %18 = icmp ult i32 %17, 26
  br i1 %18, label %32, label %19

19:                                               ; preds = %16
  %20 = add i32 %15, -224
  %21 = icmp ult i32 %20, 31
  %22 = icmp ne i32 %15, 247
  %23 = and i1 %22, %21
  br i1 %23, label %32, label %24

24:                                               ; preds = %19
  %25 = add i32 %15, -945
  %26 = icmp ult i32 %25, 25
  %27 = icmp ne i32 %15, 962
  %28 = and i1 %27, %26
  %29 = add i32 %15, -1072
  %30 = icmp ult i32 %29, 32
  %31 = or i1 %30, %28
  br i1 %31, label %32, label %34

32:                                               ; preds = %24, %19, %16
  %33 = add nsw i32 %15, -32
  br label %68

34:                                               ; preds = %24
  %35 = and i32 %15, -16
  %36 = icmp eq i32 %35, 1104
  br i1 %36, label %37, label %39

37:                                               ; preds = %34
  %38 = add nsw i32 %15, -80
  br label %75

39:                                               ; preds = %34
  switch i32 %15, label %68 [
    i32 255, label %75
    i32 181, label %40
    i32 962, label %41
    i32 223, label %42
  ]

40:                                               ; preds = %39
  br label %75

41:                                               ; preds = %39
  br label %75

42:                                               ; preds = %39
  tail call fastcc void @append_c(ptr noundef nonnull @.str.12) #44
  br label %145, !llvm.loop !98

43:                                               ; preds = %14
  %44 = add i32 %15, -65
  %45 = icmp ult i32 %44, 26
  br i1 %45, label %59, label %46

46:                                               ; preds = %43
  %47 = add i32 %15, -192
  %48 = icmp ult i32 %47, 31
  %49 = icmp ne i32 %15, 215
  %50 = and i1 %49, %48
  br i1 %50, label %59, label %51

51:                                               ; preds = %46
  %52 = add i32 %15, -913
  %53 = icmp ult i32 %52, 25
  %54 = icmp ne i32 %15, 930
  %55 = and i1 %54, %53
  %56 = add i32 %15, -1040
  %57 = icmp ult i32 %56, 32
  %58 = or i1 %57, %55
  br i1 %58, label %59, label %61

59:                                               ; preds = %51, %46, %43
  %60 = add nuw nsw i32 %15, 32
  br label %68

61:                                               ; preds = %51
  %62 = and i32 %15, -16
  %63 = icmp eq i32 %62, 1024
  br i1 %63, label %64, label %66

64:                                               ; preds = %61
  %65 = or disjoint i32 %15, 80
  br label %75

66:                                               ; preds = %61
  %67 = icmp eq i32 %15, 376
  br i1 %67, label %75, label %68

68:                                               ; preds = %39, %66, %59, %32
  %69 = phi i32 [ %33, %32 ], [ %60, %59 ], [ %15, %66 ], [ %15, %39 ]
  %70 = icmp ult i32 %69, 128
  br i1 %70, label %71, label %73

71:                                               ; preds = %68
  %72 = trunc i32 %69 to i8
  br label %138

73:                                               ; preds = %68
  %74 = icmp ult i32 %69, 2048
  br i1 %74, label %75, label %88

75:                                               ; preds = %37, %40, %41, %64, %39, %66, %73
  %76 = phi i32 [ %69, %73 ], [ 376, %39 ], [ %65, %64 ], [ 931, %41 ], [ 924, %40 ], [ %38, %37 ], [ 255, %66 ]
  %77 = lshr i32 %76, 6
  %78 = trunc i32 %77 to i8
  %79 = or disjoint i8 %78, -64
  tail call fastcc void @reserve(i64 noundef 1) #44
  %80 = load ptr, ptr %13, align 8, !tbaa !10
  %81 = load i64, ptr %7, align 8, !tbaa !8
  %82 = getelementptr inbounds i8, ptr %80, i64 %81
  store i8 %79, ptr %82, align 1, !tbaa !2
  %83 = load i64, ptr %7, align 8, !tbaa !8
  %84 = add i64 %83, 1
  store i64 %84, ptr %7, align 8, !tbaa !8
  %85 = trunc i32 %76 to i8
  %86 = and i8 %85, 63
  %87 = or disjoint i8 %86, -128
  br label %138

88:                                               ; preds = %73
  %89 = icmp ult i32 %69, 65536
  tail call fastcc void @reserve(i64 noundef 1) #44
  %90 = load ptr, ptr %13, align 8, !tbaa !10
  %91 = load i64, ptr %7, align 8, !tbaa !8
  %92 = getelementptr inbounds i8, ptr %90, i64 %91
  br i1 %89, label %93, label %111

93:                                               ; preds = %88
  %94 = lshr i32 %69, 12
  %95 = trunc i32 %94 to i8
  %96 = or disjoint i8 %95, -32
  store i8 %96, ptr %92, align 1, !tbaa !2
  %97 = load i64, ptr %7, align 8, !tbaa !8
  %98 = add i64 %97, 1
  store i64 %98, ptr %7, align 8, !tbaa !8
  %99 = lshr i32 %69, 6
  %100 = trunc i32 %99 to i8
  %101 = and i8 %100, 63
  %102 = or disjoint i8 %101, -128
  tail call fastcc void @reserve(i64 noundef 1) #44
  %103 = load ptr, ptr %13, align 8, !tbaa !10
  %104 = load i64, ptr %7, align 8, !tbaa !8
  %105 = getelementptr inbounds i8, ptr %103, i64 %104
  store i8 %102, ptr %105, align 1, !tbaa !2
  %106 = load i64, ptr %7, align 8, !tbaa !8
  %107 = add i64 %106, 1
  store i64 %107, ptr %7, align 8, !tbaa !8
  %108 = trunc i32 %69 to i8
  %109 = and i8 %108, 63
  %110 = or disjoint i8 %109, -128
  br label %138

111:                                              ; preds = %88
  %112 = lshr i32 %69, 18
  %113 = trunc i32 %112 to i8
  %114 = or i8 %113, -16
  store i8 %114, ptr %92, align 1, !tbaa !2
  %115 = load i64, ptr %7, align 8, !tbaa !8
  %116 = add i64 %115, 1
  store i64 %116, ptr %7, align 8, !tbaa !8
  %117 = lshr i32 %69, 12
  %118 = trunc i32 %117 to i8
  %119 = and i8 %118, 63
  %120 = or disjoint i8 %119, -128
  tail call fastcc void @reserve(i64 noundef 1) #44
  %121 = load ptr, ptr %13, align 8, !tbaa !10
  %122 = load i64, ptr %7, align 8, !tbaa !8
  %123 = getelementptr inbounds i8, ptr %121, i64 %122
  store i8 %120, ptr %123, align 1, !tbaa !2
  %124 = load i64, ptr %7, align 8, !tbaa !8
  %125 = add i64 %124, 1
  store i64 %125, ptr %7, align 8, !tbaa !8
  %126 = lshr i32 %69, 6
  %127 = trunc i32 %126 to i8
  %128 = and i8 %127, 63
  %129 = or disjoint i8 %128, -128
  tail call fastcc void @reserve(i64 noundef 1) #44
  %130 = load ptr, ptr %13, align 8, !tbaa !10
  %131 = load i64, ptr %7, align 8, !tbaa !8
  %132 = getelementptr inbounds i8, ptr %130, i64 %131
  store i8 %129, ptr %132, align 1, !tbaa !2
  %133 = load i64, ptr %7, align 8, !tbaa !8
  %134 = add i64 %133, 1
  store i64 %134, ptr %7, align 8, !tbaa !8
  %135 = trunc i32 %69 to i8
  %136 = and i8 %135, 63
  %137 = or disjoint i8 %136, -128
  br label %138

138:                                              ; preds = %71, %75, %93, %111
  %139 = phi i8 [ %87, %75 ], [ %137, %111 ], [ %110, %93 ], [ %72, %71 ]
  tail call fastcc void @reserve(i64 noundef 1) #44
  %140 = load ptr, ptr %13, align 8, !tbaa !10
  %141 = load i64, ptr %7, align 8, !tbaa !8
  %142 = getelementptr inbounds i8, ptr %140, i64 %141
  store i8 %139, ptr %142, align 1, !tbaa !2
  %143 = load i64, ptr %7, align 8, !tbaa !8
  %144 = add i64 %143, 1
  store i64 %144, ptr %7, align 8, !tbaa !8
  br label %145

145:                                              ; preds = %138, %42
  %146 = load i64, ptr %6, align 8, !tbaa !8
  %147 = icmp ult i64 %146, %1
  br i1 %147, label %14, label %148

148:                                              ; preds = %145
  %149 = load i64, ptr %7, align 8, !tbaa !8
  br label %150

150:                                              ; preds = %9, %148
  %151 = phi ptr [ %10, %9 ], [ %13, %148 ]
  %152 = phi i64 [ 0, %9 ], [ %149, %148 ]
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %5) #40
  %153 = load ptr, ptr %151, align 8, !tbaa !10
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 8 %5, ptr noundef %153, i64 noundef %152, i64 noundef 0) #44
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %3, ptr noundef nonnull align 8 dereferenceable(24) %5, i64 24, i1 false), !tbaa.struct !13
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %5) #40
  store i64 0, ptr %7, align 8, !tbaa !8
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %6) #40
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_string_trim(ptr nocapture noundef readonly %0, i64 noundef %1, i32 noundef %2, ptr nocapture noundef writeonly %3) local_unnamed_addr #2 {
  %5 = alloca i64, align 8
  %6 = alloca i64, align 8
  %7 = alloca %struct.FaString, align 8
  %8 = and i32 %2, 1
  %9 = icmp eq i32 %8, 0
  br i1 %9, label %18, label %10

10:                                               ; preds = %4, %13
  %11 = phi i64 [ %17, %13 ], [ 0, %4 ]
  %12 = icmp ult i64 %11, %1
  br i1 %12, label %13, label %18

13:                                               ; preds = %10
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %5) #40
  store i64 %11, ptr %5, align 8, !tbaa !8
  %14 = call fastcc i32 @decode(ptr noundef %0, i64 noundef %1, ptr noundef nonnull %5) #44
  %15 = tail call fastcc i32 @is_space(i32 noundef %14) #44, !range !60
  %16 = icmp eq i32 %15, 0
  %17 = load i64, ptr %5, align 8
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %5) #40
  br i1 %16, label %18, label %10

18:                                               ; preds = %13, %10, %4
  %19 = phi i64 [ 0, %4 ], [ %11, %10 ], [ %11, %13 ]
  %20 = and i32 %2, 2
  %21 = icmp eq i32 %20, 0
  br i1 %21, label %39, label %22

22:                                               ; preds = %18, %34
  %23 = phi i64 [ %35, %34 ], [ %1, %18 ]
  %24 = icmp ugt i64 %23, %19
  br i1 %24, label %25, label %39

25:                                               ; preds = %22, %29
  %26 = phi i64 [ %27, %29 ], [ %23, %22 ]
  %27 = add i64 %26, -1
  %28 = icmp ugt i64 %27, %19
  br i1 %28, label %29, label %34

29:                                               ; preds = %25
  %30 = getelementptr inbounds i8, ptr %0, i64 %27
  %31 = load i8, ptr %30, align 1, !tbaa !2
  %32 = and i8 %31, -64
  %33 = icmp eq i8 %32, -128
  br i1 %33, label %25, label %34, !llvm.loop !99

34:                                               ; preds = %25, %29
  %35 = phi i64 [ %19, %25 ], [ %27, %29 ]
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %6) #40
  store i64 %35, ptr %6, align 8, !tbaa !8
  %36 = call fastcc i32 @decode(ptr noundef %0, i64 noundef %23, ptr noundef nonnull %6) #44
  %37 = tail call fastcc i32 @is_space(i32 noundef %36) #44, !range !60
  %38 = icmp eq i32 %37, 0
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %6) #40
  br i1 %38, label %39, label %22

39:                                               ; preds = %34, %22, %18
  %40 = phi i64 [ %1, %18 ], [ %23, %22 ], [ %23, %34 ]
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %7) #40
  %41 = getelementptr inbounds i8, ptr %0, i64 %19
  %42 = sub i64 %40, %19
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 8 %7, ptr noundef %41, i64 noundef %42, i64 noundef 0) #44
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %3, ptr noundef nonnull align 8 dereferenceable(24) %7, i64 24, i1 false), !tbaa.struct !13
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %7) #40
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(none)
define internal fastcc noundef i32 @is_space(i32 noundef %0) unnamed_addr #20 {
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
define void @fa_string_repeat(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2, ptr nocapture noundef writeonly %3) local_unnamed_addr #2 {
  %5 = alloca %struct.FaString, align 8
  %6 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %6, align 8, !tbaa !8
  %7 = icmp sgt i64 %2, 0
  %8 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  br i1 %7, label %12, label %9

9:                                                ; preds = %12, %4
  %10 = phi i64 [ 0, %4 ], [ %18, %12 ]
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %5) #40
  %11 = load ptr, ptr %8, align 8, !tbaa !10
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 8 %5, ptr noundef %11, i64 noundef %10, i64 noundef 0) #44
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %3, ptr noundef nonnull align 8 dereferenceable(24) %5, i64 24, i1 false), !tbaa.struct !13
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %5) #40
  store i64 0, ptr %6, align 8, !tbaa !8
  ret void

12:                                               ; preds = %4, %12
  %13 = phi i64 [ %19, %12 ], [ 0, %4 ]
  tail call fastcc void @reserve(i64 noundef %1) #44
  %14 = load ptr, ptr %8, align 8, !tbaa !10
  %15 = load i64, ptr %6, align 8, !tbaa !8
  %16 = getelementptr inbounds i8, ptr %14, i64 %15
  tail call fastcc void @copy_bytes(ptr noundef %16, ptr noundef %0, i64 noundef %1) #44
  %17 = load i64, ptr %6, align 8, !tbaa !8
  %18 = add i64 %17, %1
  store i64 %18, ptr %6, align 8, !tbaa !8
  %19 = add nuw nsw i64 %13, 1
  %20 = icmp eq i64 %19, %2
  br i1 %20, label %9, label %12, !llvm.loop !100
}

; Function Attrs: nounwind optsize
define void @fa_array_join(ptr noundef %0, ptr nocapture noundef readonly %1, i64 noundef %2, ptr nocapture noundef writeonly %3) local_unnamed_addr #2 {
  %5 = alloca %struct.FaString, align 8
  %6 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 2
  %7 = load i64, ptr %6, align 8, !tbaa !22
  %8 = icmp slt i64 %7, 0
  br i1 %8, label %9, label %12, !prof !47

9:                                                ; preds = %4
  %10 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %11 = load i64, ptr %10, align 8, !tbaa !21
  tail call fastcc void @materialize_characters(ptr noundef nonnull %0, i64 noundef %11) #44
  br label %12

12:                                               ; preds = %4, %9
  %13 = load ptr, ptr %0, align 8, !tbaa !16
  %14 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %15 = load i64, ptr %14, align 8, !tbaa !21
  %16 = icmp ne i64 %2, 0
  %17 = icmp ugt i64 %15, 1
  %18 = and i1 %16, %17
  br i1 %18, label %19, label %24

19:                                               ; preds = %12
  %20 = add i64 %15, -1
  %21 = udiv i64 4611686018427387903, %2
  %22 = icmp ult i64 %20, %21
  br i1 %22, label %27, label %23

23:                                               ; preds = %19
  tail call fastcc void @out_of_memory() #47
  unreachable

24:                                               ; preds = %12
  br i1 %17, label %25, label %31

25:                                               ; preds = %24
  %26 = add i64 %15, -1
  br label %27

27:                                               ; preds = %25, %19
  %28 = phi i64 [ %26, %25 ], [ %20, %19 ]
  %29 = mul i64 %28, %2
  %30 = tail call fastcc i64 @simple_flag(ptr noundef %1, i64 noundef %2) #44, !range !23
  br label %33

31:                                               ; preds = %24
  %32 = icmp eq i64 %15, 0
  br i1 %32, label %36, label %33

33:                                               ; preds = %27, %31
  %34 = phi i64 [ 0, %31 ], [ %29, %27 ]
  %35 = phi i64 [ -9223372036854775808, %31 ], [ %30, %27 ]
  br label %39

36:                                               ; preds = %31
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %5) #40
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %5, i8 0, i64 24, i1 false)
  br label %102

37:                                               ; preds = %58
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %5) #40
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %5, i8 0, i64 24, i1 false)
  %38 = icmp ult i64 %59, 24
  br i1 %38, label %67, label %65

39:                                               ; preds = %33, %58
  %40 = phi i64 [ %59, %58 ], [ %34, %33 ]
  %41 = phi i64 [ %62, %58 ], [ %35, %33 ]
  %42 = phi i64 [ %63, %58 ], [ 0, %33 ]
  %43 = getelementptr inbounds %struct.FaString, ptr %13, i64 %42, i32 2
  %44 = load i64, ptr %43, align 8, !tbaa !22
  %45 = and i64 %44, 4611686018427387904
  %46 = icmp eq i64 %45, 0
  br i1 %46, label %50, label %47

47:                                               ; preds = %39
  %48 = lshr i64 %44, 56
  %49 = and i64 %48, 31
  br label %53

50:                                               ; preds = %39
  %51 = getelementptr inbounds %struct.FaString, ptr %13, i64 %42, i32 1
  %52 = load i64, ptr %51, align 8, !tbaa !21
  br label %53

53:                                               ; preds = %47, %50
  %54 = phi i64 [ %49, %47 ], [ %52, %50 ]
  %55 = sub i64 4611686018427387903, %40
  %56 = icmp ult i64 %54, %55
  br i1 %56, label %58, label %57

57:                                               ; preds = %53
  tail call fastcc void @out_of_memory() #47
  unreachable

58:                                               ; preds = %53
  %59 = add i64 %54, %40
  %60 = icmp eq i64 %54, 0
  %61 = select i1 %60, i64 -1, i64 %44
  %62 = and i64 %41, %61
  %63 = add nuw i64 %42, 1
  %64 = icmp eq i64 %63, %15
  br i1 %64, label %37, label %39, !llvm.loop !44

65:                                               ; preds = %37
  %66 = tail call ptr @fa_alloc(i64 noundef %59, i64 noundef 1) #45
  br label %67

67:                                               ; preds = %37, %65
  %68 = phi ptr [ %66, %65 ], [ %5, %37 ]
  br label %71

69:                                               ; preds = %92
  %70 = icmp eq i64 %62, 0
  br i1 %70, label %99, label %101

71:                                               ; preds = %67, %92
  %72 = phi i64 [ %97, %92 ], [ 0, %67 ]
  %73 = phi i64 [ %96, %92 ], [ 0, %67 ]
  %74 = icmp eq i64 %72, 0
  br i1 %74, label %78, label %75

75:                                               ; preds = %71
  %76 = getelementptr inbounds i8, ptr %68, i64 %73
  call fastcc void @copy_bytes(ptr noundef %76, ptr noundef %1, i64 noundef %2) #44
  %77 = add i64 %73, %2
  br label %78

78:                                               ; preds = %75, %71
  %79 = phi i64 [ %77, %75 ], [ %73, %71 ]
  %80 = getelementptr inbounds %struct.FaString, ptr %13, i64 %72
  %81 = getelementptr inbounds %struct.FaString, ptr %13, i64 %72, i32 2
  %82 = load i64, ptr %81, align 8, !tbaa !22
  %83 = and i64 %82, 4611686018427387904
  %84 = icmp eq i64 %83, 0
  br i1 %84, label %88, label %85

85:                                               ; preds = %78
  %86 = lshr i64 %82, 56
  %87 = and i64 %86, 31
  br label %92

88:                                               ; preds = %78
  %89 = getelementptr inbounds %struct.FaString, ptr %13, i64 %72, i32 1
  %90 = load i64, ptr %89, align 8, !tbaa !21
  %91 = load ptr, ptr %80, align 8, !tbaa !16
  br label %92

92:                                               ; preds = %85, %88
  %93 = phi i64 [ %90, %88 ], [ %87, %85 ]
  %94 = phi ptr [ %91, %88 ], [ %80, %85 ]
  %95 = getelementptr inbounds i8, ptr %68, i64 %79
  call fastcc void @copy_bytes(ptr noundef %95, ptr noundef %94, i64 noundef %93) #44
  %96 = add i64 %93, %79
  %97 = add nuw i64 %72, 1
  %98 = icmp eq i64 %97, %15
  br i1 %98, label %69, label %71, !llvm.loop !45

99:                                               ; preds = %69
  %100 = call fastcc i64 @simple_flag(ptr noundef %68, i64 noundef %59) #44, !range !23
  br i1 %38, label %102, label %110

101:                                              ; preds = %69
  br i1 %38, label %102, label %110

102:                                              ; preds = %99, %36, %101
  %103 = phi i64 [ -9223372036854775808, %36 ], [ -9223372036854775808, %101 ], [ %100, %99 ]
  %104 = phi i64 [ 0, %36 ], [ %59, %101 ], [ %59, %99 ]
  %105 = icmp eq i64 %103, 0
  %106 = select i1 %105, i64 64, i64 192
  %107 = or disjoint i64 %106, %104
  %108 = trunc i64 %107 to i8
  %109 = getelementptr inbounds i8, ptr %5, i64 23
  store i8 %108, ptr %109, align 1, !tbaa !2
  br label %115

110:                                              ; preds = %99, %101
  %111 = phi i64 [ %100, %99 ], [ %62, %101 ]
  store ptr %68, ptr %5, align 8, !tbaa !16
  %112 = getelementptr inbounds %struct.FaString, ptr %5, i64 0, i32 1
  store i64 %59, ptr %112, align 8, !tbaa !21
  %113 = or i64 %111, %59
  %114 = getelementptr inbounds %struct.FaString, ptr %5, i64 0, i32 2
  store i64 %113, ptr %114, align 8, !tbaa !22
  br label %115

115:                                              ; preds = %102, %110
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %3, ptr noundef nonnull align 8 dereferenceable(24) %5, i64 24, i1 false), !tbaa.struct !13
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %5) #40
  ret void
}

; Function Attrs: alwaysinline nounwind optsize
define void @fa_array_materialize(ptr noundef %0) local_unnamed_addr #13 {
  %2 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 2
  %3 = load i64, ptr %2, align 8, !tbaa !22
  %4 = icmp slt i64 %3, 0
  br i1 %4, label %5, label %8, !prof !47

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %7 = load i64, ptr %6, align 8, !tbaa !21
  tail call fastcc void @materialize_characters(ptr noundef nonnull %0, i64 noundef %7) #44
  br label %8

8:                                                ; preds = %5, %1
  ret void
}

; Function Attrs: alwaysinline nounwind optsize
define void @fa_string_pad(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2, i64 noundef %3, ptr nocapture noundef readonly %4, i64 noundef %5, i32 noundef %6, ptr nocapture noundef writeonly %7) local_unnamed_addr #13 {
  %9 = alloca %struct.FaString, align 8
  %10 = alloca %struct.FaString, align 8
  %11 = alloca %struct.FaString, align 8
  %12 = icmp slt i64 %3, 1
  br i1 %12, label %13, label %15

13:                                               ; preds = %8
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %9) #40
  %14 = and i64 %2, -9223372036854775808
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 8 %9, ptr noundef %0, i64 noundef %1, i64 noundef %14) #44
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %7, ptr noundef nonnull align 8 dereferenceable(24) %9, i64 24, i1 false), !tbaa.struct !13
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %9) #40
  br label %55

15:                                               ; preds = %8
  %16 = icmp sgt i64 %2, -1
  %17 = icmp ne i64 %5, 1
  %18 = or i1 %16, %17
  br i1 %18, label %24, label %19

19:                                               ; preds = %15
  %20 = load i8, ptr %4, align 1, !tbaa !2
  %21 = icmp slt i8 %20, 0
  %22 = icmp eq i8 %20, 13
  %23 = or i1 %21, %22
  br i1 %23, label %24, label %25

24:                                               ; preds = %19, %15
  tail call fastcc void @string_pad_slow(ptr noundef %0, i64 noundef %1, i64 noundef %3, ptr noundef %4, i64 noundef %5, i32 noundef %6, ptr noundef %7) #44
  br label %55

25:                                               ; preds = %19
  %26 = icmp ugt i64 %3, %1
  br i1 %26, label %28, label %27

27:                                               ; preds = %25
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %10) #40
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 8 %10, ptr noundef %0, i64 noundef %1, i64 noundef -9223372036854775808) #44
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %7, ptr noundef nonnull align 8 dereferenceable(24) %10, i64 24, i1 false), !tbaa.struct !13
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %10) #40
  br label %55

28:                                               ; preds = %25
  %29 = icmp ugt i64 %3, 4611686018427387902
  br i1 %29, label %30, label %31

30:                                               ; preds = %28
  tail call fastcc void @out_of_memory() #47
  unreachable

31:                                               ; preds = %28
  %32 = sub nsw i64 %3, %1
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %11) #40
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %11, i8 0, i64 24, i1 false)
  %33 = icmp ult i64 %3, 24
  br i1 %33, label %36, label %34

34:                                               ; preds = %31
  %35 = tail call ptr @fa_alloc(i64 noundef %3, i64 noundef 1) #45
  br label %36

36:                                               ; preds = %31, %34
  %37 = phi ptr [ %35, %34 ], [ %11, %31 ]
  %38 = icmp eq i32 %6, 0
  br i1 %38, label %42, label %39

39:                                               ; preds = %36
  %40 = load i8, ptr %4, align 1, !tbaa !2
  call void @llvm.memset.p0.i64(ptr align 1 %37, i8 %40, i64 %32, i1 false)
  %41 = getelementptr inbounds i8, ptr %37, i64 %32
  call fastcc void @copy_bytes(ptr noundef %41, ptr noundef %0, i64 noundef %1) #44
  br label %45

42:                                               ; preds = %36
  call fastcc void @copy_bytes(ptr noundef %37, ptr noundef %0, i64 noundef %1) #44
  %43 = getelementptr inbounds i8, ptr %37, i64 %1
  %44 = load i8, ptr %4, align 1, !tbaa !2
  call void @llvm.memset.p0.i64(ptr align 1 %43, i8 %44, i64 %32, i1 false)
  br label %45

45:                                               ; preds = %42, %39
  br i1 %33, label %46, label %50

46:                                               ; preds = %45
  %47 = trunc i64 %3 to i8
  %48 = or disjoint i8 %47, -64
  %49 = getelementptr inbounds i8, ptr %11, i64 23
  store i8 %48, ptr %49, align 1, !tbaa !2
  br label %54

50:                                               ; preds = %45
  store ptr %37, ptr %11, align 8, !tbaa !16
  %51 = getelementptr inbounds %struct.FaString, ptr %11, i64 0, i32 1
  store i64 %3, ptr %51, align 8, !tbaa !21
  %52 = or disjoint i64 %3, -9223372036854775808
  %53 = getelementptr inbounds %struct.FaString, ptr %11, i64 0, i32 2
  store i64 %52, ptr %53, align 8, !tbaa !22
  br label %54

54:                                               ; preds = %50, %46
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %7, ptr noundef nonnull align 8 dereferenceable(24) %11, i64 24, i1 false), !tbaa.struct !13
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %11) #40
  br label %55

55:                                               ; preds = %54, %27, %24, %13
  ret void
}

; Function Attrs: noinline nounwind optsize
define internal fastcc void @string_pad_slow(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2, ptr nocapture noundef readonly %3, i64 noundef %4, i32 noundef %5, ptr nocapture noundef writeonly %6) unnamed_addr #14 {
  %8 = alloca %struct.FaString, align 8
  %9 = tail call i64 @fa_graphemes(ptr noundef %0, i64 noundef %1) #44
  %10 = sub nsw i64 %2, %9
  %11 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %11, align 8, !tbaa !8
  %12 = icmp eq i32 %5, 0
  br i1 %12, label %13, label %20

13:                                               ; preds = %7
  tail call fastcc void @reserve(i64 noundef %1) #44
  %14 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %15 = load ptr, ptr %14, align 8, !tbaa !10
  %16 = load i64, ptr %11, align 8, !tbaa !8
  %17 = getelementptr inbounds i8, ptr %15, i64 %16
  tail call fastcc void @copy_bytes(ptr noundef %17, ptr noundef %0, i64 noundef %1) #44
  %18 = load i64, ptr %11, align 8, !tbaa !8
  %19 = add i64 %18, %1
  store i64 %19, ptr %11, align 8, !tbaa !8
  br label %20

20:                                               ; preds = %13, %7
  %21 = phi i64 [ %19, %13 ], [ 0, %7 ]
  %22 = icmp ne i64 %4, 0
  %23 = icmp sgt i64 %10, 0
  %24 = and i1 %22, %23
  br i1 %24, label %25, label %27

25:                                               ; preds = %20
  %26 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  br label %31

27:                                               ; preds = %31, %20
  %28 = phi i64 [ %21, %20 ], [ %41, %31 ]
  br i1 %12, label %29, label %46

29:                                               ; preds = %27
  %30 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  br label %53

31:                                               ; preds = %25, %31
  %32 = phi i64 [ 0, %25 ], [ %43, %31 ]
  %33 = phi i64 [ 0, %25 ], [ %44, %31 ]
  %34 = tail call fastcc i64 @next_boundary(ptr noundef %3, i64 noundef %4, i64 noundef %32) #44
  %35 = getelementptr inbounds i8, ptr %3, i64 %32
  %36 = sub i64 %34, %32
  tail call fastcc void @reserve(i64 noundef %36) #44
  %37 = load ptr, ptr %26, align 8, !tbaa !10
  %38 = load i64, ptr %11, align 8, !tbaa !8
  %39 = getelementptr inbounds i8, ptr %37, i64 %38
  tail call fastcc void @copy_bytes(ptr noundef %39, ptr noundef %35, i64 noundef %36) #44
  %40 = load i64, ptr %11, align 8, !tbaa !8
  %41 = add i64 %40, %36
  store i64 %41, ptr %11, align 8, !tbaa !8
  %42 = icmp eq i64 %34, %4
  %43 = select i1 %42, i64 0, i64 %34
  %44 = add nuw nsw i64 %33, 1
  %45 = icmp slt i64 %44, %10
  br i1 %45, label %31, label %27, !llvm.loop !101

46:                                               ; preds = %27
  tail call fastcc void @reserve(i64 noundef %1) #44
  %47 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %48 = load ptr, ptr %47, align 8, !tbaa !10
  %49 = load i64, ptr %11, align 8, !tbaa !8
  %50 = getelementptr inbounds i8, ptr %48, i64 %49
  tail call fastcc void @copy_bytes(ptr noundef %50, ptr noundef %0, i64 noundef %1) #44
  %51 = load i64, ptr %11, align 8, !tbaa !8
  %52 = add i64 %51, %1
  store i64 %52, ptr %11, align 8, !tbaa !8
  br label %53

53:                                               ; preds = %29, %46
  %54 = phi ptr [ %30, %29 ], [ %47, %46 ]
  %55 = phi i64 [ %28, %29 ], [ %52, %46 ]
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %8) #40
  %56 = load ptr, ptr %54, align 8, !tbaa !10
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 8 %8, ptr noundef %56, i64 noundef %55, i64 noundef 0) #44
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %6, ptr noundef nonnull align 8 dereferenceable(24) %8, i64 24, i1 false), !tbaa.struct !13
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %8) #40
  store i64 0, ptr %11, align 8, !tbaa !8
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_arguments_init(i32 noundef %0, ptr nocapture noundef readonly %1) local_unnamed_addr #2 {
  %3 = icmp sgt i32 %0, 1
  %4 = zext nneg i32 %0 to i64
  %5 = add nsw i64 %4, -1
  %6 = select i1 %3, i64 %5, i64 0
  store i64 %6, ptr @argument_count, align 8, !tbaa !8
  %7 = mul nsw i64 %6, 24
  %8 = add nsw i64 %7, 24
  %9 = tail call ptr @fa_alloc(i64 noundef %8, i64 noundef 8) #45
  store ptr %9, ptr @argument_items, align 8, !tbaa !10
  %10 = load i64, ptr @argument_count, align 8, !tbaa !8
  %11 = icmp eq i64 %10, 0
  br i1 %11, label %12, label %13

12:                                               ; preds = %25, %2
  ret void

13:                                               ; preds = %2, %25
  %14 = phi i64 [ %15, %25 ], [ 0, %2 ]
  %15 = add nuw i64 %14, 1
  %16 = getelementptr inbounds ptr, ptr %1, i64 %15
  %17 = load ptr, ptr %16, align 8, !tbaa !10
  %18 = getelementptr inbounds %struct.FaString, ptr %9, i64 %14
  store ptr %17, ptr %18, align 8, !tbaa !16
  br label %19

19:                                               ; preds = %19, %13
  %20 = phi i64 [ 0, %13 ], [ %24, %19 ]
  %21 = getelementptr inbounds i8, ptr %17, i64 %20
  %22 = load i8, ptr %21, align 1, !tbaa !2
  %23 = icmp eq i8 %22, 0
  %24 = add i64 %20, 1
  br i1 %23, label %25, label %19, !llvm.loop !29

25:                                               ; preds = %19
  %26 = getelementptr inbounds %struct.FaString, ptr %9, i64 %14, i32 1
  store i64 %20, ptr %26, align 8, !tbaa !21
  %27 = tail call fastcc i64 @simple_flag(ptr noundef nonnull %17, i64 noundef %20) #44, !range !23
  %28 = getelementptr inbounds %struct.FaString, ptr %9, i64 %14, i32 2
  store i64 %27, ptr %28, align 8, !tbaa !22
  %29 = icmp eq i64 %15, %10
  br i1 %29, label %12, label %13, !llvm.loop !102
}

; Function Attrs: noinline nounwind optsize
define internal fastcc void @materialize_characters(ptr noundef %0, i64 noundef %1) unnamed_addr #14 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %27, label %4

4:                                                ; preds = %2
  %5 = icmp ugt i64 %1, 384307168202282325
  br i1 %5, label %6, label %7

6:                                                ; preds = %4
  tail call fastcc void @out_of_memory() #47
  unreachable

7:                                                ; preds = %4
  %8 = mul nuw nsw i64 %1, 24
  %9 = tail call ptr @fa_alloc(i64 noundef %8, i64 noundef 16) #45
  %10 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %11 = load i64, ptr %10, align 8, !tbaa !21
  %12 = icmp sgt i64 %11, 0
  br i1 %12, label %14, label %13

13:                                               ; preds = %14, %7
  tail call fastcc void @free_array_items(ptr noundef nonnull %0, i64 noundef 1) #44
  store ptr %9, ptr %0, align 8, !tbaa !16
  br label %27

14:                                               ; preds = %7, %14
  %15 = phi i64 [ %24, %14 ], [ 0, %7 ]
  %16 = getelementptr inbounds %struct.FaString, ptr %9, i64 %15
  %17 = load ptr, ptr %0, align 8, !tbaa !16
  %18 = getelementptr inbounds i8, ptr %17, i64 %15
  %19 = load i8, ptr %18, align 1, !tbaa !2
  %20 = zext i8 %19 to i64
  %21 = getelementptr inbounds i8, ptr @ascii_table, i64 %20
  store ptr %21, ptr %16, align 8, !tbaa.struct !13
  %22 = getelementptr inbounds i8, ptr %16, i64 8
  store i64 1, ptr %22, align 8, !tbaa.struct !14
  %23 = getelementptr inbounds %struct.FaString, ptr %9, i64 %15, i32 2
  store i64 -9223372036854775808, ptr %23, align 8, !tbaa !22
  %24 = add nuw nsw i64 %15, 1
  %25 = load i64, ptr %10, align 8, !tbaa !21
  %26 = icmp slt i64 %24, %25
  br i1 %26, label %14, label %13, !llvm.loop !103

27:                                               ; preds = %2, %13
  %28 = phi i64 [ %1, %13 ], [ 0, %2 ]
  %29 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 2
  store i64 %28, ptr %29, align 8, !tbaa !22
  ret void
}

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define noundef i32 @fa_string_array_get(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef writeonly %2) local_unnamed_addr #21 {
  %4 = icmp slt i64 %1, 0
  %5 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %6 = load i64, ptr %5, align 8, !tbaa !21
  %7 = select i1 %4, i64 %6, i64 0
  %8 = add nsw i64 %7, %1
  %9 = icmp ult i64 %8, %6
  br i1 %9, label %10, label %22

10:                                               ; preds = %3
  %11 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 2
  %12 = load i64, ptr %11, align 8, !tbaa !22
  %13 = icmp slt i64 %12, 0
  %14 = load ptr, ptr %0, align 8, !tbaa !16
  br i1 %13, label %15, label %20

15:                                               ; preds = %10
  %16 = getelementptr inbounds i8, ptr %14, i64 %8
  %17 = load i8, ptr %16, align 1, !tbaa !2
  store i8 %17, ptr %2, align 8, !tbaa.struct !13
  %18 = getelementptr inbounds i8, ptr %2, i64 1
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 1 dereferenceable(22) %18, i8 0, i64 22, i1 false)
  %19 = getelementptr inbounds i8, ptr %2, i64 23
  store i8 -63, ptr %19, align 1, !tbaa.struct !104
  br label %22

20:                                               ; preds = %10
  %21 = getelementptr inbounds %struct.FaString, ptr %14, i64 %8
  tail call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %2, ptr noundef nonnull align 8 dereferenceable(24) %21, i64 24, i1 false), !tbaa.struct !13
  br label %22

22:                                               ; preds = %15, %20, %3
  %23 = phi i32 [ 0, %3 ], [ 1, %20 ], [ 1, %15 ]
  ret i32 %23
}

; Function Attrs: noinline nounwind optsize
define internal fastcc void @array_grow(ptr noundef %0, i64 noundef %1) unnamed_addr #14 {
  %3 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 2
  %4 = load i64, ptr %3, align 8, !tbaa !22
  %5 = icmp slt i64 %4, 0
  br i1 %5, label %6, label %15

6:                                                ; preds = %2
  %7 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %8 = load i64, ptr %7, align 8, !tbaa !21
  %9 = icmp sgt i64 %8, 4611686018427387903
  br i1 %9, label %10, label %11

10:                                               ; preds = %6
  tail call fastcc void @out_of_memory() #47
  unreachable

11:                                               ; preds = %6
  %12 = icmp slt i64 %8, 4
  %13 = shl nuw nsw i64 %8, 1
  %14 = select i1 %12, i64 4, i64 %13
  tail call fastcc void @materialize_characters(ptr noundef nonnull %0, i64 noundef %14) #44
  br label %25

15:                                               ; preds = %2
  %16 = icmp ult i64 %4, 4
  %17 = shl nuw nsw i64 %4, 1
  %18 = select i1 %16, i64 4, i64 %17
  %19 = mul nsw i64 %18, %1
  %20 = tail call ptr @fa_alloc(i64 noundef %19, i64 noundef 16) #45
  %21 = load ptr, ptr %0, align 8, !tbaa !16
  %22 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %23 = load i64, ptr %22, align 8, !tbaa !21
  %24 = mul nsw i64 %23, %1
  tail call fastcc void @copy_bytes(ptr noundef %20, ptr noundef %21, i64 noundef %24) #44
  tail call fastcc void @free_array_items(ptr noundef nonnull %0, i64 noundef %1) #44
  store ptr %20, ptr %0, align 8, !tbaa !16
  store i64 %18, ptr %3, align 8, !tbaa !22
  br label %25

25:                                               ; preds = %15, %11
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_array_append(ptr noundef %0, ptr noundef %1, i64 noundef %2) local_unnamed_addr #2 {
  %4 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 2
  %5 = load i64, ptr %4, align 8, !tbaa !22
  %6 = icmp slt i64 %5, 0
  br i1 %6, label %7, label %10, !prof !47

7:                                                ; preds = %3
  %8 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %9 = load i64, ptr %8, align 8, !tbaa !21
  tail call fastcc void @materialize_characters(ptr noundef nonnull %0, i64 noundef %9) #44
  br label %10

10:                                               ; preds = %3, %7
  %11 = getelementptr inbounds %struct.FaArrayHeader, ptr %1, i64 0, i32 2
  %12 = load i64, ptr %11, align 8, !tbaa !22
  %13 = icmp slt i64 %12, 0
  br i1 %13, label %14, label %17, !prof !47

14:                                               ; preds = %10
  %15 = getelementptr inbounds %struct.FaArrayHeader, ptr %1, i64 0, i32 1
  %16 = load i64, ptr %15, align 8, !tbaa !21
  tail call fastcc void @materialize_characters(ptr noundef nonnull %1, i64 noundef %16) #44
  br label %17

17:                                               ; preds = %10, %14
  %18 = getelementptr inbounds %struct.FaArrayHeader, ptr %1, i64 0, i32 1
  %19 = load i64, ptr %18, align 8, !tbaa !21
  %20 = icmp sgt i64 %19, 0
  br i1 %20, label %21, label %23

21:                                               ; preds = %17
  %22 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  br label %24

23:                                               ; preds = %31, %17
  ret void

24:                                               ; preds = %21, %31
  %25 = phi i64 [ 0, %21 ], [ %40, %31 ]
  %26 = load i64, ptr %22, align 8, !tbaa !21
  %27 = load i64, ptr %4, align 8, !tbaa !22
  %28 = icmp slt i64 %26, %27
  br i1 %28, label %31, label %29, !prof !77

29:                                               ; preds = %24
  tail call fastcc void @array_grow(ptr noundef nonnull %0, i64 noundef %2) #44
  %30 = load i64, ptr %22, align 8, !tbaa !21
  br label %31

31:                                               ; preds = %24, %29
  %32 = phi i64 [ %30, %29 ], [ %26, %24 ]
  %33 = load ptr, ptr %0, align 8, !tbaa !16
  %34 = mul nsw i64 %32, %2
  %35 = getelementptr inbounds i8, ptr %33, i64 %34
  %36 = add nsw i64 %32, 1
  store i64 %36, ptr %22, align 8, !tbaa !21
  %37 = load ptr, ptr %1, align 8, !tbaa !16
  %38 = mul nsw i64 %25, %2
  %39 = getelementptr inbounds i8, ptr %37, i64 %38
  tail call fastcc void @copy_bytes(ptr noundef %35, ptr noundef %39, i64 noundef %2) #44
  %40 = add nuw nsw i64 %25, 1
  %41 = icmp eq i64 %40, %19
  br i1 %41, label %23, label %24, !llvm.loop !105
}

; Function Attrs: mustprogress nofree noinline norecurse nosync nounwind optsize willreturn memory(none)
define i64 @fa_index_from_end(i64 noundef %0, i64 noundef %1) local_unnamed_addr #22 {
  %3 = add nsw i64 %1, %0
  %4 = tail call i64 @llvm.smax.i64(i64 %3, i64 -1)
  %5 = icmp sgt i64 %0, -1
  %6 = select i1 %5, i64 -1, i64 %4
  ret i64 %6
}

; Function Attrs: nounwind optsize
define noundef ptr @fa_array_slice(ptr noundef %0, i64 noundef %1, i64 noundef %2, i8 noundef signext %3, i64 noundef %4, ptr nocapture noundef writeonly %5) local_unnamed_addr #2 {
  %7 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 2
  %8 = load i64, ptr %7, align 8, !tbaa !22
  %9 = icmp slt i64 %8, 0
  br i1 %9, label %10, label %13, !prof !47

10:                                               ; preds = %6
  %11 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %12 = load i64, ptr %11, align 8, !tbaa !21
  tail call fastcc void @materialize_characters(ptr noundef nonnull %0, i64 noundef %12) #44
  br label %13

13:                                               ; preds = %6, %10
  %14 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %15 = load i64, ptr %14, align 8, !tbaa !21
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
  %25 = tail call ptr @fa_error_index(i64 noundef %1, i64 noundef %15) #44
  br label %63

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
  %52 = tail call ptr @fa_error_index(i64 noundef %2, i64 noundef %15) #44
  br label %63

53:                                               ; preds = %45, %30
  %54 = phi i64 [ %15, %30 ], [ %50, %45 ]
  %55 = tail call i64 @llvm.smax.i64(i64 %54, i64 %31)
  %56 = sub nsw i64 %55, %31
  %57 = tail call ptr @fa_array_new(i64 noundef %56, i64 noundef %56, i64 noundef %4) #44
  %58 = load ptr, ptr %57, align 8, !tbaa !16
  %59 = load ptr, ptr %0, align 8, !tbaa !16
  %60 = mul nsw i64 %31, %4
  %61 = getelementptr inbounds i8, ptr %59, i64 %60
  %62 = mul nsw i64 %56, %4
  tail call fastcc void @copy_bytes(ptr noundef %58, ptr noundef %61, i64 noundef %62) #44
  store ptr %57, ptr %5, align 8, !tbaa !10
  br label %63

63:                                               ; preds = %51, %53, %24
  %64 = phi ptr [ %25, %24 ], [ null, %53 ], [ %52, %51 ]
  ret ptr %64
}

; Function Attrs: nounwind optsize
define void @fa_array_reserve(ptr noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #2 {
  %4 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 2
  %5 = load i64, ptr %4, align 8, !tbaa !22
  %6 = icmp slt i64 %5, 0
  br i1 %6, label %7, label %12

7:                                                ; preds = %3
  %8 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %9 = load i64, ptr %8, align 8, !tbaa !21
  %10 = icmp slt i64 %9, %1
  br i1 %10, label %11, label %27

11:                                               ; preds = %7
  tail call fastcc void @materialize_characters(ptr noundef nonnull %0, i64 noundef %1) #44
  br label %27

12:                                               ; preds = %3
  %13 = icmp slt i64 %5, %1
  br i1 %13, label %14, label %27

14:                                               ; preds = %12
  %15 = icmp sgt i64 %2, 0
  br i1 %15, label %16, label %20

16:                                               ; preds = %14
  %17 = udiv i64 9223372036854775807, %2
  %18 = icmp ult i64 %17, %1
  br i1 %18, label %19, label %20

19:                                               ; preds = %16
  tail call fastcc void @out_of_memory() #47
  unreachable

20:                                               ; preds = %16, %14
  %21 = mul nsw i64 %2, %1
  %22 = tail call ptr @fa_alloc(i64 noundef %21, i64 noundef 16) #45
  %23 = load ptr, ptr %0, align 8, !tbaa !16
  %24 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %25 = load i64, ptr %24, align 8, !tbaa !21
  %26 = mul nsw i64 %25, %2
  tail call fastcc void @copy_bytes(ptr noundef %22, ptr noundef %23, i64 noundef %26) #44
  tail call fastcc void @free_array_items(ptr noundef nonnull %0, i64 noundef %2) #44
  store ptr %22, ptr %0, align 8, !tbaa !16
  store i64 %1, ptr %4, align 8, !tbaa !22
  br label %27

27:                                               ; preds = %12, %7, %11, %20
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @free_array_items(ptr noundef %0, i64 noundef %1) unnamed_addr #2 {
  %3 = getelementptr inbounds i8, ptr %0, i64 24
  %4 = load i64, ptr %3, align 1
  %5 = icmp ne i64 %4, 0
  %6 = load ptr, ptr %0, align 8, !tbaa !16
  %7 = getelementptr inbounds i8, ptr %0, i64 32
  %8 = icmp eq ptr %6, %7
  %9 = select i1 %5, i1 %8, i1 false
  br i1 %9, label %10, label %11

10:                                               ; preds = %2
  store i64 0, ptr %3, align 1
  br label %22

11:                                               ; preds = %2
  %12 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 2
  %13 = load i64, ptr %12, align 8, !tbaa !22
  %14 = icmp slt i64 %13, 0
  br i1 %14, label %15, label %18

15:                                               ; preds = %11
  %16 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %17 = load i64, ptr %16, align 8, !tbaa !21
  br label %20

18:                                               ; preds = %11
  %19 = mul nsw i64 %13, %1
  br label %20

20:                                               ; preds = %18, %15
  %21 = phi i64 [ %17, %15 ], [ %19, %18 ]
  tail call void @fa_free(ptr noundef %6, i64 noundef %21, i64 noundef 16) #45
  br label %22

22:                                               ; preds = %20, %10
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_array_assign(ptr noundef %0, ptr noundef %1, i64 noundef %2) local_unnamed_addr #2 {
  %4 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 2
  %5 = load i64, ptr %4, align 8, !tbaa !22
  %6 = icmp slt i64 %5, 0
  br i1 %6, label %7, label %10, !prof !47

7:                                                ; preds = %3
  %8 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %9 = load i64, ptr %8, align 8, !tbaa !21
  tail call fastcc void @materialize_characters(ptr noundef nonnull %0, i64 noundef %9) #44
  br label %10

10:                                               ; preds = %3, %7
  %11 = getelementptr inbounds %struct.FaArrayHeader, ptr %1, i64 0, i32 2
  %12 = load i64, ptr %11, align 8, !tbaa !22
  %13 = icmp slt i64 %12, 0
  br i1 %13, label %14, label %17, !prof !47

14:                                               ; preds = %10
  %15 = getelementptr inbounds %struct.FaArrayHeader, ptr %1, i64 0, i32 1
  %16 = load i64, ptr %15, align 8, !tbaa !21
  tail call fastcc void @materialize_characters(ptr noundef nonnull %1, i64 noundef %16) #44
  br label %17

17:                                               ; preds = %10, %14
  %18 = icmp eq ptr %0, %1
  br i1 %18, label %37, label %19

19:                                               ; preds = %17
  %20 = load i64, ptr %4, align 8, !tbaa !22
  %21 = getelementptr inbounds %struct.FaArrayHeader, ptr %1, i64 0, i32 1
  %22 = load i64, ptr %21, align 8, !tbaa !21
  %23 = icmp slt i64 %20, %22
  br i1 %23, label %26, label %24

24:                                               ; preds = %19
  %25 = load ptr, ptr %0, align 8, !tbaa !16
  br label %30

26:                                               ; preds = %19
  %27 = mul nsw i64 %22, %2
  %28 = tail call ptr @fa_alloc(i64 noundef %27, i64 noundef 16) #45
  tail call fastcc void @free_array_items(ptr noundef nonnull %0, i64 noundef %2) #44
  store ptr %28, ptr %0, align 8, !tbaa !16
  %29 = load i64, ptr %21, align 8, !tbaa !21
  store i64 %29, ptr %4, align 8, !tbaa !22
  br label %30

30:                                               ; preds = %24, %26
  %31 = phi i64 [ %22, %24 ], [ %29, %26 ]
  %32 = phi ptr [ %25, %24 ], [ %28, %26 ]
  %33 = load ptr, ptr %1, align 8, !tbaa !16
  %34 = mul nsw i64 %31, %2
  tail call fastcc void @copy_bytes(ptr noundef %32, ptr noundef %33, i64 noundef %34) #44
  %35 = load i64, ptr %21, align 8, !tbaa !21
  %36 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  store i64 %35, ptr %36, align 8, !tbaa !21
  br label %37

37:                                               ; preds = %17, %30
  ret void
}

; Function Attrs: nounwind optsize
define noundef ptr @fa_array_copy(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #2 {
  %3 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 2
  %4 = load i64, ptr %3, align 8, !tbaa !22
  %5 = icmp slt i64 %4, 0
  %6 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %7 = load i64, ptr %6, align 8, !tbaa !21
  br i1 %5, label %8, label %14

8:                                                ; preds = %2
  %9 = tail call ptr @fa_array_new(i64 noundef %7, i64 noundef %7, i64 noundef 1) #44
  %10 = load ptr, ptr %9, align 8, !tbaa !16
  %11 = load ptr, ptr %0, align 8, !tbaa !16
  %12 = load i64, ptr %6, align 8, !tbaa !21
  tail call fastcc void @copy_bytes(ptr noundef %10, ptr noundef %11, i64 noundef %12) #44
  %13 = getelementptr inbounds %struct.FaArrayHeader, ptr %9, i64 0, i32 2
  store i64 -1, ptr %13, align 8, !tbaa !22
  br label %20

14:                                               ; preds = %2
  %15 = tail call ptr @fa_array_new(i64 noundef %7, i64 noundef %7, i64 noundef %1) #44
  %16 = load ptr, ptr %15, align 8, !tbaa !16
  %17 = load ptr, ptr %0, align 8, !tbaa !16
  %18 = load i64, ptr %6, align 8, !tbaa !21
  %19 = mul nsw i64 %18, %1
  tail call fastcc void @copy_bytes(ptr noundef %16, ptr noundef %17, i64 noundef %19) #44
  br label %20

20:                                               ; preds = %14, %8
  %21 = phi ptr [ %9, %8 ], [ %15, %14 ]
  ret ptr %21
}

; Function Attrs: nounwind optsize
define void @fa_array_reverse(ptr noundef %0, i64 noundef %1) local_unnamed_addr #2 {
  %3 = alloca [64 x i8], align 16
  %4 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 2
  %5 = load i64, ptr %4, align 8, !tbaa !22
  %6 = icmp slt i64 %5, 0
  br i1 %6, label %7, label %10, !prof !47

7:                                                ; preds = %2
  %8 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %9 = load i64, ptr %8, align 8, !tbaa !21
  tail call fastcc void @materialize_characters(ptr noundef nonnull %0, i64 noundef %9) #44
  br label %10

10:                                               ; preds = %2, %7
  %11 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %12 = load i64, ptr %11, align 8, !tbaa !21
  %13 = icmp slt i64 %12, 2
  br i1 %13, label %39, label %14

14:                                               ; preds = %10
  call void @llvm.lifetime.start.p0(i64 64, ptr nonnull %3) #40
  %15 = add nsw i64 %12, -1
  %16 = mul nsw i64 %15, %1
  %17 = icmp sgt i64 %16, 0
  br i1 %17, label %18, label %38

18:                                               ; preds = %14
  %19 = load ptr, ptr %0, align 8, !tbaa !16
  %20 = getelementptr inbounds i8, ptr %19, i64 %16
  %21 = icmp sgt i64 %1, 0
  %22 = sub i64 0, %1
  br label %23

23:                                               ; preds = %18, %26
  %24 = phi ptr [ %19, %18 ], [ %27, %26 ]
  %25 = phi ptr [ %20, %18 ], [ %28, %26 ]
  br i1 %21, label %30, label %26

26:                                               ; preds = %30, %23
  %27 = getelementptr inbounds i8, ptr %24, i64 %1
  %28 = getelementptr inbounds i8, ptr %25, i64 %22
  %29 = icmp ult ptr %27, %28
  br i1 %29, label %23, label %38, !llvm.loop !106

30:                                               ; preds = %23, %30
  %31 = phi i64 [ %36, %30 ], [ 0, %23 ]
  %32 = sub nsw i64 %1, %31
  %33 = tail call i64 @llvm.umin.i64(i64 %32, i64 64)
  %34 = getelementptr inbounds i8, ptr %24, i64 %31
  call fastcc void @copy_bytes(ptr noundef nonnull %3, ptr noundef %34, i64 noundef %33) #44
  %35 = getelementptr inbounds i8, ptr %25, i64 %31
  tail call fastcc void @copy_bytes(ptr noundef %34, ptr noundef nonnull %35, i64 noundef %33) #44
  call fastcc void @copy_bytes(ptr noundef nonnull %35, ptr noundef nonnull %3, i64 noundef %33) #44
  %36 = add nuw nsw i64 %31, 64
  %37 = icmp slt i64 %36, %1
  br i1 %37, label %30, label %26, !llvm.loop !107

38:                                               ; preds = %26, %14
  call void @llvm.lifetime.end.p0(i64 64, ptr nonnull %3) #40
  br label %39

39:                                               ; preds = %10, %38
  ret void
}

; Function Attrs: nounwind optsize
define ptr @fa_array_insert_slot(ptr noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #2 {
  %4 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %5 = load i64, ptr %4, align 8, !tbaa !21
  %6 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 2
  %7 = load i64, ptr %6, align 8, !tbaa !22
  %8 = icmp slt i64 %5, %7
  br i1 %8, label %11, label %9, !prof !77

9:                                                ; preds = %3
  tail call fastcc void @array_grow(ptr noundef nonnull %0, i64 noundef %2) #44
  %10 = load i64, ptr %4, align 8, !tbaa !21
  br label %11

11:                                               ; preds = %3, %9
  %12 = phi i64 [ %10, %9 ], [ %5, %3 ]
  %13 = load ptr, ptr %0, align 8, !tbaa !16
  %14 = add nsw i64 %12, 1
  store i64 %14, ptr %4, align 8, !tbaa !21
  %15 = mul nsw i64 %2, %1
  %16 = getelementptr inbounds i8, ptr %13, i64 %15
  %17 = sub i64 %12, %1
  %18 = mul nsw i64 %17, %2
  %19 = getelementptr inbounds i8, ptr %16, i64 %2
  tail call void @llvm.memmove.p0.p0.i64(ptr align 1 %19, ptr align 1 %16, i64 %18, i1 false)
  ret ptr %16
}

; Function Attrs: mustprogress nocallback nofree nounwind willreturn memory(argmem: readwrite)
declare void @llvm.memmove.p0.p0.i64(ptr nocapture writeonly, ptr nocapture readonly, i64, i1 immarg) #9

; Function Attrs: nounwind optsize
define void @fa_array_remove(ptr noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #2 {
  %4 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 2
  %5 = load i64, ptr %4, align 8, !tbaa !22
  %6 = icmp slt i64 %5, 0
  br i1 %6, label %7, label %10, !prof !47

7:                                                ; preds = %3
  %8 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %9 = load i64, ptr %8, align 8, !tbaa !21
  tail call fastcc void @materialize_characters(ptr noundef nonnull %0, i64 noundef %9) #44
  br label %10

10:                                               ; preds = %3, %7
  %11 = load ptr, ptr %0, align 8, !tbaa !16
  %12 = mul nsw i64 %2, %1
  %13 = getelementptr inbounds i8, ptr %11, i64 %12
  %14 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %15 = load i64, ptr %14, align 8, !tbaa !21
  %16 = xor i64 %1, -1
  %17 = add i64 %15, %16
  %18 = mul nsw i64 %17, %2
  %19 = getelementptr inbounds i8, ptr %13, i64 %2
  tail call void @llvm.memmove.p0.p0.i64(ptr align 1 %13, ptr align 1 %19, i64 %18, i1 false)
  %20 = load i64, ptr %14, align 8, !tbaa !21
  %21 = add nsw i64 %20, -1
  store i64 %21, ptr %14, align 8, !tbaa !21
  ret void
}

; Function Attrs: alwaysinline nounwind optsize
define void @fa_array_sort(ptr noundef %0, i64 noundef %1, ptr nocapture noundef readonly %2, ptr noundef %3) local_unnamed_addr #13 {
  %5 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 2
  %6 = load i64, ptr %5, align 8, !tbaa !22
  %7 = icmp slt i64 %6, 0
  br i1 %7, label %8, label %11, !prof !47

8:                                                ; preds = %4
  %9 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %10 = load i64, ptr %9, align 8, !tbaa !21
  tail call fastcc void @materialize_characters(ptr noundef nonnull %0, i64 noundef %10) #44
  br label %11

11:                                               ; preds = %4, %8
  %12 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %13 = load i64, ptr %12, align 8, !tbaa !21
  %14 = icmp slt i64 %13, 2
  br i1 %14, label %82, label %15

15:                                               ; preds = %11
  %16 = mul nsw i64 %13, %1
  %17 = load ptr, ptr %0, align 8, !tbaa !16
  %18 = tail call ptr @fa_alloc(i64 noundef %16, i64 noundef 16) #45
  br label %19

19:                                               ; preds = %27, %15
  %20 = phi ptr [ %17, %15 ], [ %21, %27 ]
  %21 = phi ptr [ %18, %15 ], [ %20, %27 ]
  %22 = phi i64 [ 1, %15 ], [ %23, %27 ]
  %23 = shl nsw i64 %22, 1
  br label %29

24:                                               ; preds = %27
  %25 = load ptr, ptr %0, align 8, !tbaa !16
  %26 = icmp eq ptr %21, %25
  br i1 %26, label %80, label %79

27:                                               ; preds = %61
  %28 = icmp slt i64 %23, %13
  br i1 %28, label %19, label %24, !llvm.loop !108

29:                                               ; preds = %19, %61
  %30 = phi i64 [ 0, %19 ], [ %33, %61 ]
  %31 = add nsw i64 %30, %22
  %32 = tail call i64 @llvm.smin.i64(i64 %31, i64 %13)
  %33 = add nsw i64 %30, %23
  %34 = tail call i64 @llvm.smin.i64(i64 %33, i64 %13)
  %35 = icmp slt i64 %30, %32
  %36 = icmp slt i64 %31, %34
  %37 = select i1 %35, i1 %36, i1 false
  br i1 %37, label %38, label %61

38:                                               ; preds = %29, %54
  %39 = phi i64 [ %57, %54 ], [ %30, %29 ]
  %40 = phi i64 [ %56, %54 ], [ %32, %29 ]
  %41 = phi i64 [ %55, %54 ], [ %30, %29 ]
  %42 = mul nsw i64 %40, %1
  %43 = getelementptr inbounds i8, ptr %20, i64 %42
  %44 = mul nsw i64 %41, %1
  %45 = getelementptr inbounds i8, ptr %20, i64 %44
  %46 = tail call i32 %2(ptr noundef %3, ptr noundef %43, ptr noundef %45) #45
  %47 = icmp slt i32 %46, 0
  %48 = mul nsw i64 %39, %1
  %49 = getelementptr inbounds i8, ptr %21, i64 %48
  br i1 %47, label %50, label %52

50:                                               ; preds = %38
  tail call fastcc void @copy_bytes(ptr noundef %49, ptr noundef %43, i64 noundef %1) #44
  %51 = add nsw i64 %40, 1
  br label %54

52:                                               ; preds = %38
  tail call fastcc void @copy_bytes(ptr noundef %49, ptr noundef %45, i64 noundef %1) #44
  %53 = add nsw i64 %41, 1
  br label %54

54:                                               ; preds = %52, %50
  %55 = phi i64 [ %41, %50 ], [ %53, %52 ]
  %56 = phi i64 [ %51, %50 ], [ %40, %52 ]
  %57 = add nsw i64 %39, 1
  %58 = icmp slt i64 %55, %32
  %59 = icmp slt i64 %56, %34
  %60 = select i1 %58, i1 %59, i1 false
  br i1 %60, label %38, label %61, !llvm.loop !109

61:                                               ; preds = %54, %29
  %62 = phi i64 [ %30, %29 ], [ %55, %54 ]
  %63 = phi i64 [ %32, %29 ], [ %56, %54 ]
  %64 = phi i64 [ %30, %29 ], [ %57, %54 ]
  %65 = mul nsw i64 %64, %1
  %66 = getelementptr inbounds i8, ptr %21, i64 %65
  %67 = mul nsw i64 %62, %1
  %68 = getelementptr inbounds i8, ptr %20, i64 %67
  %69 = sub nsw i64 %32, %62
  %70 = mul nsw i64 %69, %1
  tail call fastcc void @copy_bytes(ptr noundef %66, ptr noundef %68, i64 noundef %70) #44
  %71 = add nsw i64 %64, %69
  %72 = mul nsw i64 %71, %1
  %73 = getelementptr inbounds i8, ptr %21, i64 %72
  %74 = mul nsw i64 %63, %1
  %75 = getelementptr inbounds i8, ptr %20, i64 %74
  %76 = sub nsw i64 %34, %63
  %77 = mul nsw i64 %76, %1
  tail call fastcc void @copy_bytes(ptr noundef %73, ptr noundef %75, i64 noundef %77) #44
  %78 = icmp slt i64 %33, %13
  br i1 %78, label %29, label %27, !llvm.loop !110

79:                                               ; preds = %24
  tail call fastcc void @copy_bytes(ptr noundef %25, ptr noundef %21, i64 noundef %16) #44
  br label %80

80:                                               ; preds = %24, %79
  %81 = phi ptr [ %21, %79 ], [ %20, %24 ]
  tail call void @fa_free(ptr noundef %81, i64 noundef %16, i64 noundef 16) #45
  br label %82

82:                                               ; preds = %80, %11
  ret void
}

; Function Attrs: nounwind optsize
define noundef ptr @fa_arguments() local_unnamed_addr #2 {
  %1 = load i64, ptr @argument_count, align 8, !tbaa !8
  %2 = icmp ult i64 %1, 2147483648
  tail call void @llvm.assume(i1 %2)
  %3 = tail call ptr @fa_array_new(i64 noundef %1, i64 noundef %1, i64 noundef 24) #44
  %4 = load ptr, ptr %3, align 8, !tbaa !16
  %5 = load ptr, ptr @argument_items, align 8, !tbaa !10
  %6 = mul nuw nsw i64 %1, 24
  tail call fastcc void @copy_bytes(ptr noundef %4, ptr noundef %5, i64 noundef %6) #44
  %7 = getelementptr inbounds %struct.FaArrayHeader, ptr %3, i64 0, i32 1
  store i64 %1, ptr %7, align 8, !tbaa !21
  %8 = getelementptr inbounds %struct.FaArrayHeader, ptr %3, i64 0, i32 2
  store i64 %1, ptr %8, align 8, !tbaa !22
  ret ptr %3
}

; Function Attrs: nounwind optsize
define void @fa_array_free(ptr noundef %0, i64 noundef %1) local_unnamed_addr #2 {
  %3 = getelementptr inbounds i8, ptr %0, i64 24
  %4 = load i64, ptr %3, align 1
  %5 = icmp ne i64 %4, 0
  %6 = load ptr, ptr %0, align 8, !tbaa !16
  %7 = getelementptr inbounds i8, ptr %0, i64 32
  %8 = icmp eq ptr %6, %7
  %9 = select i1 %5, i1 %8, i1 false
  br i1 %9, label %21, label %10

10:                                               ; preds = %2
  %11 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 2
  %12 = load i64, ptr %11, align 8, !tbaa !22
  %13 = icmp slt i64 %12, 0
  br i1 %13, label %14, label %17

14:                                               ; preds = %10
  %15 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %16 = load i64, ptr %15, align 8, !tbaa !21
  br label %19

17:                                               ; preds = %10
  %18 = mul nsw i64 %12, %1
  br label %19

19:                                               ; preds = %17, %14
  %20 = phi i64 [ %16, %14 ], [ %18, %17 ]
  tail call void @fa_free(ptr noundef %6, i64 noundef %20, i64 noundef 16) #45
  br label %21

21:                                               ; preds = %2, %19
  tail call void @fa_free(ptr noundef nonnull %0, i64 noundef 32, i64 noundef 8) #45
  ret void
}

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read)
define ptr @fa_segmented_at(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2, i32 noundef %3) local_unnamed_addr #23 {
  %5 = zext nneg i32 %3 to i64
  %6 = lshr i64 %1, %5
  %7 = icmp eq i64 %6, 0
  br i1 %7, label %15, label %8

8:                                                ; preds = %4
  %9 = tail call i64 @llvm.ctlz.i64(i64 %6, i1 true), !range !111
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
  %19 = load ptr, ptr %18, align 8, !tbaa !10
  %20 = add i64 %16, %1
  %21 = mul nsw i64 %20, %2
  %22 = getelementptr inbounds i8, ptr %19, i64 %21
  ret ptr %22
}

; Function Attrs: nounwind optsize
define noundef ptr @fa_segmented_new() local_unnamed_addr #2 {
  %1 = tail call ptr @fa_alloc(i64 noundef 400, i64 noundef 8) #45
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(400) %1, i8 0, i64 400, i1 false)
  ret ptr %1
}

; Function Attrs: nounwind optsize
define ptr @fa_segmented_push_slot(ptr nocapture noundef %0, i64 noundef %1, i32 noundef %2) local_unnamed_addr #2 {
  %4 = load i64, ptr %0, align 8, !tbaa !112
  %5 = zext nneg i32 %2 to i64
  %6 = lshr i64 %4, %5
  %7 = icmp eq i64 %6, 0
  br i1 %7, label %15, label %8

8:                                                ; preds = %3
  %9 = tail call i64 @llvm.ctlz.i64(i64 %6, i1 true), !range !111
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
  %19 = load i64, ptr %18, align 8, !tbaa !114
  %20 = icmp slt i64 %17, %19
  br i1 %20, label %21, label %24

21:                                               ; preds = %15
  %22 = getelementptr inbounds %struct.FaSegmented, ptr %0, i64 0, i32 2, i64 %17
  %23 = load ptr, ptr %22, align 8, !tbaa !10
  br label %32

24:                                               ; preds = %15
  %25 = sext i32 %2 to i64
  %26 = tail call i64 @llvm.usub.sat.i64(i64 %17, i64 1)
  %27 = add nsw i64 %26, %25
  %28 = shl i64 %1, %27
  %29 = tail call ptr @fa_alloc(i64 noundef %28, i64 noundef 16) #45
  %30 = getelementptr inbounds %struct.FaSegmented, ptr %0, i64 0, i32 2, i64 %17
  store ptr %29, ptr %30, align 8, !tbaa !10
  %31 = add nuw nsw i64 %17, 1
  store i64 %31, ptr %18, align 8, !tbaa !114
  br label %32

32:                                               ; preds = %21, %24
  %33 = phi ptr [ %23, %21 ], [ %29, %24 ]
  %34 = add nsw i64 %4, 1
  store i64 %34, ptr %0, align 8, !tbaa !112
  %35 = add i64 %16, %4
  %36 = mul nsw i64 %35, %1
  %37 = getelementptr inbounds i8, ptr %33, i64 %36
  ret ptr %37
}

; Function Attrs: nounwind optsize
define void @fa_segmented_free(ptr noundef %0, i64 noundef %1, i32 noundef %2) local_unnamed_addr #2 {
  %4 = getelementptr inbounds %struct.FaSegmented, ptr %0, i64 0, i32 1
  %5 = load i64, ptr %4, align 8, !tbaa !114
  %6 = icmp sgt i64 %5, 0
  br i1 %6, label %7, label %9

7:                                                ; preds = %3
  %8 = sext i32 %2 to i64
  br label %10

9:                                                ; preds = %10, %3
  tail call void @fa_free(ptr noundef nonnull %0, i64 noundef 400, i64 noundef 8) #45
  ret void

10:                                               ; preds = %7, %10
  %11 = phi i64 [ 0, %7 ], [ %17, %10 ]
  %12 = getelementptr inbounds %struct.FaSegmented, ptr %0, i64 0, i32 2, i64 %11
  %13 = load ptr, ptr %12, align 8, !tbaa !10
  %14 = tail call i64 @llvm.usub.sat.i64(i64 %11, i64 1)
  %15 = add i64 %14, %8
  %16 = shl i64 %1, %15
  tail call void @fa_free(ptr noundef %13, i64 noundef %16, i64 noundef 16) #45
  %17 = add nuw nsw i64 %11, 1
  %18 = load i64, ptr %4, align 8, !tbaa !114
  %19 = icmp slt i64 %17, %18
  br i1 %19, label %10, label %9, !llvm.loop !115
}

; Function Attrs: nounwind optsize
define ptr @fa_segmented_flatten(ptr nocapture noundef readonly %0, i64 noundef %1, i32 noundef %2) local_unnamed_addr #2 {
  %4 = load i64, ptr %0, align 8, !tbaa !112
  %5 = tail call ptr @fa_array_new(i64 noundef %4, i64 noundef %4, i64 noundef %1) #44
  %6 = load i64, ptr %0, align 8, !tbaa !112
  %7 = icmp sgt i64 %6, 0
  br i1 %7, label %8, label %10

8:                                                ; preds = %3
  %9 = sext i32 %2 to i64
  br label %11

10:                                               ; preds = %11, %3
  ret ptr %5

11:                                               ; preds = %8, %11
  %12 = phi i64 [ %6, %8 ], [ %28, %11 ]
  %13 = phi i64 [ 0, %8 ], [ %27, %11 ]
  %14 = phi i64 [ 0, %8 ], [ %26, %11 ]
  %15 = tail call i64 @llvm.usub.sat.i64(i64 %13, i64 1)
  %16 = add i64 %15, %9
  %17 = shl nuw i64 1, %16
  %18 = sub nsw i64 %12, %14
  %19 = tail call i64 @llvm.smin.i64(i64 %17, i64 %18)
  %20 = load ptr, ptr %5, align 8, !tbaa !16
  %21 = mul nsw i64 %14, %1
  %22 = getelementptr inbounds i8, ptr %20, i64 %21
  %23 = getelementptr inbounds %struct.FaSegmented, ptr %0, i64 0, i32 2, i64 %13
  %24 = load ptr, ptr %23, align 8, !tbaa !10
  %25 = mul nsw i64 %19, %1
  tail call fastcc void @copy_bytes(ptr noundef %22, ptr noundef %24, i64 noundef %25) #44
  %26 = add nsw i64 %19, %14
  %27 = add nuw nsw i64 %13, 1
  %28 = load i64, ptr %0, align 8, !tbaa !112
  %29 = icmp slt i64 %26, %28
  br i1 %29, label %11, label %10, !llvm.loop !116
}

; Function Attrs: nounwind optsize
define noundef ptr @fa_segmented_from_array(ptr noundef %0, i64 noundef %1, i32 noundef %2) local_unnamed_addr #2 {
  %4 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 2
  %5 = load i64, ptr %4, align 8, !tbaa !22
  %6 = icmp slt i64 %5, 0
  br i1 %6, label %7, label %10, !prof !47

7:                                                ; preds = %3
  %8 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %9 = load i64, ptr %8, align 8, !tbaa !21
  tail call fastcc void @materialize_characters(ptr noundef nonnull %0, i64 noundef %9) #44
  br label %10

10:                                               ; preds = %3, %7
  %11 = tail call noundef ptr @fa_alloc(i64 noundef 400, i64 noundef 8) #45
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(400) %11, i8 0, i64 400, i1 false)
  %12 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %13 = load i64, ptr %12, align 8, !tbaa !21
  %14 = icmp sgt i64 %13, 0
  br i1 %14, label %16, label %15

15:                                               ; preds = %16, %10
  tail call void @fa_array_free(ptr noundef nonnull %0, i64 noundef %1) #44
  ret ptr %11

16:                                               ; preds = %10, %16
  %17 = phi i64 [ %22, %16 ], [ 0, %10 ]
  %18 = tail call ptr @fa_segmented_push_slot(ptr noundef %11, i64 noundef %1, i32 noundef %2) #44
  %19 = load ptr, ptr %0, align 8, !tbaa !16
  %20 = mul nsw i64 %17, %1
  %21 = getelementptr inbounds i8, ptr %19, i64 %20
  tail call fastcc void @copy_bytes(ptr noundef %18, ptr noundef %21, i64 noundef %1) #44
  %22 = add nuw nsw i64 %17, 1
  %23 = load i64, ptr %12, align 8, !tbaa !21
  %24 = icmp slt i64 %22, %23
  br i1 %24, label %16, label %15, !llvm.loop !117
}

; Function Attrs: alwaysinline nounwind optsize
define ptr @fa_heap_alloc(i64 noundef %0, i64 noundef %1) local_unnamed_addr #13 {
  %3 = icmp ugt i64 %1, 8
  %4 = icmp eq i64 %0, 0
  %5 = add i64 %0, 15
  %6 = and i64 %5, -16
  %7 = select i1 %4, i64 16, i64 %6
  %8 = select i1 %3, i64 %7, i64 %0
  %9 = icmp ult i64 %8, 129
  br i1 %9, label %10, label %42

10:                                               ; preds = %2
  %11 = icmp ult i64 %8, 9
  %12 = add nuw nsw i64 %8, 7
  %13 = lshr i64 %12, 3
  %14 = trunc i64 %13 to i32
  %15 = add nsw i32 %14, -1
  %16 = select i1 %11, i32 0, i32 %15
  %17 = tail call align 16 ptr @llvm.threadlocal.address.p0(ptr align 16 @free_blocks)
  %18 = zext i32 %16 to i64
  %19 = getelementptr inbounds [40 x ptr], ptr %17, i64 0, i64 %18
  %20 = load ptr, ptr %19, align 8, !tbaa !10
  %21 = icmp eq ptr %20, null
  br i1 %21, label %24, label %22, !prof !47

22:                                               ; preds = %10
  %23 = load ptr, ptr %20, align 8, !tbaa !118
  store ptr %23, ptr %19, align 8, !tbaa !10
  br label %44

24:                                               ; preds = %10
  %25 = tail call align 16 ptr @llvm.threadlocal.address.p0(ptr align 16 @class_next)
  %26 = getelementptr inbounds [40 x ptr], ptr %25, i64 0, i64 %18
  %27 = load ptr, ptr %26, align 8, !tbaa !10
  %28 = add nsw i32 %16, 1
  %29 = zext nneg i32 %28 to i64
  %30 = shl nuw nsw i64 %29, 3
  %31 = icmp eq ptr %27, null
  br i1 %31, label %42, label %32

32:                                               ; preds = %24
  %33 = tail call align 16 ptr @llvm.threadlocal.address.p0(ptr align 16 @class_end)
  %34 = getelementptr inbounds [40 x ptr], ptr %33, i64 0, i64 %18
  %35 = load ptr, ptr %34, align 8, !tbaa !10
  %36 = ptrtoint ptr %35 to i64
  %37 = ptrtoint ptr %27 to i64
  %38 = sub i64 %36, %37
  %39 = icmp ult i64 %38, %30
  br i1 %39, label %42, label %40

40:                                               ; preds = %32
  %41 = getelementptr inbounds i8, ptr %27, i64 %30
  store ptr %41, ptr %26, align 8, !tbaa !10
  br label %44

42:                                               ; preds = %24, %32, %2
  %43 = tail call fastcc ptr @heap_alloc_slow(i64 noundef %8) #44
  br label %44

44:                                               ; preds = %22, %40, %42
  %45 = phi ptr [ %43, %42 ], [ %20, %22 ], [ %27, %40 ]
  ret ptr %45
}

; Function Attrs: noinline nounwind optsize
define internal fastcc ptr @heap_alloc_slow(i64 noundef %0) unnamed_addr #14 {
  %2 = icmp ugt i64 %0, 8192
  br i1 %2, label %3, label %14

3:                                                ; preds = %1
  %4 = add i64 %0, 65551
  %5 = lshr i64 %4, 16
  %6 = icmp ult i64 %4, 65536
  br i1 %6, label %77, label %7

7:                                                ; preds = %3
  %8 = tail call fastcc ptr @take_pages(i64 noundef %5) #44
  %9 = icmp eq ptr %8, null
  br i1 %9, label %77, label %10

10:                                               ; preds = %7
  store i32 65535, ptr %8, align 4, !tbaa !120
  %11 = trunc i64 %5 to i32
  %12 = getelementptr inbounds %struct.PageHeader, ptr %8, i64 0, i32 1
  store i32 %11, ptr %12, align 4, !tbaa !122
  %13 = getelementptr inbounds i8, ptr %8, i64 16
  br label %77

14:                                               ; preds = %1
  %15 = icmp ult i64 %0, 129
  br i1 %15, label %16, label %24

16:                                               ; preds = %14
  %17 = add nuw nsw i64 %0, 7
  %18 = and i64 %17, 504
  %19 = icmp ugt i64 %0, 8
  %20 = select i1 %19, i64 %18, i64 8
  %21 = lshr exact i64 %20, 3
  %22 = trunc i64 %21 to i32
  %23 = add nsw i32 %22, -1
  br label %41

24:                                               ; preds = %14, %39
  %25 = phi i32 [ %28, %39 ], [ 16, %14 ]
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
  br i1 %38, label %39, label %29, !llvm.loop !123

39:                                               ; preds = %35
  %40 = shl i64 %26, 1
  br label %24

41:                                               ; preds = %29, %16
  %42 = phi i64 [ %20, %16 ], [ %33, %29 ]
  %43 = phi i32 [ %23, %16 ], [ %31, %29 ]
  %44 = tail call align 16 ptr @llvm.threadlocal.address.p0(ptr align 16 @free_blocks)
  %45 = zext i32 %43 to i64
  %46 = getelementptr inbounds [40 x ptr], ptr %44, i64 0, i64 %45
  %47 = load ptr, ptr %46, align 8, !tbaa !10
  %48 = icmp eq ptr %47, null
  br i1 %48, label %51, label %49

49:                                               ; preds = %41
  %50 = load ptr, ptr %47, align 8, !tbaa !118
  store ptr %50, ptr %46, align 8, !tbaa !10
  br label %77

51:                                               ; preds = %41
  %52 = tail call align 16 ptr @llvm.threadlocal.address.p0(ptr align 16 @class_next)
  %53 = getelementptr inbounds [40 x ptr], ptr %52, i64 0, i64 %45
  %54 = load ptr, ptr %53, align 8, !tbaa !10
  %55 = icmp eq ptr %54, null
  br i1 %55, label %64, label %56

56:                                               ; preds = %51
  %57 = tail call align 16 ptr @llvm.threadlocal.address.p0(ptr align 16 @class_end)
  %58 = getelementptr inbounds [40 x ptr], ptr %57, i64 0, i64 %45
  %59 = load ptr, ptr %58, align 8, !tbaa !10
  %60 = ptrtoint ptr %59 to i64
  %61 = ptrtoint ptr %54 to i64
  %62 = sub i64 %60, %61
  %63 = icmp ult i64 %62, %42
  br i1 %63, label %64, label %74

64:                                               ; preds = %56, %51
  %65 = tail call fastcc ptr @take_pages(i64 noundef 1) #44
  %66 = icmp eq ptr %65, null
  br i1 %66, label %77, label %67

67:                                               ; preds = %64
  store i32 %43, ptr %65, align 4, !tbaa !120
  %68 = getelementptr inbounds %struct.PageHeader, ptr %65, i64 0, i32 1
  store i32 1, ptr %68, align 4, !tbaa !122
  %69 = getelementptr inbounds i8, ptr %65, i64 16
  store ptr %69, ptr %53, align 8, !tbaa !10
  %70 = getelementptr inbounds i8, ptr %65, i64 65536
  %71 = tail call align 16 ptr @llvm.threadlocal.address.p0(ptr align 16 @class_end)
  %72 = getelementptr inbounds [40 x ptr], ptr %71, i64 0, i64 %45
  store ptr %70, ptr %72, align 8, !tbaa !10
  %73 = load ptr, ptr %53, align 8, !tbaa !10
  br label %74

74:                                               ; preds = %67, %56
  %75 = phi ptr [ %73, %67 ], [ %54, %56 ]
  %76 = getelementptr inbounds i8, ptr %75, i64 %42
  store ptr %76, ptr %53, align 8, !tbaa !10
  br label %77

77:                                               ; preds = %49, %74, %64, %3, %7, %10
  %78 = phi ptr [ null, %3 ], [ %13, %10 ], [ null, %7 ], [ %47, %49 ], [ %75, %74 ], [ null, %64 ]
  ret ptr %78
}

; Function Attrs: nounwind optsize
define void @fa_heap_free(ptr noundef %0) local_unnamed_addr #2 {
  %2 = ptrtoint ptr %0 to i64
  %3 = and i64 %2, -65536
  %4 = inttoptr i64 %3 to ptr
  %5 = load i32, ptr %4, align 65536, !tbaa !120
  %6 = icmp eq i32 %5, 65535
  br i1 %6, label %7, label %11

7:                                                ; preds = %1
  %8 = getelementptr inbounds %struct.PageHeader, ptr %4, i64 0, i32 1
  %9 = load i32, ptr %8, align 4, !tbaa !122
  %10 = zext i32 %9 to i64
  tail call fastcc void @give_pages(ptr noundef nonnull %4, i64 noundef %10) #44
  br label %16

11:                                               ; preds = %1
  %12 = tail call align 16 ptr @llvm.threadlocal.address.p0(ptr align 16 @free_blocks)
  %13 = zext i32 %5 to i64
  %14 = getelementptr inbounds [40 x ptr], ptr %12, i64 0, i64 %13
  %15 = load ptr, ptr %14, align 8, !tbaa !10
  store ptr %15, ptr %0, align 8, !tbaa !118
  store ptr %0, ptr %14, align 8, !tbaa !10
  br label %16

16:                                               ; preds = %11, %7
  ret void
}

; Function Attrs: noinline nounwind optsize
define internal fastcc void @give_pages(ptr noundef %0, i64 noundef %1) unnamed_addr #14 {
  %3 = atomicrmw xchg ptr @spans_lock, i32 1 acquire, align 4
  %4 = icmp eq i32 %3, 0
  br i1 %4, label %14, label %8

5:                                                ; preds = %11, %8
  %6 = atomicrmw xchg ptr @spans_lock, i32 1 acquire, align 4
  %7 = icmp eq i32 %6, 0
  br i1 %7, label %14, label %8, !llvm.loop !5

8:                                                ; preds = %2, %5
  %9 = load atomic i32, ptr @spans_lock monotonic, align 4
  %10 = icmp eq i32 %9, 0
  br i1 %10, label %5, label %11

11:                                               ; preds = %8, %11
  tail call void @llvm.x86.sse2.pause()
  %12 = load atomic i32, ptr @spans_lock monotonic, align 4
  %13 = icmp eq i32 %12, 0
  br i1 %13, label %5, label %11, !llvm.loop !7

14:                                               ; preds = %5, %2
  %15 = getelementptr inbounds %struct.FreeSpan, ptr %0, i64 0, i32 1
  store i64 %1, ptr %15, align 8, !tbaa !124
  br label %16

16:                                               ; preds = %16, %14
  %17 = phi ptr [ null, %14 ], [ %19, %16 ]
  %18 = phi ptr [ @free_spans, %14 ], [ %19, %16 ]
  %19 = load ptr, ptr %18, align 8, !tbaa !10
  %20 = icmp ne ptr %19, null
  %21 = icmp ult ptr %19, %0
  %22 = and i1 %20, %21
  br i1 %22, label %16, label %23, !llvm.loop !126

23:                                               ; preds = %16
  store ptr %19, ptr %0, align 8, !tbaa !127
  %24 = shl i64 %1, 16
  %25 = getelementptr inbounds i8, ptr %0, i64 %24
  %26 = icmp eq ptr %25, %19
  %27 = select i1 %20, i1 %26, i1 false
  br i1 %27, label %28, label %33

28:                                               ; preds = %23
  %29 = getelementptr inbounds %struct.FreeSpan, ptr %19, i64 0, i32 1
  %30 = load i64, ptr %29, align 8, !tbaa !124
  %31 = add i64 %30, %1
  store i64 %31, ptr %15, align 8, !tbaa !124
  %32 = load ptr, ptr %19, align 8, !tbaa !127
  store ptr %32, ptr %0, align 8, !tbaa !127
  br label %33

33:                                               ; preds = %28, %23
  %34 = phi ptr [ %32, %28 ], [ %19, %23 ]
  %35 = phi i64 [ %31, %28 ], [ %1, %23 ]
  %36 = icmp eq ptr %17, null
  br i1 %36, label %46, label %37

37:                                               ; preds = %33
  %38 = getelementptr inbounds %struct.FreeSpan, ptr %17, i64 0, i32 1
  %39 = load i64, ptr %38, align 8, !tbaa !124
  %40 = shl i64 %39, 16
  %41 = getelementptr inbounds i8, ptr %17, i64 %40
  %42 = icmp eq ptr %41, %0
  br i1 %42, label %43, label %45

43:                                               ; preds = %37
  %44 = add i64 %39, %35
  store i64 %44, ptr %38, align 8, !tbaa !124
  store ptr %34, ptr %17, align 8, !tbaa !127
  br label %47

45:                                               ; preds = %37
  store ptr %0, ptr %17, align 8, !tbaa !127
  br label %47

46:                                               ; preds = %33
  store ptr %0, ptr @free_spans, align 8, !tbaa !10
  br label %47

47:                                               ; preds = %43, %45, %46
  store atomic i32 0, ptr @spans_lock release, align 4
  ret void
}

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define void @fa_arena_enter() local_unnamed_addr #21 {
  %1 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @tree_arena_depth)
  %2 = load i64, ptr %1, align 8, !tbaa !8
  %3 = add i64 %2, 1
  store i64 %3, ptr %1, align 8, !tbaa !8
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define void @fa_arena_enter_owned(ptr noundef %0) local_unnamed_addr #24 {
  %2 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @tree_arena_depth)
  %3 = load i64, ptr %2, align 8, !tbaa !8
  %4 = icmp eq i64 %3, 0
  br i1 %4, label %5, label %15

5:                                                ; preds = %1
  %6 = icmp eq ptr %0, null
  br i1 %6, label %12, label %7

7:                                                ; preds = %5
  %8 = ptrtoint ptr %0 to i64
  %9 = and i64 %8, -65536
  %10 = inttoptr i64 %9 to ptr
  %11 = load ptr, ptr %10, align 65536, !tbaa !128
  br label %12

12:                                               ; preds = %5, %7
  %13 = phi ptr [ %11, %7 ], [ null, %5 ]
  %14 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @tree_arena)
  store ptr %13, ptr %14, align 8, !tbaa !10
  br label %15

15:                                               ; preds = %12, %1
  %16 = add i64 %3, 1
  store i64 %16, ptr %2, align 8, !tbaa !8
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_arena_drop(ptr noundef %0) local_unnamed_addr #2 {
  %2 = icmp eq ptr %0, null
  br i1 %2, label %14, label %3

3:                                                ; preds = %1
  %4 = ptrtoint ptr %0 to i64
  %5 = and i64 %4, -65536
  %6 = inttoptr i64 %5 to ptr
  %7 = load ptr, ptr %6, align 65536, !tbaa !128
  %8 = icmp eq ptr %7, null
  br i1 %8, label %14, label %9

9:                                                ; preds = %3
  %10 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @tree_arena)
  %11 = load ptr, ptr %10, align 8, !tbaa !10
  %12 = icmp eq ptr %7, %11
  br i1 %12, label %14, label %13

13:                                               ; preds = %9
  tail call fastcc void @release_arena(ptr noundef nonnull %7) #44
  br label %14

14:                                               ; preds = %1, %13, %9, %3
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @release_arena(ptr noundef %0) unnamed_addr #2 {
  %2 = atomicrmw sub ptr @live_tree_arenas, i64 1 monotonic, align 8
  %3 = getelementptr inbounds %struct.FaArena, ptr %0, i64 0, i32 4
  %4 = load ptr, ptr %3, align 8, !tbaa !10
  %5 = icmp eq ptr %4, null
  br i1 %5, label %8, label %9

6:                                                ; preds = %9
  %7 = icmp eq ptr %0, null
  br i1 %7, label %22, label %8

8:                                                ; preds = %1, %6
  br label %17

9:                                                ; preds = %1, %9
  %10 = phi ptr [ %15, %9 ], [ %4, %1 ]
  %11 = getelementptr inbounds %struct.FaArenaCleanup, ptr %10, i64 0, i32 1
  %12 = load ptr, ptr %11, align 8, !tbaa !130
  %13 = getelementptr inbounds %struct.FaArenaCleanup, ptr %10, i64 0, i32 2
  %14 = load ptr, ptr %13, align 8, !tbaa !132
  tail call void %12(ptr noundef %14) #45
  %15 = load ptr, ptr %10, align 8, !tbaa !10
  %16 = icmp eq ptr %15, null
  br i1 %16, label %6, label %9, !llvm.loop !133

17:                                               ; preds = %8, %17
  %18 = phi ptr [ %20, %17 ], [ %0, %8 ]
  %19 = getelementptr inbounds %struct.FaArena, ptr %18, i64 0, i32 1
  %20 = load ptr, ptr %19, align 8, !tbaa !134
  tail call fastcc void @give_pages(ptr noundef nonnull %18, i64 noundef 1) #44
  %21 = icmp eq ptr %20, null
  br i1 %21, label %22, label %17, !llvm.loop !135

22:                                               ; preds = %17, %6
  ret void
}

; Function Attrs: alwaysinline nounwind optsize
define void @fa_arena_leave(ptr noundef %0) local_unnamed_addr #13 {
  %2 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @tree_arena_depth)
  %3 = load i64, ptr %2, align 8, !tbaa !8
  %4 = add i64 %3, -1
  store i64 %4, ptr %2, align 8, !tbaa !8
  %5 = icmp eq i64 %4, 0
  br i1 %5, label %6, label %21

6:                                                ; preds = %1
  %7 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @tree_arena)
  %8 = load ptr, ptr %7, align 8, !tbaa !10
  store ptr null, ptr %7, align 8, !tbaa !10
  %9 = icmp eq ptr %8, null
  br i1 %9, label %21, label %10

10:                                               ; preds = %6
  %11 = icmp eq ptr %0, null
  br i1 %11, label %17, label %12

12:                                               ; preds = %10
  %13 = ptrtoint ptr %0 to i64
  %14 = and i64 %13, -65536
  %15 = inttoptr i64 %14 to ptr
  %16 = load ptr, ptr %15, align 65536, !tbaa !128
  br label %17

17:                                               ; preds = %10, %12
  %18 = phi ptr [ %16, %12 ], [ null, %10 ]
  %19 = icmp eq ptr %18, %8
  br i1 %19, label %21, label %20

20:                                               ; preds = %17
  tail call fastcc void @release_arena(ptr noundef nonnull %8) #44
  br label %21

21:                                               ; preds = %6, %17, %20, %1
  ret void
}

; Function Attrs: alwaysinline nounwind optsize
define ptr @fa_arena_alloc(i64 noundef %0, i64 noundef %1) local_unnamed_addr #13 {
  %3 = add i64 %0, 7
  %4 = and i64 %3, -8
  %5 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @tree_arena)
  %6 = load ptr, ptr %5, align 8, !tbaa !10
  %7 = icmp eq ptr %6, null
  br i1 %7, label %29, label %8, !prof !47

8:                                                ; preds = %2
  %9 = icmp ult i64 %1, 9
  %10 = getelementptr inbounds %struct.FaArena, ptr %6, i64 0, i32 2
  %11 = load ptr, ptr %10, align 8, !tbaa !136
  %12 = ptrtoint ptr %11 to i64
  %13 = sub i64 0, %12
  %14 = add i64 %1, -1
  %15 = and i64 %14, %13
  %16 = select i1 %9, i64 0, i64 %15
  %17 = getelementptr inbounds %struct.FaArena, ptr %6, i64 0, i32 3
  %18 = load ptr, ptr %17, align 8, !tbaa !137
  %19 = ptrtoint ptr %18 to i64
  %20 = sub i64 %19, %12
  %21 = icmp ule i64 %16, %20
  %22 = sub i64 %20, %16
  %23 = icmp ule i64 %4, %22
  %24 = and i1 %21, %23
  br i1 %24, label %25, label %29, !prof !77

25:                                               ; preds = %8
  %26 = getelementptr inbounds %struct.FaArena, ptr %6, i64 0, i32 2
  %27 = getelementptr inbounds i8, ptr %11, i64 %16
  %28 = getelementptr inbounds i8, ptr %27, i64 %4
  store ptr %28, ptr %26, align 8, !tbaa !136
  br label %31

29:                                               ; preds = %8, %2
  %30 = tail call fastcc ptr @arena_refill(i64 noundef %4, i64 noundef %1) #44
  br label %31

31:                                               ; preds = %25, %29
  %32 = phi ptr [ %30, %29 ], [ %27, %25 ]
  ret ptr %32
}

; Function Attrs: noinline nounwind optsize
define internal fastcc ptr @arena_refill(i64 noundef %0, i64 noundef %1) unnamed_addr #14 {
  %3 = add i64 %1, 47
  %4 = sub i64 0, %1
  %5 = and i64 %3, %4
  %6 = tail call fastcc ptr @take_pages(i64 noundef 1) #44
  %7 = icmp eq ptr %6, null
  %8 = icmp ugt i64 %5, 65536
  %9 = or i1 %8, %7
  %10 = sub nuw nsw i64 65536, %5
  %11 = icmp ult i64 %10, %0
  %12 = select i1 %9, i1 true, i1 %11
  br i1 %12, label %13, label %14

13:                                               ; preds = %2
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @arena_refill.message, i64 noundef 20) #45
  tail call void @fa_exit(i32 noundef 101) #46
  unreachable

14:                                               ; preds = %2
  %15 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @tree_arena)
  %16 = load ptr, ptr %15, align 8, !tbaa !10
  %17 = icmp eq ptr %16, null
  br i1 %17, label %18, label %22

18:                                               ; preds = %14
  %19 = getelementptr inbounds %struct.FaArena, ptr %6, i64 0, i32 1
  store ptr null, ptr %19, align 8, !tbaa !134
  %20 = getelementptr inbounds %struct.FaArena, ptr %6, i64 0, i32 4
  store ptr null, ptr %20, align 8, !tbaa !138
  store ptr %6, ptr %15, align 8, !tbaa !10
  %21 = atomicrmw add ptr @live_tree_arenas, i64 1 monotonic, align 8
  br label %26

22:                                               ; preds = %14
  %23 = getelementptr inbounds %struct.FaArena, ptr %16, i64 0, i32 1
  %24 = load ptr, ptr %23, align 8, !tbaa !134
  %25 = getelementptr inbounds %struct.FaArena, ptr %6, i64 0, i32 1
  store ptr %24, ptr %25, align 8, !tbaa !134
  store ptr %6, ptr %23, align 8, !tbaa !134
  br label %26

26:                                               ; preds = %22, %18
  %27 = phi ptr [ %6, %18 ], [ %16, %22 ]
  store ptr %27, ptr %6, align 8, !tbaa !128
  %28 = getelementptr inbounds i8, ptr %6, i64 %5
  %29 = getelementptr inbounds i8, ptr %28, i64 %0
  %30 = getelementptr inbounds %struct.FaArena, ptr %27, i64 0, i32 2
  store ptr %29, ptr %30, align 8, !tbaa !136
  %31 = getelementptr inbounds i8, ptr %6, i64 65536
  %32 = getelementptr inbounds %struct.FaArena, ptr %27, i64 0, i32 3
  store ptr %31, ptr %32, align 8, !tbaa !137
  ret ptr %28
}

; Function Attrs: cold noinline nounwind optsize
define void @fa_arena_cleanup(ptr noundef %0, ptr noundef %1) local_unnamed_addr #19 {
  %3 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @tree_arena)
  %4 = load ptr, ptr %3, align 8, !tbaa !10
  %5 = icmp eq ptr %4, null
  br i1 %5, label %17, label %6, !prof !47

6:                                                ; preds = %2
  %7 = getelementptr inbounds %struct.FaArena, ptr %4, i64 0, i32 3
  %8 = load ptr, ptr %7, align 8, !tbaa !137
  %9 = getelementptr inbounds %struct.FaArena, ptr %4, i64 0, i32 2
  %10 = load ptr, ptr %9, align 8, !tbaa !136
  %11 = ptrtoint ptr %8 to i64
  %12 = ptrtoint ptr %10 to i64
  %13 = sub i64 %11, %12
  %14 = icmp ugt i64 %13, 23
  br i1 %14, label %15, label %17, !prof !77

15:                                               ; preds = %6
  %16 = getelementptr inbounds i8, ptr %10, i64 24
  store ptr %16, ptr %9, align 8, !tbaa !136
  br label %19

17:                                               ; preds = %6, %2
  %18 = tail call fastcc ptr @arena_refill(i64 noundef 24, i64 noundef 8) #44
  br label %19

19:                                               ; preds = %15, %17
  %20 = phi ptr [ %18, %17 ], [ %10, %15 ]
  %21 = load ptr, ptr %3, align 8, !tbaa !10
  %22 = getelementptr inbounds %struct.FaArena, ptr %21, i64 0, i32 4
  %23 = load ptr, ptr %22, align 8, !tbaa !138
  store ptr %23, ptr %20, align 8, !tbaa !139
  %24 = getelementptr inbounds %struct.FaArenaCleanup, ptr %20, i64 0, i32 1
  store ptr %1, ptr %24, align 8, !tbaa !130
  %25 = getelementptr inbounds %struct.FaArenaCleanup, ptr %20, i64 0, i32 2
  store ptr %0, ptr %25, align 8, !tbaa !132
  %26 = load ptr, ptr %3, align 8, !tbaa !10
  %27 = getelementptr inbounds %struct.FaArena, ptr %26, i64 0, i32 4
  store ptr %20, ptr %27, align 8, !tbaa !138
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_release_builder() local_unnamed_addr #2 {
  %1 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %2 = load ptr, ptr %1, align 8, !tbaa !10
  %3 = icmp eq ptr %2, null
  br i1 %3, label %8, label %4

4:                                                ; preds = %0
  %5 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_capacity)
  %6 = load i64, ptr %5, align 8, !tbaa !8
  tail call void @fa_free(ptr noundef nonnull %2, i64 noundef %6, i64 noundef 1) #45
  store ptr null, ptr %1, align 8, !tbaa !10
  store i64 0, ptr %5, align 8, !tbaa !8
  %7 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %7, align 8, !tbaa !8
  br label %8

8:                                                ; preds = %4, %0
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_leak_check() local_unnamed_addr #2 {
  tail call void @fa_flush() #44
  %1 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %2 = load ptr, ptr %1, align 8, !tbaa !10
  %3 = icmp eq ptr %2, null
  br i1 %3, label %7, label %4

4:                                                ; preds = %0
  %5 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_capacity)
  %6 = load i64, ptr %5, align 8, !tbaa !8
  tail call void @fa_free(ptr noundef nonnull %2, i64 noundef %6, i64 noundef 1) #45
  store ptr null, ptr %1, align 8, !tbaa !10
  store i64 0, ptr %5, align 8, !tbaa !8
  br label %7

7:                                                ; preds = %4, %0
  %8 = load ptr, ptr @argument_items, align 8, !tbaa !10
  %9 = icmp eq ptr %8, null
  br i1 %9, label %14, label %10

10:                                               ; preds = %7
  %11 = load i64, ptr @argument_count, align 8, !tbaa !8
  %12 = mul i64 %11, 24
  %13 = add i64 %12, 24
  tail call void @fa_free(ptr noundef nonnull %8, i64 noundef %13, i64 noundef 8) #45
  store ptr null, ptr @argument_items, align 8, !tbaa !10
  br label %14

14:                                               ; preds = %10, %7
  %15 = load i64, ptr @fa_live_blocks, align 8, !tbaa !8
  %16 = icmp ne i64 %15, 0
  %17 = load i64, ptr @live_tree_arenas, align 8
  %18 = icmp ne i64 %17, 0
  %19 = select i1 %16, i1 true, i1 %18
  br i1 %19, label %20, label %25

20:                                               ; preds = %14
  %21 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %21, align 8, !tbaa !8
  tail call fastcc void @append_c(ptr noundef nonnull @.str.13) #44
  %22 = load i64, ptr @fa_live_blocks, align 8, !tbaa !8
  %23 = load i64, ptr @live_tree_arenas, align 8, !tbaa !8
  %24 = add i64 %23, %22
  tail call void @fa_b_u64(i64 noundef %24) #44
  tail call fastcc void @append_c(ptr noundef nonnull @.str.14) #44
  tail call void @fa_b_line(i32 noundef 2) #44
  tail call void @fa_exit(i32 noundef 102) #46
  unreachable

25:                                               ; preds = %14
  ret void
}

; Function Attrs: nounwind optsize
define noundef ptr @fa_pool_new(i64 noundef %0) local_unnamed_addr #2 {
  %2 = tail call ptr @fa_alloc(i64 noundef 56, i64 noundef 8) #45
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(56) %2, i8 0, i64 40, i1 false)
  %3 = tail call i64 @llvm.smax.i64(i64 %0, i64 4)
  %4 = trunc i64 %3 to i32
  %5 = getelementptr inbounds %struct.FaPool, ptr %2, i64 0, i32 6
  store i32 %4, ptr %5, align 8, !tbaa !140
  %6 = shl i64 %3, 6
  %7 = add i64 %6, 104
  br label %8

8:                                                ; preds = %8, %1
  %9 = phi i64 [ 1, %1 ], [ %12, %8 ]
  %10 = shl i64 %9, 16
  %11 = icmp ult i64 %10, %7
  %12 = shl i64 %9, 1
  br i1 %11, label %8, label %13, !llvm.loop !142

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
  store i32 %31, ptr %32, align 4, !tbaa !143
  %33 = trunc i64 %9 to i32
  %34 = getelementptr inbounds %struct.FaPool, ptr %2, i64 0, i32 8
  store i32 %33, ptr %34, align 8, !tbaa !144
  %35 = trunc i64 %25 to i32
  %36 = getelementptr inbounds %struct.FaPool, ptr %2, i64 0, i32 9
  store i32 %35, ptr %36, align 4, !tbaa !145
  ret ptr %2
}

; Function Attrs: nounwind optsize
define noundef ptr @fa_columnar_new(i64 noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #2 {
  %4 = tail call ptr @fa_alloc(i64 noundef 56, i64 noundef 8) #45
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(56) %4, i8 0, i64 40, i1 false)
  %5 = getelementptr inbounds %struct.FaPool, ptr %4, i64 0, i32 6
  store i32 4, ptr %5, align 8, !tbaa !140
  %6 = trunc i64 %0 to i32
  %7 = getelementptr inbounds %struct.FaPool, ptr %4, i64 0, i32 7
  store i32 %6, ptr %7, align 4, !tbaa !143
  %8 = trunc i64 %1 to i32
  %9 = getelementptr inbounds %struct.FaPool, ptr %4, i64 0, i32 8
  store i32 %8, ptr %9, align 8, !tbaa !144
  %10 = trunc i64 %2 to i32
  %11 = getelementptr inbounds %struct.FaPool, ptr %4, i64 0, i32 9
  store i32 %10, ptr %11, align 4, !tbaa !145
  ret ptr %4
}

; Function Attrs: nounwind optsize
define ptr @fa_pool_add_slot(ptr nocapture noundef %0) local_unnamed_addr #2 {
  %2 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 4
  %3 = load i32, ptr %2, align 8, !tbaa !146
  %4 = add i32 %3, 63
  %5 = icmp ult i32 %4, 64
  br i1 %5, label %29, label %6

6:                                                ; preds = %1
  %7 = lshr i32 %4, 6
  %8 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 2
  %9 = load ptr, ptr %8, align 8, !tbaa !147
  %10 = zext nneg i32 %7 to i64
  br label %11

11:                                               ; preds = %6, %16
  %12 = phi i64 [ 0, %6 ], [ %17, %16 ]
  %13 = getelementptr inbounds i64, ptr %9, i64 %12
  %14 = load i64, ptr %13, align 8, !tbaa !8
  %15 = icmp eq i64 %14, 0
  br i1 %15, label %16, label %19

16:                                               ; preds = %11
  %17 = add nuw nsw i64 %12, 1
  %18 = icmp eq i64 %17, %10
  br i1 %18, label %29, label %11, !llvm.loop !148

19:                                               ; preds = %11
  %20 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 1
  %21 = load ptr, ptr %20, align 8, !tbaa !149
  %22 = shl i64 %12, 6
  %23 = tail call i64 @llvm.cttz.i64(i64 %14, i1 true), !range !111
  %24 = and i64 %22, 4294967232
  %25 = or disjoint i64 %24, %23
  %26 = getelementptr inbounds ptr, ptr %21, i64 %25
  %27 = load ptr, ptr %26, align 8, !tbaa !10
  %28 = icmp eq ptr %27, null
  br i1 %28, label %29, label %31

29:                                               ; preds = %16, %1, %19
  %30 = tail call fastcc ptr @pool_new_chunk(ptr noundef nonnull %0) #48
  br label %31

31:                                               ; preds = %29, %19
  %32 = phi ptr [ %30, %29 ], [ %27, %19 ]
  %33 = getelementptr inbounds %struct.PoolChunk, ptr %32, i64 0, i32 4
  %34 = load i32, ptr %33, align 4, !tbaa !25
  %35 = icmp eq i32 %34, 0
  br i1 %35, label %47, label %36

36:                                               ; preds = %31
  %37 = add i32 %34, -1
  %38 = load ptr, ptr %32, align 8, !tbaa !10
  %39 = zext i32 %37 to i64
  %40 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 6
  %41 = load i32, ptr %40, align 8, !tbaa !140
  %42 = zext i32 %41 to i64
  %43 = mul nuw i64 %42, %39
  %44 = getelementptr inbounds i8, ptr %38, i64 %43
  %45 = load i32, ptr %44, align 1
  store i32 %45, ptr %33, align 4, !tbaa !25
  %46 = icmp eq i32 %45, 0
  br label %51

47:                                               ; preds = %31
  %48 = getelementptr inbounds %struct.PoolChunk, ptr %32, i64 0, i32 3
  %49 = load i32, ptr %48, align 8, !tbaa !25
  %50 = add i32 %49, 1
  store i32 %50, ptr %48, align 8, !tbaa !25
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
  %60 = load i64, ptr %59, align 8, !tbaa !8
  %61 = or i64 %56, %60
  store i64 %61, ptr %59, align 8, !tbaa !8
  %62 = getelementptr inbounds %struct.PoolChunk, ptr %32, i64 0, i32 2
  %63 = load i32, ptr %62, align 4, !tbaa !25
  %64 = add i32 %63, 1
  store i32 %64, ptr %62, align 4, !tbaa !25
  %65 = load i64, ptr %0, align 8, !tbaa !150
  %66 = add nsw i64 %65, 1
  store i64 %66, ptr %0, align 8, !tbaa !150
  br i1 %52, label %67, label %87

67:                                               ; preds = %51
  %68 = getelementptr inbounds %struct.PoolChunk, ptr %32, i64 0, i32 3
  %69 = load i32, ptr %68, align 8, !tbaa !25
  %70 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 7
  %71 = load i32, ptr %70, align 4, !tbaa !143
  %72 = icmp eq i32 %69, %71
  br i1 %72, label %73, label %87

73:                                               ; preds = %67
  %74 = getelementptr inbounds %struct.PoolChunk, ptr %32, i64 0, i32 1
  %75 = load i32, ptr %74, align 8, !tbaa !25
  %76 = and i32 %75, 63
  %77 = zext nneg i32 %76 to i64
  %78 = shl nuw i64 1, %77
  %79 = xor i64 %78, -1
  %80 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 2
  %81 = load ptr, ptr %80, align 8, !tbaa !147
  %82 = lshr i32 %75, 6
  %83 = zext nneg i32 %82 to i64
  %84 = getelementptr inbounds i64, ptr %81, i64 %83
  %85 = load i64, ptr %84, align 8, !tbaa !8
  %86 = and i64 %85, %79
  store i64 %86, ptr %84, align 8, !tbaa !8
  br label %87

87:                                               ; preds = %73, %67, %51
  %88 = load ptr, ptr %32, align 8, !tbaa !10
  %89 = zext i32 %53 to i64
  %90 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 6
  %91 = load i32, ptr %90, align 8, !tbaa !140
  %92 = zext i32 %91 to i64
  %93 = mul nuw i64 %92, %89
  %94 = getelementptr inbounds i8, ptr %88, i64 %93
  ret ptr %94
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.cttz.i64(i64, i1 immarg) #7

; Function Attrs: cold noinline nounwind optsize
define internal fastcc noundef ptr @pool_new_chunk(ptr nocapture noundef %0) unnamed_addr #19 {
  %2 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 3
  %3 = load ptr, ptr %2, align 8, !tbaa !151
  store ptr null, ptr %2, align 8, !tbaa !151
  %4 = icmp eq ptr %3, null
  br i1 %4, label %5, label %41

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 8
  %7 = load i32, ptr %6, align 8, !tbaa !144
  %8 = zext i32 %7 to i64
  %9 = icmp eq i32 %7, 1
  br i1 %9, label %10, label %12

10:                                               ; preds = %5
  %11 = tail call fastcc ptr @take_pages(i64 noundef 1) #44
  br label %35

12:                                               ; preds = %5
  %13 = shl nuw nsw i64 %8, 1
  %14 = add nsw i64 %13, -1
  %15 = tail call fastcc ptr @take_pages(i64 noundef %14) #44
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
  tail call fastcc void @give_pages(ptr noundef nonnull %15, i64 noundef %26) #44
  br label %29

29:                                               ; preds = %28, %17
  %30 = xor i64 %26, -1
  %31 = add nsw i64 %30, %8
  %32 = icmp eq i64 %31, 0
  br i1 %32, label %35, label %33

33:                                               ; preds = %29
  %34 = getelementptr inbounds i8, ptr %24, i64 %18
  tail call fastcc void @give_pages(ptr noundef %34, i64 noundef %31) #44
  br label %35

35:                                               ; preds = %10, %29, %33
  %36 = phi ptr [ %11, %10 ], [ %24, %33 ], [ %24, %29 ]
  %37 = icmp eq ptr %36, null
  br i1 %37, label %38, label %39

38:                                               ; preds = %12, %35
  tail call fastcc void @out_of_memory() #47
  unreachable

39:                                               ; preds = %35
  %40 = atomicrmw add ptr @fa_live_blocks, i64 1 monotonic, align 8
  br label %41

41:                                               ; preds = %39, %1
  %42 = phi ptr [ %36, %39 ], [ %3, %1 ]
  %43 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 7
  %44 = load i32, ptr %43, align 4, !tbaa !143
  %45 = add i32 %44, 63
  %46 = lshr i32 %45, 3
  %47 = and i32 %46, 536870904
  %48 = add nuw nsw i32 %47, 24
  %49 = zext nneg i32 %48 to i64
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(1) %42, i8 0, i64 %49, i1 false)
  %50 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 9
  %51 = load i32, ptr %50, align 4, !tbaa !145
  %52 = zext i32 %51 to i64
  %53 = getelementptr inbounds i8, ptr %42, i64 %52
  store ptr %53, ptr %42, align 8, !tbaa !10
  %54 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 4
  %55 = load i32, ptr %54, align 8, !tbaa !146
  %56 = icmp eq i32 %55, 0
  br i1 %56, label %71, label %57

57:                                               ; preds = %41
  %58 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 1
  %59 = load ptr, ptr %58, align 8, !tbaa !149
  %60 = zext i32 %55 to i64
  br label %61

61:                                               ; preds = %57, %66
  %62 = phi i64 [ 0, %57 ], [ %67, %66 ]
  %63 = getelementptr inbounds ptr, ptr %59, i64 %62
  %64 = load ptr, ptr %63, align 8, !tbaa !10
  %65 = icmp eq ptr %64, null
  br i1 %65, label %69, label %66

66:                                               ; preds = %61
  %67 = add nuw nsw i64 %62, 1
  %68 = icmp eq i64 %67, %60
  br i1 %68, label %74, label %61, !llvm.loop !152

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
  %77 = load i32, ptr %76, align 4, !tbaa !153
  %78 = icmp eq i32 %55, %77
  br i1 %78, label %79, label %119

79:                                               ; preds = %74
  %80 = shl i32 %55, 1
  %81 = select i1 %56, i32 4, i32 %80
  %82 = zext i32 %81 to i64
  %83 = shl nuw nsw i64 %82, 3
  %84 = tail call ptr @fa_alloc(i64 noundef %83, i64 noundef 8) #45
  %85 = add i32 %81, 63
  %86 = lshr i32 %85, 3
  %87 = and i32 %86, 536870904
  %88 = zext nneg i32 %87 to i64
  %89 = tail call ptr @fa_alloc(i64 noundef %88, i64 noundef 8) #45
  tail call void @llvm.memset.p0.i64(ptr align 8 %84, i8 0, i64 %83, i1 false)
  tail call void @llvm.memset.p0.i64(ptr align 8 %89, i8 0, i64 %88, i1 false)
  %90 = load i32, ptr %76, align 4, !tbaa !153
  %91 = icmp eq i32 %90, 0
  br i1 %91, label %115, label %92

92:                                               ; preds = %79
  %93 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 1
  %94 = load ptr, ptr %93, align 8, !tbaa !149
  %95 = load i32, ptr %54, align 8, !tbaa !146
  %96 = zext i32 %95 to i64
  %97 = shl nuw nsw i64 %96, 3
  tail call fastcc void @copy_bytes(ptr noundef %84, ptr noundef %94, i64 noundef %97) #44
  %98 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 2
  %99 = load ptr, ptr %98, align 8, !tbaa !147
  %100 = load i32, ptr %76, align 4, !tbaa !153
  %101 = add i32 %100, 63
  %102 = lshr i32 %101, 3
  %103 = and i32 %102, 536870904
  %104 = zext nneg i32 %103 to i64
  tail call fastcc void @copy_bytes(ptr noundef %89, ptr noundef %99, i64 noundef %104) #44
  %105 = load ptr, ptr %93, align 8, !tbaa !149
  %106 = load i32, ptr %76, align 4, !tbaa !153
  %107 = zext i32 %106 to i64
  %108 = shl nuw nsw i64 %107, 3
  tail call void @fa_free(ptr noundef %105, i64 noundef %108, i64 noundef 8) #45
  %109 = load ptr, ptr %98, align 8, !tbaa !147
  %110 = load i32, ptr %76, align 4, !tbaa !153
  %111 = add i32 %110, 63
  %112 = lshr i32 %111, 3
  %113 = and i32 %112, 536870904
  %114 = zext nneg i32 %113 to i64
  tail call void @fa_free(ptr noundef %109, i64 noundef %114, i64 noundef 8) #45
  br label %115

115:                                              ; preds = %92, %79
  %116 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 1
  store ptr %84, ptr %116, align 8, !tbaa !149
  %117 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 2
  store ptr %89, ptr %117, align 8, !tbaa !147
  store i32 %81, ptr %76, align 4, !tbaa !153
  %118 = load i32, ptr %54, align 8, !tbaa !146
  br label %119

119:                                              ; preds = %115, %74
  %120 = phi i32 [ %118, %115 ], [ %55, %74 ]
  %121 = add i32 %120, 1
  store i32 %121, ptr %54, align 8, !tbaa !146
  br label %122

122:                                              ; preds = %119, %71
  %123 = phi i32 [ %75, %119 ], [ %72, %71 ]
  %124 = getelementptr inbounds %struct.PoolChunk, ptr %42, i64 0, i32 1
  store i32 %123, ptr %124, align 8, !tbaa !25
  %125 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 1
  %126 = load ptr, ptr %125, align 8, !tbaa !149
  %127 = zext i32 %123 to i64
  %128 = getelementptr inbounds ptr, ptr %126, i64 %127
  store ptr %42, ptr %128, align 8, !tbaa !10
  %129 = and i32 %123, 63
  %130 = zext nneg i32 %129 to i64
  %131 = shl nuw i64 1, %130
  %132 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 2
  %133 = load ptr, ptr %132, align 8, !tbaa !147
  %134 = lshr i32 %123, 6
  %135 = zext nneg i32 %134 to i64
  %136 = getelementptr inbounds i64, ptr %133, i64 %135
  %137 = load i64, ptr %136, align 8, !tbaa !8
  %138 = or i64 %137, %131
  store i64 %138, ptr %136, align 8, !tbaa !8
  ret ptr %42
}

; Function Attrs: nounwind optsize
define void @fa_pool_remove(ptr nocapture noundef %0, ptr noundef %1) local_unnamed_addr #2 {
  %3 = ptrtoint ptr %1 to i64
  %4 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 8
  %5 = load i32, ptr %4, align 8, !tbaa !144
  %6 = zext i32 %5 to i64
  %7 = mul nsw i64 %6, -65536
  %8 = and i64 %7, %3
  %9 = inttoptr i64 %8 to ptr
  %10 = load ptr, ptr %9, align 65536, !tbaa !10
  %11 = ptrtoint ptr %10 to i64
  %12 = sub i64 %3, %11
  %13 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 6
  %14 = load i32, ptr %13, align 8, !tbaa !140
  %15 = zext i32 %14 to i64
  %16 = sdiv i64 %12, %15
  %17 = trunc i64 %16 to i32
  %18 = and i64 %16, 63
  %19 = shl nuw i64 1, %18
  %20 = xor i64 %19, -1
  %21 = lshr i64 %16, 6
  %22 = and i64 %21, 67108863
  %23 = getelementptr inbounds %struct.PoolChunk, ptr %9, i64 0, i32 5, i64 %22
  %24 = load i64, ptr %23, align 8, !tbaa !8
  %25 = and i64 %24, %20
  store i64 %25, ptr %23, align 8, !tbaa !8
  %26 = getelementptr inbounds %struct.PoolChunk, ptr %9, i64 0, i32 4
  %27 = load i32, ptr %26, align 4
  store i32 %27, ptr %1, align 1
  %28 = add i32 %17, 1
  store i32 %28, ptr %26, align 4, !tbaa !25
  %29 = getelementptr inbounds %struct.PoolChunk, ptr %9, i64 0, i32 2
  %30 = load i32, ptr %29, align 4, !tbaa !25
  %31 = add i32 %30, -1
  store i32 %31, ptr %29, align 4, !tbaa !25
  %32 = load i64, ptr %0, align 8, !tbaa !150
  %33 = add nsw i64 %32, -1
  store i64 %33, ptr %0, align 8, !tbaa !150
  %34 = getelementptr inbounds %struct.PoolChunk, ptr %9, i64 0, i32 1
  %35 = load i32, ptr %34, align 8, !tbaa !25
  %36 = icmp eq i32 %31, 0
  br i1 %36, label %37, label %78

37:                                               ; preds = %2
  %38 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 1
  %39 = load ptr, ptr %38, align 8, !tbaa !149
  %40 = zext i32 %35 to i64
  %41 = getelementptr inbounds ptr, ptr %39, i64 %40
  store ptr null, ptr %41, align 8, !tbaa !10
  %42 = and i32 %35, 63
  %43 = zext nneg i32 %42 to i64
  %44 = shl nuw i64 1, %43
  %45 = xor i64 %44, -1
  %46 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 2
  %47 = load ptr, ptr %46, align 8, !tbaa !147
  %48 = lshr i32 %35, 6
  %49 = zext nneg i32 %48 to i64
  %50 = getelementptr inbounds i64, ptr %47, i64 %49
  %51 = load i64, ptr %50, align 8, !tbaa !8
  %52 = and i64 %51, %45
  store i64 %52, ptr %50, align 8, !tbaa !8
  %53 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 4
  %54 = load i32, ptr %53, align 8, !tbaa !146
  %55 = icmp eq i32 %54, 0
  br i1 %55, label %69, label %56

56:                                               ; preds = %37
  %57 = load ptr, ptr %38, align 8, !tbaa !149
  %58 = zext i32 %54 to i64
  br label %59

59:                                               ; preds = %56, %66
  %60 = phi i64 [ %58, %56 ], [ %61, %66 ]
  %61 = add nsw i64 %60, -1
  %62 = and i64 %61, 4294967295
  %63 = getelementptr inbounds ptr, ptr %57, i64 %62
  %64 = load ptr, ptr %63, align 8, !tbaa !10
  %65 = icmp eq ptr %64, null
  br i1 %65, label %66, label %69

66:                                               ; preds = %59
  %67 = trunc i64 %61 to i32
  store i32 %67, ptr %53, align 8, !tbaa !146
  %68 = icmp eq i32 %67, 0
  br i1 %68, label %69, label %59, !llvm.loop !154

69:                                               ; preds = %59, %66, %37
  %70 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 3
  %71 = load ptr, ptr %70, align 8, !tbaa !151
  %72 = icmp eq ptr %71, null
  br i1 %72, label %73, label %74

73:                                               ; preds = %69
  store ptr %9, ptr %70, align 8, !tbaa !151
  br label %89

74:                                               ; preds = %69
  %75 = load i32, ptr %4, align 8, !tbaa !144
  %76 = zext i32 %75 to i64
  tail call fastcc void @give_pages(ptr noundef nonnull %9, i64 noundef %76) #44
  %77 = atomicrmw sub ptr @fa_live_blocks, i64 1 monotonic, align 8
  br label %89

78:                                               ; preds = %2
  %79 = and i32 %35, 63
  %80 = zext nneg i32 %79 to i64
  %81 = shl nuw i64 1, %80
  %82 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 2
  %83 = load ptr, ptr %82, align 8, !tbaa !147
  %84 = lshr i32 %35, 6
  %85 = zext nneg i32 %84 to i64
  %86 = getelementptr inbounds i64, ptr %83, i64 %85
  %87 = load i64, ptr %86, align 8, !tbaa !8
  %88 = or i64 %87, %81
  store i64 %88, ptr %86, align 8, !tbaa !8
  br label %89

89:                                               ; preds = %73, %74, %78
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(read, inaccessiblemem: none)
define i64 @fa_pool_seek(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #25 {
  %3 = lshr i64 %1, 32
  %4 = trunc i64 %3 to i32
  %5 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 4
  %6 = load i32, ptr %5, align 8, !tbaa !146
  %7 = icmp ugt i32 %6, %4
  br i1 %7, label %8, label %49

8:                                                ; preds = %2
  %9 = trunc i64 %1 to i32
  %10 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 1
  %11 = load ptr, ptr %10, align 8, !tbaa !149
  %12 = zext i32 %6 to i64
  br label %13

13:                                               ; preds = %8, %46
  %14 = phi i64 [ %3, %8 ], [ %47, %46 ]
  %15 = phi i32 [ %9, %8 ], [ 0, %46 ]
  %16 = getelementptr inbounds ptr, ptr %11, i64 %14
  %17 = load ptr, ptr %16, align 8, !tbaa !10
  %18 = icmp eq ptr %17, null
  br i1 %18, label %46, label %19

19:                                               ; preds = %13
  %20 = getelementptr inbounds %struct.PoolChunk, ptr %17, i64 0, i32 3
  %21 = load i32, ptr %20, align 8, !tbaa !25
  %22 = icmp ult i32 %15, %21
  br i1 %22, label %23, label %46

23:                                               ; preds = %19, %38
  %24 = phi i32 [ %40, %38 ], [ %15, %19 ]
  %25 = lshr i32 %24, 6
  %26 = zext nneg i32 %25 to i64
  %27 = getelementptr inbounds %struct.PoolChunk, ptr %17, i64 0, i32 5, i64 %26
  %28 = load i64, ptr %27, align 8, !tbaa !8
  %29 = and i32 %24, 63
  %30 = zext nneg i32 %29 to i64
  %31 = lshr i64 %28, %30
  %32 = icmp eq i64 %31, 0
  br i1 %32, label %38, label %33

33:                                               ; preds = %23
  %34 = tail call i64 @llvm.cttz.i64(i64 %31, i1 true), !range !111
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
  br i1 %48, label %49, label %13, !llvm.loop !155

49:                                               ; preds = %46, %2, %42
  %50 = phi i64 [ %45, %42 ], [ -1, %2 ], [ -1, %46 ]
  ret i64 %50
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(read, inaccessiblemem: none)
define ptr @fa_pool_at(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #26 {
  %3 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 1
  %4 = load ptr, ptr %3, align 8, !tbaa !149
  %5 = lshr i64 %1, 32
  %6 = getelementptr inbounds ptr, ptr %4, i64 %5
  %7 = load ptr, ptr %6, align 8, !tbaa !10
  %8 = load ptr, ptr %7, align 8, !tbaa !10
  %9 = and i64 %1, 4294967295
  %10 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 6
  %11 = load i32, ptr %10, align 8, !tbaa !140
  %12 = zext i32 %11 to i64
  %13 = mul nuw i64 %9, %12
  %14 = getelementptr inbounds i8, ptr %8, i64 %13
  ret ptr %14
}

; Function Attrs: nounwind optsize
define void @fa_pool_clear(ptr nocapture noundef %0) local_unnamed_addr #2 {
  tail call fastcc void @pool_release_chunks(ptr noundef %0) #44
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @pool_release_chunks(ptr nocapture noundef %0) unnamed_addr #2 {
  %2 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 4
  %3 = load i32, ptr %2, align 8, !tbaa !146
  %4 = icmp eq i32 %3, 0
  br i1 %4, label %8, label %5

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 1
  %7 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 8
  br label %12

8:                                                ; preds = %24, %1
  %9 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 5
  %10 = load i32, ptr %9, align 4, !tbaa !153
  %11 = icmp eq i32 %10, 0
  br i1 %11, label %41, label %29

12:                                               ; preds = %5, %24
  %13 = phi i32 [ %3, %5 ], [ %25, %24 ]
  %14 = phi i64 [ 0, %5 ], [ %26, %24 ]
  %15 = load ptr, ptr %6, align 8, !tbaa !149
  %16 = getelementptr inbounds ptr, ptr %15, i64 %14
  %17 = load ptr, ptr %16, align 8, !tbaa !10
  %18 = icmp eq ptr %17, null
  br i1 %18, label %24, label %19

19:                                               ; preds = %12
  %20 = load i32, ptr %7, align 8, !tbaa !144
  %21 = zext i32 %20 to i64
  tail call fastcc void @give_pages(ptr noundef nonnull %17, i64 noundef %21) #44
  %22 = atomicrmw sub ptr @fa_live_blocks, i64 1 monotonic, align 8
  %23 = load i32, ptr %2, align 8, !tbaa !146
  br label %24

24:                                               ; preds = %12, %19
  %25 = phi i32 [ %13, %12 ], [ %23, %19 ]
  %26 = add nuw nsw i64 %14, 1
  %27 = zext i32 %25 to i64
  %28 = icmp ult i64 %26, %27
  br i1 %28, label %12, label %8, !llvm.loop !156

29:                                               ; preds = %8
  %30 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 1
  %31 = load ptr, ptr %30, align 8, !tbaa !149
  %32 = zext i32 %10 to i64
  %33 = shl nuw nsw i64 %32, 3
  tail call void @llvm.memset.p0.i64(ptr align 8 %31, i8 0, i64 %33, i1 false)
  %34 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 2
  %35 = load ptr, ptr %34, align 8, !tbaa !147
  %36 = load i32, ptr %9, align 4, !tbaa !153
  %37 = add i32 %36, 63
  %38 = lshr i32 %37, 3
  %39 = and i32 %38, 536870904
  %40 = zext nneg i32 %39 to i64
  tail call void @llvm.memset.p0.i64(ptr align 8 %35, i8 0, i64 %40, i1 false)
  br label %41

41:                                               ; preds = %29, %8
  store i32 0, ptr %2, align 8, !tbaa !146
  store i64 0, ptr %0, align 8, !tbaa !150
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_pool_free(ptr noundef %0) local_unnamed_addr #2 {
  tail call fastcc void @pool_release_chunks(ptr noundef %0) #44
  %2 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 3
  %3 = load ptr, ptr %2, align 8, !tbaa !151
  %4 = icmp eq ptr %3, null
  br i1 %4, label %10, label %5

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 8
  %7 = load i32, ptr %6, align 8, !tbaa !144
  %8 = zext i32 %7 to i64
  tail call fastcc void @give_pages(ptr noundef nonnull %3, i64 noundef %8) #44
  %9 = atomicrmw sub ptr @fa_live_blocks, i64 1 monotonic, align 8
  br label %10

10:                                               ; preds = %5, %1
  %11 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 5
  %12 = load i32, ptr %11, align 4, !tbaa !153
  %13 = icmp eq i32 %12, 0
  br i1 %13, label %26, label %14

14:                                               ; preds = %10
  %15 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 1
  %16 = load ptr, ptr %15, align 8, !tbaa !149
  %17 = zext i32 %12 to i64
  %18 = shl nuw nsw i64 %17, 3
  tail call void @fa_free(ptr noundef %16, i64 noundef %18, i64 noundef 8) #45
  %19 = getelementptr inbounds %struct.FaPool, ptr %0, i64 0, i32 2
  %20 = load ptr, ptr %19, align 8, !tbaa !147
  %21 = load i32, ptr %11, align 4, !tbaa !153
  %22 = add i32 %21, 63
  %23 = lshr i32 %22, 3
  %24 = and i32 %23, 536870904
  %25 = zext nneg i32 %24 to i64
  tail call void @fa_free(ptr noundef %20, i64 noundef %25, i64 noundef 8) #45
  br label %26

26:                                               ; preds = %14, %10
  tail call void @fa_free(ptr noundef nonnull %0, i64 noundef 56, i64 noundef 8) #45
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(read, inaccessiblemem: none)
define i32 @fa_links_has(ptr noundef readonly %0, ptr noundef readonly %1) local_unnamed_addr #25 {
  %3 = tail call fastcc i64 @links_find(ptr noundef %0, ptr noundef %1) #44
  %4 = icmp sgt i64 %3, -1
  %5 = zext i1 %4 to i32
  ret i32 %5
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(read, inaccessiblemem: none)
define internal fastcc i64 @links_find(ptr noundef readonly %0, ptr noundef %1) unnamed_addr #25 {
  %3 = icmp eq ptr %0, null
  br i1 %3, label %56, label %4

4:                                                ; preds = %2
  %5 = getelementptr inbounds %struct.FaLinks, ptr %0, i64 0, i32 4
  %6 = load ptr, ptr %5, align 8, !tbaa !157
  %7 = icmp eq ptr %6, null
  br i1 %7, label %8, label %23

8:                                                ; preds = %4
  %9 = getelementptr inbounds %struct.FaLinks, ptr %0, i64 0, i32 2
  %10 = load i64, ptr %9, align 8, !tbaa !159
  %11 = icmp sgt i64 %10, 0
  br i1 %11, label %12, label %56

12:                                               ; preds = %8
  %13 = getelementptr inbounds %struct.FaLinks, ptr %0, i64 0, i32 1
  %14 = load ptr, ptr %13, align 8, !tbaa !160
  br label %15

15:                                               ; preds = %12, %20
  %16 = phi i64 [ 0, %12 ], [ %21, %20 ]
  %17 = getelementptr inbounds ptr, ptr %14, i64 %16
  %18 = load ptr, ptr %17, align 8, !tbaa !10
  %19 = icmp eq ptr %18, %1
  br i1 %19, label %56, label %20

20:                                               ; preds = %15
  %21 = add nuw nsw i64 %16, 1
  %22 = icmp eq i64 %21, %10
  br i1 %22, label %56, label %15, !llvm.loop !161

23:                                               ; preds = %4
  %24 = getelementptr inbounds %struct.FaLinks, ptr %0, i64 0, i32 5
  %25 = load i64, ptr %24, align 8, !tbaa !162
  %26 = ptrtoint ptr %1 to i64
  %27 = lshr i64 %26, 33
  %28 = xor i64 %27, %26
  %29 = mul i64 %28, -49064778989728563
  %30 = lshr i64 %29, 33
  %31 = xor i64 %30, %29
  %32 = and i64 %31, %25
  %33 = getelementptr inbounds i32, ptr %6, i64 %32
  %34 = load i32, ptr %33, align 4, !tbaa !25
  %35 = icmp eq i32 %34, 0
  br i1 %35, label %56, label %36

36:                                               ; preds = %23
  %37 = getelementptr inbounds %struct.FaLinks, ptr %0, i64 0, i32 1
  %38 = load ptr, ptr %37, align 8, !tbaa !160
  br label %45

39:                                               ; preds = %45
  %40 = add i64 %47, 1
  %41 = and i64 %40, %25
  %42 = getelementptr inbounds i32, ptr %6, i64 %41
  %43 = load i32, ptr %42, align 4, !tbaa !25
  %44 = icmp eq i32 %43, 0
  br i1 %44, label %56, label %45

45:                                               ; preds = %36, %39
  %46 = phi i32 [ %34, %36 ], [ %43, %39 ]
  %47 = phi i64 [ %32, %36 ], [ %41, %39 ]
  %48 = add i32 %46, -1
  %49 = zext i32 %48 to i64
  %50 = getelementptr inbounds ptr, ptr %38, i64 %49
  %51 = load ptr, ptr %50, align 8, !tbaa !10
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
define noundef i32 @fa_links_add(ptr nocapture noundef %0, ptr noundef %1) local_unnamed_addr #2 {
  %3 = load ptr, ptr %0, align 8, !tbaa !10
  %4 = icmp eq ptr %3, null
  br i1 %4, label %5, label %7

5:                                                ; preds = %2
  %6 = tail call ptr @fa_alloc(i64 noundef 48, i64 noundef 8) #45
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %6, i8 0, i64 48, i1 false)
  store ptr %6, ptr %0, align 8, !tbaa !10
  br label %10

7:                                                ; preds = %2
  %8 = tail call fastcc i64 @links_find(ptr noundef nonnull %3, ptr noundef %1) #44
  %9 = icmp sgt i64 %8, -1
  br i1 %9, label %152, label %10

10:                                               ; preds = %7, %5
  %11 = phi ptr [ %6, %5 ], [ %3, %7 ]
  %12 = getelementptr inbounds %struct.FaLinks, ptr %11, i64 0, i32 2
  %13 = load i64, ptr %12, align 8, !tbaa !159
  %14 = getelementptr inbounds %struct.FaLinks, ptr %11, i64 0, i32 3
  %15 = load i64, ptr %14, align 8, !tbaa !163
  %16 = icmp eq i64 %13, %15
  br i1 %16, label %17, label %118

17:                                               ; preds = %10
  %18 = icmp sgt i64 %13, 0
  br i1 %18, label %19, label %40

19:                                               ; preds = %17
  %20 = load i64, ptr %11, align 8, !tbaa !164
  %21 = shl nsw i64 %20, 1
  %22 = icmp sgt i64 %21, %13
  br i1 %22, label %40, label %23

23:                                               ; preds = %19
  %24 = getelementptr inbounds %struct.FaLinks, ptr %11, i64 0, i32 1
  br label %26

25:                                               ; preds = %36
  store i64 %37, ptr %12, align 8, !tbaa !159
  br label %58

26:                                               ; preds = %23, %36
  %27 = phi i64 [ 0, %23 ], [ %38, %36 ]
  %28 = phi i64 [ 0, %23 ], [ %37, %36 ]
  %29 = load ptr, ptr %24, align 8, !tbaa !160
  %30 = getelementptr inbounds ptr, ptr %29, i64 %27
  %31 = load ptr, ptr %30, align 8, !tbaa !10
  %32 = icmp eq ptr %31, null
  br i1 %32, label %36, label %33

33:                                               ; preds = %26
  %34 = getelementptr inbounds ptr, ptr %29, i64 %28
  store ptr %31, ptr %34, align 8, !tbaa !10
  %35 = add nsw i64 %28, 1
  br label %36

36:                                               ; preds = %26, %33
  %37 = phi i64 [ %35, %33 ], [ %28, %26 ]
  %38 = add nuw nsw i64 %27, 1
  %39 = icmp eq i64 %38, %13
  br i1 %39, label %25, label %26, !llvm.loop !165

40:                                               ; preds = %19, %17
  %41 = icmp eq i64 %13, 0
  %42 = shl nsw i64 %13, 1
  %43 = select i1 %41, i64 4, i64 %42
  %44 = shl i64 %43, 3
  %45 = tail call ptr @fa_alloc(i64 noundef %44, i64 noundef 8) #45
  %46 = load i64, ptr %14, align 8, !tbaa !163
  %47 = icmp sgt i64 %46, 0
  br i1 %47, label %48, label %56

48:                                               ; preds = %40
  %49 = getelementptr inbounds %struct.FaLinks, ptr %11, i64 0, i32 1
  %50 = load ptr, ptr %49, align 8, !tbaa !160
  %51 = load i64, ptr %12, align 8, !tbaa !159
  %52 = shl i64 %51, 3
  tail call fastcc void @copy_bytes(ptr noundef %45, ptr noundef %50, i64 noundef %52) #44
  %53 = load ptr, ptr %49, align 8, !tbaa !160
  %54 = load i64, ptr %14, align 8, !tbaa !163
  %55 = shl i64 %54, 3
  tail call void @fa_free(ptr noundef %53, i64 noundef %55, i64 noundef 8) #45
  br label %56

56:                                               ; preds = %48, %40
  %57 = getelementptr inbounds %struct.FaLinks, ptr %11, i64 0, i32 1
  store ptr %45, ptr %57, align 8, !tbaa !160
  store i64 %43, ptr %14, align 8, !tbaa !163
  br label %58

58:                                               ; preds = %56, %25
  %59 = phi i64 [ %43, %56 ], [ %13, %25 ]
  %60 = getelementptr inbounds %struct.FaLinks, ptr %11, i64 0, i32 4
  %61 = load ptr, ptr %60, align 8, !tbaa !157
  %62 = icmp eq ptr %61, null
  br i1 %62, label %69, label %63

63:                                               ; preds = %58
  %64 = getelementptr inbounds %struct.FaLinks, ptr %11, i64 0, i32 5
  %65 = load i64, ptr %64, align 8, !tbaa !162
  %66 = shl i64 %65, 2
  %67 = add i64 %66, 4
  tail call void @fa_free(ptr noundef nonnull %61, i64 noundef %67, i64 noundef 4) #45
  store ptr null, ptr %60, align 8, !tbaa !157
  %68 = load i64, ptr %14, align 8, !tbaa !163
  br label %69

69:                                               ; preds = %63, %58
  %70 = phi i64 [ %68, %63 ], [ %59, %58 ]
  %71 = icmp slt i64 %70, 9
  br i1 %71, label %72, label %74

72:                                               ; preds = %69
  %73 = load i64, ptr %12, align 8, !tbaa !159
  br label %118

74:                                               ; preds = %69
  %75 = shl nuw nsw i64 %70, 1
  br label %76

76:                                               ; preds = %76, %74
  %77 = phi i64 [ %79, %76 ], [ 16, %74 ]
  %78 = icmp slt i64 %77, %75
  %79 = shl nsw i64 %77, 1
  br i1 %78, label %76, label %80, !llvm.loop !166

80:                                               ; preds = %76
  %81 = add nsw i64 %77, -1
  %82 = getelementptr inbounds %struct.FaLinks, ptr %11, i64 0, i32 5
  store i64 %81, ptr %82, align 8, !tbaa !162
  %83 = shl i64 %77, 2
  %84 = tail call ptr @fa_alloc(i64 noundef %83, i64 noundef 4) #45
  store ptr %84, ptr %60, align 8, !tbaa !157
  tail call void @llvm.memset.p0.i64(ptr align 4 %84, i8 0, i64 %83, i1 false)
  %85 = load i64, ptr %12, align 8, !tbaa !159
  %86 = icmp sgt i64 %85, 0
  br i1 %86, label %87, label %118

87:                                               ; preds = %80
  %88 = getelementptr inbounds %struct.FaLinks, ptr %11, i64 0, i32 1
  %89 = load ptr, ptr %88, align 8, !tbaa !160
  br label %90

90:                                               ; preds = %115, %87
  %91 = phi i64 [ 0, %87 ], [ %116, %115 ]
  %92 = getelementptr inbounds ptr, ptr %89, i64 %91
  %93 = load ptr, ptr %92, align 8, !tbaa !10
  %94 = icmp eq ptr %93, null
  br i1 %94, label %115, label %95

95:                                               ; preds = %90
  %96 = load i64, ptr %82, align 8, !tbaa !162
  %97 = ptrtoint ptr %93 to i64
  %98 = lshr i64 %97, 33
  %99 = xor i64 %98, %97
  %100 = mul i64 %99, -49064778989728563
  %101 = lshr i64 %100, 33
  %102 = xor i64 %101, %100
  %103 = load ptr, ptr %60, align 8, !tbaa !157
  br label %104

104:                                              ; preds = %104, %95
  %105 = phi i64 [ %102, %95 ], [ %110, %104 ]
  %106 = and i64 %105, %96
  %107 = getelementptr inbounds i32, ptr %103, i64 %106
  %108 = load i32, ptr %107, align 4, !tbaa !25
  %109 = icmp eq i32 %108, 0
  %110 = add i64 %106, 1
  br i1 %109, label %111, label %104, !llvm.loop !167

111:                                              ; preds = %104
  %112 = getelementptr inbounds i32, ptr %103, i64 %106
  %113 = trunc i64 %91 to i32
  %114 = add i32 %113, 1
  store i32 %114, ptr %112, align 4, !tbaa !25
  br label %115

115:                                              ; preds = %111, %90
  %116 = add nuw nsw i64 %91, 1
  %117 = icmp eq i64 %116, %85
  br i1 %117, label %118, label %90, !llvm.loop !168

118:                                              ; preds = %115, %72, %80, %10
  %119 = phi i64 [ %73, %72 ], [ %85, %80 ], [ %13, %10 ], [ %85, %115 ]
  %120 = getelementptr inbounds %struct.FaLinks, ptr %11, i64 0, i32 1
  %121 = load ptr, ptr %120, align 8, !tbaa !160
  %122 = getelementptr inbounds ptr, ptr %121, i64 %119
  store ptr %1, ptr %122, align 8, !tbaa !10
  %123 = add nsw i64 %119, 1
  store i64 %123, ptr %12, align 8, !tbaa !159
  %124 = load i64, ptr %11, align 8, !tbaa !164
  %125 = add nsw i64 %124, 1
  store i64 %125, ptr %11, align 8, !tbaa !164
  %126 = getelementptr inbounds %struct.FaLinks, ptr %11, i64 0, i32 4
  %127 = load ptr, ptr %126, align 8, !tbaa !157
  %128 = icmp eq ptr %127, null
  br i1 %128, label %152, label %129

129:                                              ; preds = %118
  %130 = getelementptr inbounds %struct.FaLinks, ptr %11, i64 0, i32 5
  %131 = load i64, ptr %130, align 8, !tbaa !162
  %132 = load ptr, ptr %120, align 8, !tbaa !160
  %133 = getelementptr inbounds ptr, ptr %132, i64 %119
  %134 = load ptr, ptr %133, align 8, !tbaa !10
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
  %145 = load i32, ptr %144, align 4, !tbaa !25
  %146 = icmp eq i32 %145, 0
  %147 = add i64 %143, 1
  br i1 %146, label %148, label %141, !llvm.loop !167

148:                                              ; preds = %141
  %149 = getelementptr inbounds i32, ptr %127, i64 %143
  %150 = trunc i64 %119 to i32
  %151 = add i32 %150, 1
  store i32 %151, ptr %149, align 4, !tbaa !25
  br label %152

152:                                              ; preds = %118, %148, %7
  %153 = phi i32 [ 0, %7 ], [ 1, %148 ], [ 1, %118 ]
  ret i32 %153
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define noundef i32 @fa_links_delete(ptr noundef %0, ptr noundef %1) local_unnamed_addr #17 {
  %3 = tail call fastcc i64 @links_find(ptr noundef %0, ptr noundef %1) #44
  %4 = icmp slt i64 %3, 0
  br i1 %4, label %75, label %5

5:                                                ; preds = %2
  %6 = getelementptr inbounds %struct.FaLinks, ptr %0, i64 0, i32 1
  %7 = load ptr, ptr %6, align 8, !tbaa !160
  %8 = getelementptr inbounds ptr, ptr %7, i64 %3
  store ptr null, ptr %8, align 8, !tbaa !10
  %9 = load i64, ptr %0, align 8, !tbaa !164
  %10 = add nsw i64 %9, -1
  store i64 %10, ptr %0, align 8, !tbaa !164
  %11 = getelementptr inbounds %struct.FaLinks, ptr %0, i64 0, i32 4
  %12 = load ptr, ptr %11, align 8, !tbaa !157
  %13 = icmp eq ptr %12, null
  br i1 %13, label %71, label %14

14:                                               ; preds = %5
  %15 = getelementptr inbounds %struct.FaLinks, ptr %0, i64 0, i32 5
  %16 = load i64, ptr %15, align 8, !tbaa !162
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
  %29 = load i32, ptr %28, align 4, !tbaa !25
  %30 = icmp eq i32 %29, %24
  %31 = add i64 %27, 1
  br i1 %30, label %32, label %25, !llvm.loop !169

32:                                               ; preds = %25
  %33 = and i64 %31, %16
  %34 = getelementptr inbounds i32, ptr %12, i64 %33
  %35 = load i32, ptr %34, align 4, !tbaa !25
  %36 = icmp eq i32 %35, 0
  br i1 %36, label %68, label %37

37:                                               ; preds = %32
  %38 = load ptr, ptr %6, align 8, !tbaa !160
  br label %39

39:                                               ; preds = %37, %61
  %40 = phi i32 [ %35, %37 ], [ %66, %61 ]
  %41 = phi i64 [ %33, %37 ], [ %64, %61 ]
  %42 = phi i64 [ %27, %37 ], [ %62, %61 ]
  %43 = add i32 %40, -1
  %44 = zext i32 %43 to i64
  %45 = getelementptr inbounds ptr, ptr %38, i64 %44
  %46 = load ptr, ptr %45, align 8, !tbaa !10
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
  store i32 %40, ptr %60, align 4, !tbaa !25
  br label %61

61:                                               ; preds = %39, %59
  %62 = phi i64 [ %41, %59 ], [ %42, %39 ]
  %63 = add i64 %41, 1
  %64 = and i64 %63, %16
  %65 = getelementptr inbounds i32, ptr %12, i64 %64
  %66 = load i32, ptr %65, align 4, !tbaa !25
  %67 = icmp eq i32 %66, 0
  br i1 %67, label %68, label %39

68:                                               ; preds = %61, %32
  %69 = phi i64 [ %27, %32 ], [ %62, %61 ]
  %70 = getelementptr inbounds i32, ptr %12, i64 %69
  store i32 0, ptr %70, align 4, !tbaa !25
  br label %71

71:                                               ; preds = %68, %5
  %72 = icmp eq i64 %10, 0
  br i1 %72, label %73, label %75

73:                                               ; preds = %71
  %74 = getelementptr inbounds %struct.FaLinks, ptr %0, i64 0, i32 2
  store i64 0, ptr %74, align 8, !tbaa !159
  br label %75

75:                                               ; preds = %71, %73, %2
  %76 = phi i32 [ 0, %2 ], [ 1, %73 ], [ 1, %71 ]
  ret i32 %76
}

; Function Attrs: nounwind optsize
define void @fa_links_free(ptr noundef %0) local_unnamed_addr #2 {
  %2 = icmp eq ptr %0, null
  br i1 %2, label %21, label %3

3:                                                ; preds = %1
  %4 = getelementptr inbounds %struct.FaLinks, ptr %0, i64 0, i32 4
  %5 = load ptr, ptr %4, align 8, !tbaa !157
  %6 = icmp eq ptr %5, null
  br i1 %6, label %12, label %7

7:                                                ; preds = %3
  %8 = getelementptr inbounds %struct.FaLinks, ptr %0, i64 0, i32 5
  %9 = load i64, ptr %8, align 8, !tbaa !162
  %10 = shl i64 %9, 2
  %11 = add i64 %10, 4
  tail call void @fa_free(ptr noundef nonnull %5, i64 noundef %11, i64 noundef 4) #45
  br label %12

12:                                               ; preds = %7, %3
  %13 = getelementptr inbounds %struct.FaLinks, ptr %0, i64 0, i32 3
  %14 = load i64, ptr %13, align 8, !tbaa !163
  %15 = icmp sgt i64 %14, 0
  br i1 %15, label %16, label %20

16:                                               ; preds = %12
  %17 = getelementptr inbounds %struct.FaLinks, ptr %0, i64 0, i32 1
  %18 = load ptr, ptr %17, align 8, !tbaa !160
  %19 = shl i64 %14, 3
  tail call void @fa_free(ptr noundef %18, i64 noundef %19, i64 noundef 8) #45
  br label %20

20:                                               ; preds = %16, %12
  tail call void @fa_free(ptr noundef nonnull %0, i64 noundef 48, i64 noundef 8) #45
  br label %21

21:                                               ; preds = %1, %20
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(read, inaccessiblemem: none)
define i64 @fa_links_seek(ptr noundef readonly %0, i64 noundef %1) local_unnamed_addr #25 {
  %3 = icmp eq ptr %0, null
  br i1 %3, label %19, label %4

4:                                                ; preds = %2
  %5 = getelementptr inbounds %struct.FaLinks, ptr %0, i64 0, i32 2
  %6 = load i64, ptr %5, align 8, !tbaa !159
  %7 = icmp ugt i64 %6, %1
  br i1 %7, label %8, label %19

8:                                                ; preds = %4
  %9 = getelementptr inbounds %struct.FaLinks, ptr %0, i64 0, i32 1
  %10 = load ptr, ptr %9, align 8, !tbaa !160
  br label %11

11:                                               ; preds = %8, %16
  %12 = phi i64 [ %1, %8 ], [ %17, %16 ]
  %13 = getelementptr inbounds ptr, ptr %10, i64 %12
  %14 = load ptr, ptr %13, align 8, !tbaa !10
  %15 = icmp eq ptr %14, null
  br i1 %15, label %16, label %19

16:                                               ; preds = %11
  %17 = add i64 %12, 1
  %18 = icmp eq i64 %17, %6
  br i1 %18, label %19, label %11, !llvm.loop !170

19:                                               ; preds = %11, %16, %4, %2
  %20 = phi i64 [ -1, %2 ], [ -1, %4 ], [ %12, %11 ], [ -1, %16 ]
  ret i64 %20
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(read, inaccessiblemem: none)
define ptr @fa_links_at(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #26 {
  %3 = getelementptr inbounds %struct.FaLinks, ptr %0, i64 0, i32 1
  %4 = load ptr, ptr %3, align 8, !tbaa !160
  %5 = getelementptr inbounds ptr, ptr %4, i64 %1
  %6 = load ptr, ptr %5, align 8, !tbaa !10
  ret ptr %6
}

; Function Attrs: nounwind optsize
define void @fa_relate_one(ptr noundef %0, i64 noundef %1, i64 noundef %2, ptr noundef %3, i32 noundef %4) local_unnamed_addr #2 {
  %6 = getelementptr inbounds i8, ptr %0, i64 %1
  %7 = load ptr, ptr %6, align 8, !tbaa !10
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
  %15 = load ptr, ptr %14, align 8, !tbaa !10
  %16 = tail call i32 @fa_links_delete(ptr noundef %15, ptr noundef nonnull %0) #44, !range !60
  br label %17

17:                                               ; preds = %13, %12
  store ptr %3, ptr %6, align 8, !tbaa !10
  %18 = icmp eq ptr %3, null
  br i1 %18, label %35, label %19

19:                                               ; preds = %17
  %20 = getelementptr inbounds i8, ptr %3, i64 %2
  %21 = tail call i32 @fa_links_add(ptr noundef nonnull %20, ptr noundef nonnull %0) #44, !range !60
  br label %35

22:                                               ; preds = %9
  br i1 %11, label %25, label %23

23:                                               ; preds = %22
  %24 = getelementptr inbounds i8, ptr %7, i64 %2
  store ptr null, ptr %24, align 8, !tbaa !10
  br label %25

25:                                               ; preds = %23, %22
  %26 = icmp eq ptr %3, null
  br i1 %26, label %34, label %27

27:                                               ; preds = %25
  %28 = getelementptr inbounds i8, ptr %3, i64 %2
  %29 = load ptr, ptr %28, align 8, !tbaa !10
  %30 = icmp eq ptr %29, null
  br i1 %30, label %33, label %31

31:                                               ; preds = %27
  %32 = getelementptr inbounds i8, ptr %29, i64 %1
  store ptr null, ptr %32, align 8, !tbaa !10
  br label %33

33:                                               ; preds = %31, %27
  store ptr %0, ptr %28, align 8, !tbaa !10
  br label %34

34:                                               ; preds = %33, %25
  store ptr %3, ptr %6, align 8, !tbaa !10
  br label %35

35:                                               ; preds = %17, %19, %5, %34
  ret void
}

; Function Attrs: nounwind optsize
define noundef i32 @fa_relate_add(ptr noundef %0, i64 noundef %1, i64 noundef %2, ptr noundef %3) local_unnamed_addr #2 {
  %5 = getelementptr inbounds i8, ptr %0, i64 %1
  %6 = tail call i32 @fa_links_add(ptr noundef %5, ptr noundef %3) #44, !range !60
  %7 = icmp eq i32 %6, 0
  br i1 %7, label %11, label %8

8:                                                ; preds = %4
  %9 = getelementptr inbounds i8, ptr %3, i64 %2
  %10 = tail call i32 @fa_links_add(ptr noundef %9, ptr noundef %0) #44, !range !60
  br label %11

11:                                               ; preds = %4, %8
  %12 = phi i32 [ 1, %8 ], [ 0, %4 ]
  ret i32 %12
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define noundef i32 @fa_relate_delete(ptr noundef %0, i64 noundef %1, i64 noundef %2, ptr noundef %3) local_unnamed_addr #17 {
  %5 = getelementptr inbounds i8, ptr %0, i64 %1
  %6 = load ptr, ptr %5, align 8, !tbaa !10
  %7 = tail call i32 @fa_links_delete(ptr noundef %6, ptr noundef %3) #44, !range !60
  %8 = icmp eq i32 %7, 0
  br i1 %8, label %13, label %9

9:                                                ; preds = %4
  %10 = getelementptr inbounds i8, ptr %3, i64 %2
  %11 = load ptr, ptr %10, align 8, !tbaa !10
  %12 = tail call i32 @fa_links_delete(ptr noundef %11, ptr noundef nonnull %0) #44, !range !60
  br label %13

13:                                               ; preds = %4, %9
  %14 = phi i32 [ 1, %9 ], [ 0, %4 ]
  ret i32 %14
}

; Function Attrs: nounwind optsize
define void @fa_relate_clear(ptr noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #2 {
  %4 = getelementptr inbounds i8, ptr %0, i64 %1
  %5 = load ptr, ptr %4, align 8, !tbaa !10
  %6 = icmp eq ptr %5, null
  br i1 %6, label %30, label %7

7:                                                ; preds = %3
  %8 = getelementptr inbounds %struct.FaLinks, ptr %5, i64 0, i32 2
  %9 = load i64, ptr %8, align 8, !tbaa !159
  %10 = icmp sgt i64 %9, 0
  br i1 %10, label %11, label %13

11:                                               ; preds = %7
  %12 = getelementptr inbounds %struct.FaLinks, ptr %5, i64 0, i32 1
  br label %14

13:                                               ; preds = %26, %7
  tail call void @fa_links_free(ptr noundef nonnull %5) #44
  store ptr null, ptr %4, align 8, !tbaa !10
  br label %30

14:                                               ; preds = %11, %26
  %15 = phi i64 [ %9, %11 ], [ %27, %26 ]
  %16 = phi i64 [ 0, %11 ], [ %28, %26 ]
  %17 = load ptr, ptr %12, align 8, !tbaa !160
  %18 = getelementptr inbounds ptr, ptr %17, i64 %16
  %19 = load ptr, ptr %18, align 8, !tbaa !10
  %20 = icmp eq ptr %19, null
  br i1 %20, label %26, label %21

21:                                               ; preds = %14
  %22 = getelementptr inbounds i8, ptr %19, i64 %2
  %23 = load ptr, ptr %22, align 8, !tbaa !10
  %24 = tail call i32 @fa_links_delete(ptr noundef %23, ptr noundef %0) #44, !range !60
  %25 = load i64, ptr %8, align 8, !tbaa !159
  br label %26

26:                                               ; preds = %21, %14
  %27 = phi i64 [ %25, %21 ], [ %15, %14 ]
  %28 = add nuw nsw i64 %16, 1
  %29 = icmp slt i64 %28, %27
  br i1 %29, label %14, label %13, !llvm.loop !171

30:                                               ; preds = %3, %13
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define void @fa_unlink(ptr noundef %0, i32 noundef %1, i32 noundef %2, i64 noundef %3, i64 noundef %4, ptr noundef %5) local_unnamed_addr #17 {
  %7 = icmp eq i32 %2, 0
  br i1 %7, label %42, label %8

8:                                                ; preds = %6
  %9 = getelementptr inbounds i8, ptr %0, i64 %3
  %10 = load ptr, ptr %9, align 8, !tbaa !10
  %11 = icmp eq i32 %1, 2
  br i1 %11, label %12, label %33

12:                                               ; preds = %8
  %13 = getelementptr inbounds %struct.FaLinks, ptr %10, i64 0, i32 2
  %14 = icmp eq ptr %10, null
  br i1 %14, label %89, label %15

15:                                               ; preds = %12
  %16 = getelementptr inbounds %struct.FaLinks, ptr %10, i64 0, i32 1
  %17 = load i64, ptr %13, align 8, !tbaa !159
  %18 = icmp sgt i64 %17, 0
  br i1 %18, label %19, label %89

19:                                               ; preds = %15, %29
  %20 = phi i64 [ %30, %29 ], [ 0, %15 ]
  %21 = load ptr, ptr %16, align 8, !tbaa !160
  %22 = getelementptr inbounds ptr, ptr %21, i64 %20
  %23 = load ptr, ptr %22, align 8, !tbaa !10
  %24 = icmp eq ptr %23, null
  br i1 %24, label %29, label %25

25:                                               ; preds = %19
  %26 = getelementptr inbounds i8, ptr %23, i64 %4
  %27 = load ptr, ptr %26, align 8, !tbaa !10
  %28 = tail call i32 @fa_links_delete(ptr noundef %27, ptr noundef %0) #44, !range !60
  br label %29

29:                                               ; preds = %25, %19
  %30 = add nuw nsw i64 %20, 1
  %31 = load i64, ptr %13, align 8, !tbaa !159
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
  %39 = load ptr, ptr %37, align 8, !tbaa !10
  %40 = tail call i32 @fa_links_delete(ptr noundef %39, ptr noundef nonnull %0) #44, !range !60
  br label %89

41:                                               ; preds = %35
  store ptr null, ptr %37, align 8, !tbaa !10
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
  %45 = load ptr, ptr %44, align 8, !tbaa !10
  %46 = getelementptr inbounds %struct.FaLinks, ptr %45, i64 0, i32 2
  %47 = icmp eq ptr %45, null
  br i1 %47, label %89, label %48

48:                                               ; preds = %43
  %49 = getelementptr inbounds %struct.FaLinks, ptr %45, i64 0, i32 1
  %50 = icmp eq i32 %1, 0
  %51 = load i64, ptr %46, align 8, !tbaa !159
  %52 = icmp sgt i64 %51, 0
  br i1 %52, label %53, label %89

53:                                               ; preds = %48, %65
  %54 = phi i64 [ %66, %65 ], [ 0, %48 ]
  %55 = load ptr, ptr %49, align 8, !tbaa !160
  %56 = getelementptr inbounds ptr, ptr %55, i64 %54
  %57 = load ptr, ptr %56, align 8, !tbaa !10
  %58 = icmp eq ptr %57, null
  br i1 %58, label %65, label %59

59:                                               ; preds = %53
  %60 = getelementptr inbounds i8, ptr %57, i64 %3
  br i1 %50, label %61, label %62

61:                                               ; preds = %59
  store ptr null, ptr %60, align 8, !tbaa !10
  br label %65

62:                                               ; preds = %59
  %63 = load ptr, ptr %60, align 8, !tbaa !10
  %64 = tail call i32 @fa_links_delete(ptr noundef %63, ptr noundef %0) #44, !range !60
  br label %65

65:                                               ; preds = %61, %62, %53
  %66 = add nuw nsw i64 %54, 1
  %67 = load i64, ptr %46, align 8, !tbaa !159
  %68 = icmp slt i64 %66, %67
  br i1 %68, label %53, label %89

69:                                               ; preds = %42
  %70 = getelementptr inbounds i8, ptr %0, i64 %4
  %71 = load ptr, ptr %70, align 8, !tbaa !10
  %72 = icmp eq ptr %71, null
  br i1 %72, label %89, label %73

73:                                               ; preds = %69
  %74 = getelementptr inbounds i8, ptr %71, i64 %3
  store ptr null, ptr %74, align 8, !tbaa !10
  br label %89

75:                                               ; preds = %42
  %76 = getelementptr inbounds i8, ptr %0, i64 %3
  %77 = load ptr, ptr %76, align 8, !tbaa !10
  %78 = icmp eq ptr %77, null
  %79 = icmp eq ptr %77, %0
  %80 = or i1 %78, %79
  br i1 %80, label %89, label %81

81:                                               ; preds = %75
  %82 = getelementptr inbounds i8, ptr %77, i64 %3
  store ptr null, ptr %82, align 8, !tbaa !10
  br label %89

83:                                               ; preds = %42
  %84 = icmp eq ptr %5, null
  br i1 %84, label %89, label %85

85:                                               ; preds = %83
  %86 = load ptr, ptr %5, align 8, !tbaa !10
  %87 = icmp eq ptr %86, %0
  br i1 %87, label %88, label %89

88:                                               ; preds = %85
  store ptr null, ptr %5, align 8, !tbaa !10
  br label %89

89:                                               ; preds = %29, %65, %15, %48, %12, %43, %75, %81, %69, %73, %38, %41, %33, %83, %85, %88
  ret void
}

; Function Attrs: nounwind optsize
define ptr @fa_address_map_new(i64 noundef %0) local_unnamed_addr #2 {
  %2 = tail call ptr @fa_alloc(i64 noundef 24, i64 noundef 8) #45
  %3 = shl nsw i64 %0, 1
  br label %4

4:                                                ; preds = %4, %1
  %5 = phi i64 [ 16, %1 ], [ %7, %4 ]
  %6 = icmp slt i64 %5, %3
  %7 = shl nsw i64 %5, 1
  br i1 %6, label %4, label %8, !llvm.loop !172

8:                                                ; preds = %4
  %9 = add nsw i64 %5, -1
  %10 = getelementptr inbounds %struct.FaAddressMap, ptr %2, i64 0, i32 2
  store i64 %9, ptr %10, align 8, !tbaa !173
  %11 = shl i64 %5, 3
  %12 = tail call ptr @fa_alloc(i64 noundef %11, i64 noundef 8) #45
  store ptr %12, ptr %2, align 8, !tbaa !175
  %13 = tail call ptr @fa_alloc(i64 noundef %11, i64 noundef 8) #45
  %14 = getelementptr inbounds %struct.FaAddressMap, ptr %2, i64 0, i32 1
  store ptr %13, ptr %14, align 8, !tbaa !176
  %15 = load ptr, ptr %2, align 8, !tbaa !175
  tail call void @llvm.memset.p0.i64(ptr align 8 %15, i8 0, i64 %11, i1 false)
  ret ptr %2
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define void @fa_address_map_put(ptr nocapture noundef readonly %0, ptr noundef %1, ptr noundef %2) local_unnamed_addr #17 {
  %4 = getelementptr inbounds %struct.FaAddressMap, ptr %0, i64 0, i32 2
  %5 = load i64, ptr %4, align 8, !tbaa !173
  %6 = ptrtoint ptr %1 to i64
  %7 = lshr i64 %6, 33
  %8 = xor i64 %7, %6
  %9 = mul i64 %8, -49064778989728563
  %10 = lshr i64 %9, 33
  %11 = xor i64 %10, %9
  %12 = load ptr, ptr %0, align 8, !tbaa !175
  br label %13

13:                                               ; preds = %13, %3
  %14 = phi i64 [ %11, %3 ], [ %21, %13 ]
  %15 = and i64 %14, %5
  %16 = getelementptr inbounds ptr, ptr %12, i64 %15
  %17 = load ptr, ptr %16, align 8, !tbaa !10
  %18 = icmp eq ptr %17, null
  %19 = icmp eq ptr %17, %1
  %20 = or i1 %18, %19
  %21 = add i64 %15, 1
  br i1 %20, label %22, label %13, !llvm.loop !177

22:                                               ; preds = %13
  %23 = getelementptr inbounds ptr, ptr %12, i64 %15
  store ptr %1, ptr %23, align 8, !tbaa !10
  %24 = getelementptr inbounds %struct.FaAddressMap, ptr %0, i64 0, i32 1
  %25 = load ptr, ptr %24, align 8, !tbaa !176
  %26 = getelementptr inbounds ptr, ptr %25, i64 %15
  store ptr %2, ptr %26, align 8, !tbaa !10
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(read, inaccessiblemem: none)
define ptr @fa_address_map_get(ptr nocapture noundef readonly %0, ptr noundef %1) local_unnamed_addr #25 {
  %3 = icmp eq ptr %1, null
  br i1 %3, label %33, label %4

4:                                                ; preds = %2
  %5 = getelementptr inbounds %struct.FaAddressMap, ptr %0, i64 0, i32 2
  %6 = load i64, ptr %5, align 8, !tbaa !173
  %7 = ptrtoint ptr %1 to i64
  %8 = lshr i64 %7, 33
  %9 = xor i64 %8, %7
  %10 = mul i64 %9, -49064778989728563
  %11 = lshr i64 %10, 33
  %12 = xor i64 %11, %10
  %13 = load ptr, ptr %0, align 8, !tbaa !175
  %14 = and i64 %12, %6
  %15 = getelementptr inbounds ptr, ptr %13, i64 %14
  %16 = load ptr, ptr %15, align 8, !tbaa !10
  %17 = icmp eq ptr %16, null
  br i1 %17, label %33, label %18

18:                                               ; preds = %4, %27
  %19 = phi ptr [ %31, %27 ], [ %16, %4 ]
  %20 = phi i64 [ %29, %27 ], [ %14, %4 ]
  %21 = icmp eq ptr %19, %1
  br i1 %21, label %22, label %27

22:                                               ; preds = %18
  %23 = getelementptr inbounds %struct.FaAddressMap, ptr %0, i64 0, i32 1
  %24 = load ptr, ptr %23, align 8, !tbaa !176
  %25 = getelementptr inbounds ptr, ptr %24, i64 %20
  %26 = load ptr, ptr %25, align 8, !tbaa !10
  br label %33

27:                                               ; preds = %18
  %28 = add i64 %20, 1
  %29 = and i64 %28, %6
  %30 = getelementptr inbounds ptr, ptr %13, i64 %29
  %31 = load ptr, ptr %30, align 8, !tbaa !10
  %32 = icmp eq ptr %31, null
  br i1 %32, label %33, label %18, !llvm.loop !178

33:                                               ; preds = %27, %4, %22, %2
  %34 = phi ptr [ null, %2 ], [ %26, %22 ], [ null, %4 ], [ null, %27 ]
  ret ptr %34
}

; Function Attrs: nounwind optsize
define ptr @fa_links_relink(ptr noundef readonly %0, ptr nocapture noundef readonly %1) local_unnamed_addr #2 {
  %3 = alloca ptr, align 8
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %3) #40
  store ptr null, ptr %3, align 8, !tbaa !10
  %4 = getelementptr inbounds %struct.FaLinks, ptr %0, i64 0, i32 2
  %5 = icmp eq ptr %0, null
  br i1 %5, label %12, label %6

6:                                                ; preds = %2
  %7 = getelementptr inbounds %struct.FaLinks, ptr %0, i64 0, i32 1
  %8 = load i64, ptr %4, align 8, !tbaa !159
  %9 = icmp sgt i64 %8, 0
  br i1 %9, label %14, label %10

10:                                               ; preds = %23, %6
  %11 = load ptr, ptr %3, align 8, !tbaa !10
  br label %12

12:                                               ; preds = %10, %2
  %13 = phi ptr [ %11, %10 ], [ null, %2 ]
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %3) #40
  ret ptr %13

14:                                               ; preds = %6, %23
  %15 = phi i64 [ %24, %23 ], [ 0, %6 ]
  %16 = load ptr, ptr %7, align 8, !tbaa !160
  %17 = getelementptr inbounds ptr, ptr %16, i64 %15
  %18 = load ptr, ptr %17, align 8, !tbaa !10
  %19 = tail call ptr @fa_address_map_get(ptr noundef %1, ptr noundef %18) #44
  %20 = icmp eq ptr %19, null
  br i1 %20, label %23, label %21

21:                                               ; preds = %14
  %22 = call i32 @fa_links_add(ptr noundef nonnull %3, ptr noundef nonnull %19) #44, !range !60
  br label %23

23:                                               ; preds = %21, %14
  %24 = add nuw nsw i64 %15, 1
  %25 = load i64, ptr %4, align 8, !tbaa !159
  %26 = icmp slt i64 %24, %25
  br i1 %26, label %14, label %10
}

; Function Attrs: nounwind optsize
define void @fa_address_map_free(ptr noundef %0) local_unnamed_addr #2 {
  %2 = load ptr, ptr %0, align 8, !tbaa !175
  %3 = getelementptr inbounds %struct.FaAddressMap, ptr %0, i64 0, i32 2
  %4 = load i64, ptr %3, align 8, !tbaa !173
  %5 = shl i64 %4, 3
  %6 = add i64 %5, 8
  tail call void @fa_free(ptr noundef %2, i64 noundef %6, i64 noundef 8) #45
  %7 = getelementptr inbounds %struct.FaAddressMap, ptr %0, i64 0, i32 1
  %8 = load ptr, ptr %7, align 8, !tbaa !176
  %9 = load i64, ptr %3, align 8, !tbaa !173
  %10 = shl i64 %9, 3
  %11 = add i64 %10, 8
  tail call void @fa_free(ptr noundef %8, i64 noundef %11, i64 noundef 8) #45
  tail call void @fa_free(ptr noundef nonnull %0, i64 noundef 24, i64 noundef 8) #45
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, argmem: none, inaccessiblemem: none)
define void @fa_argument_source_set(i32 noundef %0, ptr noundef %1) local_unnamed_addr #27 {
  %3 = and i32 %0, 15
  %4 = zext nneg i32 %3 to i64
  %5 = getelementptr inbounds [16 x ptr], ptr @argument_sources, i64 0, i64 %4
  store ptr %1, ptr %5, align 8, !tbaa !10
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(read, argmem: none, inaccessiblemem: none)
define ptr @fa_argument_source_get(i32 noundef %0) local_unnamed_addr #28 {
  %2 = and i32 %0, 15
  %3 = zext nneg i32 %2 to i64
  %4 = getelementptr inbounds [16 x ptr], ptr @argument_sources, i64 0, i64 %3
  %5 = load ptr, ptr %4, align 8, !tbaa !10
  ret ptr %5
}

; Function Attrs: nounwind optsize
define noundef ptr @fa_source_new() local_unnamed_addr #2 {
  %1 = tail call ptr @fa_alloc(i64 noundef 72, i64 noundef 8) #45
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %1, i8 0, i64 72, i1 false)
  %2 = load ptr, ptr @reactive_made, align 8, !tbaa !10
  %3 = load i64, ptr @reactive_made_count, align 8, !tbaa !8
  %4 = tail call fastcc ptr @grow_items(ptr noundef %2, i64 noundef %3, ptr noundef nonnull @reactive_made_capacity, i64 noundef 8) #44
  store ptr %4, ptr @reactive_made, align 8, !tbaa !10
  %5 = load i64, ptr @reactive_made_count, align 8, !tbaa !8
  %6 = getelementptr inbounds ptr, ptr %4, i64 %5
  store ptr %1, ptr %6, align 8, !tbaa !10
  %7 = add nsw i64 %5, 1
  store i64 %7, ptr @reactive_made_count, align 8, !tbaa !8
  ret ptr %1
}

; Function Attrs: nounwind optsize
define ptr @fa_source_child(ptr noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #2 {
  %4 = icmp eq ptr %0, null
  br i1 %4, label %47, label %5

5:                                                ; preds = %3
  %6 = getelementptr inbounds %struct.FaSource, ptr %0, i64 0, i32 7
  %7 = load i64, ptr %6, align 8, !tbaa !179
  %8 = icmp sgt i64 %7, 0
  br i1 %8, label %9, label %27

9:                                                ; preds = %5
  %10 = getelementptr inbounds %struct.FaSource, ptr %0, i64 0, i32 6
  %11 = load ptr, ptr %10, align 8, !tbaa !181
  br label %12

12:                                               ; preds = %9, %21
  %13 = phi i64 [ 0, %9 ], [ %22, %21 ]
  %14 = getelementptr inbounds %struct.FaChild, ptr %11, i64 %13
  %15 = load i64, ptr %14, align 8, !tbaa !182
  %16 = icmp eq i64 %15, %1
  br i1 %16, label %17, label %21

17:                                               ; preds = %12
  %18 = getelementptr inbounds %struct.FaChild, ptr %11, i64 %13, i32 1
  %19 = load i64, ptr %18, align 8, !tbaa !184
  %20 = icmp eq i64 %19, %2
  br i1 %20, label %24, label %21

21:                                               ; preds = %17, %12
  %22 = add nuw nsw i64 %13, 1
  %23 = icmp eq i64 %22, %7
  br i1 %23, label %27, label %12, !llvm.loop !185

24:                                               ; preds = %17
  %25 = getelementptr inbounds %struct.FaChild, ptr %11, i64 %13, i32 2
  %26 = load ptr, ptr %25, align 8, !tbaa !186
  br label %47

27:                                               ; preds = %21, %5
  %28 = tail call ptr @fa_alloc(i64 noundef 72, i64 noundef 8) #45
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %28, i8 0, i64 72, i1 false)
  %29 = getelementptr inbounds %struct.FaSource, ptr %28, i64 0, i32 5
  store ptr %0, ptr %29, align 8, !tbaa !187
  %30 = load ptr, ptr @reactive_made, align 8, !tbaa !10
  %31 = load i64, ptr @reactive_made_count, align 8, !tbaa !8
  %32 = tail call fastcc ptr @grow_items(ptr noundef %30, i64 noundef %31, ptr noundef nonnull @reactive_made_capacity, i64 noundef 8) #44
  store ptr %32, ptr @reactive_made, align 8, !tbaa !10
  %33 = load i64, ptr @reactive_made_count, align 8, !tbaa !8
  %34 = getelementptr inbounds ptr, ptr %32, i64 %33
  store ptr %28, ptr %34, align 8, !tbaa !10
  %35 = add nsw i64 %33, 1
  store i64 %35, ptr @reactive_made_count, align 8, !tbaa !8
  %36 = getelementptr inbounds %struct.FaSource, ptr %0, i64 0, i32 6
  %37 = load ptr, ptr %36, align 8, !tbaa !181
  %38 = load i64, ptr %6, align 8, !tbaa !179
  %39 = getelementptr inbounds %struct.FaSource, ptr %0, i64 0, i32 8
  %40 = tail call fastcc ptr @grow_items(ptr noundef %37, i64 noundef %38, ptr noundef nonnull %39, i64 noundef 24) #44
  store ptr %40, ptr %36, align 8, !tbaa !181
  %41 = load i64, ptr %6, align 8, !tbaa !179
  %42 = getelementptr inbounds %struct.FaChild, ptr %40, i64 %41
  store i64 %1, ptr %42, align 8, !tbaa.struct !188
  %43 = getelementptr inbounds i8, ptr %42, i64 8
  store i64 %2, ptr %43, align 8, !tbaa.struct !189
  %44 = getelementptr inbounds i8, ptr %42, i64 16
  store ptr %28, ptr %44, align 8, !tbaa.struct !190
  %45 = load i64, ptr %6, align 8, !tbaa !179
  %46 = add nsw i64 %45, 1
  store i64 %46, ptr %6, align 8, !tbaa !179
  br label %47

47:                                               ; preds = %24, %3, %27
  %48 = phi ptr [ %26, %24 ], [ %28, %27 ], [ null, %3 ]
  ret ptr %48
}

; Function Attrs: nounwind optsize
define internal fastcc ptr @grow_items(ptr noundef %0, i64 noundef %1, ptr nocapture noundef %2, i64 noundef %3) unnamed_addr #2 {
  %5 = load i64, ptr %2, align 8, !tbaa !8
  %6 = icmp sgt i64 %5, %1
  br i1 %6, label %20, label %7

7:                                                ; preds = %4
  %8 = icmp eq i64 %5, 0
  %9 = shl nsw i64 %5, 1
  %10 = select i1 %8, i64 4, i64 %9
  %11 = mul i64 %10, %3
  %12 = tail call ptr @fa_alloc(i64 noundef %11, i64 noundef 8) #45
  %13 = load i64, ptr %2, align 8, !tbaa !8
  %14 = icmp sgt i64 %13, 0
  br i1 %14, label %15, label %19

15:                                               ; preds = %7
  %16 = mul i64 %3, %1
  tail call fastcc void @copy_bytes(ptr noundef %12, ptr noundef %0, i64 noundef %16) #44
  %17 = load i64, ptr %2, align 8, !tbaa !8
  %18 = mul i64 %17, %3
  tail call void @fa_free(ptr noundef %0, i64 noundef %18, i64 noundef 8) #45
  br label %19

19:                                               ; preds = %15, %7
  store i64 %10, ptr %2, align 8, !tbaa !8
  br label %20

20:                                               ; preds = %4, %19
  %21 = phi ptr [ %12, %19 ], [ %0, %4 ]
  ret ptr %21
}

; Function Attrs: nounwind optsize
define ptr @fa_field_source(ptr noundef %0, i64 noundef %1) local_unnamed_addr #2 {
  %3 = ptrtoint ptr %0 to i64
  %4 = load i64, ptr @field_source_count, align 8, !tbaa !8
  %5 = icmp sgt i64 %4, 0
  br i1 %5, label %6, label %23

6:                                                ; preds = %2
  %7 = load ptr, ptr @field_sources, align 8, !tbaa !10
  br label %8

8:                                                ; preds = %6, %20
  %9 = phi i64 [ 0, %6 ], [ %21, %20 ]
  %10 = getelementptr inbounds %struct.FaChild, ptr %7, i64 %9
  %11 = load i64, ptr %10, align 8, !tbaa !182
  %12 = icmp eq i64 %11, %3
  br i1 %12, label %13, label %20

13:                                               ; preds = %8
  %14 = getelementptr inbounds %struct.FaChild, ptr %7, i64 %9, i32 1
  %15 = load i64, ptr %14, align 8, !tbaa !184
  %16 = icmp eq i64 %15, %1
  br i1 %16, label %17, label %20

17:                                               ; preds = %13
  %18 = getelementptr inbounds %struct.FaChild, ptr %7, i64 %9, i32 2
  %19 = load ptr, ptr %18, align 8, !tbaa !186
  br label %33

20:                                               ; preds = %13, %8
  %21 = add nuw nsw i64 %9, 1
  %22 = icmp eq i64 %21, %4
  br i1 %22, label %23, label %8, !llvm.loop !191

23:                                               ; preds = %20, %2
  %24 = tail call ptr @fa_source_new() #44
  %25 = load ptr, ptr @field_sources, align 8, !tbaa !10
  %26 = load i64, ptr @field_source_count, align 8, !tbaa !8
  %27 = tail call fastcc ptr @grow_items(ptr noundef %25, i64 noundef %26, ptr noundef nonnull @field_source_capacity, i64 noundef 24) #44
  store ptr %27, ptr @field_sources, align 8, !tbaa !10
  %28 = load i64, ptr @field_source_count, align 8, !tbaa !8
  %29 = getelementptr inbounds %struct.FaChild, ptr %27, i64 %28
  store i64 %3, ptr %29, align 8, !tbaa.struct !188
  %30 = getelementptr inbounds i8, ptr %29, i64 8
  store i64 %1, ptr %30, align 8, !tbaa.struct !189
  %31 = getelementptr inbounds i8, ptr %29, i64 16
  store ptr %24, ptr %31, align 8, !tbaa.struct !190
  %32 = add nsw i64 %28, 1
  store i64 %32, ptr @field_source_count, align 8, !tbaa !8
  br label %33

33:                                               ; preds = %17, %23
  %34 = phi ptr [ %24, %23 ], [ %19, %17 ]
  ret ptr %34
}

; Function Attrs: nounwind optsize
define void @fa_track(ptr noundef %0) local_unnamed_addr #2 {
  %2 = alloca i64, align 8
  %3 = load ptr, ptr @reactive_observer, align 8, !tbaa !10
  %4 = icmp eq ptr %3, null
  %5 = icmp eq ptr %0, null
  %6 = or i1 %5, %4
  br i1 %6, label %62, label %7

7:                                                ; preds = %1
  %8 = getelementptr inbounds %struct.FaObserver, ptr %3, i64 0, i32 8
  %9 = load i64, ptr %8, align 8, !tbaa !192
  %10 = icmp sgt i64 %9, 0
  %11 = getelementptr inbounds %struct.FaObserver, ptr %3, i64 0, i32 6
  %12 = load ptr, ptr %11, align 8, !tbaa !194
  br i1 %10, label %16, label %21

13:                                               ; preds = %16
  %14 = add nuw nsw i64 %17, 1
  %15 = icmp eq i64 %14, %9
  br i1 %15, label %21, label %16, !llvm.loop !195

16:                                               ; preds = %7, %13
  %17 = phi i64 [ %14, %13 ], [ 0, %7 ]
  %18 = getelementptr inbounds ptr, ptr %12, i64 %17
  %19 = load ptr, ptr %18, align 8, !tbaa !10
  %20 = icmp eq ptr %19, %0
  br i1 %20, label %62, label %13

21:                                               ; preds = %13, %7
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %2) #40
  %22 = getelementptr inbounds %struct.FaObserver, ptr %3, i64 0, i32 9
  %23 = load i64, ptr %22, align 8, !tbaa !196
  store i64 %23, ptr %2, align 8, !tbaa !8
  %24 = getelementptr inbounds %struct.FaObserver, ptr %3, i64 0, i32 6
  %25 = call fastcc ptr @grow_items(ptr noundef %12, i64 noundef %9, ptr noundef nonnull %2, i64 noundef 8) #44
  store ptr %25, ptr %24, align 8, !tbaa !194
  %26 = load i64, ptr %22, align 8, !tbaa !196
  store i64 %26, ptr %2, align 8, !tbaa !8
  %27 = getelementptr inbounds %struct.FaObserver, ptr %3, i64 0, i32 7
  %28 = load ptr, ptr %27, align 8, !tbaa !197
  %29 = load i64, ptr %8, align 8, !tbaa !192
  %30 = call fastcc ptr @grow_items(ptr noundef %28, i64 noundef %29, ptr noundef nonnull %2, i64 noundef 8) #44
  store ptr %30, ptr %27, align 8, !tbaa !197
  %31 = load i64, ptr %2, align 8, !tbaa !8
  store i64 %31, ptr %22, align 8, !tbaa !196
  %32 = load ptr, ptr %24, align 8, !tbaa !194
  %33 = load i64, ptr %8, align 8, !tbaa !192
  %34 = getelementptr inbounds ptr, ptr %32, i64 %33
  store ptr %0, ptr %34, align 8, !tbaa !10
  %35 = getelementptr inbounds %struct.FaSource, ptr %0, i64 0, i32 1
  %36 = load i64, ptr %35, align 8, !tbaa !198
  %37 = load ptr, ptr %27, align 8, !tbaa !197
  %38 = getelementptr inbounds i64, ptr %37, i64 %33
  store i64 %36, ptr %38, align 8, !tbaa !8
  %39 = load i64, ptr %8, align 8, !tbaa !192
  %40 = add nsw i64 %39, 1
  store i64 %40, ptr %8, align 8, !tbaa !192
  %41 = getelementptr inbounds %struct.FaSource, ptr %0, i64 0, i32 3
  %42 = load i64, ptr %41, align 8, !tbaa !199
  %43 = icmp sgt i64 %42, 0
  %44 = getelementptr inbounds %struct.FaSource, ptr %0, i64 0, i32 2
  %45 = load ptr, ptr %44, align 8, !tbaa !200
  br i1 %43, label %49, label %54

46:                                               ; preds = %49
  %47 = add nuw nsw i64 %50, 1
  %48 = icmp eq i64 %47, %42
  br i1 %48, label %54, label %49, !llvm.loop !201

49:                                               ; preds = %21, %46
  %50 = phi i64 [ %47, %46 ], [ 0, %21 ]
  %51 = getelementptr inbounds ptr, ptr %45, i64 %50
  %52 = load ptr, ptr %51, align 8, !tbaa !10
  %53 = icmp eq ptr %52, %3
  br i1 %53, label %61, label %46

54:                                               ; preds = %46, %21
  %55 = getelementptr inbounds %struct.FaSource, ptr %0, i64 0, i32 2
  %56 = getelementptr inbounds %struct.FaSource, ptr %0, i64 0, i32 4
  %57 = tail call fastcc ptr @grow_items(ptr noundef %45, i64 noundef %42, ptr noundef nonnull %56, i64 noundef 8) #44
  store ptr %57, ptr %55, align 8, !tbaa !200
  %58 = load i64, ptr %41, align 8, !tbaa !199
  %59 = getelementptr inbounds ptr, ptr %57, i64 %58
  store ptr %3, ptr %59, align 8, !tbaa !10
  %60 = add nsw i64 %58, 1
  store i64 %60, ptr %41, align 8, !tbaa !199
  br label %61

61:                                               ; preds = %49, %54
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %2) #40
  br label %62

62:                                               ; preds = %16, %1, %61
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_notify(ptr noundef %0) local_unnamed_addr #2 {
  %2 = icmp eq ptr %0, null
  br i1 %2, label %29, label %3

3:                                                ; preds = %1
  tail call fastcc void @bump(ptr noundef nonnull %0) #44
  %4 = getelementptr inbounds %struct.FaSource, ptr %0, i64 0, i32 5
  %5 = load ptr, ptr %4, align 8, !tbaa !187
  %6 = icmp eq ptr %5, null
  br i1 %6, label %29, label %7

7:                                                ; preds = %3, %25
  %8 = phi ptr [ %27, %25 ], [ %5, %3 ]
  %9 = getelementptr inbounds %struct.FaSource, ptr %8, i64 0, i32 1
  %10 = load i64, ptr %9, align 8, !tbaa !198
  %11 = add nsw i64 %10, 1
  store i64 %11, ptr %9, align 8, !tbaa !198
  %12 = getelementptr inbounds %struct.FaSource, ptr %8, i64 0, i32 3
  %13 = load i64, ptr %12, align 8, !tbaa !199
  %14 = icmp sgt i64 %13, 0
  br i1 %14, label %15, label %25

15:                                               ; preds = %7
  %16 = getelementptr inbounds %struct.FaSource, ptr %8, i64 0, i32 2
  br label %17

17:                                               ; preds = %17, %15
  %18 = phi i64 [ 0, %15 ], [ %22, %17 ]
  %19 = load ptr, ptr %16, align 8, !tbaa !200
  %20 = getelementptr inbounds ptr, ptr %19, i64 %18
  %21 = load ptr, ptr %20, align 8, !tbaa !10
  tail call fastcc void @mark(ptr noundef %21) #44
  %22 = add nuw nsw i64 %18, 1
  %23 = load i64, ptr %12, align 8, !tbaa !199
  %24 = icmp slt i64 %22, %23
  br i1 %24, label %17, label %25, !llvm.loop !202

25:                                               ; preds = %17, %7
  %26 = getelementptr inbounds %struct.FaSource, ptr %8, i64 0, i32 5
  %27 = load ptr, ptr %26, align 8, !tbaa !187
  %28 = icmp eq ptr %27, null
  br i1 %28, label %29, label %7, !llvm.loop !203

29:                                               ; preds = %25, %3, %1
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @bump(ptr nocapture noundef %0) unnamed_addr #2 {
  %2 = getelementptr inbounds %struct.FaSource, ptr %0, i64 0, i32 1
  %3 = load i64, ptr %2, align 8, !tbaa !198
  %4 = add nsw i64 %3, 1
  store i64 %4, ptr %2, align 8, !tbaa !198
  %5 = getelementptr inbounds %struct.FaSource, ptr %0, i64 0, i32 3
  %6 = load i64, ptr %5, align 8, !tbaa !199
  %7 = icmp sgt i64 %6, 0
  br i1 %7, label %8, label %18

8:                                                ; preds = %1
  %9 = getelementptr inbounds %struct.FaSource, ptr %0, i64 0, i32 2
  br label %10

10:                                               ; preds = %10, %8
  %11 = phi i64 [ 0, %8 ], [ %15, %10 ]
  %12 = load ptr, ptr %9, align 8, !tbaa !200
  %13 = getelementptr inbounds ptr, ptr %12, i64 %11
  %14 = load ptr, ptr %13, align 8, !tbaa !10
  tail call fastcc void @mark(ptr noundef %14) #44
  %15 = add nuw nsw i64 %11, 1
  %16 = load i64, ptr %5, align 8, !tbaa !199
  %17 = icmp slt i64 %15, %16
  br i1 %17, label %10, label %18, !llvm.loop !202

18:                                               ; preds = %10, %1
  %19 = getelementptr inbounds %struct.FaSource, ptr %0, i64 0, i32 7
  %20 = load i64, ptr %19, align 8, !tbaa !179
  %21 = icmp sgt i64 %20, 0
  br i1 %21, label %22, label %24

22:                                               ; preds = %18
  %23 = getelementptr inbounds %struct.FaSource, ptr %0, i64 0, i32 6
  br label %25

24:                                               ; preds = %25, %18
  ret void

25:                                               ; preds = %22, %25
  %26 = phi i64 [ 0, %22 ], [ %30, %25 ]
  %27 = load ptr, ptr %23, align 8, !tbaa !181
  %28 = getelementptr inbounds %struct.FaChild, ptr %27, i64 %26, i32 2
  %29 = load ptr, ptr %28, align 8, !tbaa !186
  tail call fastcc void @bump(ptr noundef %29) #44
  %30 = add nuw nsw i64 %26, 1
  %31 = load i64, ptr %19, align 8, !tbaa !179
  %32 = icmp slt i64 %30, %31
  br i1 %32, label %25, label %24, !llvm.loop !204
}

; Function Attrs: nounwind optsize
define noundef ptr @fa_derived_new(ptr noundef %0, ptr noundef %1, ptr noundef %2, i64 noundef %3) local_unnamed_addr #2 {
  %5 = tail call fastcc ptr @observer_new(ptr noundef %0, ptr noundef null, ptr noundef %1, ptr noundef %2, i64 noundef %3) #44
  store i64 1, ptr %5, align 8, !tbaa !205
  %6 = getelementptr inbounds %struct.FaObserver, ptr %5, i64 0, i32 11
  store i32 1, ptr %6, align 8, !tbaa !206
  ret ptr %5
}

; Function Attrs: nounwind optsize
define internal fastcc noundef ptr @observer_new(ptr noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef %3, i64 noundef %4) unnamed_addr #2 {
  %6 = tail call ptr @fa_alloc(i64 noundef 208, i64 noundef 8) #45
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(208) %6, i8 0, i64 208, i1 false)
  %7 = getelementptr inbounds %struct.FaObserver, ptr %6, i64 0, i32 3
  store ptr %0, ptr %7, align 8, !tbaa !207
  %8 = getelementptr inbounds %struct.FaObserver, ptr %6, i64 0, i32 4
  store ptr %1, ptr %8, align 8, !tbaa !208
  %9 = getelementptr inbounds %struct.FaObserver, ptr %6, i64 0, i32 5
  store ptr %2, ptr %9, align 8, !tbaa !209
  %10 = getelementptr inbounds %struct.FaObserver, ptr %6, i64 0, i32 2
  store ptr %3, ptr %10, align 8, !tbaa !210
  %11 = icmp sgt i64 %4, 0
  br i1 %11, label %12, label %16

12:                                               ; preds = %5
  %13 = tail call ptr @fa_alloc(i64 noundef %4, i64 noundef 16) #45
  %14 = getelementptr inbounds %struct.FaObserver, ptr %6, i64 0, i32 1
  store ptr %13, ptr %14, align 8, !tbaa !211
  tail call void @llvm.memset.p0.i64(ptr align 1 %13, i8 0, i64 %4, i1 false)
  %15 = getelementptr inbounds %struct.FaObserver, ptr %6, i64 0, i32 19
  store i64 %4, ptr %15, align 8, !tbaa !212
  br label %16

16:                                               ; preds = %12, %5
  %17 = load ptr, ptr @reactive_made, align 8, !tbaa !10
  %18 = load i64, ptr @reactive_made_count, align 8, !tbaa !8
  %19 = tail call fastcc ptr @grow_items(ptr noundef %17, i64 noundef %18, ptr noundef nonnull @reactive_made_capacity, i64 noundef 8) #44
  store ptr %19, ptr @reactive_made, align 8, !tbaa !10
  %20 = load i64, ptr @reactive_made_count, align 8, !tbaa !8
  %21 = getelementptr inbounds ptr, ptr %19, i64 %20
  store ptr %6, ptr %21, align 8, !tbaa !10
  %22 = add nsw i64 %20, 1
  store i64 %22, ptr @reactive_made_count, align 8, !tbaa !8
  ret ptr %6
}

; Function Attrs: nounwind optsize
define ptr @fa_derived_read(ptr noundef %0) local_unnamed_addr #2 {
  tail call fastcc void @refresh(ptr noundef %0) #44
  tail call void @fa_track(ptr noundef %0) #44
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 1
  %3 = load ptr, ptr %2, align 8, !tbaa !211
  ret ptr %3
}

; Function Attrs: nounwind optsize
define internal fastcc void @refresh(ptr noundef %0) unnamed_addr #2 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 11
  %3 = load i32, ptr %2, align 8, !tbaa !206
  %4 = icmp eq i32 %3, 0
  br i1 %4, label %29, label %5

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 12
  %7 = load i32, ptr %6, align 4, !tbaa !213
  %8 = icmp eq i32 %7, 0
  br i1 %8, label %13, label %9

9:                                                ; preds = %5
  %10 = tail call fastcc i32 @outdated(ptr noundef nonnull %0) #44
  %11 = icmp eq i32 %10, 0
  br i1 %11, label %12, label %13

12:                                               ; preds = %9
  store i32 0, ptr %2, align 8, !tbaa !206
  br label %29

13:                                               ; preds = %9, %5
  %14 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 13
  %15 = load i32, ptr %14, align 8, !tbaa !214
  %16 = icmp eq i32 %15, 0
  br i1 %16, label %18, label %17

17:                                               ; preds = %13
  tail call fastcc void @reactive_stop(ptr noundef nonnull @.str.42) #47
  unreachable

18:                                               ; preds = %13
  tail call fastcc void @unsubscribe(ptr noundef nonnull %0) #44
  %19 = load ptr, ptr @reactive_observer, align 8, !tbaa !10
  store ptr %0, ptr @reactive_observer, align 8, !tbaa !10
  store i32 1, ptr %14, align 8, !tbaa !214
  %20 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 16
  store i32 1, ptr %20, align 4, !tbaa !215
  %21 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 3
  %22 = load ptr, ptr %21, align 8, !tbaa !207
  tail call void %22(ptr noundef nonnull %0) #45
  store i32 0, ptr %14, align 8, !tbaa !214
  store ptr %19, ptr @reactive_observer, align 8, !tbaa !10
  store i32 0, ptr %2, align 8, !tbaa !206
  store i32 1, ptr %6, align 4, !tbaa !213
  %23 = load i32, ptr %20, align 4, !tbaa !215
  %24 = icmp eq i32 %23, 0
  br i1 %24, label %29, label %25

25:                                               ; preds = %18
  %26 = getelementptr inbounds %struct.FaSource, ptr %0, i64 0, i32 1
  %27 = load i64, ptr %26, align 8, !tbaa !216
  %28 = add nsw i64 %27, 1
  store i64 %28, ptr %26, align 8, !tbaa !216
  br label %29

29:                                               ; preds = %18, %25, %1, %12
  ret void
}

; Function Attrs: nounwind optsize
define noundef ptr @fa_watch_new(ptr noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef %3) local_unnamed_addr #2 {
  %5 = tail call fastcc ptr @observer_new(ptr noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef %3, i64 noundef 0) #44
  store i64 2, ptr %5, align 8, !tbaa !205
  %6 = getelementptr inbounds %struct.FaObserver, ptr %5, i64 0, i32 17
  store i32 1, ptr %6, align 8, !tbaa !217
  %7 = load i64, ptr @reactive_sequence, align 8, !tbaa !8
  %8 = add nsw i64 %7, 1
  store i64 %8, ptr @reactive_sequence, align 8, !tbaa !8
  %9 = getelementptr inbounds %struct.FaObserver, ptr %5, i64 0, i32 10
  store i64 %8, ptr %9, align 8, !tbaa !218
  tail call fastcc void @unsubscribe(ptr noundef nonnull %5) #44
  %10 = load ptr, ptr @reactive_observer, align 8, !tbaa !10
  store ptr null, ptr @reactive_observer, align 8, !tbaa !10
  %11 = getelementptr inbounds %struct.FaObserver, ptr %5, i64 0, i32 4
  %12 = load ptr, ptr %11, align 8, !tbaa !208
  tail call void %12(ptr noundef nonnull %5) #45
  store ptr %5, ptr @reactive_observer, align 8, !tbaa !10
  %13 = getelementptr inbounds %struct.FaObserver, ptr %5, i64 0, i32 3
  %14 = load ptr, ptr %13, align 8, !tbaa !207
  tail call void %14(ptr noundef nonnull %5) #45
  store ptr %10, ptr @reactive_observer, align 8, !tbaa !10
  ret ptr %5
}

; Function Attrs: nounwind optsize
define void @fa_dispose(ptr noundef %0) local_unnamed_addr #2 {
  %2 = icmp eq ptr %0, null
  br i1 %2, label %11, label %3

3:                                                ; preds = %1
  %4 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 15
  %5 = load i32, ptr %4, align 8, !tbaa !219
  %6 = icmp eq i32 %5, 0
  br i1 %6, label %7, label %11

7:                                                ; preds = %3
  store i32 1, ptr %4, align 8, !tbaa !219
  tail call fastcc void @unsubscribe(ptr noundef nonnull %0) #44
  %8 = load ptr, ptr @reactive_observer, align 8, !tbaa !10
  store ptr null, ptr @reactive_observer, align 8, !tbaa !10
  %9 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 4
  %10 = load ptr, ptr %9, align 8, !tbaa !208
  tail call void %10(ptr noundef nonnull %0) #45
  store ptr %8, ptr @reactive_observer, align 8, !tbaa !10
  br label %11

11:                                               ; preds = %1, %3, %7
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc void @unsubscribe(ptr noundef %0) unnamed_addr #17 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 8
  %3 = load i64, ptr %2, align 8, !tbaa !192
  %4 = icmp sgt i64 %3, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 6
  br label %8

7:                                                ; preds = %33, %1
  store i64 0, ptr %2, align 8, !tbaa !192
  ret void

8:                                                ; preds = %5, %33
  %9 = phi i64 [ 0, %5 ], [ %34, %33 ]
  %10 = load ptr, ptr %6, align 8, !tbaa !194
  %11 = getelementptr inbounds ptr, ptr %10, i64 %9
  %12 = load ptr, ptr %11, align 8, !tbaa !10
  %13 = getelementptr inbounds %struct.FaSource, ptr %12, i64 0, i32 3
  %14 = load i64, ptr %13, align 8, !tbaa !199
  %15 = icmp sgt i64 %14, 0
  br i1 %15, label %16, label %33

16:                                               ; preds = %8
  %17 = getelementptr inbounds %struct.FaSource, ptr %12, i64 0, i32 2
  %18 = load ptr, ptr %17, align 8, !tbaa !200
  br label %19

19:                                               ; preds = %16, %30
  %20 = phi i64 [ 0, %16 ], [ %31, %30 ]
  %21 = getelementptr inbounds ptr, ptr %18, i64 %20
  %22 = load ptr, ptr %21, align 8, !tbaa !10
  %23 = icmp eq ptr %22, %0
  br i1 %23, label %24, label %30

24:                                               ; preds = %19
  %25 = getelementptr inbounds ptr, ptr %18, i64 %20
  %26 = getelementptr ptr, ptr %18, i64 %14
  %27 = getelementptr ptr, ptr %26, i64 -1
  %28 = load ptr, ptr %27, align 8, !tbaa !10
  store ptr %28, ptr %25, align 8, !tbaa !10
  %29 = add nsw i64 %14, -1
  store i64 %29, ptr %13, align 8, !tbaa !199
  br label %33

30:                                               ; preds = %19
  %31 = add nuw nsw i64 %20, 1
  %32 = icmp eq i64 %31, %14
  br i1 %32, label %33, label %19, !llvm.loop !220

33:                                               ; preds = %30, %8, %24
  %34 = add nuw nsw i64 %9, 1
  %35 = icmp eq i64 %34, %3
  br i1 %35, label %7, label %8, !llvm.loop !221
}

; Function Attrs: nounwind optsize
define void @fa_reactive_flush() local_unnamed_addr #2 {
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
  tail call fastcc void @reactive_stop(ptr noundef nonnull @.str.15) #47
  unreachable

13:                                               ; preds = %7
  %14 = shl i64 %9, 3
  %15 = tail call ptr @fa_alloc(i64 noundef %14, i64 noundef 8) #45
  %16 = load ptr, ptr @reactive_pending, align 8, !tbaa !10
  tail call fastcc void @copy_bytes(ptr noundef %15, ptr noundef %16, i64 noundef %14) #44
  store i64 0, ptr @reactive_pending_count, align 8, !tbaa !8
  %17 = icmp ugt i64 %9, 1
  br i1 %17, label %19, label %18

18:                                               ; preds = %36, %13
  br label %44

19:                                               ; preds = %13, %36
  %20 = phi i64 [ %39, %36 ], [ 1, %13 ]
  %21 = getelementptr inbounds ptr, ptr %15, i64 %20
  %22 = load ptr, ptr %21, align 8, !tbaa !10
  %23 = getelementptr inbounds %struct.FaObserver, ptr %22, i64 0, i32 10
  %24 = load i64, ptr %23, align 8, !tbaa !218
  br label %25

25:                                               ; preds = %19, %33
  %26 = phi i64 [ %20, %19 ], [ %27, %33 ]
  %27 = add nsw i64 %26, -1
  %28 = getelementptr inbounds ptr, ptr %15, i64 %27
  %29 = load ptr, ptr %28, align 8, !tbaa !10
  %30 = getelementptr inbounds %struct.FaObserver, ptr %29, i64 0, i32 10
  %31 = load i64, ptr %30, align 8, !tbaa !218
  %32 = icmp sgt i64 %31, %24
  br i1 %32, label %33, label %36

33:                                               ; preds = %25
  %34 = getelementptr inbounds ptr, ptr %15, i64 %26
  store ptr %29, ptr %34, align 8, !tbaa !10
  %35 = icmp sgt i64 %26, 1
  br i1 %35, label %25, label %36, !llvm.loop !222

36:                                               ; preds = %33, %25
  %37 = phi i64 [ 0, %33 ], [ %26, %25 ]
  %38 = getelementptr inbounds ptr, ptr %15, i64 %37
  store ptr %22, ptr %38, align 8, !tbaa !10
  %39 = add nuw nsw i64 %20, 1
  %40 = icmp eq i64 %39, %9
  br i1 %40, label %18, label %19, !llvm.loop !223

41:                                               ; preds = %61
  tail call void @fa_free(ptr noundef nonnull %15, i64 noundef %14, i64 noundef 8) #45
  %42 = load i64, ptr @reactive_pending_count, align 8, !tbaa !8
  %43 = icmp sgt i64 %42, 0
  br i1 %43, label %7, label %64, !llvm.loop !224

44:                                               ; preds = %18, %61
  %45 = phi i64 [ %62, %61 ], [ 0, %18 ]
  %46 = getelementptr inbounds ptr, ptr %15, i64 %45
  %47 = load ptr, ptr %46, align 8, !tbaa !10
  %48 = getelementptr inbounds %struct.FaObserver, ptr %47, i64 0, i32 14
  store i32 0, ptr %48, align 4, !tbaa !225
  %49 = getelementptr inbounds %struct.FaObserver, ptr %47, i64 0, i32 15
  %50 = load i32, ptr %49, align 8, !tbaa !219
  %51 = icmp eq i32 %50, 0
  br i1 %51, label %52, label %61

52:                                               ; preds = %44
  %53 = tail call fastcc i32 @outdated(ptr noundef nonnull %47) #44
  %54 = icmp eq i32 %53, 0
  br i1 %54, label %61, label %55

55:                                               ; preds = %52
  tail call fastcc void @unsubscribe(ptr noundef nonnull %47) #44
  %56 = load ptr, ptr @reactive_observer, align 8, !tbaa !10
  store ptr null, ptr @reactive_observer, align 8, !tbaa !10
  %57 = getelementptr inbounds %struct.FaObserver, ptr %47, i64 0, i32 4
  %58 = load ptr, ptr %57, align 8, !tbaa !208
  tail call void %58(ptr noundef nonnull %47) #45
  store ptr %47, ptr @reactive_observer, align 8, !tbaa !10
  %59 = getelementptr inbounds %struct.FaObserver, ptr %47, i64 0, i32 3
  %60 = load ptr, ptr %59, align 8, !tbaa !207
  tail call void %60(ptr noundef nonnull %47) #45
  store ptr %56, ptr @reactive_observer, align 8, !tbaa !10
  br label %61

61:                                               ; preds = %55, %52, %44
  %62 = add nuw i64 %45, 1
  %63 = icmp eq i64 %62, %9
  br i1 %63, label %41, label %44, !llvm.loop !226

64:                                               ; preds = %41, %5
  store i1 false, ptr @reactive_flushing, align 4
  br label %65

65:                                               ; preds = %0, %64
  ret void
}

; Function Attrs: noreturn nounwind optsize
define internal fastcc void @reactive_stop(ptr noundef %0) unnamed_addr #4 {
  tail call void @fa_flush() #44
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.43, i64 noundef 6) #45
  br label %2

2:                                                ; preds = %2, %1
  %3 = phi i64 [ 0, %1 ], [ %7, %2 ]
  %4 = getelementptr inbounds i8, ptr %0, i64 %3
  %5 = load i8, ptr %4, align 1, !tbaa !2
  %6 = icmp eq i8 %5, 0
  %7 = add i64 %3, 1
  br i1 %6, label %8, label %2, !llvm.loop !29

8:                                                ; preds = %2
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull %0, i64 noundef %3) #45
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.22, i64 noundef 1) #45
  tail call void @fa_exit(i32 noundef 101) #46
  unreachable
}

; Function Attrs: nounwind optsize
define internal fastcc noundef i32 @outdated(ptr nocapture noundef readonly %0) unnamed_addr #2 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 8
  %3 = load i64, ptr %2, align 8, !tbaa !192
  %4 = icmp sgt i64 %3, 0
  br i1 %4, label %5, label %29

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 6
  %7 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 7
  br label %12

8:                                                ; preds = %20
  %9 = add nuw nsw i64 %13, 1
  %10 = load i64, ptr %2, align 8, !tbaa !192
  %11 = icmp slt i64 %9, %10
  br i1 %11, label %12, label %27, !llvm.loop !227

12:                                               ; preds = %5, %8
  %13 = phi i64 [ 0, %5 ], [ %9, %8 ]
  %14 = load ptr, ptr %6, align 8, !tbaa !194
  %15 = getelementptr inbounds ptr, ptr %14, i64 %13
  %16 = load ptr, ptr %15, align 8, !tbaa !10
  %17 = load i64, ptr %16, align 8, !tbaa !228
  %18 = icmp eq i64 %17, 1
  br i1 %18, label %19, label %20

19:                                               ; preds = %12
  tail call fastcc void @refresh(ptr noundef nonnull %16) #44
  br label %20

20:                                               ; preds = %19, %12
  %21 = getelementptr inbounds %struct.FaSource, ptr %16, i64 0, i32 1
  %22 = load i64, ptr %21, align 8, !tbaa !198
  %23 = load ptr, ptr %7, align 8, !tbaa !197
  %24 = getelementptr inbounds i64, ptr %23, i64 %13
  %25 = load i64, ptr %24, align 8, !tbaa !8
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
define ptr @fa_observer_value(ptr nocapture noundef readonly %0) local_unnamed_addr #29 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 1
  %3 = load ptr, ptr %2, align 8, !tbaa !211
  ret ptr %3
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read)
define ptr @fa_observer_env(ptr nocapture noundef readonly %0) local_unnamed_addr #29 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 2
  %3 = load ptr, ptr %2, align 8, !tbaa !210
  ret ptr %3
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(none)
define nonnull ptr @fa_observer_cleanup(ptr noundef readnone %0) local_unnamed_addr #20 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 20
  ret ptr %2
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read)
define i32 @fa_observer_computed(ptr nocapture noundef readonly %0) local_unnamed_addr #29 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 12
  %3 = load i32, ptr %2, align 4, !tbaa !213
  ret i32 %3
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: write)
define void @fa_observer_changed(ptr nocapture noundef writeonly %0, i32 noundef %1) local_unnamed_addr #30 {
  %3 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 16
  store i32 %1, ptr %3, align 4, !tbaa !215
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_reactive_release() local_unnamed_addr #2 {
  %1 = load i64, ptr @reactive_made_count, align 8, !tbaa !8
  %2 = icmp sgt i64 %1, 0
  br i1 %2, label %22, label %3

3:                                                ; preds = %69, %0
  %4 = load i64, ptr @reactive_made_capacity, align 8, !tbaa !8
  %5 = icmp sgt i64 %4, 0
  br i1 %5, label %6, label %9

6:                                                ; preds = %3
  %7 = load ptr, ptr @reactive_made, align 8, !tbaa !10
  %8 = shl i64 %4, 3
  tail call void @fa_free(ptr noundef %7, i64 noundef %8, i64 noundef 8) #45
  br label %9

9:                                                ; preds = %3, %6
  %10 = load i64, ptr @reactive_pending_capacity, align 8, !tbaa !8
  %11 = icmp sgt i64 %10, 0
  br i1 %11, label %12, label %15

12:                                               ; preds = %9
  %13 = load ptr, ptr @reactive_pending, align 8, !tbaa !10
  %14 = shl i64 %10, 3
  tail call void @fa_free(ptr noundef %13, i64 noundef %14, i64 noundef 8) #45
  br label %15

15:                                               ; preds = %9, %12
  %16 = load i64, ptr @field_source_capacity, align 8, !tbaa !8
  %17 = icmp sgt i64 %16, 0
  br i1 %17, label %18, label %21

18:                                               ; preds = %15
  %19 = load ptr, ptr @field_sources, align 8, !tbaa !10
  %20 = mul i64 %16, 24
  tail call void @fa_free(ptr noundef %19, i64 noundef %20, i64 noundef 8) #45
  br label %21

21:                                               ; preds = %15, %18
  store i64 0, ptr @reactive_made_count, align 8, !tbaa !8
  ret void

22:                                               ; preds = %0, %69
  %23 = phi i64 [ %24, %69 ], [ %1, %0 ]
  %24 = add nsw i64 %23, -1
  %25 = load ptr, ptr @reactive_made, align 8, !tbaa !10
  %26 = getelementptr inbounds ptr, ptr %25, i64 %24
  %27 = load ptr, ptr %26, align 8, !tbaa !10
  %28 = getelementptr inbounds %struct.FaSource, ptr %27, i64 0, i32 4
  %29 = load i64, ptr %28, align 8, !tbaa !229
  %30 = icmp sgt i64 %29, 0
  br i1 %30, label %31, label %35

31:                                               ; preds = %22
  %32 = getelementptr inbounds %struct.FaSource, ptr %27, i64 0, i32 2
  %33 = load ptr, ptr %32, align 8, !tbaa !200
  %34 = shl i64 %29, 3
  tail call void @fa_free(ptr noundef %33, i64 noundef %34, i64 noundef 8) #45
  br label %35

35:                                               ; preds = %22, %31
  %36 = getelementptr inbounds %struct.FaSource, ptr %27, i64 0, i32 8
  %37 = load i64, ptr %36, align 8, !tbaa !230
  %38 = icmp sgt i64 %37, 0
  br i1 %38, label %39, label %43

39:                                               ; preds = %35
  %40 = getelementptr inbounds %struct.FaSource, ptr %27, i64 0, i32 6
  %41 = load ptr, ptr %40, align 8, !tbaa !181
  %42 = mul i64 %37, 24
  tail call void @fa_free(ptr noundef %41, i64 noundef %42, i64 noundef 8) #45
  br label %43

43:                                               ; preds = %35, %39
  %44 = load i64, ptr %27, align 8, !tbaa !228
  %45 = icmp eq i64 %44, 0
  br i1 %45, label %69, label %46

46:                                               ; preds = %43
  %47 = getelementptr inbounds %struct.FaObserver, ptr %27, i64 0, i32 5
  %48 = load ptr, ptr %47, align 8, !tbaa !209
  tail call void %48(ptr noundef nonnull %27) #45
  %49 = getelementptr inbounds %struct.FaObserver, ptr %27, i64 0, i32 9
  %50 = load i64, ptr %49, align 8, !tbaa !196
  %51 = icmp sgt i64 %50, 0
  br i1 %51, label %52, label %62

52:                                               ; preds = %46
  %53 = getelementptr inbounds %struct.FaObserver, ptr %27, i64 0, i32 6
  %54 = load ptr, ptr %53, align 8, !tbaa !194
  %55 = shl i64 %50, 3
  tail call void @fa_free(ptr noundef %54, i64 noundef %55, i64 noundef 8) #45
  %56 = load i64, ptr %49, align 8, !tbaa !196
  %57 = icmp sgt i64 %56, 0
  br i1 %57, label %58, label %62

58:                                               ; preds = %52
  %59 = getelementptr inbounds %struct.FaObserver, ptr %27, i64 0, i32 7
  %60 = load ptr, ptr %59, align 8, !tbaa !197
  %61 = shl i64 %56, 3
  tail call void @fa_free(ptr noundef %60, i64 noundef %61, i64 noundef 8) #45
  br label %62

62:                                               ; preds = %46, %52, %58
  %63 = getelementptr inbounds %struct.FaObserver, ptr %27, i64 0, i32 19
  %64 = load i64, ptr %63, align 8, !tbaa !212
  %65 = icmp sgt i64 %64, 0
  br i1 %65, label %66, label %69

66:                                               ; preds = %62
  %67 = getelementptr inbounds %struct.FaObserver, ptr %27, i64 0, i32 1
  %68 = load ptr, ptr %67, align 8, !tbaa !211
  tail call void @fa_free(ptr noundef %68, i64 noundef %64, i64 noundef 16) #45
  br label %69

69:                                               ; preds = %62, %66, %43
  %70 = phi i64 [ 72, %43 ], [ 208, %66 ], [ 208, %62 ]
  tail call void @fa_free(ptr noundef nonnull %27, i64 noundef %70, i64 noundef 8) #45
  %71 = icmp ugt i64 %23, 1
  br i1 %71, label %22, label %3, !llvm.loop !231
}

; Function Attrs: cold noinline noreturn nounwind optsize
define void @fa_overflow(i64 noundef %0, i64 noundef %1, i32 noundef %2, ptr noundef %3) local_unnamed_addr #31 {
  %5 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %5, align 8, !tbaa !8
  tail call fastcc void @append_c(ptr noundef nonnull @.str.16) #44
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
  tail call fastcc void @append_u128(i64 noundef %17, i64 noundef %19, i32 noundef %14) #44
  br label %21

20:                                               ; preds = %4
  tail call fastcc void @append_u128(i64 noundef %0, i64 noundef %1, i32 noundef 0) #44
  br label %21

21:                                               ; preds = %20, %7
  tail call fastcc void @append_c(ptr noundef nonnull @.str.17) #44
  tail call fastcc void @append_c(ptr noundef %3) #44
  br label %22

22:                                               ; preds = %22, %21
  %23 = phi i64 [ 0, %21 ], [ %27, %22 ]
  %24 = getelementptr inbounds i8, ptr %3, i64 %23
  %25 = load i8, ptr %24, align 1, !tbaa !2
  %26 = icmp eq i8 %25, 0
  %27 = add i64 %23, 1
  br i1 %26, label %28, label %22, !llvm.loop !29

28:                                               ; preds = %22
  %29 = getelementptr inbounds i8, ptr %3, i64 %23
  %30 = getelementptr inbounds i8, ptr %29, i64 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %30) #47
  unreachable
}

; Function Attrs: noreturn nounwind optsize
define internal fastcc void @stop_with_builder(ptr noundef %0) unnamed_addr #4 {
  tail call void @fa_flush() #44
  %2 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %3 = load ptr, ptr %2, align 8, !tbaa !10
  %4 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %5 = load i64, ptr %4, align 8, !tbaa !8
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.43, i64 noundef 6) #45
  tail call void @fa_write(i32 noundef 2, ptr noundef %3, i64 noundef %5) #45
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.44, i64 noundef 6) #45
  br label %6

6:                                                ; preds = %6, %1
  %7 = phi i64 [ 0, %1 ], [ %11, %6 ]
  %8 = getelementptr inbounds i8, ptr %0, i64 %7
  %9 = load i8, ptr %8, align 1, !tbaa !2
  %10 = icmp eq i8 %9, 0
  %11 = add i64 %7, 1
  br i1 %10, label %12, label %6, !llvm.loop !29

12:                                               ; preds = %6
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull %0, i64 noundef %7) #45
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.22, i64 noundef 1) #45
  tail call void @fa_exit(i32 noundef 101) #46
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define void @fa_overflow_unknown(ptr noundef %0) local_unnamed_addr #31 {
  %2 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %2, align 8, !tbaa !8
  tail call fastcc void @append_c(ptr noundef nonnull @.str.18) #44
  tail call fastcc void @append_c(ptr noundef %0) #44
  br label %3

3:                                                ; preds = %3, %1
  %4 = phi i64 [ 0, %1 ], [ %8, %3 ]
  %5 = getelementptr inbounds i8, ptr %0, i64 %4
  %6 = load i8, ptr %5, align 1, !tbaa !2
  %7 = icmp eq i8 %6, 0
  %8 = add i64 %4, 1
  br i1 %7, label %9, label %3, !llvm.loop !29

9:                                                ; preds = %3
  %10 = getelementptr inbounds i8, ptr %0, i64 %4
  %11 = getelementptr inbounds i8, ptr %10, i64 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %11) #47
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define void @fa_float_overflow(double noundef %0, ptr noundef %1) local_unnamed_addr #31 {
  %3 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %3, align 8, !tbaa !8
  tail call fastcc void @append_c(ptr noundef nonnull @.str.16) #44
  tail call void @fa_b_f64(double noundef %0) #44
  tail call fastcc void @append_c(ptr noundef nonnull @.str.17) #44
  tail call fastcc void @append_c(ptr noundef %1) #44
  br label %4

4:                                                ; preds = %4, %2
  %5 = phi i64 [ 0, %2 ], [ %9, %4 ]
  %6 = getelementptr inbounds i8, ptr %1, i64 %5
  %7 = load i8, ptr %6, align 1, !tbaa !2
  %8 = icmp eq i8 %7, 0
  %9 = add i64 %5, 1
  br i1 %8, label %10, label %4, !llvm.loop !29

10:                                               ; preds = %4
  %11 = getelementptr inbounds i8, ptr %1, i64 %5
  %12 = getelementptr inbounds i8, ptr %11, i64 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %12) #47
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define void @fa_assert_failed(ptr noundef %0) local_unnamed_addr #31 {
  %2 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %2, align 8, !tbaa !8
  tail call fastcc void @append_c(ptr noundef nonnull @.str.19) #44
  tail call fastcc void @append_c(ptr noundef %0) #44
  br label %3

3:                                                ; preds = %3, %1
  %4 = phi i64 [ 0, %1 ], [ %8, %3 ]
  %5 = getelementptr inbounds i8, ptr %0, i64 %4
  %6 = load i8, ptr %5, align 1, !tbaa !2
  %7 = icmp eq i8 %6, 0
  %8 = add i64 %4, 1
  br i1 %7, label %9, label %3, !llvm.loop !29

9:                                                ; preds = %3
  %10 = getelementptr inbounds i8, ptr %0, i64 %4
  %11 = getelementptr inbounds i8, ptr %10, i64 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %11) #47
  unreachable
}

; Function Attrs: nounwind optsize
define void @fa_guard_stack() local_unnamed_addr #2 {
  %1 = alloca %struct.AlternateStack, align 8
  %2 = alloca %struct.SignalAction, align 8
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %1) #40
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %1, ptr noundef nonnull align 8 dereferenceable(24) @__const.fa_guard_stack.stack, i64 24, i1 false)
  %3 = call i32 @sigaltstack(ptr noundef nonnull %1, ptr noundef null) #45
  call void @llvm.lifetime.start.p0(i64 152, ptr nonnull %2) #40
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(152) %2, ptr noundef nonnull align 8 dereferenceable(152) @__const.fa_guard_stack.action, i64 152, i1 false)
  %4 = call i32 @sigaction(i32 noundef 11, ptr noundef nonnull %2, ptr noundef null) #45
  call void @llvm.lifetime.end.p0(i64 152, ptr nonnull %2) #40
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %1) #40
  ret void
}

; Function Attrs: optsize
declare i32 @sigaltstack(ptr noundef, ptr noundef) local_unnamed_addr #3

; Function Attrs: noreturn nounwind optsize
define internal void @on_fault(i32 %0) #4 {
  tail call void @fa_flush() #44
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @on_fault.message, i64 noundef 21) #45
  tail call void @_exit(i32 noundef 101) #46
  unreachable
}

; Function Attrs: optsize
declare i32 @sigaction(i32 noundef, ptr noundef, ptr noundef) local_unnamed_addr #3

; Function Attrs: cold noinline noreturn nounwind optsize
define void @fa_assert_failed_operands(ptr noundef %0) local_unnamed_addr #31 {
  %2 = alloca %struct.FaString, align 8
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %2) #40
  call void @fa_b_take(ptr noundef nonnull %2) #44
  tail call fastcc void @append_c(ptr noundef nonnull @.str.19) #44
  tail call fastcc void @append_c(ptr noundef %0) #44
  tail call fastcc void @append_c(ptr noundef nonnull @.str.20) #44
  %3 = getelementptr inbounds %struct.FaString, ptr %2, i64 0, i32 2
  %4 = load i64, ptr %3, align 8, !tbaa !22
  %5 = and i64 %4, 4611686018427387904
  %6 = icmp eq i64 %5, 0
  %7 = lshr i64 %4, 56
  %8 = and i64 %7, 31
  %9 = load ptr, ptr %2, align 8
  %10 = getelementptr inbounds %struct.FaString, ptr %2, i64 0, i32 1
  %11 = load i64, ptr %10, align 8
  %12 = select i1 %6, ptr %9, ptr %2
  %13 = select i1 %6, i64 %11, i64 %8
  call void @fa_b_bytes(ptr noundef %12, i64 noundef %13) #44
  br label %14

14:                                               ; preds = %14, %1
  %15 = phi i64 [ 0, %1 ], [ %19, %14 ]
  %16 = getelementptr inbounds i8, ptr %0, i64 %15
  %17 = load i8, ptr %16, align 1, !tbaa !2
  %18 = icmp eq i8 %17, 0
  %19 = add i64 %15, 1
  br i1 %18, label %20, label %14, !llvm.loop !29

20:                                               ; preds = %14
  %21 = getelementptr inbounds i8, ptr %0, i64 %15
  %22 = getelementptr inbounds i8, ptr %21, i64 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %22) #47
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define void @fa_uncaught(ptr noundef %0) local_unnamed_addr #31 {
  %2 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %2, align 8, !tbaa !8
  tail call void @fa_format_union(ptr noundef %0, i32 noundef 1) #45
  tail call void @fa_flush() #44
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.21, i64 noundef 7) #45
  %3 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %4 = load ptr, ptr %3, align 8, !tbaa !10
  %5 = load i64, ptr %2, align 8, !tbaa !8
  tail call void @fa_write(i32 noundef 2, ptr noundef %4, i64 noundef %5) #45
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.22, i64 noundef 1) #45
  tail call void @fa_exit(i32 noundef 1) #46
  unreachable
}

; Function Attrs: optsize
declare void @fa_format_union(ptr noundef, i32 noundef) local_unnamed_addr #3

; Function Attrs: cold mustprogress nofree noinline norecurse nosync nounwind optsize willreturn memory(none)
define noundef nonnull ptr @fa_error_division_by_zero() local_unnamed_addr #32 {
  ret ptr @division_by_zero
}

; Function Attrs: cold noinline nounwind optsize
define noundef ptr @fa_error_conversion(i64 noundef %0, i64 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3) local_unnamed_addr #19 {
  %5 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %5, align 8, !tbaa !8
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
  tail call fastcc void @append_u128(i64 noundef %17, i64 noundef %19, i32 noundef %14) #44
  br label %21

20:                                               ; preds = %4
  tail call fastcc void @append_u128(i64 noundef %0, i64 noundef %1, i32 noundef 0) #44
  br label %21

21:                                               ; preds = %20, %7
  tail call fastcc void @append_c(ptr noundef nonnull @.str.17) #44
  tail call fastcc void @append_c(ptr noundef %3) #44
  %22 = tail call fastcc ptr @arithmetic_error_from_builder() #44
  ret ptr %22
}

; Function Attrs: nounwind optsize
define internal fastcc noundef ptr @arithmetic_error_from_builder() unnamed_addr #2 {
  %1 = alloca %struct.FaString, align 8
  %2 = tail call ptr @fa_alloc(i64 noundef 32, i64 noundef 8) #45
  store i64 4, ptr %2, align 8, !tbaa !232
  %3 = getelementptr inbounds %struct.MessageError, ptr %2, i64 0, i32 1
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %1) #40
  %4 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %5 = load ptr, ptr %4, align 8, !tbaa !10
  %6 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %7 = load i64, ptr %6, align 8, !tbaa !8
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 8 %1, ptr noundef %5, i64 noundef %7, i64 noundef 0) #44
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %3, ptr noundef nonnull align 8 dereferenceable(24) %1, i64 24, i1 false), !tbaa.struct !13
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %1) #40
  store i64 0, ptr %6, align 8, !tbaa !8
  ret ptr %2
}

; Function Attrs: cold noinline nounwind optsize
define noundef ptr @fa_error_negative_exponent(i64 noundef %0) local_unnamed_addr #19 {
  %2 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %2, align 8, !tbaa !8
  tail call fastcc void @append_c(ptr noundef nonnull @.str.23) #44
  %3 = tail call i64 @llvm.abs.i64(i64 %0, i1 false)
  %4 = lshr i64 %0, 63
  %5 = trunc i64 %4 to i32
  tail call fastcc void @append_u64(i64 noundef %3, i32 noundef %5) #44
  %6 = tail call fastcc ptr @arithmetic_error_from_builder() #44
  ret ptr %6
}

; Function Attrs: cold noinline nounwind optsize
define noundef ptr @fa_error_float(double noundef %0, i32 noundef %1) local_unnamed_addr #19 {
  %3 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %3, align 8, !tbaa !8
  tail call fastcc void @append_double(double noundef %0) #44
  %4 = icmp eq i32 %1, 0
  %5 = select i1 %4, ptr @.str.25, ptr @.str.24
  tail call fastcc void @append_c(ptr noundef nonnull %5) #44
  %6 = tail call fastcc ptr @arithmetic_error_from_builder() #44
  ret ptr %6
}

; Function Attrs: cold noinline nounwind optsize
define noundef ptr @fa_error_shift(i64 noundef %0, i64 noundef %1) local_unnamed_addr #19 {
  %3 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %3, align 8, !tbaa !8
  tail call fastcc void @append_c(ptr noundef nonnull @.str.26) #44
  %4 = tail call i64 @llvm.abs.i64(i64 %0, i1 false)
  %5 = lshr i64 %0, 63
  %6 = trunc i64 %5 to i32
  tail call fastcc void @append_u64(i64 noundef %4, i32 noundef %6) #44
  tail call fastcc void @append_c(ptr noundef nonnull @.str.27) #44
  %7 = tail call i64 @llvm.abs.i64(i64 %1, i1 false)
  %8 = lshr i64 %1, 63
  %9 = trunc i64 %8 to i32
  tail call fastcc void @append_u64(i64 noundef %7, i32 noundef %9) #44
  %10 = tail call fastcc ptr @arithmetic_error_from_builder() #44
  ret ptr %10
}

; Function Attrs: cold noinline nounwind optsize
define noundef ptr @fa_error_checked(i64 noundef %0, i64 noundef %1, ptr nocapture noundef readonly %2) local_unnamed_addr #19 {
  %4 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %4, align 8, !tbaa !8
  tail call fastcc void @append_c(ptr noundef nonnull @.str.16) #44
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
  tail call fastcc void @append_u128(i64 noundef %14, i64 noundef %16, i32 noundef %11) #44
  tail call fastcc void @append_c(ptr noundef nonnull @.str.17) #44
  tail call fastcc void @append_c(ptr noundef %2) #44
  %17 = tail call fastcc ptr @arithmetic_error_from_builder() #44
  ret ptr %17
}

; Function Attrs: nounwind optsize
define noundef ptr @fa_parse_integer(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef writeonly %2) local_unnamed_addr #2 {
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %13, label %5

5:                                                ; preds = %3
  %6 = load i8, ptr %0, align 1, !tbaa !2
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
  store i64 0, ptr %14, align 8, !tbaa !8
  br label %62

15:                                               ; preds = %24
  %16 = icmp eq i32 %31, 0
  br i1 %16, label %17, label %19

17:                                               ; preds = %15
  %18 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %18, align 8, !tbaa !8
  br label %36

19:                                               ; preds = %15
  br i1 %12, label %20, label %83

20:                                               ; preds = %19
  %21 = getelementptr inbounds i8, ptr %0, i64 %11
  %22 = load i8, ptr %21, align 1, !tbaa !2
  %23 = zext i8 %22 to i64
  br label %72

24:                                               ; preds = %9, %24
  %25 = phi i64 [ %32, %24 ], [ %11, %9 ]
  %26 = phi i32 [ %31, %24 ], [ 1, %9 ]
  %27 = getelementptr inbounds i8, ptr %0, i64 %25
  %28 = load i8, ptr %27, align 1, !tbaa !2
  %29 = add i8 %28, -48
  %30 = icmp ult i8 %29, 10
  %31 = select i1 %30, i32 %26, i32 0
  %32 = add nuw i64 %25, 1
  %33 = icmp eq i64 %32, %1
  br i1 %33, label %15, label %24, !llvm.loop !234

34:                                               ; preds = %9
  %35 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %35, align 8, !tbaa !8
  br i1 %4, label %62, label %36

36:                                               ; preds = %17, %34
  %37 = phi ptr [ %18, %17 ], [ %35, %34 ]
  tail call fastcc void @reserve(i64 noundef 1) #44
  %38 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %39 = load ptr, ptr %38, align 8, !tbaa !10
  %40 = load i64, ptr %37, align 8, !tbaa !8
  %41 = getelementptr inbounds i8, ptr %39, i64 %40
  store i8 34, ptr %41, align 1, !tbaa !2
  %42 = load i64, ptr %37, align 8, !tbaa !8
  %43 = add i64 %42, 1
  store i64 %43, ptr %37, align 8, !tbaa !8
  br label %44

44:                                               ; preds = %36, %54
  %45 = phi i64 [ 0, %36 ], [ %60, %54 ]
  %46 = getelementptr inbounds i8, ptr %0, i64 %45
  %47 = load i8, ptr %46, align 1, !tbaa !2
  switch i8 %47, label %54 [
    i8 92, label %48
    i8 34, label %48
  ]

48:                                               ; preds = %44, %44
  tail call fastcc void @reserve(i64 noundef 1) #44
  %49 = load ptr, ptr %38, align 8, !tbaa !10
  %50 = load i64, ptr %37, align 8, !tbaa !8
  %51 = getelementptr inbounds i8, ptr %49, i64 %50
  store i8 92, ptr %51, align 1, !tbaa !2
  %52 = load i64, ptr %37, align 8, !tbaa !8
  %53 = add i64 %52, 1
  store i64 %53, ptr %37, align 8, !tbaa !8
  br label %54

54:                                               ; preds = %44, %48
  tail call fastcc void @reserve(i64 noundef 1) #44
  %55 = load ptr, ptr %38, align 8, !tbaa !10
  %56 = load i64, ptr %37, align 8, !tbaa !8
  %57 = getelementptr inbounds i8, ptr %55, i64 %56
  store i8 %47, ptr %57, align 1, !tbaa !2
  %58 = load i64, ptr %37, align 8, !tbaa !8
  %59 = add i64 %58, 1
  store i64 %59, ptr %37, align 8, !tbaa !8
  %60 = add nuw i64 %45, 1
  %61 = icmp eq i64 %60, %1
  br i1 %61, label %62, label %44, !llvm.loop !235

62:                                               ; preds = %54, %34, %13
  %63 = phi ptr [ @.str.28, %13 ], [ @.str.28, %34 ], [ @.str.29, %54 ]
  tail call fastcc void @append_c(ptr noundef nonnull %63) #44
  %64 = tail call fastcc ptr @parse_error(ptr noundef %0, i64 noundef %1) #44
  br label %101

65:                                               ; preds = %72
  %66 = getelementptr inbounds i8, ptr %0, i64 %79
  %67 = load i8, ptr %66, align 1, !tbaa !2
  %68 = zext i8 %67 to i64
  %69 = sub nsw i64 47, %68
  %70 = udiv i64 %69, 10
  %71 = icmp ugt i64 %78, %70
  br i1 %71, label %81, label %72, !llvm.loop !236

72:                                               ; preds = %20, %65
  %73 = phi i64 [ %23, %20 ], [ %68, %65 ]
  %74 = phi i64 [ 0, %20 ], [ %78, %65 ]
  %75 = phi i64 [ %11, %20 ], [ %79, %65 ]
  %76 = mul nuw i64 %74, 10
  %77 = add i64 %76, -48
  %78 = add i64 %77, %73
  %79 = add i64 %75, 1
  %80 = icmp eq i64 %79, %1
  br i1 %80, label %81, label %65, !llvm.loop !236

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
  store i64 0, ptr %90, align 8, !tbaa !8
  tail call fastcc void @reserve(i64 noundef %1) #44
  %91 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %92 = load ptr, ptr %91, align 8, !tbaa !10
  %93 = load i64, ptr %90, align 8, !tbaa !8
  %94 = getelementptr inbounds i8, ptr %92, i64 %93
  tail call fastcc void @copy_bytes(ptr noundef %94, ptr noundef nonnull %0, i64 noundef %1) #44
  %95 = load i64, ptr %90, align 8, !tbaa !8
  %96 = add i64 %95, %1
  store i64 %96, ptr %90, align 8, !tbaa !8
  tail call fastcc void @append_c(ptr noundef nonnull @.str.24) #44
  %97 = tail call fastcc ptr @parse_error(ptr noundef nonnull %0, i64 noundef %1) #44
  br label %101

98:                                               ; preds = %83
  %99 = sub i64 0, %84
  %100 = select i1 %10, i64 %84, i64 %99
  store i64 %100, ptr %2, align 8, !tbaa !8
  br label %101

101:                                              ; preds = %89, %98, %62
  %102 = phi ptr [ %64, %62 ], [ %97, %89 ], [ null, %98 ]
  ret ptr %102
}

; Function Attrs: nounwind optsize
define internal fastcc noundef ptr @parse_error(ptr nocapture noundef readonly %0, i64 noundef %1) unnamed_addr #2 {
  %3 = alloca %struct.FaString, align 8
  %4 = alloca %struct.FaString, align 8
  %5 = tail call ptr @fa_alloc(i64 noundef 56, i64 noundef 8) #45
  store i64 5, ptr %5, align 8, !tbaa !237
  %6 = getelementptr inbounds %struct.ParseError, ptr %5, i64 0, i32 1
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %4) #40
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 8 %4, ptr noundef %0, i64 noundef %1, i64 noundef 0) #44
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %6, ptr noundef nonnull align 8 dereferenceable(24) %4, i64 24, i1 false), !tbaa.struct !13
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %4) #40
  %7 = getelementptr inbounds %struct.ParseError, ptr %5, i64 0, i32 2
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %3) #40
  %8 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %9 = load ptr, ptr %8, align 8, !tbaa !10
  %10 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %11 = load i64, ptr %10, align 8, !tbaa !8
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 8 %3, ptr noundef %9, i64 noundef %11, i64 noundef 0) #44
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %7, ptr noundef nonnull align 8 dereferenceable(24) %3, i64 24, i1 false), !tbaa.struct !13
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %3) #40
  store i64 0, ptr %10, align 8, !tbaa !8
  ret ptr %5
}

; Function Attrs: nounwind optsize
define i64 @fa_pow_i64(i64 noundef %0, i64 noundef %1, i32 noundef %2, ptr noundef %3) local_unnamed_addr #2 {
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
  tail call void @fa_overflow_unknown(ptr noundef %3) #49
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
  tail call void @fa_overflow_unknown(ptr noundef %3) #49
  unreachable
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare { i64, i1 } @llvm.smul.with.overflow.i64(i64, i64) #7

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(read, inaccessiblemem: none)
define ptr @fa_table_entry(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #33 {
  %4 = load ptr, ptr %0, align 8, !tbaa !239
  %5 = load ptr, ptr %4, align 8, !tbaa !16
  %6 = ashr i64 %1, 5
  %7 = getelementptr inbounds ptr, ptr %5, i64 %6
  %8 = load ptr, ptr %7, align 8, !tbaa !10
  %9 = and i64 %1, 31
  %10 = mul nsw i64 %9, %2
  %11 = getelementptr inbounds i8, ptr %8, i64 %10
  ret ptr %11
}

; Function Attrs: nounwind optsize
define ptr @fa_table_new(i64 noundef %0, i64 noundef %1) local_unnamed_addr #2 {
  %3 = tail call ptr @fa_alloc(i64 noundef 72, i64 noundef 8) #45
  %4 = tail call i64 @llvm.smax.i64(i64 %0, i64 4)
  %5 = mul nsw i64 %4, %1
  %6 = tail call ptr @fa_alloc(i64 noundef %5, i64 noundef 16) #45
  store ptr %6, ptr %3, align 8, !tbaa !239
  %7 = getelementptr inbounds %struct.FaTable, ptr %3, i64 0, i32 1
  store i64 0, ptr %7, align 8, !tbaa !241
  %8 = getelementptr inbounds %struct.FaTable, ptr %3, i64 0, i32 2
  store i64 0, ptr %8, align 8, !tbaa !242
  %9 = getelementptr inbounds %struct.FaTable, ptr %3, i64 0, i32 3
  store i64 %4, ptr %9, align 8, !tbaa !243
  %10 = getelementptr inbounds %struct.FaTable, ptr %3, i64 0, i32 4
  store ptr null, ptr %10, align 8, !tbaa !244
  %11 = getelementptr inbounds %struct.FaTable, ptr %3, i64 0, i32 6
  store i64 0, ptr %11, align 8, !tbaa !245
  %12 = getelementptr inbounds %struct.FaTable, ptr %3, i64 0, i32 7
  store i64 0, ptr %12, align 8, !tbaa !246
  %13 = shl nuw nsw i64 %4, 1
  br label %14

14:                                               ; preds = %14, %2
  %15 = phi i64 [ 8, %2 ], [ %17, %14 ]
  %16 = icmp slt i64 %15, %13
  %17 = shl nsw i64 %15, 1
  br i1 %16, label %14, label %18, !llvm.loop !247

18:                                               ; preds = %14
  tail call fastcc void @table_index(ptr noundef nonnull %3, i64 noundef %15, i64 noundef %1) #44
  ret ptr %3
}

; Function Attrs: nounwind optsize
define internal fastcc void @table_index(ptr nocapture noundef %0, i64 noundef %1, i64 noundef %2) unnamed_addr #2 {
  %4 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 4
  %5 = load ptr, ptr %4, align 8, !tbaa !244
  %6 = icmp eq ptr %5, null
  br i1 %6, label %14, label %7

7:                                                ; preds = %3
  %8 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 5
  %9 = load i64, ptr %8, align 8, !tbaa !248
  %10 = add nsw i64 %9, 1
  %11 = icmp ult i64 %9, 2147483648
  %12 = select i1 %11, i64 2, i64 3
  %13 = shl i64 %10, %12
  tail call void @fa_free(ptr noundef nonnull %5, i64 noundef %13, i64 noundef 8) #45
  br label %14

14:                                               ; preds = %7, %3
  %15 = add nsw i64 %1, -1
  %16 = icmp ult i64 %15, 2147483648
  %17 = select i1 %16, i64 2, i64 3
  %18 = shl i64 %1, %17
  %19 = tail call ptr @fa_alloc(i64 noundef %18, i64 noundef 8) #45
  store ptr %19, ptr %4, align 8, !tbaa !244
  tail call void @llvm.memset.p0.i64(ptr align 1 %19, i8 0, i64 %18, i1 false)
  %20 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 5
  store i64 %15, ptr %20, align 8, !tbaa !248
  %21 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 1
  %22 = load i64, ptr %21, align 8, !tbaa !241
  %23 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 8
  store i64 %22, ptr %23, align 8, !tbaa !249
  %24 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 2
  %25 = load i64, ptr %24, align 8, !tbaa !242
  %26 = icmp sgt i64 %25, 0
  br i1 %26, label %27, label %30

27:                                               ; preds = %14
  %28 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 6
  %29 = load ptr, ptr %0, align 8, !tbaa !239
  br label %31

30:                                               ; preds = %78, %14
  ret void

31:                                               ; preds = %27, %78
  %32 = phi i64 [ %25, %27 ], [ %79, %78 ]
  %33 = phi i64 [ 0, %27 ], [ %80, %78 ]
  %34 = load i64, ptr %28, align 8, !tbaa !245
  %35 = icmp eq i64 %34, 0
  br i1 %35, label %42, label %36

36:                                               ; preds = %31
  %37 = load ptr, ptr %29, align 8, !tbaa !16
  %38 = lshr i64 %33, 5
  %39 = getelementptr inbounds ptr, ptr %37, i64 %38
  %40 = load ptr, ptr %39, align 8, !tbaa !10
  %41 = and i64 %33, 31
  br label %42

42:                                               ; preds = %31, %36
  %43 = phi i64 [ %41, %36 ], [ %33, %31 ]
  %44 = phi ptr [ %40, %36 ], [ %29, %31 ]
  %45 = mul nsw i64 %43, %2
  %46 = getelementptr inbounds i8, ptr %44, i64 %45
  %47 = getelementptr inbounds i8, ptr %46, i64 8
  %48 = load i64, ptr %47, align 8, !tbaa !8
  %49 = icmp eq i64 %48, 0
  br i1 %49, label %78, label %50

50:                                               ; preds = %42
  %51 = load i64, ptr %46, align 8, !tbaa !8
  %52 = load i64, ptr %20, align 8, !tbaa !248
  %53 = icmp ult i64 %52, 2147483648
  %54 = load ptr, ptr %4, align 8, !tbaa !244
  br label %55

55:                                               ; preds = %66, %50
  %56 = phi i64 [ %51, %50 ], [ %67, %66 ]
  %57 = and i64 %56, %52
  br i1 %53, label %58, label %62

58:                                               ; preds = %55
  %59 = getelementptr inbounds i32, ptr %54, i64 %57
  %60 = load i32, ptr %59, align 4, !tbaa !25
  %61 = icmp eq i32 %60, 0
  br i1 %61, label %68, label %66

62:                                               ; preds = %55
  %63 = getelementptr inbounds i64, ptr %54, i64 %57
  %64 = load i64, ptr %63, align 8, !tbaa !8
  %65 = icmp eq i64 %64, 0
  br i1 %65, label %72, label %66

66:                                               ; preds = %62, %58
  %67 = add nsw i64 %57, 1
  br label %55, !llvm.loop !250

68:                                               ; preds = %58
  %69 = getelementptr inbounds i32, ptr %54, i64 %57
  %70 = trunc i64 %33 to i32
  %71 = add i32 %70, 1
  store i32 %71, ptr %69, align 4, !tbaa !25
  br label %75

72:                                               ; preds = %62
  %73 = getelementptr inbounds i64, ptr %54, i64 %57
  %74 = add nuw nsw i64 %33, 1
  store i64 %74, ptr %73, align 8, !tbaa !8
  br label %75

75:                                               ; preds = %68, %72
  %76 = add nsw i64 %57, 1
  store i64 %76, ptr %47, align 8, !tbaa !8
  %77 = load i64, ptr %24, align 8, !tbaa !242
  br label %78

78:                                               ; preds = %75, %42
  %79 = phi i64 [ %77, %75 ], [ %32, %42 ]
  %80 = add nuw nsw i64 %33, 1
  %81 = icmp slt i64 %80, %79
  br i1 %81, label %31, label %30, !llvm.loop !251
}

; Function Attrs: nounwind optsize
define ptr @fa_table_new_growing(i64 noundef %0, i64 noundef %1) local_unnamed_addr #2 {
  %3 = tail call ptr @fa_alloc(i64 noundef 72, i64 noundef 8) #45
  %4 = tail call i64 @llvm.smax.i64(i64 %0, i64 4)
  %5 = tail call ptr @fa_alloc(i64 noundef 24, i64 noundef 8) #45
  store ptr null, ptr %5, align 8, !tbaa !16
  %6 = getelementptr inbounds %struct.FaTableEntries, ptr %5, i64 0, i32 1
  store i64 0, ptr %6, align 8, !tbaa !21
  %7 = getelementptr inbounds %struct.FaTableEntries, ptr %5, i64 0, i32 2
  store i64 0, ptr %7, align 8, !tbaa !22
  store ptr %5, ptr %3, align 8, !tbaa !239
  %8 = getelementptr inbounds %struct.FaTable, ptr %3, i64 0, i32 1
  store i64 0, ptr %8, align 8, !tbaa !241
  %9 = getelementptr inbounds %struct.FaTable, ptr %3, i64 0, i32 2
  store i64 0, ptr %9, align 8, !tbaa !242
  %10 = getelementptr inbounds %struct.FaTable, ptr %3, i64 0, i32 3
  store i64 %4, ptr %10, align 8, !tbaa !243
  %11 = getelementptr inbounds %struct.FaTable, ptr %3, i64 0, i32 4
  store ptr null, ptr %11, align 8, !tbaa !244
  %12 = getelementptr inbounds %struct.FaTable, ptr %3, i64 0, i32 6
  store i64 1, ptr %12, align 8, !tbaa !245
  %13 = getelementptr inbounds %struct.FaTable, ptr %3, i64 0, i32 7
  store i64 0, ptr %13, align 8, !tbaa !246
  %14 = shl nuw nsw i64 %4, 1
  br label %15

15:                                               ; preds = %15, %2
  %16 = phi i64 [ 8, %2 ], [ %18, %15 ]
  %17 = icmp slt i64 %16, %14
  %18 = shl nsw i64 %16, 1
  br i1 %17, label %15, label %19, !llvm.loop !247

19:                                               ; preds = %15
  tail call fastcc void @table_index(ptr noundef nonnull %3, i64 noundef %16, i64 noundef %1) #44
  ret ptr %3
}

; Function Attrs: nounwind optsize
define ptr @fa_table_new_map(i64 noundef %0, i64 noundef %1) local_unnamed_addr #2 {
  %3 = tail call ptr @fa_table_new_growing(i64 noundef %0, i64 noundef %1) #44
  %4 = getelementptr inbounds %struct.FaTable, ptr %3, i64 0, i32 6
  store i64 2, ptr %4, align 8, !tbaa !245
  ret ptr %3
}

; Function Attrs: alwaysinline nounwind optsize
define i64 @fa_table_find(ptr nocapture noundef readonly %0, i64 noundef %1, ptr noundef %2, ptr noundef readonly %3, i64 noundef %4) local_unnamed_addr #13 {
  %6 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 5
  %7 = load i64, ptr %6, align 8, !tbaa !248
  %8 = icmp ult i64 %7, 2147483648
  %9 = and i64 %7, %1
  %10 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 4
  %11 = load ptr, ptr %10, align 8, !tbaa !244
  br i1 %8, label %12, label %48

12:                                               ; preds = %5
  %13 = getelementptr inbounds i32, ptr %11, i64 %9
  %14 = load i32, ptr %13, align 4, !tbaa !25
  %15 = icmp eq i32 %14, 0
  br i1 %15, label %83, label %16

16:                                               ; preds = %12
  %17 = icmp eq ptr %3, null
  br label %18

18:                                               ; preds = %16, %40
  %19 = phi ptr [ %11, %16 ], [ %41, %40 ]
  %20 = phi i64 [ %7, %16 ], [ %42, %40 ]
  %21 = phi i32 [ %14, %16 ], [ %46, %40 ]
  %22 = phi i64 [ %9, %16 ], [ %44, %40 ]
  %23 = icmp sgt i32 %21, 0
  br i1 %23, label %24, label %40

24:                                               ; preds = %18
  %25 = add nsw i32 %21, -1
  %26 = zext nneg i32 %25 to i64
  %27 = load ptr, ptr %0, align 8, !tbaa !239
  %28 = mul nsw i64 %26, %4
  %29 = getelementptr inbounds i8, ptr %27, i64 %28
  %30 = load i64, ptr %29, align 8, !tbaa !8
  %31 = icmp eq i64 %30, %1
  br i1 %31, label %32, label %40

32:                                               ; preds = %24
  br i1 %17, label %83, label %33

33:                                               ; preds = %32
  %34 = getelementptr inbounds i8, ptr %29, i64 16
  %35 = tail call i32 %3(ptr noundef nonnull %34, ptr noundef %2) #45
  %36 = icmp eq i32 %35, 0
  br i1 %36, label %37, label %83

37:                                               ; preds = %33
  %38 = load i64, ptr %6, align 8, !tbaa !248
  %39 = load ptr, ptr %10, align 8, !tbaa !244
  br label %40

40:                                               ; preds = %37, %24, %18
  %41 = phi ptr [ %39, %37 ], [ %19, %24 ], [ %19, %18 ]
  %42 = phi i64 [ %38, %37 ], [ %20, %24 ], [ %20, %18 ]
  %43 = add nsw i64 %22, 1
  %44 = and i64 %42, %43
  %45 = getelementptr inbounds i32, ptr %41, i64 %44
  %46 = load i32, ptr %45, align 4, !tbaa !25
  %47 = icmp eq i32 %46, 0
  br i1 %47, label %83, label %18

48:                                               ; preds = %5
  %49 = getelementptr inbounds i64, ptr %11, i64 %9
  %50 = load i64, ptr %49, align 8, !tbaa !8
  %51 = icmp eq i64 %50, 0
  br i1 %51, label %83, label %52

52:                                               ; preds = %48
  %53 = icmp eq ptr %3, null
  br label %54

54:                                               ; preds = %52, %75
  %55 = phi ptr [ %11, %52 ], [ %76, %75 ]
  %56 = phi i64 [ %7, %52 ], [ %77, %75 ]
  %57 = phi i64 [ %50, %52 ], [ %81, %75 ]
  %58 = phi i64 [ %9, %52 ], [ %79, %75 ]
  %59 = icmp sgt i64 %57, 0
  br i1 %59, label %60, label %75

60:                                               ; preds = %54
  %61 = add nsw i64 %57, -1
  %62 = load ptr, ptr %0, align 8, !tbaa !239
  %63 = mul nsw i64 %61, %4
  %64 = getelementptr inbounds i8, ptr %62, i64 %63
  %65 = load i64, ptr %64, align 8, !tbaa !8
  %66 = icmp eq i64 %65, %1
  br i1 %66, label %67, label %75

67:                                               ; preds = %60
  br i1 %53, label %83, label %68

68:                                               ; preds = %67
  %69 = getelementptr inbounds i8, ptr %64, i64 16
  %70 = tail call i32 %3(ptr noundef nonnull %69, ptr noundef %2) #45
  %71 = icmp eq i32 %70, 0
  br i1 %71, label %72, label %83

72:                                               ; preds = %68
  %73 = load i64, ptr %6, align 8, !tbaa !248
  %74 = load ptr, ptr %10, align 8, !tbaa !244
  br label %75

75:                                               ; preds = %72, %60, %54
  %76 = phi ptr [ %74, %72 ], [ %55, %60 ], [ %55, %54 ]
  %77 = phi i64 [ %73, %72 ], [ %56, %60 ], [ %56, %54 ]
  %78 = add nsw i64 %58, 1
  %79 = and i64 %77, %78
  %80 = getelementptr inbounds i64, ptr %76, i64 %79
  %81 = load i64, ptr %80, align 8, !tbaa !8
  %82 = icmp eq i64 %81, 0
  br i1 %82, label %83, label %54

83:                                               ; preds = %75, %68, %67, %40, %33, %32, %48, %12
  %84 = phi i64 [ -1, %12 ], [ -1, %48 ], [ -1, %40 ], [ %26, %33 ], [ %26, %32 ], [ -1, %75 ], [ %61, %68 ], [ %61, %67 ]
  ret i64 %84
}

; Function Attrs: alwaysinline nounwind optsize
define i64 @fa_table_find_growing(ptr nocapture noundef readonly %0, i64 noundef %1, ptr noundef %2, ptr noundef readonly %3, i64 noundef %4) local_unnamed_addr #13 {
  %6 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 5
  %7 = load i64, ptr %6, align 8, !tbaa !248
  %8 = icmp ult i64 %7, 2147483648
  %9 = and i64 %7, %1
  %10 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 4
  %11 = load ptr, ptr %10, align 8, !tbaa !244
  br i1 %8, label %12, label %53

12:                                               ; preds = %5
  %13 = getelementptr inbounds i32, ptr %11, i64 %9
  %14 = load i32, ptr %13, align 4, !tbaa !25
  %15 = icmp eq i32 %14, 0
  br i1 %15, label %93, label %16

16:                                               ; preds = %12
  %17 = icmp eq ptr %3, null
  br label %18

18:                                               ; preds = %16, %45
  %19 = phi ptr [ %11, %16 ], [ %46, %45 ]
  %20 = phi i64 [ %7, %16 ], [ %47, %45 ]
  %21 = phi i32 [ %14, %16 ], [ %51, %45 ]
  %22 = phi i64 [ %9, %16 ], [ %49, %45 ]
  %23 = icmp sgt i32 %21, 0
  br i1 %23, label %24, label %45

24:                                               ; preds = %18
  %25 = add nsw i32 %21, -1
  %26 = zext nneg i32 %25 to i64
  %27 = load ptr, ptr %0, align 8, !tbaa !239
  %28 = load ptr, ptr %27, align 8, !tbaa !16
  %29 = lshr i64 %26, 5
  %30 = getelementptr inbounds ptr, ptr %28, i64 %29
  %31 = load ptr, ptr %30, align 8, !tbaa !10
  %32 = and i64 %26, 31
  %33 = mul nsw i64 %32, %4
  %34 = getelementptr inbounds i8, ptr %31, i64 %33
  %35 = load i64, ptr %34, align 8, !tbaa !8
  %36 = icmp eq i64 %35, %1
  br i1 %36, label %37, label %45

37:                                               ; preds = %24
  br i1 %17, label %93, label %38

38:                                               ; preds = %37
  %39 = getelementptr inbounds i8, ptr %34, i64 16
  %40 = tail call i32 %3(ptr noundef nonnull %39, ptr noundef %2) #45
  %41 = icmp eq i32 %40, 0
  br i1 %41, label %42, label %93

42:                                               ; preds = %38
  %43 = load i64, ptr %6, align 8, !tbaa !248
  %44 = load ptr, ptr %10, align 8, !tbaa !244
  br label %45

45:                                               ; preds = %42, %24, %18
  %46 = phi ptr [ %44, %42 ], [ %19, %24 ], [ %19, %18 ]
  %47 = phi i64 [ %43, %42 ], [ %20, %24 ], [ %20, %18 ]
  %48 = add nsw i64 %22, 1
  %49 = and i64 %47, %48
  %50 = getelementptr inbounds i32, ptr %46, i64 %49
  %51 = load i32, ptr %50, align 4, !tbaa !25
  %52 = icmp eq i32 %51, 0
  br i1 %52, label %93, label %18

53:                                               ; preds = %5
  %54 = getelementptr inbounds i64, ptr %11, i64 %9
  %55 = load i64, ptr %54, align 8, !tbaa !8
  %56 = icmp eq i64 %55, 0
  br i1 %56, label %93, label %57

57:                                               ; preds = %53
  %58 = icmp eq ptr %3, null
  br label %59

59:                                               ; preds = %57, %85
  %60 = phi ptr [ %11, %57 ], [ %86, %85 ]
  %61 = phi i64 [ %7, %57 ], [ %87, %85 ]
  %62 = phi i64 [ %55, %57 ], [ %91, %85 ]
  %63 = phi i64 [ %9, %57 ], [ %89, %85 ]
  %64 = icmp sgt i64 %62, 0
  br i1 %64, label %65, label %85

65:                                               ; preds = %59
  %66 = add nsw i64 %62, -1
  %67 = load ptr, ptr %0, align 8, !tbaa !239
  %68 = load ptr, ptr %67, align 8, !tbaa !16
  %69 = lshr i64 %66, 5
  %70 = getelementptr inbounds ptr, ptr %68, i64 %69
  %71 = load ptr, ptr %70, align 8, !tbaa !10
  %72 = and i64 %66, 31
  %73 = mul nsw i64 %72, %4
  %74 = getelementptr inbounds i8, ptr %71, i64 %73
  %75 = load i64, ptr %74, align 8, !tbaa !8
  %76 = icmp eq i64 %75, %1
  br i1 %76, label %77, label %85

77:                                               ; preds = %65
  br i1 %58, label %93, label %78

78:                                               ; preds = %77
  %79 = getelementptr inbounds i8, ptr %74, i64 16
  %80 = tail call i32 %3(ptr noundef nonnull %79, ptr noundef %2) #45
  %81 = icmp eq i32 %80, 0
  br i1 %81, label %82, label %93

82:                                               ; preds = %78
  %83 = load i64, ptr %6, align 8, !tbaa !248
  %84 = load ptr, ptr %10, align 8, !tbaa !244
  br label %85

85:                                               ; preds = %82, %65, %59
  %86 = phi ptr [ %84, %82 ], [ %60, %65 ], [ %60, %59 ]
  %87 = phi i64 [ %83, %82 ], [ %61, %65 ], [ %61, %59 ]
  %88 = add nsw i64 %63, 1
  %89 = and i64 %87, %88
  %90 = getelementptr inbounds i64, ptr %86, i64 %89
  %91 = load i64, ptr %90, align 8, !tbaa !8
  %92 = icmp eq i64 %91, 0
  br i1 %92, label %93, label %59

93:                                               ; preds = %85, %78, %77, %45, %38, %37, %53, %12
  %94 = phi i64 [ -1, %12 ], [ -1, %53 ], [ -1, %45 ], [ %26, %38 ], [ %26, %37 ], [ -1, %85 ], [ %66, %78 ], [ %66, %77 ]
  ret i64 %94
}

; Function Attrs: alwaysinline nounwind optsize
define i64 @fa_table_probe(ptr nocapture noundef readonly %0, i64 noundef %1, ptr noundef %2, ptr noundef readonly %3, i64 noundef %4) local_unnamed_addr #13 {
  %6 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 5
  %7 = load i64, ptr %6, align 8, !tbaa !248
  %8 = icmp ult i64 %7, 2147483648
  %9 = and i64 %7, %1
  %10 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 4
  %11 = load ptr, ptr %10, align 8, !tbaa !244
  br i1 %8, label %12, label %60

12:                                               ; preds = %5
  %13 = getelementptr inbounds i32, ptr %11, i64 %9
  %14 = load i32, ptr %13, align 4, !tbaa !25
  %15 = icmp eq i32 %14, 0
  br i1 %15, label %21, label %16

16:                                               ; preds = %12
  %17 = icmp eq ptr %3, null
  br label %24

18:                                               ; preds = %50
  %19 = icmp slt i64 %54, 0
  %20 = select i1 %19, i64 %56, i64 %54
  br label %21

21:                                               ; preds = %18, %12
  %22 = phi i64 [ %9, %12 ], [ %20, %18 ]
  %23 = xor i64 %22, -1
  br label %107

24:                                               ; preds = %16, %50
  %25 = phi ptr [ %11, %16 ], [ %51, %50 ]
  %26 = phi i64 [ %7, %16 ], [ %52, %50 ]
  %27 = phi i32 [ %14, %16 ], [ %58, %50 ]
  %28 = phi i64 [ %9, %16 ], [ %56, %50 ]
  %29 = phi i64 [ -1, %16 ], [ %54, %50 ]
  %30 = icmp sgt i32 %27, 0
  br i1 %30, label %31, label %47

31:                                               ; preds = %24
  %32 = add nsw i32 %27, -1
  %33 = zext nneg i32 %32 to i64
  %34 = load ptr, ptr %0, align 8, !tbaa !239
  %35 = mul nsw i64 %33, %4
  %36 = getelementptr inbounds i8, ptr %34, i64 %35
  %37 = load i64, ptr %36, align 8, !tbaa !8
  %38 = icmp eq i64 %37, %1
  br i1 %38, label %39, label %50

39:                                               ; preds = %31
  br i1 %17, label %107, label %40

40:                                               ; preds = %39
  %41 = getelementptr inbounds i8, ptr %36, i64 16
  %42 = tail call i32 %3(ptr noundef nonnull %41, ptr noundef %2) #45
  %43 = icmp eq i32 %42, 0
  br i1 %43, label %44, label %107

44:                                               ; preds = %40
  %45 = load i64, ptr %6, align 8, !tbaa !248
  %46 = load ptr, ptr %10, align 8, !tbaa !244
  br label %50

47:                                               ; preds = %24
  %48 = icmp slt i64 %29, 0
  %49 = select i1 %48, i64 %28, i64 %29
  br label %50

50:                                               ; preds = %44, %47, %31
  %51 = phi ptr [ %25, %47 ], [ %25, %31 ], [ %46, %44 ]
  %52 = phi i64 [ %26, %47 ], [ %26, %31 ], [ %45, %44 ]
  %53 = phi i64 [ %49, %47 ], [ %29, %31 ], [ %29, %44 ]
  %54 = freeze i64 %53
  %55 = add nsw i64 %28, 1
  %56 = and i64 %52, %55
  %57 = getelementptr inbounds i32, ptr %51, i64 %56
  %58 = load i32, ptr %57, align 4, !tbaa !25
  %59 = icmp eq i32 %58, 0
  br i1 %59, label %18, label %24

60:                                               ; preds = %5
  %61 = getelementptr inbounds i64, ptr %11, i64 %9
  %62 = load i64, ptr %61, align 8, !tbaa !8
  %63 = icmp eq i64 %62, 0
  br i1 %63, label %69, label %64

64:                                               ; preds = %60
  %65 = icmp eq ptr %3, null
  br label %72

66:                                               ; preds = %97
  %67 = icmp slt i64 %101, 0
  %68 = select i1 %67, i64 %103, i64 %101
  br label %69

69:                                               ; preds = %66, %60
  %70 = phi i64 [ %9, %60 ], [ %68, %66 ]
  %71 = xor i64 %70, -1
  br label %107

72:                                               ; preds = %64, %97
  %73 = phi ptr [ %11, %64 ], [ %98, %97 ]
  %74 = phi i64 [ %7, %64 ], [ %99, %97 ]
  %75 = phi i64 [ %62, %64 ], [ %105, %97 ]
  %76 = phi i64 [ %9, %64 ], [ %103, %97 ]
  %77 = phi i64 [ -1, %64 ], [ %101, %97 ]
  %78 = icmp sgt i64 %75, 0
  br i1 %78, label %79, label %94

79:                                               ; preds = %72
  %80 = add nsw i64 %75, -1
  %81 = load ptr, ptr %0, align 8, !tbaa !239
  %82 = mul nsw i64 %80, %4
  %83 = getelementptr inbounds i8, ptr %81, i64 %82
  %84 = load i64, ptr %83, align 8, !tbaa !8
  %85 = icmp eq i64 %84, %1
  br i1 %85, label %86, label %97

86:                                               ; preds = %79
  br i1 %65, label %107, label %87

87:                                               ; preds = %86
  %88 = getelementptr inbounds i8, ptr %83, i64 16
  %89 = tail call i32 %3(ptr noundef nonnull %88, ptr noundef %2) #45
  %90 = icmp eq i32 %89, 0
  br i1 %90, label %91, label %107

91:                                               ; preds = %87
  %92 = load i64, ptr %6, align 8, !tbaa !248
  %93 = load ptr, ptr %10, align 8, !tbaa !244
  br label %97

94:                                               ; preds = %72
  %95 = icmp slt i64 %77, 0
  %96 = select i1 %95, i64 %76, i64 %77
  br label %97

97:                                               ; preds = %91, %94, %79
  %98 = phi ptr [ %73, %94 ], [ %73, %79 ], [ %93, %91 ]
  %99 = phi i64 [ %74, %94 ], [ %74, %79 ], [ %92, %91 ]
  %100 = phi i64 [ %96, %94 ], [ %77, %79 ], [ %77, %91 ]
  %101 = freeze i64 %100
  %102 = add nsw i64 %76, 1
  %103 = and i64 %99, %102
  %104 = getelementptr inbounds i64, ptr %98, i64 %103
  %105 = load i64, ptr %104, align 8, !tbaa !8
  %106 = icmp eq i64 %105, 0
  br i1 %106, label %66, label %72

107:                                              ; preds = %86, %87, %39, %40, %69, %21
  %108 = phi i64 [ %23, %21 ], [ %71, %69 ], [ %33, %40 ], [ %33, %39 ], [ %80, %87 ], [ %80, %86 ]
  ret i64 %108
}

; Function Attrs: alwaysinline nounwind optsize
define i64 @fa_table_probe_growing(ptr nocapture noundef readonly %0, i64 noundef %1, ptr noundef %2, ptr noundef readonly %3, i64 noundef %4) local_unnamed_addr #13 {
  %6 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 5
  %7 = load i64, ptr %6, align 8, !tbaa !248
  %8 = icmp ult i64 %7, 2147483648
  %9 = and i64 %7, %1
  %10 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 4
  %11 = load ptr, ptr %10, align 8, !tbaa !244
  br i1 %8, label %12, label %65

12:                                               ; preds = %5
  %13 = getelementptr inbounds i32, ptr %11, i64 %9
  %14 = load i32, ptr %13, align 4, !tbaa !25
  %15 = icmp eq i32 %14, 0
  br i1 %15, label %21, label %16

16:                                               ; preds = %12
  %17 = icmp eq ptr %3, null
  br label %24

18:                                               ; preds = %55
  %19 = icmp slt i64 %59, 0
  %20 = select i1 %19, i64 %61, i64 %59
  br label %21

21:                                               ; preds = %18, %12
  %22 = phi i64 [ %9, %12 ], [ %20, %18 ]
  %23 = xor i64 %22, -1
  br label %117

24:                                               ; preds = %16, %55
  %25 = phi ptr [ %11, %16 ], [ %56, %55 ]
  %26 = phi i64 [ %7, %16 ], [ %57, %55 ]
  %27 = phi i32 [ %14, %16 ], [ %63, %55 ]
  %28 = phi i64 [ %9, %16 ], [ %61, %55 ]
  %29 = phi i64 [ -1, %16 ], [ %59, %55 ]
  %30 = icmp sgt i32 %27, 0
  br i1 %30, label %31, label %52

31:                                               ; preds = %24
  %32 = add nsw i32 %27, -1
  %33 = zext nneg i32 %32 to i64
  %34 = load ptr, ptr %0, align 8, !tbaa !239
  %35 = load ptr, ptr %34, align 8, !tbaa !16
  %36 = lshr i64 %33, 5
  %37 = getelementptr inbounds ptr, ptr %35, i64 %36
  %38 = load ptr, ptr %37, align 8, !tbaa !10
  %39 = and i64 %33, 31
  %40 = mul nsw i64 %39, %4
  %41 = getelementptr inbounds i8, ptr %38, i64 %40
  %42 = load i64, ptr %41, align 8, !tbaa !8
  %43 = icmp eq i64 %42, %1
  br i1 %43, label %44, label %55

44:                                               ; preds = %31
  br i1 %17, label %117, label %45

45:                                               ; preds = %44
  %46 = getelementptr inbounds i8, ptr %41, i64 16
  %47 = tail call i32 %3(ptr noundef nonnull %46, ptr noundef %2) #45
  %48 = icmp eq i32 %47, 0
  br i1 %48, label %49, label %117

49:                                               ; preds = %45
  %50 = load i64, ptr %6, align 8, !tbaa !248
  %51 = load ptr, ptr %10, align 8, !tbaa !244
  br label %55

52:                                               ; preds = %24
  %53 = icmp slt i64 %29, 0
  %54 = select i1 %53, i64 %28, i64 %29
  br label %55

55:                                               ; preds = %49, %52, %31
  %56 = phi ptr [ %25, %52 ], [ %25, %31 ], [ %51, %49 ]
  %57 = phi i64 [ %26, %52 ], [ %26, %31 ], [ %50, %49 ]
  %58 = phi i64 [ %54, %52 ], [ %29, %31 ], [ %29, %49 ]
  %59 = freeze i64 %58
  %60 = add nsw i64 %28, 1
  %61 = and i64 %57, %60
  %62 = getelementptr inbounds i32, ptr %56, i64 %61
  %63 = load i32, ptr %62, align 4, !tbaa !25
  %64 = icmp eq i32 %63, 0
  br i1 %64, label %18, label %24

65:                                               ; preds = %5
  %66 = getelementptr inbounds i64, ptr %11, i64 %9
  %67 = load i64, ptr %66, align 8, !tbaa !8
  %68 = icmp eq i64 %67, 0
  br i1 %68, label %74, label %69

69:                                               ; preds = %65
  %70 = icmp eq ptr %3, null
  br label %77

71:                                               ; preds = %107
  %72 = icmp slt i64 %111, 0
  %73 = select i1 %72, i64 %113, i64 %111
  br label %74

74:                                               ; preds = %71, %65
  %75 = phi i64 [ %9, %65 ], [ %73, %71 ]
  %76 = xor i64 %75, -1
  br label %117

77:                                               ; preds = %69, %107
  %78 = phi ptr [ %11, %69 ], [ %108, %107 ]
  %79 = phi i64 [ %7, %69 ], [ %109, %107 ]
  %80 = phi i64 [ %67, %69 ], [ %115, %107 ]
  %81 = phi i64 [ %9, %69 ], [ %113, %107 ]
  %82 = phi i64 [ -1, %69 ], [ %111, %107 ]
  %83 = icmp sgt i64 %80, 0
  br i1 %83, label %84, label %104

84:                                               ; preds = %77
  %85 = add nsw i64 %80, -1
  %86 = load ptr, ptr %0, align 8, !tbaa !239
  %87 = load ptr, ptr %86, align 8, !tbaa !16
  %88 = lshr i64 %85, 5
  %89 = getelementptr inbounds ptr, ptr %87, i64 %88
  %90 = load ptr, ptr %89, align 8, !tbaa !10
  %91 = and i64 %85, 31
  %92 = mul nsw i64 %91, %4
  %93 = getelementptr inbounds i8, ptr %90, i64 %92
  %94 = load i64, ptr %93, align 8, !tbaa !8
  %95 = icmp eq i64 %94, %1
  br i1 %95, label %96, label %107

96:                                               ; preds = %84
  br i1 %70, label %117, label %97

97:                                               ; preds = %96
  %98 = getelementptr inbounds i8, ptr %93, i64 16
  %99 = tail call i32 %3(ptr noundef nonnull %98, ptr noundef %2) #45
  %100 = icmp eq i32 %99, 0
  br i1 %100, label %101, label %117

101:                                              ; preds = %97
  %102 = load i64, ptr %6, align 8, !tbaa !248
  %103 = load ptr, ptr %10, align 8, !tbaa !244
  br label %107

104:                                              ; preds = %77
  %105 = icmp slt i64 %82, 0
  %106 = select i1 %105, i64 %81, i64 %82
  br label %107

107:                                              ; preds = %101, %104, %84
  %108 = phi ptr [ %78, %104 ], [ %78, %84 ], [ %103, %101 ]
  %109 = phi i64 [ %79, %104 ], [ %79, %84 ], [ %102, %101 ]
  %110 = phi i64 [ %106, %104 ], [ %82, %84 ], [ %82, %101 ]
  %111 = freeze i64 %110
  %112 = add nsw i64 %81, 1
  %113 = and i64 %109, %112
  %114 = getelementptr inbounds i64, ptr %108, i64 %113
  %115 = load i64, ptr %114, align 8, !tbaa !8
  %116 = icmp eq i64 %115, 0
  br i1 %116, label %71, label %77

117:                                              ; preds = %96, %97, %44, %45, %74, %21
  %118 = phi i64 [ %23, %21 ], [ %76, %74 ], [ %33, %45 ], [ %33, %44 ], [ %85, %97 ], [ %85, %96 ]
  ret i64 %118
}

; Function Attrs: nounwind optsize
define noundef ptr @fa_table_append(ptr nocapture noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3) local_unnamed_addr #2 {
  %5 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 2
  %6 = load i64, ptr %5, align 8, !tbaa !242
  %7 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 3
  %8 = load i64, ptr %7, align 8, !tbaa !243
  %9 = icmp slt i64 %6, %8
  br i1 %9, label %14, label %10

10:                                               ; preds = %4
  %11 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 7
  %12 = load i64, ptr %11, align 8, !tbaa !246
  %13 = icmp eq i64 %12, 0
  br i1 %13, label %24, label %14

14:                                               ; preds = %10, %4
  %15 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 8
  %16 = load i64, ptr %15, align 8, !tbaa !249
  %17 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 5
  %18 = load i64, ptr %17, align 8, !tbaa !248
  %19 = add nsw i64 %18, 1
  %20 = sdiv i64 %19, 4
  %21 = mul nsw i64 %20, 3
  %22 = icmp slt i64 %16, %21
  br i1 %22, label %81, label %23

23:                                               ; preds = %14
  tail call fastcc void @table_index(ptr noundef nonnull %0, i64 noundef %19, i64 noundef %2) #44
  br label %85

24:                                               ; preds = %10
  %25 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 6
  %26 = load i64, ptr %25, align 8, !tbaa !245
  %27 = icmp eq i64 %26, 0
  br i1 %27, label %36, label %28

28:                                               ; preds = %24
  %29 = shl nsw i64 %8, 1
  store i64 %29, ptr %7, align 8, !tbaa !243
  %30 = shl nsw i64 %8, 2
  br label %31

31:                                               ; preds = %31, %28
  %32 = phi i64 [ 8, %28 ], [ %34, %31 ]
  %33 = icmp slt i64 %32, %30
  %34 = shl nsw i64 %32, 1
  br i1 %33, label %31, label %35, !llvm.loop !247

35:                                               ; preds = %31
  tail call fastcc void @table_index(ptr noundef %0, i64 noundef %32, i64 noundef %2) #44
  br label %85

36:                                               ; preds = %24
  %37 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 1
  %38 = load i64, ptr %37, align 8, !tbaa !241
  %39 = shl nsw i64 %38, 1
  %40 = icmp sgt i64 %39, %6
  %41 = zext i1 %40 to i64
  %42 = shl nsw i64 %8, %41
  %43 = mul nsw i64 %42, %2
  %44 = tail call ptr @fa_alloc(i64 noundef %43, i64 noundef 16) #45
  %45 = load i64, ptr %5, align 8, !tbaa !242
  %46 = icmp sgt i64 %45, 0
  br i1 %46, label %56, label %47

47:                                               ; preds = %71, %36
  %48 = phi i64 [ 0, %36 ], [ %73, %71 ]
  %49 = load ptr, ptr %0, align 8, !tbaa !239
  %50 = load i64, ptr %7, align 8, !tbaa !243
  %51 = mul nsw i64 %50, %2
  tail call void @fa_free(ptr noundef %49, i64 noundef %51, i64 noundef 16) #45
  store ptr %44, ptr %0, align 8, !tbaa !239
  store i64 %48, ptr %5, align 8, !tbaa !242
  store i64 %42, ptr %7, align 8, !tbaa !243
  %52 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 5
  %53 = load i64, ptr %52, align 8, !tbaa !248
  %54 = add nsw i64 %53, 1
  %55 = shl nsw i64 %42, 1
  br label %76

56:                                               ; preds = %36, %71
  %57 = phi i64 [ %72, %71 ], [ %45, %36 ]
  %58 = phi i64 [ %74, %71 ], [ 0, %36 ]
  %59 = phi i64 [ %73, %71 ], [ 0, %36 ]
  %60 = load ptr, ptr %0, align 8, !tbaa !239
  %61 = mul nsw i64 %58, %2
  %62 = getelementptr inbounds i8, ptr %60, i64 %61
  %63 = getelementptr inbounds i8, ptr %62, i64 8
  %64 = load i64, ptr %63, align 8, !tbaa !8
  %65 = icmp eq i64 %64, 0
  br i1 %65, label %71, label %66

66:                                               ; preds = %56
  %67 = mul nsw i64 %59, %2
  %68 = getelementptr inbounds i8, ptr %44, i64 %67
  tail call fastcc void @copy_bytes(ptr noundef %68, ptr noundef nonnull %62, i64 noundef %2) #44
  %69 = add nsw i64 %59, 1
  %70 = load i64, ptr %5, align 8, !tbaa !242
  br label %71

71:                                               ; preds = %66, %56
  %72 = phi i64 [ %70, %66 ], [ %57, %56 ]
  %73 = phi i64 [ %69, %66 ], [ %59, %56 ]
  %74 = add nuw nsw i64 %58, 1
  %75 = icmp slt i64 %74, %72
  br i1 %75, label %56, label %47, !llvm.loop !252

76:                                               ; preds = %76, %47
  %77 = phi i64 [ %54, %47 ], [ %79, %76 ]
  %78 = icmp slt i64 %77, %55
  %79 = shl nsw i64 %77, 1
  br i1 %78, label %76, label %80, !llvm.loop !253

80:                                               ; preds = %76
  tail call fastcc void @table_index(ptr noundef nonnull %0, i64 noundef %77, i64 noundef %2) #44
  br label %85

81:                                               ; preds = %14
  %82 = icmp sgt i64 %3, -1
  br i1 %82, label %85, label %83

83:                                               ; preds = %81
  %84 = xor i64 %3, -1
  br label %109

85:                                               ; preds = %80, %35, %23, %81
  %86 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 5
  %87 = load i64, ptr %86, align 8, !tbaa !248
  %88 = and i64 %87, %1
  %89 = icmp ult i64 %87, 2147483648
  %90 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 4
  %91 = load ptr, ptr %90, align 8, !tbaa !244
  br label %92

92:                                               ; preds = %104, %85
  %93 = phi i64 [ %88, %85 ], [ %106, %104 ]
  br i1 %89, label %94, label %98

94:                                               ; preds = %92
  %95 = getelementptr inbounds i32, ptr %91, i64 %93
  %96 = load i32, ptr %95, align 4, !tbaa !25
  %97 = sext i32 %96 to i64
  br label %101

98:                                               ; preds = %92
  %99 = getelementptr inbounds i64, ptr %91, i64 %93
  %100 = load i64, ptr %99, align 8, !tbaa !8
  br label %101

101:                                              ; preds = %94, %98
  %102 = phi i64 [ %97, %94 ], [ %100, %98 ]
  %103 = icmp sgt i64 %102, 0
  br i1 %103, label %104, label %107

104:                                              ; preds = %101
  %105 = add nsw i64 %93, 1
  %106 = and i64 %105, %87
  br label %92, !llvm.loop !254

107:                                              ; preds = %101
  %108 = load i64, ptr %5, align 8, !tbaa !242
  br label %109

109:                                              ; preds = %107, %83
  %110 = phi i64 [ %6, %83 ], [ %108, %107 ]
  %111 = phi i64 [ %84, %83 ], [ %93, %107 ]
  %112 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 7
  %113 = load i64, ptr %112, align 8, !tbaa !246
  %114 = icmp eq i64 %113, 0
  br i1 %114, label %126, label %115

115:                                              ; preds = %109
  %116 = add nsw i64 %113, -1
  %117 = load ptr, ptr %0, align 8, !tbaa !239
  %118 = load ptr, ptr %117, align 8, !tbaa !16
  %119 = ashr i64 %116, 5
  %120 = getelementptr inbounds ptr, ptr %118, i64 %119
  %121 = load ptr, ptr %120, align 8, !tbaa !10
  %122 = and i64 %116, 31
  %123 = mul nsw i64 %122, %2
  %124 = getelementptr inbounds i8, ptr %121, i64 %123
  %125 = load i64, ptr %124, align 8, !tbaa !8
  store i64 %125, ptr %112, align 8, !tbaa !246
  br label %139

126:                                              ; preds = %109
  %127 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 6
  %128 = load i64, ptr %127, align 8, !tbaa !245
  %129 = icmp eq i64 %128, 0
  br i1 %129, label %134, label %130

130:                                              ; preds = %126
  %131 = tail call fastcc ptr @table_stable_slot(ptr noundef nonnull %0, i64 noundef %110, i64 noundef %2) #44
  %132 = load i64, ptr %5, align 8, !tbaa !242
  %133 = add nsw i64 %132, 1
  store i64 %133, ptr %5, align 8, !tbaa !242
  br label %139

134:                                              ; preds = %126
  %135 = load ptr, ptr %0, align 8, !tbaa !239
  %136 = mul nsw i64 %110, %2
  %137 = getelementptr inbounds i8, ptr %135, i64 %136
  %138 = add nsw i64 %110, 1
  store i64 %138, ptr %5, align 8, !tbaa !242
  br label %139

139:                                              ; preds = %130, %134, %115
  %140 = phi i64 [ %116, %115 ], [ %110, %130 ], [ %110, %134 ]
  %141 = phi ptr [ %124, %115 ], [ %131, %130 ], [ %137, %134 ]
  store i64 %1, ptr %141, align 8, !tbaa !8
  %142 = add nsw i64 %111, 1
  %143 = getelementptr inbounds i8, ptr %141, i64 8
  store i64 %142, ptr %143, align 8, !tbaa !8
  %144 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 5
  %145 = load i64, ptr %144, align 8, !tbaa !248
  %146 = icmp ult i64 %145, 2147483648
  %147 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 4
  %148 = load ptr, ptr %147, align 8, !tbaa !244
  br i1 %146, label %149, label %153

149:                                              ; preds = %139
  %150 = getelementptr inbounds i32, ptr %148, i64 %111
  %151 = load i32, ptr %150, align 4, !tbaa !25
  %152 = sext i32 %151 to i64
  br label %156

153:                                              ; preds = %139
  %154 = getelementptr inbounds i64, ptr %148, i64 %111
  %155 = load i64, ptr %154, align 8, !tbaa !8
  br label %156

156:                                              ; preds = %149, %153
  %157 = phi i64 [ %152, %149 ], [ %155, %153 ]
  %158 = icmp eq i64 %157, 0
  br i1 %158, label %159, label %163

159:                                              ; preds = %156
  %160 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 8
  %161 = load i64, ptr %160, align 8, !tbaa !249
  %162 = add nsw i64 %161, 1
  store i64 %162, ptr %160, align 8, !tbaa !249
  br label %163

163:                                              ; preds = %159, %156
  %164 = add nsw i64 %140, 1
  br i1 %146, label %165, label %168

165:                                              ; preds = %163
  %166 = trunc i64 %164 to i32
  %167 = getelementptr inbounds i32, ptr %148, i64 %111
  store i32 %166, ptr %167, align 4, !tbaa !25
  br label %170

168:                                              ; preds = %163
  %169 = getelementptr inbounds i64, ptr %148, i64 %111
  store i64 %164, ptr %169, align 8, !tbaa !8
  br label %170

170:                                              ; preds = %165, %168
  %171 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 1
  %172 = load i64, ptr %171, align 8, !tbaa !241
  %173 = add nsw i64 %172, 1
  store i64 %173, ptr %171, align 8, !tbaa !241
  ret ptr %141
}

; Function Attrs: nounwind optsize
define internal fastcc ptr @table_stable_slot(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2) unnamed_addr #2 {
  %4 = load ptr, ptr %0, align 8, !tbaa !239
  %5 = ashr i64 %1, 5
  %6 = getelementptr inbounds %struct.FaTableEntries, ptr %4, i64 0, i32 1
  %7 = load i64, ptr %6, align 8, !tbaa !21
  %8 = icmp eq i64 %5, %7
  br i1 %8, label %9, label %35

9:                                                ; preds = %3
  %10 = getelementptr inbounds %struct.FaTableEntries, ptr %4, i64 0, i32 2
  %11 = load i64, ptr %10, align 8, !tbaa !22
  %12 = icmp eq i64 %5, %11
  br i1 %12, label %13, label %28

13:                                               ; preds = %9
  %14 = icmp ult i64 %1, 32
  %15 = shl nsw i64 %5, 1
  %16 = select i1 %14, i64 4, i64 %15
  %17 = shl nsw i64 %16, 3
  %18 = tail call ptr @fa_alloc(i64 noundef %17, i64 noundef 8) #45
  %19 = load ptr, ptr %4, align 8, !tbaa !16
  %20 = icmp eq ptr %19, null
  br i1 %20, label %27, label %21

21:                                               ; preds = %13
  %22 = load i64, ptr %6, align 8, !tbaa !21
  %23 = shl i64 %22, 3
  tail call fastcc void @copy_bytes(ptr noundef %18, ptr noundef nonnull %19, i64 noundef %23) #44
  %24 = load ptr, ptr %4, align 8, !tbaa !16
  %25 = load i64, ptr %10, align 8, !tbaa !22
  %26 = shl i64 %25, 3
  tail call void @fa_free(ptr noundef %24, i64 noundef %26, i64 noundef 8) #45
  br label %27

27:                                               ; preds = %21, %13
  store ptr %18, ptr %4, align 8, !tbaa !16
  store i64 %16, ptr %10, align 8, !tbaa !22
  br label %28

28:                                               ; preds = %27, %9
  %29 = shl i64 %2, 5
  %30 = tail call ptr @fa_alloc(i64 noundef %29, i64 noundef 16) #45
  %31 = load ptr, ptr %4, align 8, !tbaa !16
  %32 = getelementptr inbounds ptr, ptr %31, i64 %5
  store ptr %30, ptr %32, align 8, !tbaa !10
  %33 = load i64, ptr %6, align 8, !tbaa !21
  %34 = add nsw i64 %33, 1
  store i64 %34, ptr %6, align 8, !tbaa !21
  br label %35

35:                                               ; preds = %28, %3
  %36 = load ptr, ptr %4, align 8, !tbaa !16
  %37 = getelementptr inbounds ptr, ptr %36, i64 %5
  %38 = load ptr, ptr %37, align 8, !tbaa !10
  %39 = and i64 %1, 31
  %40 = mul nsw i64 %39, %2
  %41 = getelementptr inbounds i8, ptr %38, i64 %40
  ret ptr %41
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define void @fa_table_kill(ptr nocapture noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #24 {
  %4 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 6
  %5 = load i64, ptr %4, align 8, !tbaa !245
  %6 = icmp eq i64 %5, 0
  %7 = load ptr, ptr %0, align 8, !tbaa !239
  br i1 %6, label %16, label %8

8:                                                ; preds = %3
  %9 = load ptr, ptr %7, align 8, !tbaa !16
  %10 = ashr i64 %1, 5
  %11 = getelementptr inbounds ptr, ptr %9, i64 %10
  %12 = load ptr, ptr %11, align 8, !tbaa !10
  %13 = and i64 %1, 31
  %14 = mul nsw i64 %13, %2
  %15 = getelementptr inbounds i8, ptr %12, i64 %14
  br label %19

16:                                               ; preds = %3
  %17 = mul nsw i64 %2, %1
  %18 = getelementptr inbounds i8, ptr %7, i64 %17
  br label %19

19:                                               ; preds = %8, %16
  %20 = phi ptr [ %15, %8 ], [ %18, %16 ]
  %21 = getelementptr inbounds i8, ptr %20, i64 8
  %22 = load i64, ptr %21, align 8, !tbaa !8
  %23 = add nsw i64 %22, -1
  %24 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 5
  %25 = load i64, ptr %24, align 8, !tbaa !248
  %26 = icmp ult i64 %25, 2147483648
  %27 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 4
  %28 = load ptr, ptr %27, align 8, !tbaa !244
  br i1 %26, label %29, label %31

29:                                               ; preds = %19
  %30 = getelementptr inbounds i32, ptr %28, i64 %23
  store i32 -1, ptr %30, align 4, !tbaa !25
  br label %33

31:                                               ; preds = %19
  %32 = getelementptr inbounds i64, ptr %28, i64 %23
  store i64 -1, ptr %32, align 8, !tbaa !8
  br label %33

33:                                               ; preds = %29, %31
  store i64 0, ptr %21, align 8, !tbaa !8
  %34 = load i64, ptr %4, align 8, !tbaa !245
  %35 = icmp eq i64 %34, 2
  br i1 %35, label %36, label %40

36:                                               ; preds = %33
  %37 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 7
  %38 = load i64, ptr %37, align 8, !tbaa !246
  store i64 %38, ptr %20, align 8, !tbaa !8
  %39 = add nsw i64 %1, 1
  store i64 %39, ptr %37, align 8, !tbaa !246
  br label %40

40:                                               ; preds = %36, %33
  %41 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 1
  %42 = load i64, ptr %41, align 8, !tbaa !241
  %43 = add nsw i64 %42, -1
  store i64 %43, ptr %41, align 8, !tbaa !241
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, argmem: readwrite, inaccessiblemem: none)
define void @fa_table_clear(ptr nocapture noundef %0) local_unnamed_addr #34 {
  %2 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 1
  store i64 0, ptr %2, align 8, !tbaa !241
  %3 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 2
  store i64 0, ptr %3, align 8, !tbaa !242
  %4 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 7
  store i64 0, ptr %4, align 8, !tbaa !246
  %5 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 8
  store i64 0, ptr %5, align 8, !tbaa !249
  %6 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 4
  %7 = load ptr, ptr %6, align 8, !tbaa !244
  %8 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 5
  %9 = load i64, ptr %8, align 8, !tbaa !248
  %10 = add nsw i64 %9, 1
  %11 = icmp ult i64 %9, 2147483648
  %12 = select i1 %11, i64 2, i64 3
  %13 = shl i64 %10, %12
  tail call void @llvm.memset.p0.i64(ptr align 1 %7, i8 0, i64 %13, i1 false)
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_table_free(ptr noundef %0, i64 noundef %1) local_unnamed_addr #2 {
  %3 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 6
  %4 = load i64, ptr %3, align 8, !tbaa !245
  %5 = icmp eq i64 %4, 0
  %6 = load ptr, ptr %0, align 8, !tbaa !239
  br i1 %5, label %29, label %7

7:                                                ; preds = %2
  %8 = getelementptr inbounds %struct.FaTableEntries, ptr %6, i64 0, i32 1
  %9 = load i64, ptr %8, align 8, !tbaa !21
  %10 = icmp sgt i64 %9, 0
  br i1 %10, label %11, label %13

11:                                               ; preds = %7
  %12 = shl i64 %1, 5
  br label %16

13:                                               ; preds = %16, %7
  %14 = load ptr, ptr %6, align 8, !tbaa !16
  %15 = icmp eq ptr %14, null
  br i1 %15, label %28, label %24

16:                                               ; preds = %11, %16
  %17 = phi i64 [ 0, %11 ], [ %21, %16 ]
  %18 = load ptr, ptr %6, align 8, !tbaa !16
  %19 = getelementptr inbounds ptr, ptr %18, i64 %17
  %20 = load ptr, ptr %19, align 8, !tbaa !10
  tail call void @fa_free(ptr noundef %20, i64 noundef %12, i64 noundef 16) #45
  %21 = add nuw nsw i64 %17, 1
  %22 = load i64, ptr %8, align 8, !tbaa !21
  %23 = icmp slt i64 %21, %22
  br i1 %23, label %16, label %13, !llvm.loop !255

24:                                               ; preds = %13
  %25 = getelementptr inbounds %struct.FaTableEntries, ptr %6, i64 0, i32 2
  %26 = load i64, ptr %25, align 8, !tbaa !22
  %27 = shl i64 %26, 3
  tail call void @fa_free(ptr noundef nonnull %14, i64 noundef %27, i64 noundef 8) #45
  br label %28

28:                                               ; preds = %24, %13
  tail call void @fa_free(ptr noundef nonnull %6, i64 noundef 24, i64 noundef 8) #45
  br label %33

29:                                               ; preds = %2
  %30 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 3
  %31 = load i64, ptr %30, align 8, !tbaa !243
  %32 = mul nsw i64 %31, %1
  tail call void @fa_free(ptr noundef %6, i64 noundef %32, i64 noundef 16) #45
  br label %33

33:                                               ; preds = %29, %28
  %34 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 4
  %35 = load ptr, ptr %34, align 8, !tbaa !244
  %36 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 5
  %37 = load i64, ptr %36, align 8, !tbaa !248
  %38 = add nsw i64 %37, 1
  %39 = icmp ult i64 %37, 2147483648
  %40 = select i1 %39, i64 2, i64 3
  %41 = shl i64 %38, %40
  tail call void @fa_free(ptr noundef %35, i64 noundef %41, i64 noundef 8) #45
  tail call void @fa_free(ptr noundef nonnull %0, i64 noundef 72, i64 noundef 8) #45
  ret void
}

; Function Attrs: nounwind optsize
define ptr @fa_table_copy(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #2 {
  %3 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 6
  %4 = load i64, ptr %3, align 8, !tbaa !245
  %5 = icmp eq i64 %4, 0
  %6 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 1
  %7 = load i64, ptr %6, align 8, !tbaa !241
  %8 = tail call i64 @llvm.smax.i64(i64 %7, i64 4)
  %9 = icmp eq i64 %4, 2
  br i1 %9, label %10, label %13

10:                                               ; preds = %2
  %11 = tail call ptr @fa_table_new_growing(i64 noundef %8, i64 noundef %1) #44
  %12 = getelementptr inbounds %struct.FaTable, ptr %11, i64 0, i32 6
  store i64 2, ptr %12, align 8, !tbaa !245
  br label %18

13:                                               ; preds = %2
  br i1 %5, label %16, label %14

14:                                               ; preds = %13
  %15 = tail call ptr @fa_table_new_growing(i64 noundef %8, i64 noundef %1) #44
  br label %18

16:                                               ; preds = %13
  %17 = tail call ptr @fa_table_new(i64 noundef %8, i64 noundef %1) #44
  br label %18

18:                                               ; preds = %14, %16, %10
  %19 = phi ptr [ %11, %10 ], [ %15, %14 ], [ %17, %16 ]
  %20 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 2
  %21 = load i64, ptr %20, align 8, !tbaa !242
  %22 = icmp sgt i64 %21, 0
  br i1 %22, label %30, label %23

23:                                               ; preds = %63, %18
  %24 = phi i64 [ 0, %18 ], [ %65, %63 ]
  %25 = getelementptr inbounds %struct.FaTable, ptr %19, i64 0, i32 2
  store i64 %24, ptr %25, align 8, !tbaa !242
  %26 = getelementptr inbounds %struct.FaTable, ptr %19, i64 0, i32 1
  store i64 %24, ptr %26, align 8, !tbaa !241
  %27 = getelementptr inbounds %struct.FaTable, ptr %19, i64 0, i32 5
  %28 = load i64, ptr %27, align 8, !tbaa !248
  %29 = add nsw i64 %28, 1
  tail call fastcc void @table_index(ptr noundef %19, i64 noundef %29, i64 noundef %1) #44
  ret ptr %19

30:                                               ; preds = %18, %63
  %31 = phi i64 [ %64, %63 ], [ %21, %18 ]
  %32 = phi i64 [ %65, %63 ], [ 0, %18 ]
  %33 = phi i64 [ %66, %63 ], [ 0, %18 ]
  %34 = load ptr, ptr %0, align 8, !tbaa !239
  br i1 %5, label %35, label %41

35:                                               ; preds = %30
  %36 = mul nsw i64 %33, %1
  %37 = getelementptr inbounds i8, ptr %34, i64 %36
  %38 = getelementptr inbounds i8, ptr %37, i64 8
  %39 = load i64, ptr %38, align 8, !tbaa !8
  %40 = icmp eq i64 %39, 0
  br i1 %40, label %63, label %54

41:                                               ; preds = %30
  %42 = load ptr, ptr %34, align 8, !tbaa !16
  %43 = lshr i64 %33, 5
  %44 = getelementptr inbounds ptr, ptr %42, i64 %43
  %45 = load ptr, ptr %44, align 8, !tbaa !10
  %46 = and i64 %33, 31
  %47 = mul nsw i64 %46, %1
  %48 = getelementptr inbounds i8, ptr %45, i64 %47
  %49 = getelementptr inbounds i8, ptr %48, i64 8
  %50 = load i64, ptr %49, align 8, !tbaa !8
  %51 = icmp eq i64 %50, 0
  br i1 %51, label %63, label %52

52:                                               ; preds = %41
  %53 = tail call fastcc ptr @table_stable_slot(ptr noundef %19, i64 noundef %32, i64 noundef %1) #44
  br label %58

54:                                               ; preds = %35
  %55 = load ptr, ptr %19, align 8, !tbaa !239
  %56 = mul nsw i64 %32, %1
  %57 = getelementptr inbounds i8, ptr %55, i64 %56
  br label %58

58:                                               ; preds = %54, %52
  %59 = phi ptr [ %48, %52 ], [ %37, %54 ]
  %60 = phi ptr [ %53, %52 ], [ %57, %54 ]
  tail call fastcc void @copy_bytes(ptr noundef %60, ptr noundef nonnull %59, i64 noundef %1) #44
  %61 = add nsw i64 %32, 1
  %62 = load i64, ptr %20, align 8, !tbaa !242
  br label %63

63:                                               ; preds = %41, %58, %35
  %64 = phi i64 [ %62, %58 ], [ %31, %35 ], [ %31, %41 ]
  %65 = phi i64 [ %61, %58 ], [ %32, %35 ], [ %32, %41 ]
  %66 = add nuw nsw i64 %33, 1
  %67 = icmp slt i64 %66, %64
  br i1 %67, label %30, label %23, !llvm.loop !256
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: read)
define i64 @fa_hash_bytes(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #12 {
  %3 = mul i64 %1, -7046029254386353131
  %4 = xor i64 %3, -3750763034362895579
  %5 = icmp ult i64 %1, 8
  br i1 %5, label %18, label %6

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %16, %6 ], [ 8, %2 ]
  %8 = phi i64 [ %15, %6 ], [ %4, %2 ]
  %9 = phi i64 [ %7, %6 ], [ 0, %2 ]
  %10 = getelementptr inbounds i8, ptr %0, i64 %9
  %11 = load i64, ptr %10, align 1
  %12 = xor i64 %11, %8
  %13 = mul i64 %12, -7046029254386353131
  %14 = lshr i64 %13, 29
  %15 = xor i64 %14, %13
  %16 = add nuw i64 %7, 8
  %17 = icmp ugt i64 %16, %1
  br i1 %17, label %18, label %6, !llvm.loop !257

18:                                               ; preds = %6, %2
  %19 = phi i64 [ 0, %2 ], [ %7, %6 ]
  %20 = phi i64 [ %4, %2 ], [ %15, %6 ]
  %21 = icmp ult i64 %19, %1
  br i1 %21, label %22, label %55

22:                                               ; preds = %18
  %23 = sub i64 %1, %19
  %24 = icmp ugt i64 %23, 3
  %25 = getelementptr inbounds i8, ptr %0, i64 %19
  br i1 %24, label %26, label %35

26:                                               ; preds = %22
  %27 = load i32, ptr %25, align 1
  %28 = getelementptr inbounds i8, ptr %0, i64 %1
  %29 = getelementptr inbounds i8, ptr %28, i64 -4
  %30 = load i32, ptr %29, align 1
  %31 = zext i32 %27 to i64
  %32 = zext i32 %30 to i64
  %33 = shl nuw i64 %32, 32
  %34 = or disjoint i64 %33, %31
  br label %51

35:                                               ; preds = %22
  %36 = load i8, ptr %25, align 1, !tbaa !2
  %37 = zext i8 %36 to i64
  %38 = lshr i64 %23, 1
  %39 = or disjoint i64 %38, %19
  %40 = getelementptr inbounds i8, ptr %0, i64 %39
  %41 = load i8, ptr %40, align 1, !tbaa !2
  %42 = zext i8 %41 to i64
  %43 = shl nuw nsw i64 %42, 8
  %44 = or disjoint i64 %43, %37
  %45 = getelementptr i8, ptr %0, i64 %1
  %46 = getelementptr i8, ptr %45, i64 -1
  %47 = load i8, ptr %46, align 1, !tbaa !2
  %48 = zext i8 %47 to i64
  %49 = shl nuw nsw i64 %48, 16
  %50 = or disjoint i64 %44, %49
  br label %51

51:                                               ; preds = %35, %26
  %52 = phi i64 [ %34, %26 ], [ %50, %35 ]
  %53 = xor i64 %52, %20
  %54 = mul i64 %53, -7046029254386353131
  br label %55

55:                                               ; preds = %51, %18
  %56 = phi i64 [ %54, %51 ], [ %20, %18 ]
  %57 = lshr i64 %56, 33
  %58 = xor i64 %57, %56
  %59 = mul i64 %58, -49064778989728563
  %60 = lshr i64 %59, 33
  %61 = xor i64 %60, %59
  ret i64 %61
}

; Function Attrs: alwaysinline nofree norecurse nosync nounwind optsize memory(read, inaccessiblemem: none)
define i64 @fa_hash_string_bits(i64 noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #35 {
  %4 = and i64 %2, 4611686018427387904
  %5 = icmp eq i64 %4, 0
  br i1 %5, label %6, label %9

6:                                                ; preds = %3
  %7 = inttoptr i64 %0 to ptr
  %8 = tail call i64 @fa_hash_bytes(ptr noundef %7, i64 noundef %1) #44
  br label %67

9:                                                ; preds = %3
  %10 = lshr i64 %2, 56
  %11 = and i64 %10, 31
  %12 = mul i64 %11, -7046029254386353131
  %13 = xor i64 %12, -3750763034362895579
  %14 = icmp ugt i64 %11, 7
  br i1 %14, label %15, label %26

15:                                               ; preds = %9
  %16 = xor i64 %13, %0
  %17 = mul i64 %16, -7046029254386353131
  %18 = lshr i64 %17, 29
  %19 = xor i64 %18, %17
  %20 = icmp ugt i64 %11, 15
  br i1 %20, label %21, label %26

21:                                               ; preds = %15
  %22 = xor i64 %19, %1
  %23 = mul i64 %22, -7046029254386353131
  %24 = lshr i64 %23, 29
  %25 = xor i64 %24, %23
  br label %26

26:                                               ; preds = %9, %21, %15
  %27 = phi i1 [ true, %21 ], [ false, %15 ], [ false, %9 ]
  %28 = phi i64 [ %25, %21 ], [ %19, %15 ], [ %13, %9 ]
  %29 = and i64 %10, 7
  %30 = icmp eq i64 %29, 0
  br i1 %30, label %60, label %31

31:                                               ; preds = %26
  %32 = select i1 %14, i64 %1, i64 %0
  %33 = select i1 %27, i64 %2, i64 %32
  %34 = icmp ugt i64 %29, 3
  br i1 %34, label %35, label %42

35:                                               ; preds = %31
  %36 = and i64 %33, 4294967295
  %37 = shl nuw nsw i64 %29, 3
  %38 = add nsw i64 %37, -32
  %39 = lshr i64 %33, %38
  %40 = shl i64 %39, 32
  %41 = or disjoint i64 %40, %36
  br label %56

42:                                               ; preds = %31
  %43 = and i64 %33, 255
  %44 = shl nuw nsw i64 %10, 2
  %45 = and i64 %44, 8
  %46 = lshr i64 %33, %45
  %47 = shl i64 %46, 8
  %48 = and i64 %47, 65280
  %49 = or disjoint i64 %48, %43
  %50 = shl nuw nsw i64 %29, 3
  %51 = add nsw i64 %50, -8
  %52 = lshr i64 %33, %51
  %53 = shl i64 %52, 16
  %54 = and i64 %53, 16711680
  %55 = or disjoint i64 %49, %54
  br label %56

56:                                               ; preds = %42, %35
  %57 = phi i64 [ %41, %35 ], [ %55, %42 ]
  %58 = xor i64 %57, %28
  %59 = mul i64 %58, -7046029254386353131
  br label %60

60:                                               ; preds = %56, %26
  %61 = phi i64 [ %59, %56 ], [ %28, %26 ]
  %62 = lshr i64 %61, 33
  %63 = xor i64 %62, %61
  %64 = mul i64 %63, -49064778989728563
  %65 = lshr i64 %64, 33
  %66 = xor i64 %65, %64
  br label %67

67:                                               ; preds = %60, %6
  %68 = phi i64 [ %66, %60 ], [ %8, %6 ]
  ret i64 %68
}

; Function Attrs: cold noinline nounwind optsize
define noundef ptr @fa_error_key_not_found() local_unnamed_addr #19 {
  %1 = alloca %struct.FaString, align 8
  %2 = tail call ptr @fa_alloc(i64 noundef 32, i64 noundef 8) #45
  store i64 2, ptr %2, align 8, !tbaa !232
  %3 = getelementptr inbounds %struct.MessageError, ptr %2, i64 0, i32 1
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %1) #40
  %4 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %5 = load ptr, ptr %4, align 8, !tbaa !10
  %6 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %7 = load i64, ptr %6, align 8, !tbaa !8
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 8 %1, ptr noundef %5, i64 noundef %7, i64 noundef 0) #44
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %3, ptr noundef nonnull align 8 dereferenceable(24) %1, i64 24, i1 false), !tbaa.struct !13
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %1) #40
  store i64 0, ptr %6, align 8, !tbaa !8
  ret ptr %2
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(argmem: readwrite)
define noundef ptr @fa_shared_retain(ptr noundef returned %0) local_unnamed_addr #36 {
  %2 = icmp eq ptr %0, null
  br i1 %2, label %5, label %3

3:                                                ; preds = %1
  %4 = atomicrmw add ptr %0, i64 1 monotonic, align 8
  br label %5

5:                                                ; preds = %3, %1
  ret ptr %0
}

; Function Attrs: nounwind optsize
define void @fa_shared_release(ptr noundef %0) local_unnamed_addr #2 {
  %2 = icmp eq ptr %0, null
  br i1 %2, label %14, label %3

3:                                                ; preds = %1
  %4 = atomicrmw sub ptr %0, i64 1 acq_rel, align 8
  %5 = icmp eq i64 %4, 1
  br i1 %5, label %6, label %14

6:                                                ; preds = %3
  %7 = getelementptr inbounds %struct.FaShared, ptr %0, i64 0, i32 2
  %8 = load ptr, ptr %7, align 8, !tbaa !258
  %9 = icmp eq ptr %8, null
  br i1 %9, label %11, label %10

10:                                               ; preds = %6
  tail call void %8(ptr noundef nonnull %0) #45
  br label %11

11:                                               ; preds = %10, %6
  %12 = getelementptr inbounds %struct.FaShared, ptr %0, i64 0, i32 1
  %13 = load i64, ptr %12, align 8, !tbaa !260
  tail call void @fa_free(ptr noundef nonnull %0, i64 noundef %13, i64 noundef 16) #45
  br label %14

14:                                               ; preds = %11, %3, %1
  ret void
}

; Function Attrs: nounwind optsize
define ptr @fa_atomic_new(i64 noundef %0) local_unnamed_addr #2 {
  %2 = tail call fastcc ptr @shared_new(i64 noundef 40, ptr noundef null) #44
  %3 = getelementptr inbounds %struct.FaAtomicBlock, ptr %2, i64 0, i32 1
  store i64 %0, ptr %3, align 8, !tbaa !261
  ret ptr %2
}

; Function Attrs: nounwind optsize
define internal fastcc ptr @shared_new(i64 noundef %0, ptr noundef %1) unnamed_addr #2 {
  %3 = tail call ptr @fa_alloc(i64 noundef %0, i64 noundef 16) #45
  %4 = icmp eq ptr %3, null
  br i1 %4, label %5, label %6

5:                                                ; preds = %2
  tail call fastcc void @out_of_memory() #47
  unreachable

6:                                                ; preds = %2
  tail call void @llvm.memset.p0.i64(ptr nonnull align 8 %3, i8 0, i64 %0, i1 false)
  store i64 1, ptr %3, align 8, !tbaa !263
  %7 = getelementptr inbounds %struct.FaShared, ptr %3, i64 0, i32 1
  store i64 %0, ptr %7, align 8, !tbaa !260
  %8 = getelementptr inbounds %struct.FaShared, ptr %3, i64 0, i32 2
  store ptr %1, ptr %8, align 8, !tbaa !258
  ret ptr %3
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(argmem: readwrite)
define void @fa_remote_source_start(ptr nocapture noundef %0) local_unnamed_addr #36 {
  %2 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 12
  %3 = atomicrmw add ptr %2, i64 1 acq_rel, align 8
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_remote_source_end(ptr nocapture noundef %0) local_unnamed_addr #2 {
  %2 = alloca i64, align 8
  %3 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 14
  %4 = atomicrmw add ptr %3, i64 1 seq_cst, align 8
  %5 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 12
  %6 = atomicrmw sub ptr %5, i64 1 seq_cst, align 8
  %7 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 18
  %8 = atomicrmw xchg ptr %7, i32 0 seq_cst, align 8
  %9 = icmp eq i32 %8, 0
  br i1 %9, label %16, label %10

10:                                               ; preds = %1
  %11 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 20
  %12 = load i32, ptr %11, align 8, !tbaa !264
  %13 = icmp sgt i32 %12, -1
  br i1 %13, label %14, label %16

14:                                               ; preds = %10
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %2) #40
  store i64 1, ptr %2, align 8, !tbaa !8
  %15 = call i64 @write(i32 noundef %12, ptr noundef nonnull %2, i64 noundef 8) #45
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %2) #40
  br label %16

16:                                               ; preds = %14, %10, %1
  %17 = atomicrmw sub ptr %3, i64 1 seq_cst, align 8
  ret void
}

; Function Attrs: optsize
declare extern_weak i64 @write(i32 noundef, ptr noundef, i64 noundef) local_unnamed_addr #3

; Function Attrs: nounwind optsize
define void @fa_task_spawn(ptr noundef %0, i32 noundef %1) local_unnamed_addr #2 {
  %3 = tail call fastcc ptr @executor() #44
  %4 = icmp eq i32 %1, 0
  %5 = getelementptr inbounds i8, ptr %0, i64 44
  %6 = load i32, ptr %5, align 4, !tbaa !267
  br i1 %4, label %12, label %7

7:                                                ; preds = %2
  %8 = or i32 %6, 1
  %9 = getelementptr inbounds %struct.FaExecutor, ptr %3, i64 0, i32 11
  %10 = load i64, ptr %9, align 8, !tbaa !268
  %11 = add nsw i64 %10, 1
  store i64 %11, ptr %9, align 8, !tbaa !268
  br label %12

12:                                               ; preds = %2, %7
  %13 = phi i32 [ %8, %7 ], [ %6, %2 ]
  %14 = getelementptr inbounds i8, ptr %0, i64 16
  %15 = getelementptr inbounds i8, ptr %0, i64 44
  %16 = and i32 %13, -5
  store i32 %16, ptr %15, align 4, !tbaa !267
  %17 = getelementptr inbounds %struct.FaExecutor, ptr %3, i64 0, i32 1
  %18 = getelementptr inbounds i8, ptr %0, i64 24
  store ptr null, ptr %18, align 8, !tbaa !269
  %19 = load ptr, ptr %17, align 8, !tbaa !10
  %20 = getelementptr inbounds i8, ptr %0, i64 32
  store ptr %19, ptr %20, align 8, !tbaa !270
  %21 = icmp eq ptr %19, null
  %22 = getelementptr inbounds %struct.FaTask, ptr %19, i64 0, i32 1
  %23 = select i1 %21, ptr %3, ptr %22
  store ptr %14, ptr %23, align 8, !tbaa !10
  store ptr %14, ptr %17, align 8, !tbaa !10
  %24 = getelementptr inbounds %struct.FaExecutor, ptr %3, i64 0, i32 4
  %25 = load <2 x i64>, ptr %24, align 8, !tbaa !8
  %26 = add nsw <2 x i64> %25, <i64 1, i64 1>
  store <2 x i64> %26, ptr %24, align 8, !tbaa !8
  %27 = getelementptr inbounds i8, ptr %0, i64 40
  store atomic i32 1, ptr %27 release, align 8
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc ptr @executor() unnamed_addr #2 {
  %1 = alloca %struct.FaEpollEvent, align 1
  %2 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @current_executor)
  %3 = load ptr, ptr %2, align 8, !tbaa !10
  %4 = icmp eq ptr %3, null
  br i1 %4, label %5, label %25

5:                                                ; preds = %0
  %6 = tail call ptr @fa_alloc(i64 noundef 488, i64 noundef 16) #45
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(488) %6, i8 0, i64 488, i1 false)
  %7 = getelementptr inbounds %struct.FaExecutor, ptr %6, i64 0, i32 17
  %8 = getelementptr inbounds %struct.FaExecutor, ptr %6, i64 0, i32 15
  store ptr %7, ptr %8, align 8, !tbaa !271
  %9 = getelementptr inbounds %struct.FaExecutor, ptr %6, i64 0, i32 16
  store ptr %7, ptr %9, align 8, !tbaa !272
  %10 = getelementptr inbounds %struct.FaExecutor, ptr %6, i64 0, i32 19
  store i32 -1, ptr %10, align 4, !tbaa !273
  %11 = getelementptr inbounds %struct.FaExecutor, ptr %6, i64 0, i32 20
  store i32 -1, ptr %11, align 8, !tbaa !264
  %12 = or i1 icmp eq (ptr @epoll_create1, ptr null), icmp eq (ptr @eventfd, ptr null)
  %13 = or i1 %12, icmp eq (ptr @pthread_create, ptr null)
  br i1 %13, label %23, label %14

14:                                               ; preds = %5
  %15 = tail call i32 @epoll_create1(i32 noundef 524288) #45
  store i32 %15, ptr %10, align 4, !tbaa !273
  %16 = tail call i32 @eventfd(i32 noundef 0, i32 noundef 526336) #45
  store i32 %16, ptr %11, align 8, !tbaa !264
  %17 = load i32, ptr %10, align 4, !tbaa !273
  %18 = icmp sgt i32 %17, -1
  %19 = icmp sgt i32 %16, -1
  %20 = select i1 %18, i1 %19, i1 false
  br i1 %20, label %21, label %23

21:                                               ; preds = %14
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %1) #40
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(12) %1, ptr noundef nonnull align 1 dereferenceable(12) @__const.executor.event, i64 12, i1 false)
  %22 = call i32 @epoll_ctl(i32 noundef %17, i32 noundef 1, i32 noundef %16, ptr noundef nonnull %1) #45
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %1) #40
  br label %23

23:                                               ; preds = %5, %14, %21
  %24 = call align 4 ptr @llvm.threadlocal.address.p0(ptr align 4 @fa_coop_budget)
  store i32 128, ptr %24, align 4, !tbaa !25
  store ptr %6, ptr %2, align 8, !tbaa !10
  br label %25

25:                                               ; preds = %0, %23
  %26 = phi ptr [ %6, %23 ], [ %3, %0 ]
  ret ptr %26
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define void @fa_task_await(ptr nocapture noundef writeonly %0, ptr noundef %1) local_unnamed_addr #37 {
  %3 = getelementptr inbounds i8, ptr %0, i64 16
  store ptr %1, ptr %3, align 8, !tbaa !274
  %4 = getelementptr inbounds i8, ptr %1, i64 40
  store atomic i32 2, ptr %4 release, align 8
  %5 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @current_executor)
  %6 = load ptr, ptr %5, align 8, !tbaa !10
  %7 = icmp eq ptr %6, null
  br i1 %7, label %16, label %8

8:                                                ; preds = %2
  %9 = getelementptr inbounds %struct.FaExecutor, ptr %6, i64 0, i32 10
  %10 = load ptr, ptr %9, align 8, !tbaa !275
  %11 = icmp eq ptr %10, %1
  %12 = icmp eq ptr %10, null
  %13 = or i1 %11, %12
  br i1 %13, label %14, label %16

14:                                               ; preds = %8
  %15 = getelementptr inbounds %struct.FaExecutor, ptr %6, i64 0, i32 25
  store ptr null, ptr %15, align 8, !tbaa !276
  br label %16

16:                                               ; preds = %8, %14, %2
  ret void
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define void @fa_task_finish(ptr noundef %0) local_unnamed_addr #37 {
  %2 = getelementptr inbounds i8, ptr %0, i64 16
  %3 = getelementptr inbounds i8, ptr %0, i64 44
  %4 = load i32, ptr %3, align 4, !tbaa !267
  %5 = or i32 %4, 2
  store i32 %5, ptr %3, align 4, !tbaa !267
  %6 = getelementptr inbounds i8, ptr %0, i64 40
  store atomic i32 3, ptr %6 release, align 8
  %7 = and i32 %4, 1
  %8 = icmp eq i32 %7, 0
  br i1 %8, label %16, label %9

9:                                                ; preds = %1
  %10 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @current_executor)
  %11 = load ptr, ptr %10, align 8, !tbaa !10
  %12 = getelementptr inbounds %struct.FaExecutor, ptr %11, i64 0, i32 11
  %13 = load i64, ptr %12, align 8, !tbaa !268
  %14 = add nsw i64 %13, -1
  store i64 %14, ptr %12, align 8, !tbaa !268
  %15 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @frame_to_free)
  store ptr %0, ptr %15, align 8, !tbaa !10
  br label %42

16:                                               ; preds = %1
  %17 = load ptr, ptr %2, align 8, !tbaa !274
  %18 = icmp eq ptr %17, null
  br i1 %18, label %42, label %19

19:                                               ; preds = %16
  store ptr null, ptr %2, align 8, !tbaa !274
  %20 = ptrtoint ptr %17 to i64
  %21 = and i64 %20, 1
  %22 = icmp eq i64 %21, 0
  br i1 %22, label %40, label %23

23:                                               ; preds = %19
  %24 = and i64 %20, -2
  %25 = inttoptr i64 %24 to ptr
  %26 = getelementptr inbounds i8, ptr %0, i64 24
  store ptr null, ptr %26, align 8, !tbaa !269
  %27 = getelementptr inbounds %struct.FaJoin, ptr %25, i64 0, i32 3
  %28 = load ptr, ptr %27, align 8, !tbaa !277
  %29 = icmp eq ptr %28, null
  %30 = getelementptr inbounds %struct.FaJoin, ptr %25, i64 0, i32 2
  %31 = getelementptr inbounds %struct.FaTask, ptr %28, i64 0, i32 1
  %32 = select i1 %29, ptr %30, ptr %31
  store ptr %2, ptr %32, align 8, !tbaa !10
  store ptr %2, ptr %27, align 8, !tbaa !277
  %33 = getelementptr inbounds %struct.FaJoin, ptr %25, i64 0, i32 1
  %34 = load i64, ptr %33, align 8, !tbaa !279
  %35 = add nsw i64 %34, -1
  store i64 %35, ptr %33, align 8, !tbaa !279
  %36 = load ptr, ptr %25, align 8, !tbaa !280
  %37 = icmp eq ptr %36, null
  br i1 %37, label %42, label %38

38:                                               ; preds = %23
  %39 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @next_frame)
  store ptr %36, ptr %39, align 8, !tbaa !10
  store ptr null, ptr %25, align 8, !tbaa !280
  br label %42

40:                                               ; preds = %19
  %41 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @next_frame)
  store ptr %17, ptr %41, align 8, !tbaa !10
  br label %42

42:                                               ; preds = %38, %23, %40, %16, %9
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_task_drop(ptr noundef %0) local_unnamed_addr #2 {
  %2 = icmp ult ptr %0, inttoptr (i64 2 to ptr)
  br i1 %2, label %18, label %3

3:                                                ; preds = %1
  %4 = getelementptr inbounds i8, ptr %0, i64 16
  %5 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @current_executor)
  %6 = load ptr, ptr %5, align 8, !tbaa !10
  %7 = icmp eq ptr %6, null
  br i1 %7, label %17, label %8

8:                                                ; preds = %3
  %9 = getelementptr inbounds i8, ptr %0, i64 40
  %10 = load atomic i32, ptr %9 acquire, align 8
  %11 = icmp eq i32 %10, 5
  br i1 %11, label %12, label %13

12:                                               ; preds = %8
  tail call fastcc void @drain_remote(ptr noundef nonnull %6) #44
  br label %13

13:                                               ; preds = %12, %8
  %14 = load atomic i32, ptr %9 acquire, align 8
  %15 = icmp eq i32 %14, 1
  br i1 %15, label %16, label %17

16:                                               ; preds = %13
  tail call fastcc void @ready_unlink(ptr noundef nonnull %6, ptr noundef nonnull %4) #44
  br label %17

17:                                               ; preds = %13, %16, %3
  tail call void @fa_destroy(ptr noundef %0) #45
  br label %18

18:                                               ; preds = %1, %17
  ret void
}

; Function Attrs: nofree norecurse nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc void @drain_remote(ptr noundef %0) unnamed_addr #38 {
  %2 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 16
  %3 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 17, i32 1
  %4 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 15
  %5 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 17
  %6 = ptrtoint ptr %5 to i64
  %7 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 1
  %8 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 4
  br label %9

9:                                                ; preds = %38, %1
  %10 = load ptr, ptr %4, align 8, !tbaa !271
  %11 = getelementptr inbounds %struct.FaTask, ptr %10, i64 0, i32 1
  %12 = load atomic i64, ptr %11 acquire, align 8
  %13 = inttoptr i64 %12 to ptr
  %14 = icmp eq ptr %10, %5
  br i1 %14, label %15, label %21

15:                                               ; preds = %9
  %16 = icmp eq i64 %12, 0
  br i1 %16, label %52, label %17

17:                                               ; preds = %15
  store ptr %13, ptr %4, align 8, !tbaa !271
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
  br i1 %28, label %29, label %52

29:                                               ; preds = %25
  store atomic i64 0, ptr %3 monotonic, align 8
  %30 = atomicrmw xchg ptr %2, i64 %6 acq_rel, align 8
  %31 = inttoptr i64 %30 to ptr
  %32 = getelementptr inbounds %struct.FaTask, ptr %31, i64 0, i32 1
  store atomic i64 %6, ptr %32 release, align 8
  %33 = getelementptr inbounds %struct.FaTask, ptr %23, i64 0, i32 1
  %34 = load atomic i64, ptr %33 acquire, align 8
  %35 = icmp eq i64 %34, 0
  br i1 %35, label %52, label %36

36:                                               ; preds = %29
  %37 = inttoptr i64 %34 to ptr
  br label %38

38:                                               ; preds = %36, %21
  %39 = phi ptr [ %37, %36 ], [ %22, %21 ]
  store ptr %39, ptr %4, align 8, !tbaa !271
  %40 = getelementptr inbounds %struct.FaTask, ptr %23, i64 0, i32 4
  %41 = load i32, ptr %40, align 4, !tbaa !267
  %42 = and i32 %41, -5
  store i32 %42, ptr %40, align 4, !tbaa !267
  %43 = getelementptr inbounds %struct.FaTask, ptr %23, i64 0, i32 1
  store ptr null, ptr %43, align 8, !tbaa !269
  %44 = load ptr, ptr %7, align 8, !tbaa !10
  %45 = getelementptr inbounds %struct.FaTask, ptr %23, i64 0, i32 2
  store ptr %44, ptr %45, align 8, !tbaa !270
  %46 = icmp eq ptr %44, null
  %47 = getelementptr inbounds %struct.FaTask, ptr %44, i64 0, i32 1
  %48 = select i1 %46, ptr %0, ptr %47
  store ptr %23, ptr %48, align 8, !tbaa !10
  store ptr %23, ptr %7, align 8, !tbaa !10
  %49 = load <2 x i64>, ptr %8, align 8, !tbaa !8
  %50 = add nsw <2 x i64> %49, <i64 1, i64 1>
  store <2 x i64> %50, ptr %8, align 8, !tbaa !8
  %51 = getelementptr inbounds %struct.FaTask, ptr %23, i64 0, i32 3
  store atomic i32 1, ptr %51 release, align 8
  br label %9, !llvm.loop !281

52:                                               ; preds = %15, %25, %29
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, argmem: readwrite, inaccessiblemem: none)
define internal fastcc void @ready_unlink(ptr nocapture noundef %0, ptr nocapture noundef %1) unnamed_addr #34 {
  %3 = getelementptr inbounds %struct.FaTask, ptr %1, i64 0, i32 4
  %4 = load i32, ptr %3, align 4, !tbaa !267
  %5 = and i32 %4, 4
  %6 = icmp eq i32 %5, 0
  br i1 %6, label %22, label %7

7:                                                ; preds = %2
  %8 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 2
  %9 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 3
  %10 = getelementptr inbounds %struct.FaTask, ptr %1, i64 0, i32 2
  %11 = load ptr, ptr %10, align 8, !tbaa !270
  %12 = icmp eq ptr %11, null
  %13 = getelementptr inbounds %struct.FaTask, ptr %1, i64 0, i32 1
  %14 = load ptr, ptr %13, align 8, !tbaa !269
  %15 = getelementptr inbounds %struct.FaTask, ptr %11, i64 0, i32 1
  %16 = select i1 %12, ptr %8, ptr %15
  store ptr %14, ptr %16, align 8, !tbaa !10
  %17 = icmp eq ptr %14, null
  %18 = load ptr, ptr %10, align 8, !tbaa !270
  %19 = getelementptr inbounds %struct.FaTask, ptr %14, i64 0, i32 2
  %20 = select i1 %17, ptr %9, ptr %19
  store ptr %18, ptr %20, align 8, !tbaa !10
  store ptr null, ptr %13, align 8, !tbaa !269
  store ptr null, ptr %10, align 8, !tbaa !270
  %21 = and i32 %4, -5
  store i32 %21, ptr %3, align 4, !tbaa !267
  br label %38

22:                                               ; preds = %2
  %23 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 1
  %24 = getelementptr inbounds %struct.FaTask, ptr %1, i64 0, i32 2
  %25 = load ptr, ptr %24, align 8, !tbaa !270
  %26 = icmp eq ptr %25, null
  %27 = getelementptr inbounds %struct.FaTask, ptr %1, i64 0, i32 1
  %28 = load ptr, ptr %27, align 8, !tbaa !269
  %29 = getelementptr inbounds %struct.FaTask, ptr %25, i64 0, i32 1
  %30 = select i1 %26, ptr %0, ptr %29
  store ptr %28, ptr %30, align 8, !tbaa !10
  %31 = icmp eq ptr %28, null
  %32 = load ptr, ptr %24, align 8, !tbaa !270
  %33 = getelementptr inbounds %struct.FaTask, ptr %28, i64 0, i32 2
  %34 = select i1 %31, ptr %23, ptr %33
  store ptr %32, ptr %34, align 8, !tbaa !10
  store ptr null, ptr %27, align 8, !tbaa !269
  store ptr null, ptr %24, align 8, !tbaa !270
  %35 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 5
  %36 = load i64, ptr %35, align 8, !tbaa !282
  %37 = add nsw i64 %36, -1
  store i64 %37, ptr %35, align 8, !tbaa !282
  br label %38

38:                                               ; preds = %22, %7
  %39 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 4
  %40 = load i64, ptr %39, align 8, !tbaa !283
  %41 = add nsw i64 %40, -1
  store i64 %41, ptr %39, align 8, !tbaa !283
  ret void
}

; Function Attrs: optsize
declare void @fa_destroy(ptr noundef) local_unnamed_addr #3

; Function Attrs: nounwind optsize
define void @fa_task_yield(ptr noundef %0) local_unnamed_addr #2 {
  %2 = tail call fastcc ptr @executor() #44
  %3 = getelementptr inbounds %struct.FaExecutor, ptr %2, i64 0, i32 23
  store i32 1, ptr %3, align 8, !tbaa !284
  %4 = getelementptr inbounds i8, ptr %0, i64 16
  %5 = getelementptr inbounds i8, ptr %0, i64 44
  %6 = load i32, ptr %5, align 4, !tbaa !267
  %7 = or i32 %6, 4
  store i32 %7, ptr %5, align 4, !tbaa !267
  %8 = getelementptr inbounds %struct.FaExecutor, ptr %2, i64 0, i32 2
  %9 = getelementptr inbounds %struct.FaExecutor, ptr %2, i64 0, i32 3
  %10 = getelementptr inbounds i8, ptr %0, i64 24
  store ptr null, ptr %10, align 8, !tbaa !269
  %11 = load ptr, ptr %9, align 8, !tbaa !10
  %12 = getelementptr inbounds i8, ptr %0, i64 32
  store ptr %11, ptr %12, align 8, !tbaa !270
  %13 = icmp eq ptr %11, null
  %14 = getelementptr inbounds %struct.FaTask, ptr %11, i64 0, i32 1
  %15 = select i1 %13, ptr %8, ptr %14
  store ptr %4, ptr %15, align 8, !tbaa !10
  store ptr %4, ptr %9, align 8, !tbaa !10
  %16 = getelementptr inbounds %struct.FaExecutor, ptr %2, i64 0, i32 4
  %17 = load i64, ptr %16, align 8, !tbaa !283
  %18 = add nsw i64 %17, 1
  store i64 %18, ptr %16, align 8, !tbaa !283
  %19 = getelementptr inbounds i8, ptr %0, i64 40
  store atomic i32 1, ptr %19 release, align 8
  ret void
}

; Function Attrs: nounwind optsize
define i32 @fa_coop_back_edge() local_unnamed_addr #2 {
  %1 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @current_executor)
  %2 = load ptr, ptr %1, align 8, !tbaa !10
  %3 = tail call align 4 ptr @llvm.threadlocal.address.p0(ptr align 4 @fa_coop_budget)
  store i32 128, ptr %3, align 4, !tbaa !25
  %4 = icmp eq ptr %2, null
  br i1 %4, label %15, label %5

5:                                                ; preds = %0
  %6 = tail call i64 @fa_clock() #45
  %7 = getelementptr inbounds %struct.FaExecutor, ptr %2, i64 0, i32 24
  %8 = load i64, ptr %7, align 8, !tbaa !285
  %9 = icmp eq i64 %8, 0
  br i1 %9, label %10, label %11

10:                                               ; preds = %5
  store i64 %6, ptr %7, align 8, !tbaa !285
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
declare i64 @fa_clock() local_unnamed_addr #3

; Function Attrs: nounwind optsize
define void @fa_sleep_async(ptr noundef %0, ptr noundef %1, i64 noundef %2) local_unnamed_addr #2 {
  %4 = tail call fastcc ptr @executor() #44
  store ptr %1, ptr %0, align 8, !tbaa !286
  %5 = getelementptr inbounds %struct.FaWait, ptr %0, i64 0, i32 1
  store ptr %4, ptr %5, align 8, !tbaa !288
  %6 = getelementptr inbounds %struct.FaWait, ptr %0, i64 0, i32 8
  store i64 -1, ptr %6, align 8, !tbaa !289
  %7 = getelementptr inbounds %struct.FaWait, ptr %0, i64 0, i32 11
  store ptr null, ptr %7, align 8, !tbaa !290
  %8 = icmp slt i64 %2, 1
  br i1 %8, label %9, label %26

9:                                                ; preds = %3
  %10 = getelementptr inbounds i8, ptr %1, i64 16
  %11 = getelementptr inbounds i8, ptr %1, i64 44
  %12 = load i32, ptr %11, align 4, !tbaa !267
  %13 = or i32 %12, 4
  store i32 %13, ptr %11, align 4, !tbaa !267
  %14 = getelementptr inbounds %struct.FaExecutor, ptr %4, i64 0, i32 2
  %15 = getelementptr inbounds %struct.FaExecutor, ptr %4, i64 0, i32 3
  %16 = getelementptr inbounds i8, ptr %1, i64 24
  store ptr null, ptr %16, align 8, !tbaa !269
  %17 = load ptr, ptr %15, align 8, !tbaa !10
  %18 = getelementptr inbounds i8, ptr %1, i64 32
  store ptr %17, ptr %18, align 8, !tbaa !270
  %19 = icmp eq ptr %17, null
  %20 = getelementptr inbounds %struct.FaTask, ptr %17, i64 0, i32 1
  %21 = select i1 %19, ptr %14, ptr %20
  store ptr %10, ptr %21, align 8, !tbaa !10
  store ptr %10, ptr %15, align 8, !tbaa !10
  %22 = getelementptr inbounds %struct.FaExecutor, ptr %4, i64 0, i32 4
  %23 = load i64, ptr %22, align 8, !tbaa !283
  %24 = add nsw i64 %23, 1
  store i64 %24, ptr %22, align 8, !tbaa !283
  %25 = getelementptr inbounds i8, ptr %1, i64 40
  store atomic i32 1, ptr %25 release, align 8
  br label %38

26:                                               ; preds = %3
  %27 = tail call i64 @fa_clock() #45
  %28 = mul nsw i64 %2, 1000000
  %29 = add nsw i64 %27, %28
  tail call fastcc void @timer_insert(ptr noundef %4, ptr noundef nonnull %0, i64 noundef %29) #44
  %30 = getelementptr inbounds i8, ptr %1, i64 40
  store atomic i32 2, ptr %30 release, align 8
  %31 = getelementptr inbounds %struct.FaExecutor, ptr %4, i64 0, i32 10
  %32 = load ptr, ptr %31, align 8, !tbaa !275
  %33 = icmp eq ptr %32, %1
  %34 = icmp eq ptr %32, null
  %35 = or i1 %33, %34
  br i1 %35, label %36, label %38

36:                                               ; preds = %26
  %37 = getelementptr inbounds %struct.FaExecutor, ptr %4, i64 0, i32 25
  store ptr null, ptr %37, align 8, !tbaa !276
  br label %38

38:                                               ; preds = %36, %26, %9
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @timer_insert(ptr noundef %0, ptr noundef %1, i64 noundef %2) unnamed_addr #2 {
  %4 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 1
  store ptr %0, ptr %4, align 8, !tbaa !288
  %5 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 7
  %6 = load i64, ptr %5, align 8, !tbaa !291
  %7 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 8
  %8 = load i64, ptr %7, align 8, !tbaa !292
  %9 = icmp eq i64 %6, %8
  br i1 %9, label %13, label %10

10:                                               ; preds = %3
  %11 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 6
  %12 = load ptr, ptr %11, align 8, !tbaa !293
  br label %43

13:                                               ; preds = %3
  %14 = icmp slt i64 %6, 16
  %15 = shl nuw nsw i64 %6, 1
  %16 = select i1 %14, i64 16, i64 %15
  %17 = shl i64 %16, 4
  %18 = tail call ptr @fa_alloc(i64 noundef %17, i64 noundef 64) #45
  %19 = load i64, ptr %5, align 8, !tbaa !291
  %20 = icmp sgt i64 %19, 0
  br i1 %20, label %21, label %23

21:                                               ; preds = %13
  %22 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 6
  br label %27

23:                                               ; preds = %27, %13
  %24 = phi i64 [ %19, %13 ], [ %33, %27 ]
  %25 = load i64, ptr %7, align 8, !tbaa !292
  %26 = icmp sgt i64 %25, 0
  br i1 %26, label %35, label %40

27:                                               ; preds = %21, %27
  %28 = phi i64 [ 0, %21 ], [ %32, %27 ]
  %29 = getelementptr inbounds %struct.FaTimer, ptr %18, i64 %28
  %30 = load ptr, ptr %22, align 8, !tbaa !293
  %31 = getelementptr inbounds %struct.FaTimer, ptr %30, i64 %28
  tail call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %29, ptr noundef nonnull align 8 dereferenceable(16) %31, i64 16, i1 false), !tbaa.struct !189
  %32 = add nuw nsw i64 %28, 1
  %33 = load i64, ptr %5, align 8, !tbaa !291
  %34 = icmp slt i64 %32, %33
  br i1 %34, label %27, label %23, !llvm.loop !294

35:                                               ; preds = %23
  %36 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 6
  %37 = load ptr, ptr %36, align 8, !tbaa !293
  %38 = shl i64 %25, 4
  tail call void @fa_free(ptr noundef %37, i64 noundef %38, i64 noundef 64) #45
  %39 = load i64, ptr %5, align 8, !tbaa !291
  br label %40

40:                                               ; preds = %35, %23
  %41 = phi i64 [ %39, %35 ], [ %24, %23 ]
  %42 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 6
  store ptr %18, ptr %42, align 8, !tbaa !293
  store i64 %16, ptr %7, align 8, !tbaa !292
  br label %43

43:                                               ; preds = %10, %40
  %44 = phi ptr [ %18, %40 ], [ %12, %10 ]
  %45 = phi i64 [ %41, %40 ], [ %6, %10 ]
  %46 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 6
  store i64 %2, ptr %46, align 8, !tbaa !295
  %47 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 9
  %48 = load i64, ptr %47, align 8, !tbaa !296
  %49 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 7
  store i64 %48, ptr %49, align 8, !tbaa !297
  %50 = add nsw i64 %48, 1
  store i64 %50, ptr %47, align 8, !tbaa !296
  %51 = add nsw i64 %45, 1
  store i64 %51, ptr %5, align 8, !tbaa !291
  %52 = getelementptr inbounds %struct.FaTimer, ptr %44, i64 %45
  store i64 %2, ptr %52, align 8, !tbaa.struct !189
  %53 = getelementptr inbounds i8, ptr %52, i64 8
  store ptr %1, ptr %53, align 8, !tbaa.struct !190
  %54 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 8
  store i64 %45, ptr %54, align 8, !tbaa !289
  %55 = load i64, ptr %5, align 8, !tbaa !291
  %56 = add nsw i64 %55, -1
  tail call fastcc void @timer_up(ptr noundef nonnull %0, i64 noundef %56) #44
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define void @fa_timer_cancel(ptr nocapture noundef readonly %0) local_unnamed_addr #17 {
  %2 = getelementptr inbounds %struct.FaWait, ptr %0, i64 0, i32 8
  %3 = load i64, ptr %2, align 8, !tbaa !289
  %4 = icmp sgt i64 %3, -1
  br i1 %4, label %5, label %8

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaWait, ptr %0, i64 0, i32 1
  %7 = load ptr, ptr %6, align 8, !tbaa !288
  tail call fastcc void @timer_remove(ptr noundef %7, i64 noundef %3) #44
  br label %8

8:                                                ; preds = %5, %1
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc void @timer_remove(ptr nocapture noundef %0, i64 noundef %1) unnamed_addr #17 {
  %3 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 6
  %4 = load ptr, ptr %3, align 8, !tbaa !293
  %5 = getelementptr inbounds %struct.FaTimer, ptr %4, i64 %1, i32 1
  %6 = load ptr, ptr %5, align 8, !tbaa !298
  %7 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 7
  %8 = load i64, ptr %7, align 8, !tbaa !291
  %9 = add nsw i64 %8, -1
  store i64 %9, ptr %7, align 8, !tbaa !291
  %10 = icmp sgt i64 %9, %1
  br i1 %10, label %11, label %115

11:                                               ; preds = %2
  %12 = getelementptr inbounds %struct.FaTimer, ptr %4, i64 %9
  %13 = load i64, ptr %12, align 8, !tbaa.struct !189
  %14 = getelementptr inbounds i8, ptr %12, i64 8
  %15 = load ptr, ptr %14, align 8, !tbaa.struct !190
  %16 = getelementptr inbounds %struct.FaTimer, ptr %4, i64 %1
  store i64 %13, ptr %16, align 8, !tbaa.struct !189
  %17 = getelementptr inbounds i8, ptr %16, i64 8
  store ptr %15, ptr %17, align 8, !tbaa.struct !190
  %18 = getelementptr inbounds %struct.FaWait, ptr %15, i64 0, i32 8
  store i64 %1, ptr %18, align 8, !tbaa !289
  %19 = icmp sgt i64 %1, 0
  %20 = load ptr, ptr %3, align 8, !tbaa !293
  br i1 %19, label %21, label %38

21:                                               ; preds = %11
  %22 = add nsw i64 %1, -1
  %23 = lshr i64 %22, 2
  %24 = getelementptr inbounds %struct.FaTimer, ptr %20, i64 %23
  %25 = load i64, ptr %24, align 8
  %26 = getelementptr inbounds { i64, ptr }, ptr %24, i64 0, i32 1
  %27 = load ptr, ptr %26, align 8
  %28 = icmp slt i64 %13, %25
  br i1 %28, label %37, label %29

29:                                               ; preds = %21
  %30 = icmp eq i64 %13, %25
  br i1 %30, label %31, label %38

31:                                               ; preds = %29
  %32 = getelementptr inbounds %struct.FaWait, ptr %15, i64 0, i32 7
  %33 = load i64, ptr %32, align 8, !tbaa !297
  %34 = getelementptr inbounds %struct.FaWait, ptr %27, i64 0, i32 7
  %35 = load i64, ptr %34, align 8, !tbaa !297
  %36 = icmp slt i64 %33, %35
  br i1 %36, label %37, label %38

37:                                               ; preds = %21, %31
  tail call fastcc void @timer_up(ptr noundef nonnull %0, i64 noundef %1) #44
  br label %115

38:                                               ; preds = %29, %31, %11
  %39 = getelementptr inbounds %struct.FaTimer, ptr %20, i64 %1
  %40 = load i64, ptr %39, align 8, !tbaa.struct !189
  %41 = getelementptr inbounds i8, ptr %39, i64 8
  %42 = load ptr, ptr %41, align 8, !tbaa.struct !190
  %43 = shl nsw i64 %1, 2
  %44 = or disjoint i64 %43, 1
  %45 = load i64, ptr %7, align 8, !tbaa !291
  %46 = icmp slt i64 %44, %45
  br i1 %46, label %47, label %109

47:                                               ; preds = %38
  %48 = getelementptr inbounds %struct.FaWait, ptr %42, i64 0, i32 7
  br label %49

49:                                               ; preds = %99, %47
  %50 = phi i64 [ %45, %47 ], [ %105, %99 ]
  %51 = phi i64 [ %44, %47 ], [ %104, %99 ]
  %52 = phi i64 [ %43, %47 ], [ %103, %99 ]
  %53 = phi i64 [ %1, %47 ], [ %60, %99 ]
  %54 = add i64 %52, 5
  %55 = tail call i64 @llvm.smin.i64(i64 %54, i64 %50)
  %56 = or disjoint i64 %52, 2
  %57 = icmp slt i64 %56, %55
  %58 = load ptr, ptr %3, align 8, !tbaa !293
  br i1 %57, label %73, label %59

59:                                               ; preds = %95, %49
  %60 = phi i64 [ %51, %49 ], [ %96, %95 ]
  %61 = getelementptr inbounds %struct.FaTimer, ptr %58, i64 %60
  %62 = load i64, ptr %61, align 8
  %63 = getelementptr inbounds { i64, ptr }, ptr %61, i64 0, i32 1
  %64 = load ptr, ptr %63, align 8
  %65 = icmp slt i64 %62, %40
  br i1 %65, label %99, label %66

66:                                               ; preds = %59
  %67 = icmp eq i64 %62, %40
  br i1 %67, label %68, label %109

68:                                               ; preds = %66
  %69 = getelementptr inbounds %struct.FaWait, ptr %64, i64 0, i32 7
  %70 = load i64, ptr %69, align 8, !tbaa !297
  %71 = load i64, ptr %48, align 8, !tbaa !297
  %72 = icmp slt i64 %70, %71
  br i1 %72, label %99, label %109

73:                                               ; preds = %49, %95
  %74 = phi i64 [ %97, %95 ], [ %56, %49 ]
  %75 = phi i64 [ %96, %95 ], [ %51, %49 ]
  %76 = getelementptr inbounds %struct.FaTimer, ptr %58, i64 %74
  %77 = getelementptr inbounds %struct.FaTimer, ptr %58, i64 %75
  %78 = load i64, ptr %76, align 8
  %79 = getelementptr inbounds { i64, ptr }, ptr %76, i64 0, i32 1
  %80 = load ptr, ptr %79, align 8
  %81 = load i64, ptr %77, align 8
  %82 = getelementptr inbounds { i64, ptr }, ptr %77, i64 0, i32 1
  %83 = load ptr, ptr %82, align 8
  %84 = icmp slt i64 %78, %81
  br i1 %84, label %95, label %85

85:                                               ; preds = %73
  %86 = icmp eq i64 %78, %81
  br i1 %86, label %87, label %94

87:                                               ; preds = %85
  %88 = getelementptr inbounds %struct.FaWait, ptr %80, i64 0, i32 7
  %89 = load i64, ptr %88, align 8, !tbaa !297
  %90 = getelementptr inbounds %struct.FaWait, ptr %83, i64 0, i32 7
  %91 = load i64, ptr %90, align 8, !tbaa !297
  %92 = icmp sge i64 %89, %91
  %93 = freeze i1 %92
  br i1 %93, label %94, label %95

94:                                               ; preds = %87, %85
  br label %95

95:                                               ; preds = %94, %87, %73
  %96 = phi i64 [ %75, %94 ], [ %74, %87 ], [ %74, %73 ]
  %97 = add nsw i64 %74, 1
  %98 = icmp eq i64 %97, %55
  br i1 %98, label %59, label %73, !llvm.loop !300

99:                                               ; preds = %68, %59
  %100 = getelementptr inbounds %struct.FaTimer, ptr %58, i64 %53
  store i64 %62, ptr %100, align 8, !tbaa.struct !189
  %101 = getelementptr inbounds i8, ptr %100, i64 8
  store ptr %64, ptr %101, align 8, !tbaa.struct !190
  %102 = getelementptr inbounds %struct.FaWait, ptr %64, i64 0, i32 8
  store i64 %53, ptr %102, align 8, !tbaa !289
  %103 = shl nsw i64 %60, 2
  %104 = or disjoint i64 %103, 1
  %105 = load i64, ptr %7, align 8, !tbaa !291
  %106 = icmp slt i64 %104, %105
  br i1 %106, label %49, label %107

107:                                              ; preds = %99
  %108 = load ptr, ptr %3, align 8, !tbaa !293
  br label %109

109:                                              ; preds = %66, %68, %38, %107
  %110 = phi ptr [ %20, %38 ], [ %108, %107 ], [ %58, %68 ], [ %58, %66 ]
  %111 = phi i64 [ %1, %38 ], [ %60, %107 ], [ %53, %68 ], [ %53, %66 ]
  %112 = getelementptr inbounds %struct.FaTimer, ptr %110, i64 %111
  store i64 %40, ptr %112, align 8, !tbaa.struct !189
  %113 = getelementptr inbounds i8, ptr %112, i64 8
  store ptr %42, ptr %113, align 8, !tbaa.struct !190
  %114 = getelementptr inbounds %struct.FaWait, ptr %42, i64 0, i32 8
  store i64 %111, ptr %114, align 8, !tbaa !289
  br label %115

115:                                              ; preds = %37, %109, %2
  %116 = getelementptr inbounds %struct.FaWait, ptr %6, i64 0, i32 8
  store i64 -1, ptr %116, align 8, !tbaa !289
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_sleep(i64 noundef %0) local_unnamed_addr #2 {
  %2 = icmp sgt i64 %0, 0
  br i1 %2, label %3, label %5

3:                                                ; preds = %1
  %4 = mul nsw i64 %0, 1000000
  tail call void @fa_nanosleep(i64 noundef %4) #45
  br label %5

5:                                                ; preds = %3, %1
  ret void
}

; Function Attrs: optsize
declare void @fa_nanosleep(i64 noundef) local_unnamed_addr #3

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: write)
define void @fa_join_init(ptr nocapture noundef writeonly %0) local_unnamed_addr #30 {
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %0, i8 0, i64 32, i1 false)
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, argmem: readwrite, inaccessiblemem: none)
define void @fa_join_add(ptr noundef %0, ptr noundef %1) local_unnamed_addr #34 {
  %3 = icmp ult ptr %1, inttoptr (i64 2 to ptr)
  br i1 %3, label %23, label %4

4:                                                ; preds = %2
  %5 = getelementptr inbounds i8, ptr %1, i64 16
  %6 = load ptr, ptr %1, align 8, !tbaa !10
  %7 = icmp eq ptr %6, null
  br i1 %7, label %8, label %16

8:                                                ; preds = %4
  %9 = getelementptr inbounds i8, ptr %1, i64 24
  store ptr null, ptr %9, align 8, !tbaa !269
  %10 = getelementptr inbounds %struct.FaJoin, ptr %0, i64 0, i32 3
  %11 = load ptr, ptr %10, align 8, !tbaa !277
  %12 = icmp eq ptr %11, null
  %13 = getelementptr inbounds %struct.FaJoin, ptr %0, i64 0, i32 2
  %14 = getelementptr inbounds %struct.FaTask, ptr %11, i64 0, i32 1
  %15 = select i1 %12, ptr %13, ptr %14
  store ptr %5, ptr %15, align 8, !tbaa !10
  store ptr %5, ptr %10, align 8, !tbaa !277
  br label %23

16:                                               ; preds = %4
  %17 = ptrtoint ptr %0 to i64
  %18 = or i64 %17, 1
  %19 = inttoptr i64 %18 to ptr
  store ptr %19, ptr %5, align 8, !tbaa !274
  %20 = getelementptr inbounds %struct.FaJoin, ptr %0, i64 0, i32 1
  %21 = load i64, ptr %20, align 8, !tbaa !279
  %22 = add nsw i64 %21, 1
  store i64 %22, ptr %20, align 8, !tbaa !279
  br label %23

23:                                               ; preds = %8, %16, %2
  ret void
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define ptr @fa_join_next(ptr nocapture noundef %0, ptr noundef %1) local_unnamed_addr #37 {
  %3 = getelementptr inbounds %struct.FaJoin, ptr %0, i64 0, i32 2
  %4 = load ptr, ptr %3, align 8, !tbaa !301
  %5 = icmp eq ptr %4, null
  br i1 %5, label %14, label %6

6:                                                ; preds = %2
  %7 = getelementptr inbounds %struct.FaTask, ptr %4, i64 0, i32 1
  %8 = load ptr, ptr %7, align 8, !tbaa !269
  store ptr %8, ptr %3, align 8, !tbaa !301
  %9 = icmp eq ptr %8, null
  br i1 %9, label %10, label %12

10:                                               ; preds = %6
  %11 = getelementptr inbounds %struct.FaJoin, ptr %0, i64 0, i32 3
  store ptr null, ptr %11, align 8, !tbaa !277
  br label %12

12:                                               ; preds = %10, %6
  store ptr null, ptr %7, align 8, !tbaa !269
  %13 = getelementptr inbounds i8, ptr %4, i64 -16
  br label %22

14:                                               ; preds = %2
  %15 = getelementptr inbounds %struct.FaJoin, ptr %0, i64 0, i32 1
  %16 = load i64, ptr %15, align 8, !tbaa !279
  %17 = icmp eq i64 %16, 0
  br i1 %17, label %22, label %18

18:                                               ; preds = %14
  %19 = icmp eq ptr %1, null
  br i1 %19, label %22, label %20

20:                                               ; preds = %18
  store ptr %1, ptr %0, align 8, !tbaa !280
  %21 = getelementptr inbounds i8, ptr %1, i64 40
  store atomic i32 2, ptr %21 release, align 8
  br label %22

22:                                               ; preds = %18, %14, %20, %12
  %23 = phi ptr [ %13, %12 ], [ null, %20 ], [ inttoptr (i64 1 to ptr), %14 ], [ null, %18 ]
  ret ptr %23
}

; Function Attrs: cold noinline noreturn nounwind optsize
define void @fa_task_stuck() local_unnamed_addr #31 {
  tail call fastcc void @deadlock(ptr noundef nonnull @.str.30) #49
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define internal fastcc void @deadlock(ptr noundef %0) unnamed_addr #31 {
  tail call void @fa_flush() #44
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.45, i64 noundef 16) #45
  br label %2

2:                                                ; preds = %2, %1
  %3 = phi i64 [ 0, %1 ], [ %7, %2 ]
  %4 = getelementptr inbounds i8, ptr %0, i64 %3
  %5 = load i8, ptr %4, align 1, !tbaa !2
  %6 = icmp eq i8 %5, 0
  %7 = add i64 %3, 1
  br i1 %6, label %8, label %2, !llvm.loop !29

8:                                                ; preds = %2
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull %0, i64 noundef %3) #45
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.22, i64 noundef 1) #45
  tail call void @fa_exit(i32 noundef 101) #46
  unreachable
}

; Function Attrs: nounwind optsize
define noundef ptr @fa_io_register(i32 noundef %0) local_unnamed_addr #2 {
  %2 = alloca %struct.FaEpollEvent, align 4
  %3 = tail call fastcc ptr @executor() #44
  %4 = getelementptr inbounds %struct.FaExecutor, ptr %3, i64 0, i32 19
  %5 = load i32, ptr %4, align 4, !tbaa !273
  %6 = icmp slt i32 %5, 0
  br i1 %6, label %20, label %7

7:                                                ; preds = %1
  %8 = tail call ptr @fa_alloc(i64 noundef 32, i64 noundef 16) #45
  %9 = getelementptr inbounds i8, ptr %8, i64 8
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %9, i8 0, i64 16, i1 false)
  store i32 %0, ptr %8, align 8, !tbaa !302
  %10 = getelementptr inbounds %struct.FaIo, ptr %8, i64 0, i32 1
  store i32 5, ptr %10, align 4, !tbaa !304
  %11 = getelementptr inbounds %struct.FaIo, ptr %8, i64 0, i32 4
  store ptr %3, ptr %11, align 8, !tbaa !305
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %2) #40
  store i32 -2147475451, ptr %2, align 4, !tbaa !306
  %12 = getelementptr inbounds %struct.FaEpollEvent, ptr %2, i64 0, i32 1
  %13 = ptrtoint ptr %8 to i64
  store i64 %13, ptr %12, align 4, !tbaa !308
  %14 = load i32, ptr %4, align 4, !tbaa !273
  %15 = call i32 @epoll_ctl(i32 noundef %14, i32 noundef 1, i32 noundef %0, ptr noundef nonnull %2) #45
  %16 = icmp eq i32 %15, 0
  br i1 %16, label %18, label %17

17:                                               ; preds = %7
  call void @fa_free(ptr noundef nonnull %8, i64 noundef 32, i64 noundef 16) #45
  br label %18

18:                                               ; preds = %7, %17
  %19 = phi ptr [ null, %17 ], [ %8, %7 ]
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %2) #40
  br label %20

20:                                               ; preds = %1, %18
  %21 = phi ptr [ %19, %18 ], [ null, %1 ]
  ret ptr %21
}

; Function Attrs: optsize
declare extern_weak i32 @epoll_ctl(i32 noundef, i32 noundef, i32 noundef, ptr noundef) local_unnamed_addr #3

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define noundef i32 @fa_io_wait(ptr nocapture noundef %0, i32 noundef %1, ptr noundef %2, ptr noundef %3) local_unnamed_addr #37 {
  %5 = icmp eq i32 %1, 0
  %6 = select i1 %5, i32 -2, i32 -5
  %7 = getelementptr inbounds %struct.FaIo, ptr %0, i64 0, i32 1
  %8 = load i32, ptr %7, align 4, !tbaa !304
  %9 = and i32 %8, %6
  store i32 %9, ptr %7, align 4, !tbaa !304
  store ptr %3, ptr %2, align 8, !tbaa !286
  %10 = getelementptr inbounds %struct.FaIo, ptr %0, i64 0, i32 4
  %11 = load ptr, ptr %10, align 8, !tbaa !305
  %12 = getelementptr inbounds %struct.FaWait, ptr %2, i64 0, i32 1
  store ptr %11, ptr %12, align 8, !tbaa !288
  %13 = getelementptr inbounds %struct.FaWait, ptr %2, i64 0, i32 9
  store i32 0, ptr %13, align 8, !tbaa !309
  %14 = getelementptr inbounds %struct.FaWait, ptr %2, i64 0, i32 10
  store i32 0, ptr %14, align 4, !tbaa !310
  %15 = getelementptr inbounds %struct.FaWait, ptr %2, i64 0, i32 8
  store i64 -1, ptr %15, align 8, !tbaa !289
  %16 = getelementptr inbounds %struct.FaIo, ptr %0, i64 0, i32 2
  %17 = getelementptr inbounds %struct.FaIo, ptr %0, i64 0, i32 3
  %18 = select i1 %5, ptr %16, ptr %17
  store ptr %2, ptr %18, align 8, !tbaa !10
  %19 = getelementptr inbounds %struct.FaExecutor, ptr %11, i64 0, i32 13
  %20 = load i64, ptr %19, align 8, !tbaa !311
  %21 = add nsw i64 %20, 1
  store i64 %21, ptr %19, align 8, !tbaa !311
  %22 = getelementptr inbounds i8, ptr %3, i64 40
  store atomic i32 2, ptr %22 release, align 8
  ret i32 1
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define void @fa_io_cancel(ptr nocapture noundef %0, ptr noundef readnone %1) local_unnamed_addr #24 {
  %3 = getelementptr inbounds %struct.FaIo, ptr %0, i64 0, i32 2
  %4 = load ptr, ptr %3, align 8, !tbaa !312
  %5 = icmp eq ptr %4, %1
  br i1 %5, label %6, label %12

6:                                                ; preds = %2
  store ptr null, ptr %3, align 8, !tbaa !312
  %7 = getelementptr inbounds %struct.FaIo, ptr %0, i64 0, i32 4
  %8 = load ptr, ptr %7, align 8, !tbaa !305
  %9 = getelementptr inbounds %struct.FaExecutor, ptr %8, i64 0, i32 13
  %10 = load i64, ptr %9, align 8, !tbaa !311
  %11 = add nsw i64 %10, -1
  store i64 %11, ptr %9, align 8, !tbaa !311
  br label %12

12:                                               ; preds = %6, %2
  %13 = getelementptr inbounds %struct.FaIo, ptr %0, i64 0, i32 3
  %14 = load ptr, ptr %13, align 8, !tbaa !313
  %15 = icmp eq ptr %14, %1
  br i1 %15, label %16, label %22

16:                                               ; preds = %12
  store ptr null, ptr %13, align 8, !tbaa !313
  %17 = getelementptr inbounds %struct.FaIo, ptr %0, i64 0, i32 4
  %18 = load ptr, ptr %17, align 8, !tbaa !305
  %19 = getelementptr inbounds %struct.FaExecutor, ptr %18, i64 0, i32 13
  %20 = load i64, ptr %19, align 8, !tbaa !311
  %21 = add nsw i64 %20, -1
  store i64 %21, ptr %19, align 8, !tbaa !311
  br label %22

22:                                               ; preds = %16, %12
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_io_close(ptr noundef %0) local_unnamed_addr #2 {
  %2 = alloca %struct.FaEpollEvent, align 1
  %3 = icmp eq ptr %0, null
  br i1 %3, label %11, label %4

4:                                                ; preds = %1
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %2) #40
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 1 dereferenceable(12) %2, i8 0, i64 12, i1 false)
  %5 = getelementptr inbounds %struct.FaIo, ptr %0, i64 0, i32 4
  %6 = load ptr, ptr %5, align 8, !tbaa !305
  %7 = getelementptr inbounds %struct.FaExecutor, ptr %6, i64 0, i32 19
  %8 = load i32, ptr %7, align 4, !tbaa !273
  %9 = load i32, ptr %0, align 8, !tbaa !302
  %10 = call i32 @epoll_ctl(i32 noundef %8, i32 noundef 2, i32 noundef %9, ptr noundef nonnull %2) #45
  call void @fa_free(ptr noundef nonnull %0, i64 noundef 32, i64 noundef 16) #45
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %2) #40
  br label %11

11:                                               ; preds = %1, %4
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_run_main(ptr noundef %0, i32 noundef %1) local_unnamed_addr #2 {
  %3 = alloca [256 x %struct.FaEpollEvent], align 16
  %4 = alloca i64, align 8
  %5 = tail call fastcc ptr @executor() #44
  %6 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 10
  store ptr %0, ptr %6, align 8, !tbaa !275
  %7 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 21
  store i32 %1, ptr %7, align 4, !tbaa !314
  %8 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 11
  %9 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 12
  %10 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 4
  %11 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 5
  %12 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 2
  %13 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 7
  %14 = tail call align 4 ptr @llvm.threadlocal.address.p0(ptr align 4 @fa_coop_budget)
  %15 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 24
  %16 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @frame_to_free)
  %17 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @next_frame)
  %18 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @fa_turn_site)
  %19 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 28
  %20 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  %21 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 22
  %22 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 13
  %23 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 6
  %24 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 19
  %25 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 18
  %26 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 15
  %27 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 16
  %28 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 23
  %29 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 20
  br label %30

30:                                               ; preds = %301, %2
  %31 = load ptr, ptr %0, align 8, !tbaa !10
  %32 = icmp eq ptr %31, null
  br i1 %32, label %33, label %39

33:                                               ; preds = %30
  %34 = load i64, ptr %8, align 8, !tbaa !268
  %35 = icmp sgt i64 %34, 0
  br i1 %35, label %39, label %36

36:                                               ; preds = %33
  %37 = load atomic i64, ptr %9 acquire, align 8
  %38 = icmp sgt i64 %37, 0
  br i1 %38, label %39, label %302

39:                                               ; preds = %33, %30, %36
  %40 = load i64, ptr %10, align 8, !tbaa !283
  %41 = call i64 @llvm.smin.i64(i64 %40, i64 61)
  %42 = load i64, ptr %11, align 8, !tbaa !282
  %43 = icmp sgt i64 %42, 0
  br i1 %43, label %44, label %47

44:                                               ; preds = %39
  %45 = load ptr, ptr %12, align 8, !tbaa !315
  %46 = icmp eq ptr %45, null
  br i1 %46, label %47, label %48

47:                                               ; preds = %44, %39
  br label %48

48:                                               ; preds = %47, %44
  %49 = phi i64 [ %41, %47 ], [ 30, %44 ]
  %50 = icmp sgt i64 %40, 0
  br i1 %50, label %51, label %131

51:                                               ; preds = %48, %125
  %52 = phi i64 [ %128, %125 ], [ %41, %48 ]
  %53 = phi i64 [ %129, %125 ], [ 0, %48 ]
  %54 = phi i64 [ %76, %125 ], [ 0, %48 ]
  %55 = and i64 %53, 7
  %56 = icmp eq i64 %55, 7
  br i1 %56, label %57, label %62

57:                                               ; preds = %51
  %58 = load i64, ptr %13, align 8, !tbaa !291
  %59 = icmp sgt i64 %58, 0
  br i1 %59, label %60, label %62

60:                                               ; preds = %57
  %61 = call i64 @fa_clock() #45
  call fastcc void @fire_timers(ptr noundef nonnull %5, i64 noundef %61) #44
  br label %62

62:                                               ; preds = %60, %57, %51
  %63 = load ptr, ptr %5, align 8, !tbaa !316
  %64 = icmp eq ptr %63, null
  br i1 %64, label %72, label %65

65:                                               ; preds = %62
  %66 = icmp slt i64 %54, %49
  br i1 %66, label %70, label %67

67:                                               ; preds = %65
  %68 = load ptr, ptr %12, align 8, !tbaa !315
  %69 = icmp eq ptr %68, null
  br i1 %69, label %70, label %75

70:                                               ; preds = %67, %65
  %71 = add nsw i64 %54, 1
  br label %75

72:                                               ; preds = %62
  %73 = load ptr, ptr %12, align 8, !tbaa !315
  %74 = icmp eq ptr %73, null
  br i1 %74, label %131, label %75

75:                                               ; preds = %72, %70, %67
  %76 = phi i64 [ %54, %72 ], [ %71, %70 ], [ %54, %67 ]
  %77 = phi ptr [ %73, %72 ], [ %63, %70 ], [ %68, %67 ]
  call fastcc void @ready_unlink(ptr noundef nonnull %5, ptr noundef nonnull %77) #44
  %78 = getelementptr inbounds i8, ptr %77, i64 -16
  store i32 128, ptr %14, align 4, !tbaa !25
  store i64 0, ptr %15, align 8, !tbaa !285
  %79 = load i32, ptr %7, align 4, !tbaa !314
  %80 = icmp eq i32 %79, 0
  br i1 %80, label %83, label %81

81:                                               ; preds = %75
  %82 = call i64 @fa_clock() #45
  br label %83

83:                                               ; preds = %81, %75
  %84 = phi i64 [ %82, %81 ], [ 0, %75 ]
  br label %85

85:                                               ; preds = %91, %83
  %86 = phi ptr [ %78, %83 ], [ %92, %91 ]
  %87 = getelementptr inbounds i8, ptr %86, i64 40
  store atomic i32 4, ptr %87 release, align 8
  call void @fa_resume(ptr noundef nonnull %86) #45
  %88 = load ptr, ptr %16, align 8, !tbaa !10
  %89 = icmp eq ptr %88, null
  br i1 %89, label %91, label %90

90:                                               ; preds = %85
  store ptr null, ptr %16, align 8, !tbaa !10
  call void @fa_destroy(ptr noundef nonnull %88) #45
  br label %91

91:                                               ; preds = %90, %85
  %92 = load ptr, ptr %17, align 8, !tbaa !10
  store ptr null, ptr %17, align 8, !tbaa !10
  %93 = icmp eq ptr %92, null
  br i1 %93, label %94, label %85, !llvm.loop !317

94:                                               ; preds = %91
  %95 = load i32, ptr %7, align 4, !tbaa !314
  %96 = icmp eq i32 %95, 0
  br i1 %96, label %125, label %97

97:                                               ; preds = %94
  %98 = call i64 @fa_clock() #45
  %99 = sub nsw i64 %98, %84
  %100 = icmp sgt i64 %99, 50000000
  br i1 %100, label %101, label %125

101:                                              ; preds = %97
  %102 = load ptr, ptr %18, align 8, !tbaa !10
  %103 = load i32, ptr %19, align 8, !tbaa !318
  %104 = icmp sgt i32 %103, 0
  br i1 %104, label %105, label %117

105:                                              ; preds = %101
  %106 = zext nneg i32 %103 to i64
  br label %110

107:                                              ; preds = %110
  %108 = add nuw nsw i64 %111, 1
  %109 = icmp eq i64 %108, %106
  br i1 %109, label %115, label %110, !llvm.loop !319

110:                                              ; preds = %107, %105
  %111 = phi i64 [ 0, %105 ], [ %108, %107 ]
  %112 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 27, i64 %111
  %113 = load ptr, ptr %112, align 8, !tbaa !10
  %114 = icmp eq ptr %113, %102
  br i1 %114, label %125, label %107

115:                                              ; preds = %107
  %116 = icmp slt i32 %103, 32
  br i1 %116, label %117, label %121

117:                                              ; preds = %115, %101
  %118 = sext i32 %103 to i64
  %119 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 27, i64 %118
  store ptr %102, ptr %119, align 8, !tbaa !10
  %120 = add nsw i32 %103, 1
  store i32 %120, ptr %19, align 8, !tbaa !318
  br label %121

121:                                              ; preds = %117, %115
  call void @fa_flush() #44
  store i64 0, ptr %20, align 8, !tbaa !8
  call fastcc void @append_c(ptr noundef nonnull @.str.46) #44
  %122 = udiv i64 %99, 1000000
  call fastcc void @append_u64(i64 noundef %122, i32 noundef 0) #44
  call fastcc void @append_c(ptr noundef nonnull @.str.47) #44
  %123 = icmp eq ptr %102, null
  %124 = select i1 %123, ptr @.str.48, ptr %102
  call fastcc void @append_c(ptr noundef nonnull %124) #44
  call fastcc void @append_c(ptr noundef nonnull @.str.49) #44
  call void @fa_b_line(i32 noundef 2) #44
  br label %125

125:                                              ; preds = %110, %121, %97, %94
  %126 = load i64, ptr %21, align 8, !tbaa !320
  %127 = add nsw i64 %126, 1
  store i64 %127, ptr %21, align 8, !tbaa !320
  %128 = add nsw i64 %52, -1
  %129 = add nuw nsw i64 %53, 1
  %130 = icmp sgt i64 %52, 1
  br i1 %130, label %51, label %131, !llvm.loop !321

131:                                              ; preds = %125, %72, %48
  %132 = load i64, ptr %13, align 8, !tbaa !291
  %133 = icmp sgt i64 %132, 0
  br i1 %133, label %134, label %139

134:                                              ; preds = %131
  %135 = call i64 @fa_clock() #45
  %136 = load i64, ptr %13, align 8, !tbaa !291
  %137 = icmp sgt i64 %136, 0
  br i1 %137, label %138, label %139

138:                                              ; preds = %134
  call fastcc void @fire_timers(ptr noundef nonnull %5, i64 noundef %135) #44
  br label %139

139:                                              ; preds = %138, %134, %131
  %140 = phi i64 [ %135, %138 ], [ %135, %134 ], [ 0, %131 ]
  call fastcc void @drain_remote(ptr noundef nonnull %5) #44
  %141 = load i64, ptr %10, align 8, !tbaa !283
  %142 = icmp sgt i64 %141, 0
  br i1 %142, label %143, label %152

143:                                              ; preds = %139
  %144 = load i32, ptr %24, align 4, !tbaa !273
  %145 = icmp sgt i32 %144, -1
  br i1 %145, label %146, label %301

146:                                              ; preds = %143
  %147 = load i64, ptr %21, align 8, !tbaa !320
  %148 = icmp sgt i64 %147, 60
  br i1 %148, label %181, label %149

149:                                              ; preds = %146
  %150 = load i32, ptr %28, align 8, !tbaa !284
  %151 = icmp eq i32 %150, 0
  br i1 %151, label %301, label %181

152:                                              ; preds = %139
  store i64 0, ptr %21, align 8, !tbaa !320
  %153 = load ptr, ptr %6, align 8, !tbaa !275
  %154 = load ptr, ptr %153, align 8, !tbaa !10
  %155 = icmp eq ptr %154, null
  br i1 %155, label %156, label %162

156:                                              ; preds = %152
  %157 = load i64, ptr %8, align 8, !tbaa !268
  %158 = icmp eq i64 %157, 0
  br i1 %158, label %159, label %162

159:                                              ; preds = %156
  %160 = load atomic i64, ptr %9 acquire, align 8
  %161 = icmp eq i64 %160, 0
  br i1 %161, label %301, label %162

162:                                              ; preds = %159, %156, %152
  %163 = load i64, ptr %13, align 8, !tbaa !291
  %164 = icmp sgt i64 %163, 0
  br i1 %164, label %165, label %170

165:                                              ; preds = %162
  %166 = load ptr, ptr %23, align 8, !tbaa !293
  %167 = load i64, ptr %166, align 8, !tbaa !322
  %168 = sub nsw i64 %167, %140
  %169 = call i64 @llvm.smax.i64(i64 %168, i64 0)
  br label %177

170:                                              ; preds = %162
  %171 = load atomic i64, ptr %9 acquire, align 8
  %172 = icmp sgt i64 %171, 0
  br i1 %172, label %177, label %173

173:                                              ; preds = %170
  %174 = load i64, ptr %22, align 8, !tbaa !311
  %175 = icmp sgt i64 %174, 0
  br i1 %175, label %177, label %176

176:                                              ; preds = %173
  call fastcc void @deadlock_stop(ptr noundef nonnull %5) #49
  unreachable

177:                                              ; preds = %173, %170, %165
  %178 = phi i64 [ %169, %165 ], [ -1, %173 ], [ -1, %170 ]
  %179 = load i32, ptr %24, align 4, !tbaa !273
  %180 = icmp sgt i32 %179, -1
  br i1 %180, label %182, label %298

181:                                              ; preds = %146, %149
  store i64 0, ptr %21, align 8, !tbaa !320
  store i32 0, ptr %28, align 8, !tbaa !284
  br label %210

182:                                              ; preds = %177
  %183 = icmp slt i64 %178, 0
  br i1 %183, label %189, label %184

184:                                              ; preds = %182
  %185 = add nuw nsw i64 %178, 999999
  %186 = udiv i64 %185, 1000000
  %187 = trunc i64 %186 to i32
  %188 = icmp eq i64 %178, 0
  br i1 %188, label %210, label %189

189:                                              ; preds = %184, %182
  %190 = phi i32 [ %187, %184 ], [ -1, %182 ]
  store atomic i32 1, ptr %25 seq_cst, align 8
  %191 = load ptr, ptr %26, align 8, !tbaa !271
  %192 = getelementptr inbounds %struct.FaTask, ptr %191, i64 0, i32 1
  %193 = load atomic i64, ptr %192 acquire, align 8
  %194 = icmp eq i64 %193, 0
  br i1 %194, label %195, label %200

195:                                              ; preds = %189
  %196 = load atomic i64, ptr %27 acquire, align 8
  %197 = inttoptr i64 %196 to ptr
  %198 = load ptr, ptr %26, align 8, !tbaa !271
  %199 = icmp eq ptr %198, %197
  br i1 %199, label %201, label %200

200:                                              ; preds = %195, %189
  br label %201

201:                                              ; preds = %200, %195
  %202 = phi i32 [ 0, %200 ], [ %190, %195 ]
  br i1 %183, label %203, label %210

203:                                              ; preds = %201
  %204 = load i64, ptr %22, align 8, !tbaa !311
  %205 = icmp eq i64 %204, 0
  br i1 %205, label %206, label %210

206:                                              ; preds = %203
  %207 = load atomic i64, ptr %9 seq_cst, align 8
  %208 = icmp eq i64 %207, 0
  %209 = select i1 %208, i32 0, i32 %202
  br label %210

210:                                              ; preds = %181, %206, %203, %201, %184
  %211 = phi i32 [ %202, %203 ], [ %202, %201 ], [ %187, %184 ], [ %209, %206 ], [ 0, %181 ]
  call void @llvm.lifetime.start.p0(i64 3072, ptr nonnull %3) #40
  %212 = load i32, ptr %24, align 4, !tbaa !273
  %213 = call i32 @epoll_wait(i32 noundef %212, ptr noundef nonnull %3, i32 noundef 256, i32 noundef %211) #45
  store atomic i32 0, ptr %25 seq_cst, align 8
  %214 = icmp sgt i32 %213, 0
  br i1 %214, label %215, label %217

215:                                              ; preds = %210
  %216 = zext nneg i32 %213 to i64
  br label %218

217:                                              ; preds = %295, %210
  call fastcc void @drain_remote(ptr noundef %5) #44
  call void @llvm.lifetime.end.p0(i64 3072, ptr nonnull %3) #40
  br label %301

218:                                              ; preds = %295, %215
  %219 = phi i64 [ 0, %215 ], [ %296, %295 ]
  %220 = getelementptr inbounds [256 x %struct.FaEpollEvent], ptr %3, i64 0, i64 %219, i32 1
  %221 = load i64, ptr %220, align 4, !tbaa !308
  %222 = icmp eq i64 %221, 0
  br i1 %222, label %223, label %226

223:                                              ; preds = %218
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %4) #40
  %224 = load i32, ptr %29, align 8, !tbaa !264
  %225 = call i64 @read(i32 noundef %224, ptr noundef nonnull %4, i64 noundef 8) #45
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %4) #40
  br label %295

226:                                              ; preds = %218
  %227 = getelementptr inbounds [256 x %struct.FaEpollEvent], ptr %3, i64 0, i64 %219
  %228 = inttoptr i64 %221 to ptr
  %229 = load i32, ptr %227, align 4, !tbaa !306
  %230 = and i32 %229, 8217
  %231 = icmp eq i32 %230, 0
  br i1 %231, label %262, label %232

232:                                              ; preds = %226
  %233 = getelementptr inbounds %struct.FaIo, ptr %228, i64 0, i32 1
  %234 = load i32, ptr %233, align 4, !tbaa !304
  %235 = or i32 %234, 1
  store i32 %235, ptr %233, align 4, !tbaa !304
  %236 = getelementptr inbounds %struct.FaIo, ptr %228, i64 0, i32 2
  %237 = load ptr, ptr %236, align 8, !tbaa !312
  %238 = icmp eq ptr %237, null
  br i1 %238, label %262, label %239

239:                                              ; preds = %232
  %240 = getelementptr inbounds %struct.FaWait, ptr %237, i64 0, i32 9
  store i32 1, ptr %240, align 8, !tbaa !309
  %241 = getelementptr inbounds %struct.FaIo, ptr %228, i64 0, i32 4
  %242 = load ptr, ptr %241, align 8, !tbaa !305
  %243 = getelementptr inbounds %struct.FaExecutor, ptr %242, i64 0, i32 13
  %244 = load i64, ptr %243, align 8, !tbaa !311
  %245 = add nsw i64 %244, -1
  store i64 %245, ptr %243, align 8, !tbaa !311
  %246 = load ptr, ptr %237, align 8, !tbaa !286
  %247 = getelementptr inbounds i8, ptr %246, i64 16
  %248 = getelementptr inbounds i8, ptr %246, i64 44
  %249 = load i32, ptr %248, align 4, !tbaa !267
  %250 = and i32 %249, -5
  store i32 %250, ptr %248, align 4, !tbaa !267
  %251 = getelementptr inbounds %struct.FaExecutor, ptr %242, i64 0, i32 1
  %252 = getelementptr inbounds i8, ptr %246, i64 24
  store ptr null, ptr %252, align 8, !tbaa !269
  %253 = load ptr, ptr %251, align 8, !tbaa !10
  %254 = getelementptr inbounds i8, ptr %246, i64 32
  store ptr %253, ptr %254, align 8, !tbaa !270
  %255 = icmp eq ptr %253, null
  %256 = getelementptr inbounds %struct.FaTask, ptr %253, i64 0, i32 1
  %257 = select i1 %255, ptr %242, ptr %256
  store ptr %247, ptr %257, align 8, !tbaa !10
  store ptr %247, ptr %251, align 8, !tbaa !10
  %258 = getelementptr inbounds %struct.FaExecutor, ptr %242, i64 0, i32 4
  %259 = load <2 x i64>, ptr %258, align 8, !tbaa !8
  %260 = add nsw <2 x i64> %259, <i64 1, i64 1>
  store <2 x i64> %260, ptr %258, align 8, !tbaa !8
  %261 = getelementptr inbounds i8, ptr %246, i64 40
  store atomic i32 1, ptr %261 release, align 8
  store ptr null, ptr %236, align 8, !tbaa !312
  br label %262

262:                                              ; preds = %239, %232, %226
  %263 = and i32 %229, 28
  %264 = icmp eq i32 %263, 0
  br i1 %264, label %295, label %265

265:                                              ; preds = %262
  %266 = getelementptr inbounds %struct.FaIo, ptr %228, i64 0, i32 1
  %267 = load i32, ptr %266, align 4, !tbaa !304
  %268 = or i32 %267, 4
  store i32 %268, ptr %266, align 4, !tbaa !304
  %269 = getelementptr inbounds %struct.FaIo, ptr %228, i64 0, i32 3
  %270 = load ptr, ptr %269, align 8, !tbaa !313
  %271 = icmp eq ptr %270, null
  br i1 %271, label %295, label %272

272:                                              ; preds = %265
  %273 = getelementptr inbounds %struct.FaWait, ptr %270, i64 0, i32 9
  store i32 1, ptr %273, align 8, !tbaa !309
  %274 = getelementptr inbounds %struct.FaIo, ptr %228, i64 0, i32 4
  %275 = load ptr, ptr %274, align 8, !tbaa !305
  %276 = getelementptr inbounds %struct.FaExecutor, ptr %275, i64 0, i32 13
  %277 = load i64, ptr %276, align 8, !tbaa !311
  %278 = add nsw i64 %277, -1
  store i64 %278, ptr %276, align 8, !tbaa !311
  %279 = load ptr, ptr %270, align 8, !tbaa !286
  %280 = getelementptr inbounds i8, ptr %279, i64 16
  %281 = getelementptr inbounds i8, ptr %279, i64 44
  %282 = load i32, ptr %281, align 4, !tbaa !267
  %283 = and i32 %282, -5
  store i32 %283, ptr %281, align 4, !tbaa !267
  %284 = getelementptr inbounds %struct.FaExecutor, ptr %275, i64 0, i32 1
  %285 = getelementptr inbounds i8, ptr %279, i64 24
  store ptr null, ptr %285, align 8, !tbaa !269
  %286 = load ptr, ptr %284, align 8, !tbaa !10
  %287 = getelementptr inbounds i8, ptr %279, i64 32
  store ptr %286, ptr %287, align 8, !tbaa !270
  %288 = icmp eq ptr %286, null
  %289 = getelementptr inbounds %struct.FaTask, ptr %286, i64 0, i32 1
  %290 = select i1 %288, ptr %275, ptr %289
  store ptr %280, ptr %290, align 8, !tbaa !10
  store ptr %280, ptr %284, align 8, !tbaa !10
  %291 = getelementptr inbounds %struct.FaExecutor, ptr %275, i64 0, i32 4
  %292 = load <2 x i64>, ptr %291, align 8, !tbaa !8
  %293 = add nsw <2 x i64> %292, <i64 1, i64 1>
  store <2 x i64> %293, ptr %291, align 8, !tbaa !8
  %294 = getelementptr inbounds i8, ptr %279, i64 40
  store atomic i32 1, ptr %294 release, align 8
  store ptr null, ptr %269, align 8, !tbaa !313
  br label %295

295:                                              ; preds = %272, %265, %262, %223
  %296 = add nuw nsw i64 %219, 1
  %297 = icmp eq i64 %296, %216
  br i1 %297, label %217, label %218, !llvm.loop !323

298:                                              ; preds = %177
  %299 = icmp sgt i64 %178, 0
  br i1 %299, label %300, label %301

300:                                              ; preds = %298
  call void @fa_nanosleep(i64 noundef %178) #45
  br label %301

301:                                              ; preds = %300, %298, %217, %143, %149, %159
  br label %30, !llvm.loop !324

302:                                              ; preds = %36
  call void @fa_destroy(ptr noundef nonnull %0) #45
  %303 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 8
  %304 = load i64, ptr %303, align 8, !tbaa !292
  %305 = icmp sgt i64 %304, 0
  br i1 %305, label %306, label %309

306:                                              ; preds = %302
  %307 = load ptr, ptr %23, align 8, !tbaa !293
  %308 = shl i64 %304, 4
  call void @fa_free(ptr noundef %307, i64 noundef %308, i64 noundef 64) #45
  br label %309

309:                                              ; preds = %306, %302
  %310 = load i32, ptr %24, align 4, !tbaa !273
  %311 = icmp sgt i32 %310, -1
  br i1 %311, label %312, label %314

312:                                              ; preds = %309
  %313 = call i32 @close(i32 noundef %310) #45
  br label %314

314:                                              ; preds = %312, %309
  %315 = load i32, ptr %29, align 8, !tbaa !264
  %316 = icmp sgt i32 %315, -1
  br i1 %316, label %317, label %319

317:                                              ; preds = %314
  %318 = call i32 @close(i32 noundef %315) #45
  br label %319

319:                                              ; preds = %317, %314
  %320 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 14
  %321 = load atomic i64, ptr %320 seq_cst, align 8
  %322 = icmp eq i64 %321, 0
  br i1 %322, label %329, label %323

323:                                              ; preds = %319, %326
  br i1 icmp ne (ptr @sched_yield, ptr null), label %324, label %326

324:                                              ; preds = %323
  %325 = call i32 @sched_yield() #45
  br label %326

326:                                              ; preds = %324, %323
  %327 = load atomic i64, ptr %320 seq_cst, align 8
  %328 = icmp eq i64 %327, 0
  br i1 %328, label %329, label %323, !llvm.loop !325

329:                                              ; preds = %326, %319
  %330 = call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @current_executor)
  store ptr null, ptr %330, align 8, !tbaa !10
  call void @fa_free(ptr noundef nonnull %5, i64 noundef 488, i64 noundef 16) #45
  ret void
}

; Function Attrs: optsize
declare extern_weak i32 @close(i32 noundef) local_unnamed_addr #3

; Function Attrs: optsize
declare extern_weak i32 @sched_yield() #3

; Function Attrs: nounwind optsize
define noundef i32 @fa_blocking_submit(ptr noundef %0, ptr noundef %1, ptr noundef %2) local_unnamed_addr #2 {
  %4 = alloca i64, align 8
  %5 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 9
  br i1 icmp eq (ptr @pthread_create, ptr null), label %6, label %9

6:                                                ; preds = %3
  %7 = load ptr, ptr %0, align 8, !tbaa !326
  tail call void %7(ptr noundef nonnull %0) #45
  %8 = getelementptr inbounds %struct.FaJob, ptr %0, i64 0, i32 5
  store i32 2, ptr %8, align 8, !tbaa !328
  store i32 1, ptr %5, align 8, !tbaa !309
  br label %47

9:                                                ; preds = %3
  %10 = tail call fastcc ptr @executor() #44
  store ptr %2, ptr %1, align 8, !tbaa !286
  %11 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 1
  store ptr %10, ptr %11, align 8, !tbaa !288
  store i32 0, ptr %5, align 8, !tbaa !309
  %12 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 10
  store i32 0, ptr %12, align 4, !tbaa !310
  %13 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 8
  store i64 -1, ptr %13, align 8, !tbaa !289
  %14 = getelementptr inbounds %struct.FaJob, ptr %0, i64 0, i32 2
  store ptr %1, ptr %14, align 8, !tbaa !329
  %15 = getelementptr inbounds %struct.FaJob, ptr %0, i64 0, i32 3
  store ptr %10, ptr %15, align 8, !tbaa !330
  %16 = getelementptr inbounds %struct.FaJob, ptr %0, i64 0, i32 5
  store i32 0, ptr %16, align 8, !tbaa !328
  %17 = getelementptr inbounds %struct.FaJob, ptr %0, i64 0, i32 4
  store ptr null, ptr %17, align 8, !tbaa !331
  %18 = getelementptr inbounds %struct.FaExecutor, ptr %10, i64 0, i32 12
  %19 = atomicrmw add ptr %18, i64 1 acq_rel, align 8
  %20 = getelementptr inbounds i8, ptr %2, i64 40
  store atomic i32 2, ptr %20 release, align 8
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %21, label %23

21:                                               ; preds = %9
  %22 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull @blocking) #45
  br label %23

23:                                               ; preds = %9, %21
  %24 = load ptr, ptr getelementptr inbounds (%struct.anon, ptr @blocking, i64 0, i32 3), align 8, !tbaa !332
  %25 = icmp eq ptr %24, null
  br i1 %25, label %28, label %26

26:                                               ; preds = %23
  %27 = getelementptr inbounds %struct.FaJob, ptr %24, i64 0, i32 4
  store ptr %0, ptr %27, align 8, !tbaa !331
  br label %29

28:                                               ; preds = %23
  store ptr %0, ptr getelementptr inbounds (%struct.anon, ptr @blocking, i64 0, i32 2), align 16, !tbaa !335
  br label %29

29:                                               ; preds = %28, %26
  store ptr %0, ptr getelementptr inbounds (%struct.anon, ptr @blocking, i64 0, i32 3), align 8, !tbaa !332
  %30 = load i32, ptr getelementptr inbounds (%struct.anon, ptr @blocking, i64 0, i32 5), align 4, !tbaa !336
  %31 = icmp sgt i32 %30, 0
  br i1 %31, label %32, label %34

32:                                               ; preds = %29
  %33 = tail call i32 @pthread_cond_signal(ptr noundef nonnull getelementptr inbounds (%struct.anon, ptr @blocking, i64 0, i32 1)) #45
  br label %44

34:                                               ; preds = %29
  %35 = load i32, ptr getelementptr inbounds (%struct.anon, ptr @blocking, i64 0, i32 4), align 16, !tbaa !337
  %36 = icmp slt i32 %35, 64
  br i1 %36, label %37, label %44

37:                                               ; preds = %34
  %38 = add nsw i32 %35, 1
  store i32 %38, ptr getelementptr inbounds (%struct.anon, ptr @blocking, i64 0, i32 4), align 16, !tbaa !337
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %4) #40
  %39 = call i32 @pthread_create(ptr noundef nonnull %4, ptr noundef null, ptr noundef nonnull @blocking_thread, ptr noundef null) #45
  br i1 icmp ne (ptr @pthread_detach, ptr null), label %40, label %43

40:                                               ; preds = %37
  %41 = load i64, ptr %4, align 8, !tbaa !8
  %42 = call i32 @pthread_detach(i64 noundef %41) #45
  br label %43

43:                                               ; preds = %40, %37
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %4) #40
  br label %44

44:                                               ; preds = %34, %43, %32
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %45, label %47

45:                                               ; preds = %44
  %46 = call i32 @pthread_mutex_unlock(ptr noundef nonnull @blocking) #45
  br label %47

47:                                               ; preds = %45, %44, %6
  %48 = phi i32 [ 0, %6 ], [ 1, %44 ], [ 1, %45 ]
  ret i32 %48
}

; Function Attrs: optsize
declare extern_weak i32 @pthread_cond_signal(ptr noundef) local_unnamed_addr #3

; Function Attrs: optsize
declare extern_weak i32 @pthread_create(ptr noundef, ptr noundef, ptr noundef, ptr noundef) #3

; Function Attrs: nounwind optsize
define internal noundef ptr @blocking_thread(ptr nocapture readnone %0) #2 {
  %2 = alloca %struct.FaTimespec, align 8
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %3, label %5

3:                                                ; preds = %1
  %4 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull @blocking) #45
  br label %5

5:                                                ; preds = %1, %3
  %6 = or i1 icmp eq (ptr @pthread_cond_timedwait, ptr null), icmp eq (ptr @clock_gettime, ptr null)
  br label %7

7:                                                ; preds = %72, %5
  %8 = load ptr, ptr getelementptr inbounds (%struct.anon, ptr @blocking, i64 0, i32 2), align 16, !tbaa !335
  %9 = icmp eq ptr %8, null
  br i1 %9, label %10, label %40

10:                                               ; preds = %7
  %11 = load i32, ptr getelementptr inbounds (%struct.anon, ptr @blocking, i64 0, i32 5), align 4, !tbaa !336
  br label %12

12:                                               ; preds = %10, %36
  %13 = phi i32 [ %11, %10 ], [ %38, %36 ]
  %14 = add nsw i32 %13, 1
  store i32 %14, ptr getelementptr inbounds (%struct.anon, ptr @blocking, i64 0, i32 5), align 4, !tbaa !336
  br i1 %6, label %15, label %20

15:                                               ; preds = %12
  %16 = call i32 @pthread_cond_wait(ptr noundef nonnull getelementptr inbounds (%struct.anon, ptr @blocking, i64 0, i32 1), ptr noundef nonnull @blocking) #45
  %17 = load i32, ptr getelementptr inbounds (%struct.anon, ptr @blocking, i64 0, i32 5), align 4, !tbaa !336
  %18 = add nsw i32 %17, -1
  store i32 %18, ptr getelementptr inbounds (%struct.anon, ptr @blocking, i64 0, i32 5), align 4, !tbaa !336
  %19 = load ptr, ptr getelementptr inbounds (%struct.anon, ptr @blocking, i64 0, i32 2), align 16, !tbaa !335
  br label %36

20:                                               ; preds = %12
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %2) #40
  %21 = call i32 @clock_gettime(i32 noundef 0, ptr noundef nonnull %2) #45
  %22 = load i64, ptr %2, align 8, !tbaa !338
  %23 = add nsw i64 %22, 10
  store i64 %23, ptr %2, align 8, !tbaa !338
  %24 = call i32 @pthread_cond_timedwait(ptr noundef nonnull getelementptr inbounds (%struct.anon, ptr @blocking, i64 0, i32 1), ptr noundef nonnull @blocking, ptr noundef nonnull %2) #45
  %25 = icmp ne i32 %24, 0
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %2) #40
  %26 = load i32, ptr getelementptr inbounds (%struct.anon, ptr @blocking, i64 0, i32 5), align 4, !tbaa !336
  %27 = add nsw i32 %26, -1
  store i32 %27, ptr getelementptr inbounds (%struct.anon, ptr @blocking, i64 0, i32 5), align 4, !tbaa !336
  %28 = load ptr, ptr getelementptr inbounds (%struct.anon, ptr @blocking, i64 0, i32 2), align 16
  %29 = icmp eq ptr %28, null
  %30 = select i1 %25, i1 %29, i1 false
  br i1 %30, label %31, label %36

31:                                               ; preds = %20
  %32 = load i32, ptr getelementptr inbounds (%struct.anon, ptr @blocking, i64 0, i32 4), align 16, !tbaa !337
  %33 = add nsw i32 %32, -1
  store i32 %33, ptr getelementptr inbounds (%struct.anon, ptr @blocking, i64 0, i32 4), align 16, !tbaa !337
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %34, label %75

34:                                               ; preds = %31
  %35 = call i32 @pthread_mutex_unlock(ptr noundef nonnull @blocking) #45
  br label %75

36:                                               ; preds = %15, %20
  %37 = phi ptr [ %19, %15 ], [ %28, %20 ]
  %38 = phi i32 [ %18, %15 ], [ %27, %20 ]
  %39 = icmp eq ptr %37, null
  br i1 %39, label %12, label %40

40:                                               ; preds = %36, %7
  %41 = phi ptr [ %8, %7 ], [ %37, %36 ]
  %42 = getelementptr inbounds %struct.FaJob, ptr %41, i64 0, i32 4
  %43 = load ptr, ptr %42, align 8, !tbaa !331
  store ptr %43, ptr getelementptr inbounds (%struct.anon, ptr @blocking, i64 0, i32 2), align 16, !tbaa !335
  %44 = icmp eq ptr %43, null
  br i1 %44, label %45, label %46

45:                                               ; preds = %40
  store ptr null, ptr getelementptr inbounds (%struct.anon, ptr @blocking, i64 0, i32 3), align 8, !tbaa !332
  br label %46

46:                                               ; preds = %45, %40
  %47 = getelementptr inbounds %struct.FaJob, ptr %41, i64 0, i32 5
  store i32 1, ptr %47, align 8, !tbaa !328
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %48, label %50

48:                                               ; preds = %46
  %49 = call i32 @pthread_mutex_unlock(ptr noundef nonnull @blocking) #45
  br label %50

50:                                               ; preds = %46, %48
  %51 = load ptr, ptr %41, align 8, !tbaa !326
  call void %51(ptr noundef nonnull %41) #45
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %52, label %54

52:                                               ; preds = %50
  %53 = call i32 @pthread_mutex_lock(ptr noundef nonnull @blocking) #45
  br label %54

54:                                               ; preds = %50, %52
  %55 = getelementptr inbounds %struct.FaJob, ptr %41, i64 0, i32 3
  %56 = load ptr, ptr %55, align 8, !tbaa !330
  %57 = load i32, ptr %47, align 8, !tbaa !328
  %58 = icmp eq i32 %57, 3
  br i1 %58, label %59, label %65

59:                                               ; preds = %54
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %60, label %62

60:                                               ; preds = %59
  %61 = call i32 @pthread_mutex_unlock(ptr noundef nonnull @blocking) #45
  br label %62

62:                                               ; preds = %59, %60
  %63 = getelementptr inbounds %struct.FaJob, ptr %41, i64 0, i32 1
  %64 = load ptr, ptr %63, align 8, !tbaa !340
  call void %64(ptr noundef nonnull %41) #45
  br label %71

65:                                               ; preds = %54
  store i32 2, ptr %47, align 8, !tbaa !328
  %66 = getelementptr inbounds %struct.FaJob, ptr %41, i64 0, i32 2
  %67 = load ptr, ptr %66, align 8, !tbaa !329
  %68 = getelementptr inbounds %struct.FaWait, ptr %67, i64 0, i32 9
  store i32 1, ptr %68, align 8, !tbaa !309
  call fastcc void @wake(ptr noundef %67) #44
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %69, label %71

69:                                               ; preds = %65
  %70 = call i32 @pthread_mutex_unlock(ptr noundef nonnull @blocking) #45
  br label %71

71:                                               ; preds = %69, %65, %62
  call void @fa_remote_source_end(ptr noundef %56) #44
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %73, label %72

72:                                               ; preds = %71, %73
  br label %7

73:                                               ; preds = %71
  %74 = call i32 @pthread_mutex_lock(ptr noundef nonnull @blocking) #45
  br label %72

75:                                               ; preds = %31, %34
  ret ptr null
}

; Function Attrs: optsize
declare extern_weak i32 @pthread_detach(i64 noundef) #3

; Function Attrs: nounwind optsize
define void @fa_blocking_cancel(ptr noundef %0) local_unnamed_addr #2 {
  %2 = getelementptr inbounds %struct.FaJob, ptr %0, i64 0, i32 3
  %3 = load ptr, ptr %2, align 8, !tbaa !330
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %4, label %6

4:                                                ; preds = %1
  %5 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull @blocking) #45
  br label %6

6:                                                ; preds = %1, %4
  %7 = getelementptr inbounds %struct.FaJob, ptr %0, i64 0, i32 5
  %8 = load i32, ptr %7, align 8, !tbaa !328
  switch i32 %8, label %30 [
    i32 0, label %9
    i32 1, label %27
  ]

9:                                                ; preds = %6, %9
  %10 = phi ptr [ %14, %9 ], [ getelementptr inbounds (%struct.anon, ptr @blocking, i64 0, i32 2), %6 ]
  %11 = phi ptr [ %12, %9 ], [ null, %6 ]
  %12 = load ptr, ptr %10, align 8, !tbaa !10
  %13 = icmp eq ptr %12, %0
  %14 = getelementptr inbounds %struct.FaJob, ptr %12, i64 0, i32 4
  br i1 %13, label %15, label %9, !llvm.loop !341

15:                                               ; preds = %9
  %16 = getelementptr inbounds %struct.FaJob, ptr %0, i64 0, i32 4
  %17 = load ptr, ptr %16, align 8, !tbaa !331
  store ptr %17, ptr %10, align 8, !tbaa !10
  %18 = load ptr, ptr getelementptr inbounds (%struct.anon, ptr @blocking, i64 0, i32 3), align 8, !tbaa !332
  %19 = icmp eq ptr %18, %0
  br i1 %19, label %20, label %21

20:                                               ; preds = %15
  store ptr %11, ptr getelementptr inbounds (%struct.anon, ptr @blocking, i64 0, i32 3), align 8, !tbaa !332
  br label %21

21:                                               ; preds = %20, %15
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %22, label %24

22:                                               ; preds = %21
  %23 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull @blocking) #45
  br label %24

24:                                               ; preds = %21, %22
  %25 = getelementptr inbounds %struct.FaJob, ptr %0, i64 0, i32 1
  %26 = load ptr, ptr %25, align 8, !tbaa !340
  tail call void %26(ptr noundef nonnull %0) #45
  tail call void @fa_remote_source_end(ptr noundef %3) #44
  br label %38

27:                                               ; preds = %6
  store i32 3, ptr %7, align 8, !tbaa !328
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %28, label %38

28:                                               ; preds = %27
  %29 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull @blocking) #45
  br label %38

30:                                               ; preds = %6
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %31, label %33

31:                                               ; preds = %30
  %32 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull @blocking) #45
  br label %33

33:                                               ; preds = %30, %31
  %34 = getelementptr inbounds %struct.FaJob, ptr %0, i64 0, i32 2
  %35 = load ptr, ptr %34, align 8, !tbaa !329
  tail call fastcc void @forget_remote_wake(ptr noundef %35) #44
  %36 = getelementptr inbounds %struct.FaJob, ptr %0, i64 0, i32 1
  %37 = load ptr, ptr %36, align 8, !tbaa !340
  tail call void %37(ptr noundef nonnull %0) #45
  br label %38

38:                                               ; preds = %28, %27, %33, %24
  ret void
}

; Function Attrs: nofree norecurse nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc void @forget_remote_wake(ptr nocapture noundef readonly %0) unnamed_addr #38 {
  %2 = getelementptr inbounds %struct.FaWait, ptr %0, i64 0, i32 10
  %3 = load i32, ptr %2, align 4, !tbaa !310
  %4 = icmp eq i32 %3, 0
  br i1 %4, label %14, label %5

5:                                                ; preds = %1
  %6 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @current_executor)
  %7 = load ptr, ptr %6, align 8, !tbaa !10
  tail call fastcc void @drain_remote(ptr noundef %7) #44
  %8 = load ptr, ptr %0, align 8, !tbaa !286
  %9 = getelementptr inbounds i8, ptr %8, i64 40
  %10 = load atomic i32, ptr %9 acquire, align 8
  %11 = icmp eq i32 %10, 1
  br i1 %11, label %12, label %14

12:                                               ; preds = %5
  %13 = getelementptr inbounds i8, ptr %8, i64 16
  tail call fastcc void @ready_unlink(ptr noundef %7, ptr noundef nonnull %13) #44
  br label %14

14:                                               ; preds = %5, %12, %1
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_tcp_close(ptr nocapture noundef %0) local_unnamed_addr #2 {
  tail call fastcc void @socket_close(ptr noundef %0) #44
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @socket_close(ptr nocapture noundef %0) unnamed_addr #2 {
  %2 = getelementptr inbounds %struct.FaSocket, ptr %0, i64 0, i32 4
  %3 = load ptr, ptr %2, align 8, !tbaa !342
  %4 = icmp eq ptr %3, null
  br i1 %4, label %13, label %5

5:                                                ; preds = %1
  store ptr null, ptr %2, align 8, !tbaa !342
  %6 = getelementptr inbounds %struct.FaIo, ptr %3, i64 0, i32 4
  %7 = load ptr, ptr %6, align 8, !tbaa !305
  %8 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @current_executor)
  %9 = load ptr, ptr %8, align 8, !tbaa !10
  %10 = icmp eq ptr %7, %9
  br i1 %10, label %11, label %12

11:                                               ; preds = %5
  tail call void @fa_io_close(ptr noundef nonnull %3) #44
  br label %13

12:                                               ; preds = %5
  tail call void @fa_free(ptr noundef nonnull %3, i64 noundef 32, i64 noundef 16) #45
  br label %13

13:                                               ; preds = %11, %12, %1
  %14 = getelementptr inbounds %struct.FaSocket, ptr %0, i64 0, i32 1
  %15 = load i32, ptr %14, align 8, !tbaa !344
  %16 = icmp sgt i32 %15, -1
  br i1 %16, label %17, label %19

17:                                               ; preds = %13
  %18 = tail call i32 @close(i32 noundef %15) #45
  br label %19

19:                                               ; preds = %17, %13
  store i32 -1, ptr %14, align 8, !tbaa !344
  ret void
}

; Function Attrs: nounwind optsize
define ptr @fa_tcp_listen(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #2 {
  %4 = alloca i32, align 4
  %5 = alloca i32, align 4
  %6 = alloca %struct.FaSocketAddress, align 8
  %7 = alloca i32, align 4
  %8 = tail call fastcc ptr @shared_new(i64 noundef 56, ptr noundef nonnull @socket_drop) #44
  %9 = getelementptr inbounds %struct.FaSocket, ptr %8, i64 0, i32 1
  store i32 -1, ptr %9, align 8, !tbaa !344
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %4) #40
  store i32 0, ptr %4, align 4, !tbaa !25
  %10 = select i1 icmp ne (ptr @socket, ptr null), i1 icmp eq (ptr @poll, ptr null), i1 true
  br i1 %10, label %11, label %13

11:                                               ; preds = %3
  %12 = getelementptr inbounds %struct.FaSocket, ptr %8, i64 0, i32 2
  store i32 38, ptr %12, align 4, !tbaa !345
  br label %69

13:                                               ; preds = %3
  %14 = icmp eq i64 %1, 0
  br i1 %14, label %20, label %15

15:                                               ; preds = %13
  %16 = call fastcc i32 @parse_host(ptr noundef %0, i64 noundef %1, ptr noundef nonnull %4) #44, !range !60
  %17 = icmp eq i32 %16, 0
  br i1 %17, label %18, label %20

18:                                               ; preds = %15
  %19 = getelementptr inbounds %struct.FaSocket, ptr %8, i64 0, i32 2
  store i32 -1, ptr %19, align 4, !tbaa !345
  br label %69

20:                                               ; preds = %15, %13
  %21 = icmp ugt i64 %2, 65535
  br i1 %21, label %22, label %24

22:                                               ; preds = %20
  %23 = getelementptr inbounds %struct.FaSocket, ptr %8, i64 0, i32 2
  store i32 22, ptr %23, align 4, !tbaa !345
  br label %69

24:                                               ; preds = %20
  %25 = tail call i32 @socket(i32 noundef 2, i32 noundef 526337, i32 noundef 0) #45
  %26 = icmp slt i32 %25, 0
  br i1 %26, label %27, label %34

27:                                               ; preds = %24
  br i1 icmp ne (ptr @__errno_location, ptr null), label %28, label %31

28:                                               ; preds = %27
  %29 = tail call ptr @__errno_location() #45
  %30 = load i32, ptr %29, align 4, !tbaa !25
  br label %31

31:                                               ; preds = %27, %28
  %32 = phi i32 [ %30, %28 ], [ 5, %27 ]
  %33 = getelementptr inbounds %struct.FaSocket, ptr %8, i64 0, i32 2
  store i32 %32, ptr %33, align 4, !tbaa !345
  br label %69

34:                                               ; preds = %24
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %5) #40
  store i32 1, ptr %5, align 4, !tbaa !25
  %35 = call i32 @setsockopt(i32 noundef %25, i32 noundef 1, i32 noundef 2, ptr noundef nonnull %5, i32 noundef 4) #45
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %6) #40
  %36 = load i32, ptr %4, align 4, !tbaa !25
  %37 = trunc i64 %2 to i16
  %38 = call i16 @llvm.bswap.i16(i16 %37)
  %39 = zext i32 %36 to i64
  %40 = shl nuw i64 %39, 32
  %41 = zext i16 %38 to i64
  %42 = shl nuw nsw i64 %41, 16
  %43 = or disjoint i64 %40, %42
  %44 = or disjoint i64 %43, 2
  store i64 %44, ptr %6, align 8
  %45 = getelementptr inbounds { i64, i64 }, ptr %6, i64 0, i32 1
  store i64 0, ptr %45, align 8
  %46 = call i32 @bind(i32 noundef %25, ptr noundef nonnull %6, i32 noundef 16) #45
  %47 = icmp eq i32 %46, 0
  br i1 %47, label %48, label %51

48:                                               ; preds = %34
  %49 = call i32 @listen(i32 noundef %25, i32 noundef 4096) #45
  %50 = icmp eq i32 %49, 0
  br i1 %50, label %59, label %51

51:                                               ; preds = %48, %34
  br i1 icmp ne (ptr @__errno_location, ptr null), label %52, label %55

52:                                               ; preds = %51
  %53 = call ptr @__errno_location() #45
  %54 = load i32, ptr %53, align 4, !tbaa !25
  br label %55

55:                                               ; preds = %51, %52
  %56 = phi i32 [ %54, %52 ], [ 5, %51 ]
  %57 = getelementptr inbounds %struct.FaSocket, ptr %8, i64 0, i32 2
  store i32 %56, ptr %57, align 4, !tbaa !345
  %58 = call i32 @close(i32 noundef %25) #45
  br label %68

59:                                               ; preds = %48
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %7) #40
  store i32 16, ptr %7, align 4, !tbaa !25
  %60 = call i32 @getsockname(i32 noundef %25, ptr noundef nonnull %6, ptr noundef nonnull %7) #45
  %61 = icmp eq i32 %60, 0
  %62 = getelementptr inbounds %struct.FaSocketAddress, ptr %6, i64 0, i32 1
  %63 = load i16, ptr %62, align 2
  %64 = call i16 @llvm.bswap.i16(i16 %63)
  %65 = zext i16 %64 to i64
  %66 = select i1 %61, i64 %65, i64 %2
  store i32 %25, ptr %9, align 8, !tbaa !344
  %67 = getelementptr inbounds %struct.FaSocket, ptr %8, i64 0, i32 3
  store i64 %66, ptr %67, align 8, !tbaa !346
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %7) #40
  br label %68

68:                                               ; preds = %59, %55
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %6) #40
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %5) #40
  br label %69

69:                                               ; preds = %31, %68, %22, %18, %11
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %4) #40
  ret ptr %8
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define internal fastcc noundef i32 @parse_host(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef writeonly %2) unnamed_addr #10 {
  %4 = alloca [4 x i32], align 16
  %5 = icmp eq i64 %1, 9
  br i1 %5, label %8, label %20

6:                                                ; preds = %8
  %7 = icmp eq i32 %16, 0
  br i1 %7, label %20, label %19

8:                                                ; preds = %3, %8
  %9 = phi i64 [ %17, %8 ], [ 0, %3 ]
  %10 = phi i32 [ %16, %8 ], [ 1, %3 ]
  %11 = getelementptr inbounds i8, ptr %0, i64 %9
  %12 = load i8, ptr %11, align 1, !tbaa !2
  %13 = getelementptr inbounds [10 x i8], ptr @parse_host.local, i64 0, i64 %9
  %14 = load i8, ptr %13, align 1, !tbaa !2
  %15 = icmp eq i8 %12, %14
  %16 = select i1 %15, i32 %10, i32 0
  %17 = add nuw nsw i64 %9, 1
  %18 = icmp eq i64 %17, 9
  br i1 %18, label %6, label %8, !llvm.loop !347

19:                                               ; preds = %6
  store i32 16777343, ptr %2, align 4, !tbaa !25
  br label %87

20:                                               ; preds = %6, %3
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %4) #40
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 16 dereferenceable(16) %4, i8 0, i64 16, i1 false)
  %21 = icmp eq i64 %1, 0
  br i1 %21, label %64, label %22

22:                                               ; preds = %20, %48
  %23 = phi i64 [ %51, %48 ], [ 0, %20 ]
  %24 = phi i64 [ %50, %48 ], [ 0, %20 ]
  %25 = phi i64 [ %49, %48 ], [ 0, %20 ]
  %26 = getelementptr inbounds i8, ptr %0, i64 %23
  %27 = load i8, ptr %26, align 1, !tbaa !2
  %28 = zext i8 %27 to i32
  %29 = icmp eq i8 %27, 46
  br i1 %29, label %30, label %36

30:                                               ; preds = %22
  %31 = icmp eq i64 %24, 0
  %32 = icmp eq i64 %25, 3
  %33 = select i1 %31, i1 true, i1 %32
  br i1 %33, label %85, label %34

34:                                               ; preds = %30
  %35 = add i64 %25, 1
  br label %48

36:                                               ; preds = %22
  %37 = add i8 %27, -48
  %38 = icmp ult i8 %37, 10
  %39 = icmp ult i64 %24, 3
  %40 = select i1 %38, i1 %39, i1 false
  br i1 %40, label %41, label %85

41:                                               ; preds = %36
  %42 = getelementptr inbounds [4 x i32], ptr %4, i64 0, i64 %25
  %43 = load i32, ptr %42, align 4, !tbaa !25
  %44 = mul i32 %43, 10
  %45 = add nsw i32 %28, -48
  %46 = add i32 %45, %44
  store i32 %46, ptr %42, align 4, !tbaa !25
  %47 = add nuw nsw i64 %24, 1
  br label %48

48:                                               ; preds = %41, %34
  %49 = phi i64 [ %25, %41 ], [ %35, %34 ]
  %50 = phi i64 [ %47, %41 ], [ 0, %34 ]
  %51 = add nuw i64 %23, 1
  %52 = icmp eq i64 %51, %1
  br i1 %52, label %53, label %22, !llvm.loop !348

53:                                               ; preds = %48
  %54 = load i32, ptr %4, align 16
  %55 = getelementptr inbounds [4 x i32], ptr %4, i64 0, i64 1
  %56 = load i32, ptr %55, align 4
  %57 = getelementptr inbounds [4 x i32], ptr %4, i64 0, i64 2
  %58 = load i32, ptr %57, align 8
  %59 = getelementptr inbounds [4 x i32], ptr %4, i64 0, i64 3
  %60 = load i32, ptr %59, align 4
  %61 = icmp ne i64 %49, 3
  %62 = icmp eq i64 %50, 0
  %63 = select i1 %61, i1 true, i1 %62
  br label %64

64:                                               ; preds = %53, %20
  %65 = phi i32 [ 0, %20 ], [ %60, %53 ]
  %66 = phi i32 [ 0, %20 ], [ %58, %53 ]
  %67 = phi i32 [ 0, %20 ], [ %56, %53 ]
  %68 = phi i32 [ 0, %20 ], [ %54, %53 ]
  %69 = phi i1 [ true, %20 ], [ %63, %53 ]
  %70 = icmp ugt i32 %68, 255
  %71 = select i1 %69, i1 true, i1 %70
  %72 = icmp ugt i32 %67, 255
  %73 = select i1 %71, i1 true, i1 %72
  %74 = icmp ugt i32 %66, 255
  %75 = select i1 %73, i1 true, i1 %74
  %76 = icmp ugt i32 %65, 255
  %77 = select i1 %75, i1 true, i1 %76
  br i1 %77, label %85, label %78

78:                                               ; preds = %64
  %79 = shl nuw nsw i32 %67, 8
  %80 = shl nuw nsw i32 %66, 16
  %81 = shl nuw i32 %65, 24
  %82 = or i32 %79, %80
  %83 = or i32 %82, %81
  %84 = or i32 %83, %68
  store i32 %84, ptr %2, align 4, !tbaa !25
  br label %85

85:                                               ; preds = %36, %30, %64, %78
  %86 = phi i32 [ 1, %78 ], [ 0, %64 ], [ 0, %30 ], [ 0, %36 ]
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %4) #40
  br label %87

87:                                               ; preds = %19, %85
  %88 = phi i32 [ 1, %19 ], [ %86, %85 ]
  ret i32 %88
}

; Function Attrs: optsize
declare extern_weak i32 @socket(i32 noundef, i32 noundef, i32 noundef) #3

; Function Attrs: optsize
declare extern_weak i32 @setsockopt(i32 noundef, i32 noundef, i32 noundef, ptr noundef, i32 noundef) local_unnamed_addr #3

; Function Attrs: optsize
declare extern_weak i32 @bind(i32 noundef, ptr noundef, i32 noundef) local_unnamed_addr #3

; Function Attrs: optsize
declare extern_weak i32 @listen(i32 noundef, i32 noundef) local_unnamed_addr #3

; Function Attrs: optsize
declare extern_weak i32 @getsockname(i32 noundef, ptr noundef, ptr noundef) local_unnamed_addr #3

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read)
define i64 @fa_tcp_port(ptr nocapture noundef readonly %0) local_unnamed_addr #29 {
  %2 = getelementptr inbounds %struct.FaSocket, ptr %0, i64 0, i32 3
  %3 = load i64, ptr %2, align 8, !tbaa !346
  ret i64 %3
}

; Function Attrs: nounwind optsize
define void @fa_tcp_cancel(ptr noundef readonly %0, ptr noundef readnone %1, ptr noundef %2) local_unnamed_addr #2 {
  %4 = icmp eq ptr %0, null
  br i1 %4, label %29, label %5

5:                                                ; preds = %3
  %6 = getelementptr inbounds %struct.FaSocket, ptr %0, i64 0, i32 4
  %7 = load ptr, ptr %6, align 8, !tbaa !342
  %8 = icmp eq ptr %7, null
  br i1 %8, label %29, label %9

9:                                                ; preds = %5
  %10 = getelementptr inbounds %struct.FaIo, ptr %7, i64 0, i32 2
  %11 = load ptr, ptr %10, align 8, !tbaa !312
  %12 = icmp eq ptr %11, %1
  br i1 %12, label %13, label %19

13:                                               ; preds = %9
  store ptr null, ptr %10, align 8, !tbaa !312
  %14 = getelementptr inbounds %struct.FaIo, ptr %7, i64 0, i32 4
  %15 = load ptr, ptr %14, align 8, !tbaa !305
  %16 = getelementptr inbounds %struct.FaExecutor, ptr %15, i64 0, i32 13
  %17 = load i64, ptr %16, align 8, !tbaa !311
  %18 = add nsw i64 %17, -1
  store i64 %18, ptr %16, align 8, !tbaa !311
  br label %19

19:                                               ; preds = %13, %9
  %20 = getelementptr inbounds %struct.FaIo, ptr %7, i64 0, i32 3
  %21 = load ptr, ptr %20, align 8, !tbaa !313
  %22 = icmp eq ptr %21, %1
  br i1 %22, label %23, label %29

23:                                               ; preds = %19
  store ptr null, ptr %20, align 8, !tbaa !313
  %24 = getelementptr inbounds %struct.FaIo, ptr %7, i64 0, i32 4
  %25 = load ptr, ptr %24, align 8, !tbaa !305
  %26 = getelementptr inbounds %struct.FaExecutor, ptr %25, i64 0, i32 13
  %27 = load i64, ptr %26, align 8, !tbaa !311
  %28 = add nsw i64 %27, -1
  store i64 %28, ptr %26, align 8, !tbaa !311
  br label %29

29:                                               ; preds = %23, %19, %5, %3
  %30 = icmp eq ptr %2, null
  br i1 %30, label %60, label %31

31:                                               ; preds = %29
  %32 = load ptr, ptr %2, align 8, !tbaa !10
  %33 = icmp eq ptr %32, null
  br i1 %33, label %60, label %34

34:                                               ; preds = %31
  %35 = getelementptr inbounds %struct.FaSocket, ptr %32, i64 0, i32 4
  %36 = load ptr, ptr %35, align 8, !tbaa !342
  %37 = icmp eq ptr %36, null
  br i1 %37, label %58, label %38

38:                                               ; preds = %34
  %39 = getelementptr inbounds %struct.FaIo, ptr %36, i64 0, i32 2
  %40 = load ptr, ptr %39, align 8, !tbaa !312
  %41 = icmp eq ptr %40, %1
  br i1 %41, label %42, label %48

42:                                               ; preds = %38
  store ptr null, ptr %39, align 8, !tbaa !312
  %43 = getelementptr inbounds %struct.FaIo, ptr %36, i64 0, i32 4
  %44 = load ptr, ptr %43, align 8, !tbaa !305
  %45 = getelementptr inbounds %struct.FaExecutor, ptr %44, i64 0, i32 13
  %46 = load i64, ptr %45, align 8, !tbaa !311
  %47 = add nsw i64 %46, -1
  store i64 %47, ptr %45, align 8, !tbaa !311
  br label %48

48:                                               ; preds = %42, %38
  %49 = getelementptr inbounds %struct.FaIo, ptr %36, i64 0, i32 3
  %50 = load ptr, ptr %49, align 8, !tbaa !313
  %51 = icmp eq ptr %50, %1
  br i1 %51, label %52, label %58

52:                                               ; preds = %48
  store ptr null, ptr %49, align 8, !tbaa !313
  %53 = getelementptr inbounds %struct.FaIo, ptr %36, i64 0, i32 4
  %54 = load ptr, ptr %53, align 8, !tbaa !305
  %55 = getelementptr inbounds %struct.FaExecutor, ptr %54, i64 0, i32 13
  %56 = load i64, ptr %55, align 8, !tbaa !311
  %57 = add nsw i64 %56, -1
  store i64 %57, ptr %55, align 8, !tbaa !311
  br label %58

58:                                               ; preds = %52, %48, %34
  %59 = load ptr, ptr %2, align 8, !tbaa !10
  tail call void @fa_shared_release(ptr noundef %59) #44
  store ptr null, ptr %2, align 8, !tbaa !10
  br label %60

60:                                               ; preds = %58, %31, %29
  ret void
}

; Function Attrs: nounwind optsize
define ptr @fa_tcp_accept(ptr nocapture noundef readonly %0, ptr nocapture noundef writeonly %1) local_unnamed_addr #2 {
  %3 = alloca %struct.FaPollFd, align 4
  %4 = alloca ptr, align 8
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %4) #40
  store ptr null, ptr %4, align 8, !tbaa !10
  %5 = call fastcc i32 @try_accept(ptr noundef %0, ptr noundef %1, ptr noundef nonnull %4) #44
  %6 = icmp eq i32 %5, 1
  br i1 %6, label %7, label %16

7:                                                ; preds = %2
  %8 = getelementptr inbounds %struct.FaSocket, ptr %0, i64 0, i32 1
  %9 = getelementptr inbounds %struct.FaPollFd, ptr %3, i64 0, i32 1
  %10 = getelementptr inbounds %struct.FaPollFd, ptr %3, i64 0, i32 2
  br label %11

11:                                               ; preds = %7, %11
  %12 = load i32, ptr %8, align 8, !tbaa !344
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %3) #40
  store i32 %12, ptr %3, align 4, !tbaa !349
  store i16 1, ptr %9, align 4, !tbaa !352
  store i16 0, ptr %10, align 2, !tbaa !353
  %13 = call i32 @poll(ptr noundef nonnull %3, i64 noundef 1, i32 noundef -1) #45
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %3) #40
  %14 = call fastcc i32 @try_accept(ptr noundef %0, ptr noundef %1, ptr noundef nonnull %4) #44
  %15 = icmp eq i32 %14, 1
  br i1 %15, label %11, label %16, !llvm.loop !354

16:                                               ; preds = %11, %2
  %17 = load ptr, ptr %4, align 8, !tbaa !10
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %4) #40
  ret ptr %17
}

; Function Attrs: nounwind optsize
define internal fastcc noundef i32 @try_accept(ptr nocapture noundef readonly %0, ptr nocapture noundef writeonly %1, ptr nocapture noundef writeonly %2) unnamed_addr #2 {
  %4 = alloca i32, align 4
  %5 = getelementptr inbounds %struct.FaSocket, ptr %0, i64 0, i32 1
  %6 = load i32, ptr %5, align 8, !tbaa !344
  %7 = icmp slt i32 %6, 0
  br i1 %7, label %11, label %8

8:                                                ; preds = %3
  %9 = tail call i32 @accept4(i32 noundef %6, ptr noundef null, ptr noundef null, i32 noundef 526336) #45
  %10 = icmp sgt i32 %9, -1
  br i1 %10, label %17, label %22

11:                                               ; preds = %3
  %12 = getelementptr inbounds %struct.FaSocket, ptr %0, i64 0, i32 2
  %13 = load i32, ptr %12, align 4, !tbaa !345
  %14 = icmp eq i32 %13, 0
  %15 = select i1 %14, i32 9, i32 %13
  %16 = tail call fastcc ptr @net_error(i32 noundef %15, ptr noundef nonnull @.str.54) #44
  store ptr %16, ptr %2, align 8, !tbaa !10
  br label %33

17:                                               ; preds = %29, %8
  %18 = phi i32 [ %9, %8 ], [ %31, %29 ]
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %4) #40
  store i32 1, ptr %4, align 4, !tbaa !25
  %19 = call i32 @setsockopt(i32 noundef %18, i32 noundef 6, i32 noundef 1, ptr noundef nonnull %4, i32 noundef 4) #45
  %20 = call fastcc ptr @shared_new(i64 noundef 56, ptr noundef nonnull @socket_drop) #44
  %21 = getelementptr inbounds %struct.FaSocket, ptr %20, i64 0, i32 1
  store i32 %18, ptr %21, align 8, !tbaa !344
  store ptr %20, ptr %1, align 8, !tbaa !10
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %4) #40
  br label %33

22:                                               ; preds = %8, %29
  br i1 icmp ne (ptr @__errno_location, ptr null), label %23, label %26

23:                                               ; preds = %22
  %24 = tail call ptr @__errno_location() #45
  %25 = load i32, ptr %24, align 4, !tbaa !25
  switch i32 %25, label %26 [
    i32 103, label %29
    i32 4, label %29
    i32 11, label %33
  ]

26:                                               ; preds = %22, %23
  %27 = phi i32 [ %25, %23 ], [ 5, %22 ]
  %28 = tail call fastcc ptr @net_error(i32 noundef %27, ptr noundef nonnull @.str.55) #44
  store ptr %28, ptr %2, align 8, !tbaa !10
  br label %33

29:                                               ; preds = %23, %23
  %30 = load i32, ptr %5, align 8, !tbaa !344
  %31 = tail call i32 @accept4(i32 noundef %30, ptr noundef null, ptr noundef null, i32 noundef 526336) #45
  %32 = icmp sgt i32 %31, -1
  br i1 %32, label %17, label %22

33:                                               ; preds = %23, %26, %17, %11
  %34 = phi i32 [ 2, %11 ], [ 2, %26 ], [ 0, %17 ], [ 1, %23 ]
  ret i32 %34
}

; Function Attrs: nounwind optsize
define noundef i32 @fa_tcp_accept_async(ptr nocapture noundef %0, ptr noundef %1, ptr noundef %2, ptr nocapture noundef writeonly %3, ptr nocapture noundef writeonly %4) local_unnamed_addr #2 {
  br label %6

6:                                                ; preds = %10, %5
  %7 = tail call fastcc i32 @try_accept(ptr noundef %0, ptr noundef %3, ptr noundef %4) #44
  switch i32 %7, label %10 [
    i32 0, label %8
    i32 2, label %13
  ]

8:                                                ; preds = %6
  %9 = tail call fastcc i32 @io_done(ptr noundef %2) #44, !range !355
  br label %13

10:                                               ; preds = %6
  %11 = tail call fastcc i32 @io_wait(ptr noundef %0, i32 noundef 0, ptr noundef %1, ptr noundef %2) #44, !range !60
  %12 = icmp eq i32 %11, 0
  br i1 %12, label %6, label %13

13:                                               ; preds = %6, %10, %8
  %14 = phi i32 [ %9, %8 ], [ 0, %6 ], [ 1, %10 ]
  ret i32 %14
}

; Function Attrs: nounwind optsize
define internal fastcc noundef i32 @io_done(ptr noundef %0) unnamed_addr #2 {
  %2 = tail call align 4 ptr @llvm.threadlocal.address.p0(ptr align 4 @fa_coop_budget)
  %3 = load i32, ptr %2, align 4, !tbaa !25
  %4 = add nsw i32 %3, -1
  store i32 %4, ptr %2, align 4, !tbaa !25
  %5 = icmp sgt i32 %3, 1
  br i1 %5, label %30, label %6

6:                                                ; preds = %1
  %7 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @current_executor)
  %8 = load ptr, ptr %7, align 8, !tbaa !10
  %9 = icmp eq ptr %8, null
  br i1 %9, label %12, label %10

10:                                               ; preds = %6
  %11 = getelementptr inbounds %struct.FaExecutor, ptr %8, i64 0, i32 23
  store i32 1, ptr %11, align 8, !tbaa !284
  br label %12

12:                                               ; preds = %10, %6
  %13 = tail call fastcc ptr @executor() #44
  %14 = getelementptr inbounds i8, ptr %0, i64 16
  %15 = getelementptr inbounds i8, ptr %0, i64 44
  %16 = load i32, ptr %15, align 4, !tbaa !267
  %17 = or i32 %16, 4
  store i32 %17, ptr %15, align 4, !tbaa !267
  %18 = getelementptr inbounds %struct.FaExecutor, ptr %13, i64 0, i32 2
  %19 = getelementptr inbounds %struct.FaExecutor, ptr %13, i64 0, i32 3
  %20 = getelementptr inbounds i8, ptr %0, i64 24
  store ptr null, ptr %20, align 8, !tbaa !269
  %21 = load ptr, ptr %19, align 8, !tbaa !10
  %22 = getelementptr inbounds i8, ptr %0, i64 32
  store ptr %21, ptr %22, align 8, !tbaa !270
  %23 = icmp eq ptr %21, null
  %24 = getelementptr inbounds %struct.FaTask, ptr %21, i64 0, i32 1
  %25 = select i1 %23, ptr %18, ptr %24
  store ptr %14, ptr %25, align 8, !tbaa !10
  store ptr %14, ptr %19, align 8, !tbaa !10
  %26 = getelementptr inbounds %struct.FaExecutor, ptr %13, i64 0, i32 4
  %27 = load i64, ptr %26, align 8, !tbaa !283
  %28 = add nsw i64 %27, 1
  store i64 %28, ptr %26, align 8, !tbaa !283
  %29 = getelementptr inbounds i8, ptr %0, i64 40
  store atomic i32 1, ptr %29 release, align 8
  br label %30

30:                                               ; preds = %1, %12
  %31 = phi i32 [ 2, %12 ], [ 0, %1 ]
  ret i32 %31
}

; Function Attrs: nounwind optsize
define internal fastcc noundef i32 @io_wait(ptr nocapture noundef %0, i32 noundef %1, ptr noundef %2, ptr noundef %3) unnamed_addr #2 {
  %5 = alloca %struct.FaPollFd, align 4
  %6 = tail call fastcc ptr @executor() #44
  %7 = getelementptr inbounds %struct.FaSocket, ptr %0, i64 0, i32 4
  %8 = load ptr, ptr %7, align 8, !tbaa !342
  %9 = icmp eq ptr %8, null
  br i1 %9, label %15, label %10

10:                                               ; preds = %4
  %11 = getelementptr inbounds %struct.FaIo, ptr %8, i64 0, i32 4
  %12 = load ptr, ptr %11, align 8, !tbaa !305
  %13 = icmp eq ptr %12, %6
  br i1 %13, label %27, label %14

14:                                               ; preds = %10
  tail call void @fa_free(ptr noundef nonnull %8, i64 noundef 32, i64 noundef 16) #45
  br label %15

15:                                               ; preds = %4, %14
  %16 = getelementptr inbounds %struct.FaSocket, ptr %0, i64 0, i32 1
  %17 = load i32, ptr %16, align 8, !tbaa !344
  %18 = tail call ptr @fa_io_register(i32 noundef %17) #44
  store ptr %18, ptr %7, align 8, !tbaa !342
  %19 = icmp eq ptr %18, null
  br i1 %19, label %20, label %27

20:                                               ; preds = %15
  %21 = load i32, ptr %16, align 8, !tbaa !344
  %22 = icmp eq i32 %1, 0
  %23 = select i1 %22, i16 1, i16 4
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %5) #40
  store i32 %21, ptr %5, align 4, !tbaa !349
  %24 = getelementptr inbounds %struct.FaPollFd, ptr %5, i64 0, i32 1
  store i16 %23, ptr %24, align 4, !tbaa !352
  %25 = getelementptr inbounds %struct.FaPollFd, ptr %5, i64 0, i32 2
  store i16 0, ptr %25, align 2, !tbaa !353
  %26 = call i32 @poll(ptr noundef nonnull %5, i64 noundef 1, i32 noundef -1) #45
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %5) #40
  br label %72

27:                                               ; preds = %10, %15
  %28 = phi ptr [ %18, %15 ], [ %8, %10 ]
  %29 = icmp eq i32 %1, 0
  %30 = getelementptr inbounds %struct.FaIo, ptr %28, i64 0, i32 3
  %31 = getelementptr inbounds %struct.FaIo, ptr %28, i64 0, i32 2
  %32 = select i1 %29, ptr %31, ptr %30
  %33 = load ptr, ptr %32, align 8, !tbaa !10
  %34 = icmp eq ptr %33, null
  %35 = icmp eq ptr %33, %2
  %36 = or i1 %34, %35
  br i1 %36, label %57, label %37

37:                                               ; preds = %27
  store ptr %3, ptr %2, align 8, !tbaa !286
  %38 = getelementptr inbounds %struct.FaIo, ptr %28, i64 0, i32 4
  %39 = load ptr, ptr %38, align 8, !tbaa !305
  %40 = getelementptr inbounds %struct.FaWait, ptr %2, i64 0, i32 1
  store ptr %39, ptr %40, align 8, !tbaa !288
  %41 = getelementptr inbounds i8, ptr %3, i64 16
  %42 = getelementptr inbounds i8, ptr %3, i64 44
  %43 = load i32, ptr %42, align 4, !tbaa !267
  %44 = or i32 %43, 4
  store i32 %44, ptr %42, align 4, !tbaa !267
  %45 = getelementptr inbounds %struct.FaExecutor, ptr %39, i64 0, i32 2
  %46 = getelementptr inbounds %struct.FaExecutor, ptr %39, i64 0, i32 3
  %47 = getelementptr inbounds i8, ptr %3, i64 24
  store ptr null, ptr %47, align 8, !tbaa !269
  %48 = load ptr, ptr %46, align 8, !tbaa !10
  %49 = getelementptr inbounds i8, ptr %3, i64 32
  store ptr %48, ptr %49, align 8, !tbaa !270
  %50 = icmp eq ptr %48, null
  %51 = getelementptr inbounds %struct.FaTask, ptr %48, i64 0, i32 1
  %52 = select i1 %50, ptr %45, ptr %51
  store ptr %41, ptr %52, align 8, !tbaa !10
  store ptr %41, ptr %46, align 8, !tbaa !10
  %53 = getelementptr inbounds %struct.FaExecutor, ptr %39, i64 0, i32 4
  %54 = load i64, ptr %53, align 8, !tbaa !283
  %55 = add nsw i64 %54, 1
  store i64 %55, ptr %53, align 8, !tbaa !283
  %56 = getelementptr inbounds i8, ptr %3, i64 40
  store atomic i32 1, ptr %56 release, align 8
  br label %72

57:                                               ; preds = %27
  %58 = select i1 %29, i32 -2, i32 -5
  %59 = getelementptr inbounds %struct.FaIo, ptr %28, i64 0, i32 1
  %60 = load i32, ptr %59, align 4, !tbaa !304
  %61 = and i32 %60, %58
  store i32 %61, ptr %59, align 4, !tbaa !304
  store ptr %3, ptr %2, align 8, !tbaa !286
  %62 = getelementptr inbounds %struct.FaIo, ptr %28, i64 0, i32 4
  %63 = load ptr, ptr %62, align 8, !tbaa !305
  %64 = getelementptr inbounds %struct.FaWait, ptr %2, i64 0, i32 1
  store ptr %63, ptr %64, align 8, !tbaa !288
  %65 = getelementptr inbounds %struct.FaWait, ptr %2, i64 0, i32 9
  store i32 0, ptr %65, align 8, !tbaa !309
  %66 = getelementptr inbounds %struct.FaWait, ptr %2, i64 0, i32 10
  store i32 0, ptr %66, align 4, !tbaa !310
  %67 = getelementptr inbounds %struct.FaWait, ptr %2, i64 0, i32 8
  store i64 -1, ptr %67, align 8, !tbaa !289
  store ptr %2, ptr %32, align 8, !tbaa !10
  %68 = getelementptr inbounds %struct.FaExecutor, ptr %63, i64 0, i32 13
  %69 = load i64, ptr %68, align 8, !tbaa !311
  %70 = add nsw i64 %69, 1
  store i64 %70, ptr %68, align 8, !tbaa !311
  %71 = getelementptr inbounds i8, ptr %3, i64 40
  store atomic i32 2, ptr %71 release, align 8
  br label %72

72:                                               ; preds = %37, %57, %20
  %73 = phi i32 [ 0, %20 ], [ 1, %57 ], [ 1, %37 ]
  ret i32 %73
}

; Function Attrs: nounwind optsize
define ptr @fa_tcp_connect(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2, ptr nocapture noundef %3) local_unnamed_addr #2 {
  %5 = alloca %struct.FaPollFd, align 4
  %6 = alloca ptr, align 8
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %6) #40
  store ptr null, ptr %6, align 8, !tbaa !10
  store ptr null, ptr %3, align 8, !tbaa !10
  %7 = call fastcc i32 @connect_start(ptr noundef %0, i64 noundef %1, i64 noundef %2, ptr noundef nonnull %3, ptr noundef nonnull %6) #44, !range !355
  %8 = icmp eq i32 %7, 1
  br i1 %8, label %9, label %22

9:                                                ; preds = %4
  %10 = call fastcc i32 @connect_check(ptr noundef nonnull %3, ptr noundef nonnull %6) #44, !range !355
  %11 = icmp eq i32 %10, 1
  br i1 %11, label %12, label %22

12:                                               ; preds = %9
  %13 = getelementptr inbounds %struct.FaPollFd, ptr %5, i64 0, i32 1
  %14 = getelementptr inbounds %struct.FaPollFd, ptr %5, i64 0, i32 2
  br label %15

15:                                               ; preds = %12, %15
  %16 = load ptr, ptr %3, align 8, !tbaa !10
  %17 = getelementptr inbounds %struct.FaSocket, ptr %16, i64 0, i32 1
  %18 = load i32, ptr %17, align 8, !tbaa !344
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %5) #40
  store i32 %18, ptr %5, align 4, !tbaa !349
  store i16 4, ptr %13, align 4, !tbaa !352
  store i16 0, ptr %14, align 2, !tbaa !353
  %19 = call i32 @poll(ptr noundef nonnull %5, i64 noundef 1, i32 noundef -1) #45
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %5) #40
  %20 = call fastcc i32 @connect_check(ptr noundef nonnull %3, ptr noundef nonnull %6) #44, !range !355
  %21 = icmp eq i32 %20, 1
  br i1 %21, label %15, label %22, !llvm.loop !356

22:                                               ; preds = %15, %9, %4
  %23 = load ptr, ptr %6, align 8, !tbaa !10
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %6) #40
  ret ptr %23
}

; Function Attrs: nounwind optsize
define internal fastcc noundef i32 @connect_start(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2, ptr nocapture noundef %3, ptr nocapture noundef writeonly %4) unnamed_addr #2 {
  %6 = alloca i32, align 4
  %7 = alloca i32, align 4
  %8 = alloca %struct.FaSocketAddress, align 8
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %6) #40
  %9 = select i1 icmp ne (ptr @socket, ptr null), i1 icmp eq (ptr @poll, ptr null), i1 true
  br i1 %9, label %10, label %12

10:                                               ; preds = %5
  %11 = tail call fastcc ptr @net_error(i32 noundef 38, ptr noundef nonnull @.str.61) #44
  store ptr %11, ptr %4, align 8, !tbaa !10
  br label %71

12:                                               ; preds = %5
  %13 = call fastcc i32 @parse_host(ptr noundef %0, i64 noundef %1, ptr noundef nonnull %6) #44, !range !60
  %14 = icmp eq i32 %13, 0
  br i1 %14, label %15, label %17

15:                                               ; preds = %12
  %16 = tail call fastcc ptr @net_error(i32 noundef -1, ptr noundef nonnull @.str.61) #44
  store ptr %16, ptr %4, align 8, !tbaa !10
  br label %71

17:                                               ; preds = %12
  %18 = icmp ugt i64 %2, 65535
  br i1 %18, label %19, label %21

19:                                               ; preds = %17
  %20 = tail call fastcc ptr @net_error(i32 noundef 22, ptr noundef nonnull @.str.61) #44
  store ptr %20, ptr %4, align 8, !tbaa !10
  br label %71

21:                                               ; preds = %17
  %22 = tail call i32 @socket(i32 noundef 2, i32 noundef 526337, i32 noundef 0) #45
  %23 = icmp slt i32 %22, 0
  br i1 %23, label %24, label %31

24:                                               ; preds = %21
  br i1 icmp ne (ptr @__errno_location, ptr null), label %25, label %28

25:                                               ; preds = %24
  %26 = tail call ptr @__errno_location() #45
  %27 = load i32, ptr %26, align 4, !tbaa !25
  br label %28

28:                                               ; preds = %24, %25
  %29 = phi i32 [ %27, %25 ], [ 5, %24 ]
  %30 = tail call fastcc ptr @net_error(i32 noundef %29, ptr noundef nonnull @.str.61) #44
  store ptr %30, ptr %4, align 8, !tbaa !10
  br label %71

31:                                               ; preds = %21
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %7) #40
  store i32 1, ptr %7, align 4, !tbaa !25
  %32 = call i32 @setsockopt(i32 noundef %22, i32 noundef 6, i32 noundef 1, ptr noundef nonnull %7, i32 noundef 4) #45
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %8) #40
  %33 = load i32, ptr %6, align 4, !tbaa !25
  %34 = trunc i64 %2 to i16
  %35 = call i16 @llvm.bswap.i16(i16 %34)
  %36 = zext i32 %33 to i64
  %37 = shl nuw i64 %36, 32
  %38 = zext i16 %35 to i64
  %39 = shl nuw nsw i64 %38, 16
  %40 = or disjoint i64 %37, %39
  %41 = or disjoint i64 %40, 2
  store i64 %41, ptr %8, align 8
  %42 = getelementptr inbounds { i64, i64 }, ptr %8, i64 0, i32 1
  store i64 0, ptr %42, align 8
  br label %43

43:                                               ; preds = %47, %31
  %44 = call i32 @connect(i32 noundef %22, ptr noundef nonnull %8, i32 noundef 16) #45
  %45 = icmp eq i32 %44, 0
  br i1 %45, label %51, label %46

46:                                               ; preds = %43
  br i1 icmp ne (ptr @__errno_location, ptr null), label %47, label %62

47:                                               ; preds = %46
  %48 = call ptr @__errno_location() #45
  %49 = load i32, ptr %48, align 4, !tbaa !25
  %50 = icmp eq i32 %49, 4
  br i1 %50, label %43, label %54, !llvm.loop !357

51:                                               ; preds = %43
  %52 = call fastcc ptr @shared_new(i64 noundef 56, ptr noundef nonnull @socket_drop) #44
  %53 = getelementptr inbounds %struct.FaSocket, ptr %52, i64 0, i32 1
  store i32 %22, ptr %53, align 8, !tbaa !344
  store ptr %52, ptr %3, align 8, !tbaa !10
  br label %69

54:                                               ; preds = %47
  %55 = call fastcc ptr @shared_new(i64 noundef 56, ptr noundef nonnull @socket_drop) #44
  %56 = getelementptr inbounds %struct.FaSocket, ptr %55, i64 0, i32 1
  store i32 %22, ptr %56, align 8, !tbaa !344
  store ptr %55, ptr %3, align 8, !tbaa !10
  br i1 icmp ne (ptr @__errno_location, ptr null), label %57, label %65

57:                                               ; preds = %54
  %58 = call ptr @__errno_location() #45
  %59 = load i32, ptr %58, align 4, !tbaa !25
  switch i32 %59, label %60 [
    i32 115, label %69
    i32 11, label %69
  ]

60:                                               ; preds = %57
  %61 = load ptr, ptr %3, align 8, !tbaa !10
  br label %65

62:                                               ; preds = %46
  %63 = call fastcc ptr @shared_new(i64 noundef 56, ptr noundef nonnull @socket_drop) #44
  %64 = getelementptr inbounds %struct.FaSocket, ptr %63, i64 0, i32 1
  store i32 %22, ptr %64, align 8, !tbaa !344
  store ptr %63, ptr %3, align 8, !tbaa !10
  br label %65

65:                                               ; preds = %60, %62, %54
  %66 = phi ptr [ %61, %60 ], [ %55, %54 ], [ %63, %62 ]
  %67 = phi i32 [ %59, %60 ], [ 5, %54 ], [ 5, %62 ]
  call void @fa_shared_release(ptr noundef %66) #44
  store ptr null, ptr %3, align 8, !tbaa !10
  %68 = call fastcc ptr @net_error(i32 noundef %67, ptr noundef nonnull @.str.61) #44
  store ptr %68, ptr %4, align 8, !tbaa !10
  br label %69

69:                                               ; preds = %51, %65, %57, %57
  %70 = phi i32 [ 0, %51 ], [ 2, %65 ], [ 1, %57 ], [ 1, %57 ]
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %8) #40
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %7) #40
  br label %71

71:                                               ; preds = %28, %69, %19, %15, %10
  %72 = phi i32 [ 2, %19 ], [ 2, %15 ], [ 2, %10 ], [ 2, %28 ], [ %70, %69 ]
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %6) #40
  ret i32 %72
}

; Function Attrs: nounwind optsize
define internal fastcc noundef i32 @connect_check(ptr nocapture noundef %0, ptr nocapture noundef writeonly %1) unnamed_addr #2 {
  %3 = alloca %struct.FaPollFd, align 4
  %4 = alloca i32, align 4
  %5 = alloca i32, align 4
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %3) #40
  %6 = load ptr, ptr %0, align 8, !tbaa !10
  %7 = getelementptr inbounds %struct.FaSocket, ptr %6, i64 0, i32 1
  %8 = load i32, ptr %7, align 8, !tbaa !344
  store i32 %8, ptr %3, align 4, !tbaa !349
  %9 = getelementptr inbounds %struct.FaPollFd, ptr %3, i64 0, i32 1
  store i16 4, ptr %9, align 4, !tbaa !352
  %10 = getelementptr inbounds %struct.FaPollFd, ptr %3, i64 0, i32 2
  store i16 0, ptr %10, align 2, !tbaa !353
  %11 = call i32 @poll(ptr noundef nonnull %3, i64 noundef 1, i32 noundef 0) #45
  %12 = icmp eq i32 %11, 0
  br i1 %12, label %26, label %13

13:                                               ; preds = %2
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %4) #40
  store i32 0, ptr %4, align 4, !tbaa !25
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %5) #40
  store i32 4, ptr %5, align 4, !tbaa !25
  %14 = load ptr, ptr %0, align 8, !tbaa !10
  %15 = getelementptr inbounds %struct.FaSocket, ptr %14, i64 0, i32 1
  %16 = load i32, ptr %15, align 8, !tbaa !344
  %17 = call i32 @getsockopt(i32 noundef %16, i32 noundef 1, i32 noundef 4, ptr noundef nonnull %4, ptr noundef nonnull %5) #45
  %18 = load i32, ptr %4, align 4, !tbaa !25
  %19 = icmp eq i32 %18, 0
  br i1 %19, label %24, label %20

20:                                               ; preds = %13
  %21 = load ptr, ptr %0, align 8, !tbaa !10
  call void @fa_shared_release(ptr noundef %21) #44
  store ptr null, ptr %0, align 8, !tbaa !10
  %22 = load i32, ptr %4, align 4, !tbaa !25
  %23 = call fastcc ptr @net_error(i32 noundef %22, ptr noundef nonnull @.str.61) #44
  store ptr %23, ptr %1, align 8, !tbaa !10
  br label %24

24:                                               ; preds = %13, %20
  %25 = phi i32 [ 2, %20 ], [ 0, %13 ]
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %5) #40
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %4) #40
  br label %26

26:                                               ; preds = %2, %24
  %27 = phi i32 [ %25, %24 ], [ 1, %2 ]
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %3) #40
  ret i32 %27
}

; Function Attrs: nounwind optsize
define noundef i32 @fa_tcp_connect_async(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2, ptr noundef %3, ptr noundef %4, ptr nocapture noundef %5, ptr nocapture noundef writeonly %6) local_unnamed_addr #2 {
  %8 = load ptr, ptr %5, align 8, !tbaa !10
  %9 = icmp eq ptr %8, null
  br i1 %9, label %10, label %12

10:                                               ; preds = %7
  %11 = tail call fastcc i32 @connect_start(ptr noundef %0, i64 noundef %1, i64 noundef %2, ptr noundef nonnull %5, ptr noundef %6) #44, !range !355
  br label %12

12:                                               ; preds = %7, %10
  %13 = phi i32 [ 1, %7 ], [ %11, %10 ]
  br label %14

14:                                               ; preds = %25, %12
  %15 = phi i32 [ %13, %12 ], [ %19, %25 ]
  switch i32 %15, label %18 [
    i32 0, label %16
    i32 2, label %26
  ]

16:                                               ; preds = %14
  %17 = tail call fastcc i32 @io_done(ptr noundef %4) #44, !range !355
  br label %26

18:                                               ; preds = %14
  %19 = tail call fastcc i32 @connect_check(ptr noundef nonnull %5, ptr noundef %6) #44, !range !355
  %20 = icmp eq i32 %19, 1
  br i1 %20, label %21, label %25

21:                                               ; preds = %18
  %22 = load ptr, ptr %5, align 8, !tbaa !10
  %23 = tail call fastcc i32 @io_wait(ptr noundef %22, i32 noundef 1, ptr noundef %3, ptr noundef %4) #44, !range !60
  %24 = icmp eq i32 %23, 0
  br i1 %24, label %25, label %26

25:                                               ; preds = %21, %18
  br label %14

26:                                               ; preds = %21, %14, %16
  %27 = phi i32 [ %17, %16 ], [ 1, %21 ], [ 0, %14 ]
  ret i32 %27
}

; Function Attrs: nounwind optsize
define ptr @fa_tcp_read(ptr nocapture noundef readonly %0, ptr nocapture noundef readonly %1, ptr nocapture noundef writeonly %2) local_unnamed_addr #2 {
  %4 = alloca %struct.FaPollFd, align 4
  %5 = alloca ptr, align 8
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %5) #40
  store ptr null, ptr %5, align 8, !tbaa !10
  %6 = call fastcc i32 @try_read(ptr noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef nonnull %5) #44
  %7 = icmp eq i32 %6, 1
  br i1 %7, label %8, label %17

8:                                                ; preds = %3
  %9 = getelementptr inbounds %struct.FaSocket, ptr %0, i64 0, i32 1
  %10 = getelementptr inbounds %struct.FaPollFd, ptr %4, i64 0, i32 1
  %11 = getelementptr inbounds %struct.FaPollFd, ptr %4, i64 0, i32 2
  br label %12

12:                                               ; preds = %8, %12
  %13 = load i32, ptr %9, align 8, !tbaa !344
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %4) #40
  store i32 %13, ptr %4, align 4, !tbaa !349
  store i16 1, ptr %10, align 4, !tbaa !352
  store i16 0, ptr %11, align 2, !tbaa !353
  %14 = call i32 @poll(ptr noundef nonnull %4, i64 noundef 1, i32 noundef -1) #45
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %4) #40
  %15 = call fastcc i32 @try_read(ptr noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef nonnull %5) #44
  %16 = icmp eq i32 %15, 1
  br i1 %16, label %12, label %17, !llvm.loop !358

17:                                               ; preds = %12, %3
  %18 = load ptr, ptr %5, align 8, !tbaa !10
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %5) #40
  ret ptr %18
}

; Function Attrs: nounwind optsize
define internal fastcc noundef i32 @try_read(ptr nocapture noundef readonly %0, ptr nocapture noundef readonly %1, ptr nocapture noundef writeonly %2, ptr nocapture noundef writeonly %3) unnamed_addr #2 {
  %5 = getelementptr inbounds %struct.FaSocket, ptr %0, i64 0, i32 1
  %6 = load i32, ptr %5, align 8, !tbaa !344
  %7 = icmp slt i32 %6, 0
  br i1 %7, label %10, label %8

8:                                                ; preds = %4
  %9 = getelementptr inbounds %struct.FaArrayHeader, ptr %1, i64 0, i32 1
  br label %12

10:                                               ; preds = %4
  %11 = tail call fastcc ptr @net_error(i32 noundef 9, ptr noundef nonnull @.str.62) #44
  store ptr %11, ptr %3, align 8, !tbaa !10
  br label %26

12:                                               ; preds = %8, %20
  %13 = load i32, ptr %5, align 8, !tbaa !344
  %14 = load ptr, ptr %1, align 8, !tbaa !16
  %15 = load i64, ptr %9, align 8, !tbaa !21
  %16 = tail call i64 @read(i32 noundef %13, ptr noundef %14, i64 noundef %15) #45
  %17 = icmp sgt i64 %16, -1
  br i1 %17, label %18, label %19

18:                                               ; preds = %12
  store i64 %16, ptr %2, align 8, !tbaa !8
  br label %26

19:                                               ; preds = %12
  br i1 icmp ne (ptr @__errno_location, ptr null), label %20, label %23

20:                                               ; preds = %19
  %21 = tail call ptr @__errno_location() #45
  %22 = load i32, ptr %21, align 4, !tbaa !25
  switch i32 %22, label %23 [
    i32 4, label %12
    i32 11, label %26
  ]

23:                                               ; preds = %19, %20
  %24 = phi i32 [ %22, %20 ], [ 5, %19 ]
  %25 = tail call fastcc ptr @net_error(i32 noundef %24, ptr noundef nonnull @.str.62) #44
  store ptr %25, ptr %3, align 8, !tbaa !10
  br label %26

26:                                               ; preds = %20, %23, %18, %10
  %27 = phi i32 [ 2, %10 ], [ 2, %23 ], [ 0, %18 ], [ 1, %20 ]
  ret i32 %27
}

; Function Attrs: nounwind optsize
define noundef i32 @fa_tcp_read_async(ptr nocapture noundef %0, ptr nocapture noundef readonly %1, ptr noundef %2, ptr noundef %3, ptr nocapture noundef writeonly %4, ptr nocapture noundef writeonly %5) local_unnamed_addr #2 {
  br label %7

7:                                                ; preds = %11, %6
  %8 = tail call fastcc i32 @try_read(ptr noundef %0, ptr noundef %1, ptr noundef %4, ptr noundef %5) #44
  switch i32 %8, label %11 [
    i32 0, label %9
    i32 2, label %14
  ]

9:                                                ; preds = %7
  %10 = tail call fastcc i32 @io_done(ptr noundef %3) #44, !range !355
  br label %14

11:                                               ; preds = %7
  %12 = tail call fastcc i32 @io_wait(ptr noundef %0, i32 noundef 0, ptr noundef %2, ptr noundef %3) #44, !range !60
  %13 = icmp eq i32 %12, 0
  br i1 %13, label %7, label %14

14:                                               ; preds = %7, %11, %9
  %15 = phi i32 [ %10, %9 ], [ 0, %7 ], [ 1, %11 ]
  ret i32 %15
}

; Function Attrs: nounwind optsize
define ptr @fa_tcp_write(ptr nocapture noundef readonly %0, ptr nocapture noundef readonly %1, i64 noundef %2) local_unnamed_addr #2 {
  %4 = alloca %struct.FaPollFd, align 4
  %5 = alloca ptr, align 8
  %6 = alloca i64, align 8
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %5) #40
  store ptr null, ptr %5, align 8, !tbaa !10
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %6) #40
  store i64 0, ptr %6, align 8, !tbaa !8
  %7 = call fastcc i32 @try_write(ptr noundef %0, ptr noundef %1, i64 noundef %2, ptr noundef nonnull %6, ptr noundef nonnull %5) #44
  %8 = icmp eq i32 %7, 1
  br i1 %8, label %9, label %18

9:                                                ; preds = %3
  %10 = getelementptr inbounds %struct.FaSocket, ptr %0, i64 0, i32 1
  %11 = getelementptr inbounds %struct.FaPollFd, ptr %4, i64 0, i32 1
  %12 = getelementptr inbounds %struct.FaPollFd, ptr %4, i64 0, i32 2
  br label %13

13:                                               ; preds = %9, %13
  %14 = load i32, ptr %10, align 8, !tbaa !344
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %4) #40
  store i32 %14, ptr %4, align 4, !tbaa !349
  store i16 4, ptr %11, align 4, !tbaa !352
  store i16 0, ptr %12, align 2, !tbaa !353
  %15 = call i32 @poll(ptr noundef nonnull %4, i64 noundef 1, i32 noundef -1) #45
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %4) #40
  %16 = call fastcc i32 @try_write(ptr noundef %0, ptr noundef %1, i64 noundef %2, ptr noundef nonnull %6, ptr noundef nonnull %5) #44
  %17 = icmp eq i32 %16, 1
  br i1 %17, label %13, label %18, !llvm.loop !359

18:                                               ; preds = %13, %3
  %19 = load ptr, ptr %5, align 8, !tbaa !10
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %6) #40
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %5) #40
  ret ptr %19
}

; Function Attrs: nounwind optsize
define internal fastcc noundef i32 @try_write(ptr nocapture noundef readonly %0, ptr nocapture noundef readonly %1, i64 noundef %2, ptr nocapture noundef %3, ptr nocapture noundef writeonly %4) unnamed_addr #2 {
  %6 = getelementptr inbounds %struct.FaSocket, ptr %0, i64 0, i32 1
  %7 = load i32, ptr %6, align 8, !tbaa !344
  %8 = icmp slt i32 %7, 0
  br i1 %8, label %37, label %9

9:                                                ; preds = %5
  %10 = icmp slt i64 %2, 0
  %11 = getelementptr inbounds %struct.FaArrayHeader, ptr %1, i64 0, i32 1
  %12 = load i64, ptr %11, align 8, !tbaa !21
  %13 = tail call i64 @llvm.smin.i64(i64 %12, i64 %2)
  %14 = select i1 %10, i64 %12, i64 %13
  %15 = load i64, ptr %3, align 8, !tbaa !8
  %16 = icmp slt i64 %15, %14
  br i1 %16, label %17, label %40

17:                                               ; preds = %9, %34
  %18 = phi i64 [ %35, %34 ], [ %15, %9 ]
  %19 = load i32, ptr %6, align 8, !tbaa !344
  %20 = load ptr, ptr %1, align 8, !tbaa !16
  %21 = getelementptr inbounds i8, ptr %20, i64 %18
  %22 = sub nsw i64 %14, %18
  %23 = tail call i64 @send(i32 noundef %19, ptr noundef %21, i64 noundef %22, i32 noundef 16384) #45
  %24 = icmp sgt i64 %23, -1
  br i1 %24, label %25, label %28

25:                                               ; preds = %17
  %26 = load i64, ptr %3, align 8, !tbaa !8
  %27 = add nsw i64 %26, %23
  store i64 %27, ptr %3, align 8, !tbaa !8
  br label %34, !llvm.loop !360

28:                                               ; preds = %17
  br i1 icmp ne (ptr @__errno_location, ptr null), label %29, label %37

29:                                               ; preds = %28
  %30 = tail call ptr @__errno_location() #45
  %31 = load i32, ptr %30, align 4, !tbaa !25
  switch i32 %31, label %37 [
    i32 4, label %32
    i32 11, label %40
  ]

32:                                               ; preds = %29
  %33 = load i64, ptr %3, align 8, !tbaa !8
  br label %34

34:                                               ; preds = %32, %25
  %35 = phi i64 [ %33, %32 ], [ %27, %25 ]
  %36 = icmp slt i64 %35, %14
  br i1 %36, label %17, label %40

37:                                               ; preds = %29, %28, %5
  %38 = phi i32 [ 9, %5 ], [ %31, %29 ], [ 5, %28 ]
  %39 = tail call fastcc ptr @net_error(i32 noundef %38, ptr noundef nonnull @.str.63) #44
  store ptr %39, ptr %4, align 8, !tbaa !10
  br label %40

40:                                               ; preds = %34, %29, %37, %9
  %41 = phi i32 [ 0, %9 ], [ 2, %37 ], [ 0, %34 ], [ 1, %29 ]
  ret i32 %41
}

; Function Attrs: nounwind optsize
define noundef i32 @fa_tcp_write_async(ptr nocapture noundef %0, ptr nocapture noundef readonly %1, i64 noundef %2, ptr noundef %3, ptr noundef %4, ptr nocapture noundef %5, ptr nocapture noundef writeonly %6) local_unnamed_addr #2 {
  br label %8

8:                                                ; preds = %12, %7
  %9 = tail call fastcc i32 @try_write(ptr noundef %0, ptr noundef %1, i64 noundef %2, ptr noundef %5, ptr noundef %6) #44
  switch i32 %9, label %12 [
    i32 0, label %10
    i32 2, label %15
  ]

10:                                               ; preds = %8
  %11 = tail call fastcc i32 @io_done(ptr noundef %4) #44, !range !355
  br label %15

12:                                               ; preds = %8
  %13 = tail call fastcc i32 @io_wait(ptr noundef %0, i32 noundef 1, ptr noundef %3, ptr noundef %4) #44, !range !60
  %14 = icmp eq i32 %13, 0
  br i1 %14, label %8, label %15

15:                                               ; preds = %8, %12, %10
  %16 = phi i32 [ %11, %10 ], [ 0, %8 ], [ 1, %12 ]
  ret i32 %16
}

; Function Attrs: nounwind optsize
define noundef ptr @fa_file_job(i32 noundef %0, ptr nocapture noundef readonly %1, i64 noundef %2, ptr nocapture noundef readonly %3, i64 noundef %4) local_unnamed_addr #2 {
  %6 = tail call ptr @fa_alloc(i64 noundef 96, i64 noundef 16) #45
  %7 = getelementptr inbounds i8, ptr %6, i64 16
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %7, i8 0, i64 80, i1 false)
  store ptr @file_job_run, ptr %6, align 8, !tbaa !361
  %8 = getelementptr inbounds %struct.FaJob, ptr %6, i64 0, i32 1
  store ptr @file_job_release, ptr %8, align 8, !tbaa !363
  %9 = getelementptr inbounds %struct.FaFileJob, ptr %6, i64 0, i32 1
  store i32 %0, ptr %9, align 8, !tbaa !364
  %10 = add i64 %2, 1
  %11 = tail call ptr @fa_alloc(i64 noundef %10, i64 noundef 1) #45
  %12 = getelementptr inbounds %struct.FaFileJob, ptr %6, i64 0, i32 3
  store ptr %11, ptr %12, align 8, !tbaa !365
  tail call fastcc void @copy_bytes(ptr noundef %11, ptr noundef %1, i64 noundef %2) #44
  %13 = load ptr, ptr %12, align 8, !tbaa !365
  %14 = getelementptr inbounds i8, ptr %13, i64 %2
  store i8 0, ptr %14, align 1, !tbaa !2
  %15 = getelementptr inbounds %struct.FaFileJob, ptr %6, i64 0, i32 4
  store i64 %2, ptr %15, align 8, !tbaa !366
  %16 = icmp eq i32 %0, 2
  %17 = icmp ne i64 %4, 0
  %18 = and i1 %16, %17
  br i1 %18, label %19, label %24

19:                                               ; preds = %5
  %20 = tail call ptr @fa_alloc(i64 noundef %4, i64 noundef 16) #45
  %21 = getelementptr inbounds %struct.FaFileJob, ptr %6, i64 0, i32 5
  store ptr %20, ptr %21, align 8, !tbaa !367
  tail call fastcc void @copy_bytes(ptr noundef %20, ptr noundef %3, i64 noundef %4) #44
  %22 = getelementptr inbounds %struct.FaFileJob, ptr %6, i64 0, i32 6
  store i64 %4, ptr %22, align 8, !tbaa !368
  %23 = getelementptr inbounds %struct.FaFileJob, ptr %6, i64 0, i32 7
  store i64 %4, ptr %23, align 8, !tbaa !369
  br label %24

24:                                               ; preds = %19, %5
  ret ptr %6
}

; Function Attrs: nounwind optsize
define internal void @file_job_run(ptr nocapture noundef %0) #2 {
  br i1 icmp eq (ptr @open, ptr null), label %2, label %4

2:                                                ; preds = %1
  %3 = getelementptr inbounds %struct.FaFileJob, ptr %0, i64 0, i32 2
  store i32 38, ptr %3, align 4, !tbaa !370
  br label %127

4:                                                ; preds = %1
  %5 = getelementptr inbounds %struct.FaFileJob, ptr %0, i64 0, i32 1
  %6 = load i32, ptr %5, align 8, !tbaa !364
  %7 = icmp eq i32 %6, 2
  %8 = getelementptr inbounds %struct.FaFileJob, ptr %0, i64 0, i32 3
  %9 = load ptr, ptr %8, align 8, !tbaa !365
  br i1 %7, label %10, label %65

10:                                               ; preds = %4
  %11 = tail call i32 (ptr, i32, ...) @open(ptr noundef %9, i32 noundef 524865, i32 noundef 438) #45
  %12 = icmp slt i32 %11, 0
  br i1 %12, label %19, label %13

13:                                               ; preds = %10
  %14 = getelementptr inbounds %struct.FaFileJob, ptr %0, i64 0, i32 6
  %15 = load i64, ptr %14, align 8, !tbaa !368
  %16 = icmp eq i64 %15, 0
  br i1 %16, label %52, label %17

17:                                               ; preds = %13
  %18 = getelementptr inbounds %struct.FaFileJob, ptr %0, i64 0, i32 5
  br label %26

19:                                               ; preds = %10
  br i1 icmp ne (ptr @__errno_location, ptr null), label %20, label %23

20:                                               ; preds = %19
  %21 = tail call ptr @__errno_location() #45
  %22 = load i32, ptr %21, align 4, !tbaa !25
  br label %23

23:                                               ; preds = %19, %20
  %24 = phi i32 [ %22, %20 ], [ 5, %19 ]
  %25 = getelementptr inbounds %struct.FaFileJob, ptr %0, i64 0, i32 2
  store i32 %24, ptr %25, align 4, !tbaa !370
  br label %127

26:                                               ; preds = %17, %48
  %27 = phi i64 [ %15, %17 ], [ %50, %48 ]
  %28 = phi i64 [ 0, %17 ], [ %49, %48 ]
  %29 = load ptr, ptr %18, align 8, !tbaa !367
  %30 = getelementptr inbounds i8, ptr %29, i64 %28
  %31 = sub i64 %27, %28
  %32 = tail call i64 @write(i32 noundef %11, ptr noundef %30, i64 noundef %31) #45
  %33 = icmp slt i64 %32, 0
  br i1 %33, label %34, label %46

34:                                               ; preds = %26
  br i1 icmp ne (ptr @__errno_location, ptr null), label %35, label %43

35:                                               ; preds = %34
  %36 = tail call ptr @__errno_location() #45
  %37 = load i32, ptr %36, align 4, !tbaa !25
  %38 = icmp eq i32 %37, 4
  br i1 %38, label %48, label %39, !llvm.loop !371

39:                                               ; preds = %35
  br i1 icmp ne (ptr @__errno_location, ptr null), label %40, label %43

40:                                               ; preds = %39
  %41 = tail call ptr @__errno_location() #45
  %42 = load i32, ptr %41, align 4, !tbaa !25
  br label %43

43:                                               ; preds = %34, %40, %39
  %44 = phi i32 [ %42, %40 ], [ 5, %39 ], [ 5, %34 ]
  %45 = getelementptr inbounds %struct.FaFileJob, ptr %0, i64 0, i32 2
  store i32 %44, ptr %45, align 4, !tbaa !370
  br label %52

46:                                               ; preds = %26
  %47 = add i64 %32, %28
  br label %48

48:                                               ; preds = %35, %46
  %49 = phi i64 [ %47, %46 ], [ %28, %35 ]
  %50 = load i64, ptr %14, align 8, !tbaa !368
  %51 = icmp ult i64 %49, %50
  br i1 %51, label %26, label %52

52:                                               ; preds = %48, %13, %43
  %53 = tail call i32 @close(i32 noundef %11) #45
  %54 = icmp eq i32 %53, 0
  br i1 %54, label %127, label %55

55:                                               ; preds = %52
  %56 = getelementptr inbounds %struct.FaFileJob, ptr %0, i64 0, i32 2
  %57 = load i32, ptr %56, align 4, !tbaa !370
  %58 = icmp eq i32 %57, 0
  br i1 %58, label %59, label %127

59:                                               ; preds = %55
  br i1 icmp ne (ptr @__errno_location, ptr null), label %60, label %63

60:                                               ; preds = %59
  %61 = tail call ptr @__errno_location() #45
  %62 = load i32, ptr %61, align 4, !tbaa !25
  br label %63

63:                                               ; preds = %59, %60
  %64 = phi i32 [ %62, %60 ], [ 5, %59 ]
  store i32 %64, ptr %56, align 4, !tbaa !370
  br label %127

65:                                               ; preds = %4
  %66 = tail call i32 (ptr, i32, ...) @open(ptr noundef %9, i32 noundef 524288) #45
  %67 = icmp slt i32 %66, 0
  br i1 %67, label %72, label %68

68:                                               ; preds = %65
  %69 = getelementptr inbounds %struct.FaFileJob, ptr %0, i64 0, i32 6
  %70 = getelementptr inbounds %struct.FaFileJob, ptr %0, i64 0, i32 7
  %71 = getelementptr inbounds %struct.FaFileJob, ptr %0, i64 0, i32 5
  br label %79

72:                                               ; preds = %65
  br i1 icmp ne (ptr @__errno_location, ptr null), label %73, label %76

73:                                               ; preds = %72
  %74 = tail call ptr @__errno_location() #45
  %75 = load i32, ptr %74, align 4, !tbaa !25
  br label %76

76:                                               ; preds = %72, %73
  %77 = phi i32 [ %75, %73 ], [ 5, %72 ]
  %78 = getelementptr inbounds %struct.FaFileJob, ptr %0, i64 0, i32 2
  store i32 %77, ptr %78, align 4, !tbaa !370
  br label %127

79:                                               ; preds = %112, %68
  %80 = load i64, ptr %69, align 8, !tbaa !368
  %81 = load i64, ptr %70, align 8, !tbaa !369
  %82 = icmp eq i64 %80, %81
  br i1 %82, label %85, label %83

83:                                               ; preds = %79
  %84 = load ptr, ptr %71, align 8, !tbaa !367
  br label %99

85:                                               ; preds = %79
  %86 = icmp ult i64 %80, 4096
  %87 = shl i64 %80, 1
  %88 = select i1 %86, i64 4096, i64 %87
  %89 = tail call ptr @fa_alloc(i64 noundef %88, i64 noundef 16) #45
  %90 = load i64, ptr %70, align 8, !tbaa !369
  %91 = icmp eq i64 %90, 0
  br i1 %91, label %97, label %92

92:                                               ; preds = %85
  %93 = load ptr, ptr %71, align 8, !tbaa !367
  %94 = load i64, ptr %69, align 8, !tbaa !368
  tail call fastcc void @copy_bytes(ptr noundef %89, ptr noundef %93, i64 noundef %94) #44
  %95 = load ptr, ptr %71, align 8, !tbaa !367
  %96 = load i64, ptr %70, align 8, !tbaa !369
  tail call void @fa_free(ptr noundef %95, i64 noundef %96, i64 noundef 16) #45
  br label %97

97:                                               ; preds = %92, %85
  store ptr %89, ptr %71, align 8, !tbaa !367
  store i64 %88, ptr %70, align 8, !tbaa !369
  %98 = load i64, ptr %69, align 8, !tbaa !368
  br label %99

99:                                               ; preds = %83, %97
  %100 = phi i64 [ %81, %83 ], [ %88, %97 ]
  %101 = phi i64 [ %80, %83 ], [ %98, %97 ]
  %102 = phi ptr [ %84, %83 ], [ %89, %97 ]
  %103 = getelementptr inbounds i8, ptr %102, i64 %101
  %104 = sub i64 %100, %101
  %105 = tail call i64 @read(i32 noundef %66, ptr noundef %103, i64 noundef %104) #45
  %106 = icmp slt i64 %105, 0
  br i1 %106, label %107, label %120

107:                                              ; preds = %99
  br i1 icmp ne (ptr @__errno_location, ptr null), label %108, label %117

108:                                              ; preds = %107
  %109 = tail call ptr @__errno_location() #45
  %110 = load i32, ptr %109, align 4, !tbaa !25
  %111 = icmp eq i32 %110, 4
  br i1 %111, label %112, label %113

112:                                              ; preds = %108, %122
  br label %79

113:                                              ; preds = %108
  br i1 icmp ne (ptr @__errno_location, ptr null), label %114, label %117

114:                                              ; preds = %113
  %115 = tail call ptr @__errno_location() #45
  %116 = load i32, ptr %115, align 4, !tbaa !25
  br label %117

117:                                              ; preds = %107, %113, %114
  %118 = phi i32 [ %116, %114 ], [ 5, %113 ], [ 5, %107 ]
  %119 = getelementptr inbounds %struct.FaFileJob, ptr %0, i64 0, i32 2
  store i32 %118, ptr %119, align 4, !tbaa !370
  br label %125

120:                                              ; preds = %99
  %121 = icmp eq i64 %105, 0
  br i1 %121, label %125, label %122

122:                                              ; preds = %120
  %123 = load i64, ptr %69, align 8, !tbaa !368
  %124 = add i64 %123, %105
  store i64 %124, ptr %69, align 8, !tbaa !368
  br label %112

125:                                              ; preds = %120, %117
  %126 = tail call i32 @close(i32 noundef %66) #45
  br label %127

127:                                              ; preds = %76, %125, %23, %63, %55, %52, %2
  ret void
}

; Function Attrs: nounwind optsize
define internal void @file_job_release(ptr noundef %0) #2 {
  %2 = getelementptr inbounds %struct.FaFileJob, ptr %0, i64 0, i32 3
  %3 = load ptr, ptr %2, align 8, !tbaa !365
  %4 = getelementptr inbounds %struct.FaFileJob, ptr %0, i64 0, i32 4
  %5 = load i64, ptr %4, align 8, !tbaa !366
  %6 = add i64 %5, 1
  tail call void @fa_free(ptr noundef %3, i64 noundef %6, i64 noundef 1) #45
  %7 = getelementptr inbounds %struct.FaFileJob, ptr %0, i64 0, i32 7
  %8 = load i64, ptr %7, align 8, !tbaa !369
  %9 = icmp eq i64 %8, 0
  br i1 %9, label %13, label %10

10:                                               ; preds = %1
  %11 = getelementptr inbounds %struct.FaFileJob, ptr %0, i64 0, i32 5
  %12 = load ptr, ptr %11, align 8, !tbaa !367
  tail call void @fa_free(ptr noundef %12, i64 noundef %8, i64 noundef 16) #45
  br label %13

13:                                               ; preds = %10, %1
  tail call void @fa_free(ptr noundef nonnull %0, i64 noundef 96, i64 noundef 16) #45
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_file_run(ptr nocapture noundef %0) local_unnamed_addr #2 {
  tail call void @file_job_run(ptr noundef %0) #44
  ret void
}

; Function Attrs: nounwind optsize
define noundef i32 @fa_file_submit(ptr noundef %0, ptr noundef %1, ptr noundef %2) local_unnamed_addr #2 {
  %4 = tail call i32 @fa_blocking_submit(ptr noundef %0, ptr noundef %1, ptr noundef %2) #44, !range !60
  ret i32 %4
}

; Function Attrs: nounwind optsize
define void @fa_file_cancel(ptr noundef %0) local_unnamed_addr #2 {
  tail call void @fa_blocking_cancel(ptr noundef %0) #44
  ret void
}

; Function Attrs: nounwind optsize
define noundef ptr @fa_file_finish(ptr noundef %0, ptr nocapture noundef writeonly %1) local_unnamed_addr #2 {
  %3 = alloca %struct.FaString, align 8
  %4 = alloca %struct.FaString, align 8
  %5 = alloca %struct.FaString, align 8
  %6 = getelementptr inbounds %struct.FaFileJob, ptr %0, i64 0, i32 2
  %7 = load i32, ptr %6, align 4, !tbaa !370
  %8 = icmp eq i32 %7, 0
  br i1 %8, label %21, label %9

9:                                                ; preds = %2
  %10 = getelementptr inbounds %struct.FaFileJob, ptr %0, i64 0, i32 3
  %11 = load ptr, ptr %10, align 8, !tbaa !365
  %12 = getelementptr inbounds %struct.FaFileJob, ptr %0, i64 0, i32 4
  %13 = load i64, ptr %12, align 8, !tbaa !366
  %14 = tail call ptr @fa_alloc(i64 noundef 56, i64 noundef 8) #45
  store i64 9, ptr %14, align 8, !tbaa !237
  %15 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %15, align 8, !tbaa !8
  tail call fastcc void @append_error(i32 noundef %7) #44
  %16 = getelementptr inbounds %struct.FileErrorBox, ptr %14, i64 0, i32 2
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %3) #40
  %17 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %18 = load ptr, ptr %17, align 8, !tbaa !10
  %19 = load i64, ptr %15, align 8, !tbaa !8
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 8 %3, ptr noundef %18, i64 noundef %19, i64 noundef 0) #44
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %16, ptr noundef nonnull align 8 dereferenceable(24) %3, i64 24, i1 false), !tbaa.struct !13
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %3) #40
  store i64 0, ptr %15, align 8, !tbaa !8
  %20 = getelementptr inbounds %struct.FileErrorBox, ptr %14, i64 0, i32 1
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %4) #40
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 8 %4, ptr noundef %11, i64 noundef %13, i64 noundef 0) #44
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %20, ptr noundef nonnull align 8 dereferenceable(24) %4, i64 24, i1 false), !tbaa.struct !13
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %4) #40
  br label %40

21:                                               ; preds = %2
  %22 = getelementptr inbounds %struct.FaFileJob, ptr %0, i64 0, i32 1
  %23 = load i32, ptr %22, align 8, !tbaa !364
  switch i32 %23, label %40 [
    i32 0, label %24
    i32 1, label %29
  ]

24:                                               ; preds = %21
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %5) #40
  %25 = getelementptr inbounds %struct.FaFileJob, ptr %0, i64 0, i32 5
  %26 = load ptr, ptr %25, align 8, !tbaa !367
  %27 = getelementptr inbounds %struct.FaFileJob, ptr %0, i64 0, i32 6
  %28 = load i64, ptr %27, align 8, !tbaa !368
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 8 %5, ptr noundef %26, i64 noundef %28, i64 noundef 0) #44
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %1, ptr noundef nonnull align 8 dereferenceable(24) %5, i64 24, i1 false), !tbaa.struct !13
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %5) #40
  br label %40

29:                                               ; preds = %21
  %30 = getelementptr inbounds %struct.FaFileJob, ptr %0, i64 0, i32 6
  %31 = load i64, ptr %30, align 8, !tbaa !368
  %32 = tail call ptr @fa_array_new(i64 noundef %31, i64 noundef %31, i64 noundef 1) #44
  %33 = load i64, ptr %30, align 8, !tbaa !368
  %34 = icmp eq i64 %33, 0
  br i1 %34, label %39, label %35

35:                                               ; preds = %29
  %36 = load ptr, ptr %32, align 8, !tbaa !16
  %37 = getelementptr inbounds %struct.FaFileJob, ptr %0, i64 0, i32 5
  %38 = load ptr, ptr %37, align 8, !tbaa !367
  tail call fastcc void @copy_bytes(ptr noundef %36, ptr noundef %38, i64 noundef %33) #44
  br label %39

39:                                               ; preds = %35, %29
  store ptr %32, ptr %1, align 8, !tbaa !10
  br label %40

40:                                               ; preds = %21, %24, %39, %9
  %41 = phi ptr [ %14, %9 ], [ null, %24 ], [ null, %39 ], [ null, %21 ]
  tail call void @file_job_release(ptr noundef nonnull %0) #44
  ret ptr %41
}

; Function Attrs: nounwind optsize
define void @fa_worker_stop(ptr noundef %0) local_unnamed_addr #2 {
  %2 = icmp eq ptr %0, null
  br i1 %2, label %80, label %3

3:                                                ; preds = %1
  %4 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 10
  %5 = load i32, ptr %4, align 16, !tbaa !372
  %6 = icmp eq i32 %5, 0
  br i1 %6, label %7, label %80

7:                                                ; preds = %3
  store i32 1, ptr %4, align 16, !tbaa !372
  %8 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 8
  %9 = load i32, ptr %8, align 8, !tbaa !374
  %10 = icmp eq i32 %9, 0
  br i1 %10, label %80, label %11

11:                                               ; preds = %7
  %12 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 12
  %13 = load ptr, ptr %12, align 8, !tbaa !375
  %14 = icmp eq ptr %13, null
  br i1 %14, label %16, label %15

15:                                               ; preds = %11
  br i1 icmp ne (ptr @pthread_join, ptr null), label %74, label %78

16:                                               ; preds = %11
  %17 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 1
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %18, label %20

18:                                               ; preds = %16
  %19 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %17) #45
  br label %20

20:                                               ; preds = %16, %18
  %21 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 3
  %22 = load ptr, ptr %21, align 8, !tbaa !10
  %23 = icmp eq ptr %22, null
  br i1 %23, label %41, label %24

24:                                               ; preds = %20, %35
  %25 = phi ptr [ %39, %35 ], [ %22, %20 ]
  %26 = phi ptr [ %38, %35 ], [ null, %20 ]
  %27 = phi ptr [ %37, %35 ], [ %21, %20 ]
  %28 = phi ptr [ %36, %35 ], [ null, %20 ]
  %29 = getelementptr inbounds i8, ptr %25, i64 188
  %30 = load i32, ptr %29, align 4, !tbaa !267
  %31 = and i32 %30, 1
  %32 = icmp eq i32 %31, 0
  br i1 %32, label %35, label %33

33:                                               ; preds = %24
  %34 = load ptr, ptr %25, align 8, !tbaa !376
  store ptr %34, ptr %27, align 8, !tbaa !10
  store ptr %26, ptr %25, align 8, !tbaa !376
  br label %35

35:                                               ; preds = %24, %33
  %36 = phi ptr [ %28, %33 ], [ %25, %24 ]
  %37 = phi ptr [ %27, %33 ], [ %25, %24 ]
  %38 = phi ptr [ %25, %33 ], [ %26, %24 ]
  %39 = load ptr, ptr %37, align 8, !tbaa !10
  %40 = icmp eq ptr %39, null
  br i1 %40, label %41, label %24, !llvm.loop !378

41:                                               ; preds = %35, %20
  %42 = phi ptr [ null, %20 ], [ %36, %35 ]
  %43 = phi ptr [ null, %20 ], [ %38, %35 ]
  %44 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 4
  store ptr %42, ptr %44, align 8, !tbaa !379
  %45 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 9
  store i32 1, ptr %45, align 4, !tbaa !380
  %46 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 2
  %47 = tail call i32 @pthread_cond_broadcast(ptr noundef nonnull %46) #45
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %48, label %50

48:                                               ; preds = %41
  %49 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %17) #45
  br label %50

50:                                               ; preds = %41, %48
  %51 = icmp eq ptr %43, null
  br i1 %51, label %73, label %52

52:                                               ; preds = %50, %71
  %53 = phi ptr [ %54, %71 ], [ %43, %50 ]
  %54 = load ptr, ptr %53, align 8, !tbaa !376
  %55 = getelementptr inbounds %struct.FaCall, ptr %53, i64 0, i32 2
  %56 = load ptr, ptr %55, align 8, !tbaa !381
  %57 = getelementptr inbounds %struct.FaCall, ptr %53, i64 0, i32 4
  %58 = load ptr, ptr %57, align 8, !tbaa !382
  %59 = getelementptr inbounds %struct.FaCall, ptr %53, i64 0, i32 6
  %60 = load ptr, ptr %59, align 8, !tbaa !383
  tail call void %58(ptr noundef %60) #45
  %61 = icmp eq ptr %56, null
  br i1 %61, label %68, label %62

62:                                               ; preds = %52
  %63 = getelementptr inbounds %struct.FaExecutor, ptr %56, i64 0, i32 11
  %64 = load i64, ptr %63, align 8, !tbaa !268
  %65 = add nsw i64 %64, -1
  store i64 %65, ptr %63, align 8, !tbaa !268
  %66 = getelementptr inbounds %struct.FaCall, ptr %53, i64 0, i32 7
  %67 = load i64, ptr %66, align 8, !tbaa !384
  tail call void @fa_free(ptr noundef nonnull %53, i64 noundef %67, i64 noundef 16) #45
  tail call void @fa_remote_source_end(ptr noundef nonnull %56) #44
  br label %71

68:                                               ; preds = %52
  %69 = getelementptr inbounds %struct.FaCall, ptr %53, i64 0, i32 7
  %70 = load i64, ptr %69, align 8, !tbaa !384
  tail call void @fa_free(ptr noundef nonnull %53, i64 noundef %70, i64 noundef 16) #45
  br label %71

71:                                               ; preds = %68, %62
  tail call void @fa_shared_release(ptr noundef nonnull %0) #44
  %72 = icmp eq ptr %54, null
  br i1 %72, label %73, label %52, !llvm.loop !385

73:                                               ; preds = %71, %50
  br i1 icmp ne (ptr @pthread_join, ptr null), label %74, label %78

74:                                               ; preds = %73, %15
  %75 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 7
  %76 = load i64, ptr %75, align 16, !tbaa !386
  %77 = tail call i32 @pthread_join(i64 noundef %76, ptr noundef null) #45
  br label %78

78:                                               ; preds = %74, %73, %15
  %79 = atomicrmw sub ptr @workers_alive, i64 1 acq_rel, align 8
  br label %80

80:                                               ; preds = %78, %7, %1, %3
  ret void
}

; Function Attrs: optsize
declare extern_weak i32 @pthread_join(i64 noundef, ptr noundef) #3

; Function Attrs: optsize
declare extern_weak i32 @pthread_cond_broadcast(ptr noundef) local_unnamed_addr #3

; Function Attrs: nounwind optsize
define void @fa_worker_detach(ptr noundef %0) local_unnamed_addr #2 {
  %2 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 8
  %3 = load i32, ptr %2, align 8, !tbaa !374
  %4 = icmp eq i32 %3, 0
  br i1 %4, label %26, label %5

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 10
  %7 = load i32, ptr %6, align 16, !tbaa !372
  %8 = icmp eq i32 %7, 0
  br i1 %8, label %9, label %26

9:                                                ; preds = %5
  %10 = atomicrmw add ptr %0, i64 1 monotonic, align 8
  %11 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 1
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %12, label %14

12:                                               ; preds = %9
  %13 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %11) #45
  br label %14

14:                                               ; preds = %9, %12
  %15 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 11
  store i32 1, ptr %15, align 4, !tbaa !387
  %16 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 9
  store i32 1, ptr %16, align 4, !tbaa !380
  %17 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 2
  %18 = tail call i32 @pthread_cond_broadcast(ptr noundef nonnull %17) #45
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %19, label %21

19:                                               ; preds = %14
  %20 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %11) #45
  br label %21

21:                                               ; preds = %14, %19
  store i32 1, ptr %6, align 16, !tbaa !372
  br i1 icmp ne (ptr @pthread_detach, ptr null), label %22, label %26

22:                                               ; preds = %21
  %23 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 7
  %24 = load i64, ptr %23, align 16, !tbaa !386
  %25 = tail call i32 @pthread_detach(i64 noundef %24) #45
  br label %26

26:                                               ; preds = %1, %5, %22, %21
  ret void
}

; Function Attrs: nounwind optsize
define ptr @fa_worker_new(i64 noundef %0, ptr noundef %1) local_unnamed_addr #2 {
  %3 = add i64 %0, 256
  %4 = tail call fastcc ptr @shared_new(i64 noundef %3, ptr noundef nonnull @worker_drop) #44
  %5 = getelementptr inbounds %struct.FaWorker, ptr %4, i64 0, i32 6
  store ptr %1, ptr %5, align 8, !tbaa !388
  ret ptr %4
}

; Function Attrs: nounwind optsize
define internal void @worker_drop(ptr noundef %0) #2 {
  tail call void @fa_worker_stop(ptr noundef %0) #44
  %2 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 6
  %3 = load ptr, ptr %2, align 8, !tbaa !388
  %4 = icmp eq ptr %3, null
  br i1 %4, label %6, label %5

5:                                                ; preds = %1
  tail call void %3(ptr noundef nonnull %0) #45
  br label %6

6:                                                ; preds = %5, %1
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_worker_start(ptr noundef %0) local_unnamed_addr #2 {
  br i1 icmp eq (ptr @pthread_create, ptr null), label %7, label %2

2:                                                ; preds = %1
  %3 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 8
  store i32 1, ptr %3, align 8, !tbaa !374
  %4 = atomicrmw add ptr @workers_alive, i64 1 acq_rel, align 8
  %5 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 7
  %6 = tail call i32 @pthread_create(ptr noundef nonnull %5, ptr noundef null, ptr noundef nonnull @worker_thread, ptr noundef %0) #45
  br label %7

7:                                                ; preds = %1, %2
  ret void
}

; Function Attrs: nounwind optsize
define internal noundef ptr @worker_thread(ptr noundef %0) #2 {
  %2 = tail call align 4 ptr @llvm.threadlocal.address.p0(ptr align 4 @fa_blocking_depth)
  store i32 1, ptr %2, align 4, !tbaa !25
  %3 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 1
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %4, label %6

4:                                                ; preds = %1
  %5 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %3) #45
  br label %6

6:                                                ; preds = %1, %4
  %7 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 9
  %8 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 3
  %9 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 2
  %10 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 4
  %11 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 5
  %12 = getelementptr inbounds i8, ptr %0, i64 256
  br label %13

13:                                               ; preds = %63, %6
  %14 = load ptr, ptr %8, align 16, !tbaa !389
  %15 = icmp eq ptr %14, null
  br i1 %15, label %16, label %23

16:                                               ; preds = %13, %19
  %17 = load i32, ptr %7, align 4, !tbaa !380
  %18 = icmp eq i32 %17, 0
  br i1 %18, label %19, label %65

19:                                               ; preds = %16
  %20 = tail call i32 @pthread_cond_wait(ptr noundef nonnull %9, ptr noundef nonnull %3) #45
  %21 = load ptr, ptr %8, align 16, !tbaa !389
  %22 = icmp eq ptr %21, null
  br i1 %22, label %16, label %23, !llvm.loop !390

23:                                               ; preds = %19, %13
  %24 = phi ptr [ %14, %13 ], [ %21, %19 ]
  %25 = load ptr, ptr %24, align 8, !tbaa !376
  store ptr %25, ptr %8, align 16, !tbaa !389
  %26 = icmp eq ptr %25, null
  br i1 %26, label %27, label %28

27:                                               ; preds = %23
  store ptr null, ptr %10, align 8, !tbaa !379
  br label %28

28:                                               ; preds = %27, %23
  %29 = getelementptr inbounds %struct.FaCall, ptr %24, i64 0, i32 8
  store i32 1, ptr %29, align 8, !tbaa !391
  store ptr %24, ptr %11, align 16, !tbaa !392
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %30, label %32

30:                                               ; preds = %28
  %31 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %3) #45
  br label %32

32:                                               ; preds = %28, %30
  %33 = getelementptr inbounds %struct.FaCall, ptr %24, i64 0, i32 3
  %34 = load ptr, ptr %33, align 8, !tbaa !393
  %35 = getelementptr inbounds %struct.FaCall, ptr %24, i64 0, i32 6
  %36 = load ptr, ptr %35, align 8, !tbaa !383
  %37 = getelementptr inbounds i8, ptr %24, i64 144
  tail call void %34(ptr noundef nonnull %12, ptr noundef %36, ptr noundef nonnull %37) #45
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %38, label %40

38:                                               ; preds = %32
  %39 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %3) #45
  br label %40

40:                                               ; preds = %32, %38
  store ptr null, ptr %11, align 16, !tbaa !392
  %41 = load i32, ptr %29, align 8, !tbaa !391
  %42 = icmp eq i32 %41, 3
  br i1 %42, label %43, label %54

43:                                               ; preds = %40
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %44, label %46

44:                                               ; preds = %43
  %45 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %3) #45
  br label %46

46:                                               ; preds = %43, %44
  %47 = getelementptr inbounds i8, ptr %24, i64 188
  %48 = load i32, ptr %47, align 4, !tbaa !267
  %49 = or i32 %48, 2
  store i32 %49, ptr %47, align 4, !tbaa !267
  %50 = getelementptr inbounds %struct.FaCall, ptr %24, i64 0, i32 5
  %51 = load ptr, ptr %50, align 8, !tbaa !394
  tail call void %51(ptr noundef nonnull %37) #45
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %52, label %63

52:                                               ; preds = %46
  %53 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %3) #45
  br label %63

54:                                               ; preds = %40
  %55 = getelementptr inbounds i8, ptr %24, i64 188
  %56 = load i32, ptr %55, align 4, !tbaa !267
  %57 = or i32 %56, 2
  store i32 %57, ptr %55, align 4, !tbaa !267
  store i32 2, ptr %29, align 8, !tbaa !391
  %58 = getelementptr inbounds %struct.FaCall, ptr %24, i64 0, i32 2
  %59 = load ptr, ptr %58, align 8, !tbaa !381
  %60 = icmp eq ptr %59, null
  br i1 %60, label %63, label %61

61:                                               ; preds = %54
  %62 = getelementptr inbounds i8, ptr %24, i64 160
  tail call fastcc void @remote_push(ptr noundef nonnull %59, ptr noundef nonnull %62) #44
  tail call void @fa_remote_source_end(ptr noundef nonnull %59) #44
  br label %63

63:                                               ; preds = %46, %52, %54, %61
  %64 = tail call i32 @pthread_cond_broadcast(ptr noundef nonnull %9) #45
  br label %13

65:                                               ; preds = %16
  %66 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 11
  %67 = load i32, ptr %66, align 4, !tbaa !387
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %68, label %70

68:                                               ; preds = %65
  %69 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %3) #45
  br label %70

70:                                               ; preds = %65, %68
  tail call void @fa_release_builder() #44
  %71 = icmp eq i32 %67, 0
  br i1 %71, label %74, label %72

72:                                               ; preds = %70
  %73 = atomicrmw sub ptr @workers_alive, i64 1 acq_rel, align 8
  tail call void @fa_shared_release(ptr noundef nonnull %0) #44
  br label %74

74:                                               ; preds = %72, %70
  ret ptr null
}

; Function Attrs: nounwind optsize
define nonnull ptr @fa_call_new(i64 noundef %0) local_unnamed_addr #2 {
  %2 = add i64 %0, 144
  %3 = tail call ptr @fa_alloc(i64 noundef %2, i64 noundef 16) #45
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(192) %3, i8 0, i64 192, i1 false)
  %4 = getelementptr inbounds %struct.FaCall, ptr %3, i64 0, i32 7
  store i64 %2, ptr %4, align 8, !tbaa !384
  %5 = getelementptr inbounds i8, ptr %3, i64 144
  ret ptr %5
}

; Function Attrs: nounwind optsize
define void @fa_call_free(ptr noundef %0) local_unnamed_addr #2 {
  %2 = getelementptr inbounds i8, ptr %0, i64 -144
  %3 = getelementptr inbounds i8, ptr %0, i64 -88
  %4 = load i64, ptr %3, align 8, !tbaa !384
  tail call void @fa_free(ptr noundef nonnull %2, i64 noundef %4, i64 noundef 16) #45
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_worker_call(ptr noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef %3, ptr noundef %4, ptr noundef %5, i32 noundef %6) local_unnamed_addr #2 {
  %8 = getelementptr inbounds i8, ptr %1, i64 -144
  %9 = getelementptr inbounds i8, ptr %1, i64 -136
  store ptr %0, ptr %9, align 8, !tbaa !395
  %10 = getelementptr inbounds i8, ptr %1, i64 -120
  store ptr %2, ptr %10, align 8, !tbaa !393
  %11 = getelementptr inbounds i8, ptr %1, i64 -96
  store ptr %3, ptr %11, align 8, !tbaa !383
  %12 = getelementptr inbounds i8, ptr %1, i64 -112
  store ptr %4, ptr %12, align 8, !tbaa !382
  %13 = getelementptr inbounds i8, ptr %1, i64 -104
  store ptr %5, ptr %13, align 8, !tbaa !394
  store ptr null, ptr %8, align 8, !tbaa !376
  %14 = icmp eq ptr %0, null
  br i1 %14, label %17, label %15

15:                                               ; preds = %7
  %16 = atomicrmw add ptr %0, i64 1 monotonic, align 8
  br label %17

17:                                               ; preds = %7, %15
  %18 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 8
  %19 = load i32, ptr %18, align 8, !tbaa !374
  %20 = icmp eq i32 %19, 0
  br i1 %20, label %29, label %21

21:                                               ; preds = %17
  %22 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 10
  %23 = load i32, ptr %22, align 16, !tbaa !372
  %24 = icmp eq i32 %23, 0
  br i1 %24, label %25, label %29

25:                                               ; preds = %21
  %26 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @current_worker)
  %27 = load ptr, ptr %26, align 8, !tbaa !10
  %28 = icmp eq ptr %27, %0
  br i1 %28, label %29, label %41

29:                                               ; preds = %25, %21, %17
  %30 = getelementptr inbounds i8, ptr %1, i64 -80
  store i32 1, ptr %30, align 8, !tbaa !391
  %31 = tail call align 4 ptr @llvm.threadlocal.address.p0(ptr align 4 @fa_blocking_depth)
  %32 = load i32, ptr %31, align 4, !tbaa !25
  store i32 1, ptr %31, align 4, !tbaa !25
  %33 = getelementptr inbounds i8, ptr %0, i64 256
  tail call void %2(ptr noundef nonnull %33, ptr noundef %3, ptr noundef nonnull %1) #45
  store i32 %32, ptr %31, align 4, !tbaa !25
  %34 = getelementptr inbounds i8, ptr %1, i64 44
  %35 = load i32, ptr %34, align 4, !tbaa !267
  %36 = or i32 %35, 2
  store i32 %36, ptr %34, align 4, !tbaa !267
  store i32 2, ptr %30, align 8, !tbaa !391
  store ptr null, ptr %1, align 8, !tbaa !10
  %37 = getelementptr inbounds i8, ptr %1, i64 40
  store atomic i32 3, ptr %37 release, align 8
  %38 = icmp eq i32 %6, 2
  br i1 %38, label %39, label %84

39:                                               ; preds = %29
  %40 = load ptr, ptr %13, align 8, !tbaa !394
  tail call void %40(ptr noundef nonnull %1) #45
  tail call void @fa_shared_release(ptr noundef nonnull %0) #44
  br label %84

41:                                               ; preds = %25
  %42 = icmp eq i32 %6, 0
  br i1 %42, label %57, label %43

43:                                               ; preds = %41
  %44 = tail call fastcc ptr @executor() #44
  %45 = getelementptr inbounds i8, ptr %1, i64 -128
  store ptr %44, ptr %45, align 8, !tbaa !381
  %46 = getelementptr inbounds %struct.FaExecutor, ptr %44, i64 0, i32 12
  %47 = atomicrmw add ptr %46, i64 1 acq_rel, align 8
  %48 = getelementptr inbounds i8, ptr %1, i64 40
  store atomic i32 2, ptr %48 release, align 8
  %49 = icmp eq i32 %6, 2
  br i1 %49, label %50, label %57

50:                                               ; preds = %43
  %51 = getelementptr inbounds i8, ptr %1, i64 44
  %52 = load i32, ptr %51, align 4, !tbaa !267
  %53 = or i32 %52, 1
  store i32 %53, ptr %51, align 4, !tbaa !267
  %54 = getelementptr inbounds %struct.FaExecutor, ptr %44, i64 0, i32 11
  %55 = load i64, ptr %54, align 8, !tbaa !268
  %56 = add nsw i64 %55, 1
  store i64 %56, ptr %54, align 8, !tbaa !268
  br label %57

57:                                               ; preds = %43, %50, %41
  %58 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 1
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %59, label %61

59:                                               ; preds = %57
  %60 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %58) #45
  br label %61

61:                                               ; preds = %57, %59
  %62 = getelementptr inbounds i8, ptr %1, i64 -80
  store i32 0, ptr %62, align 8, !tbaa !391
  %63 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 4
  %64 = load ptr, ptr %63, align 8, !tbaa !379
  %65 = icmp eq ptr %64, null
  br i1 %65, label %67, label %66

66:                                               ; preds = %61
  store ptr %8, ptr %64, align 8, !tbaa !376
  br label %69

67:                                               ; preds = %61
  %68 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 3
  store ptr %8, ptr %68, align 16, !tbaa !389
  br label %69

69:                                               ; preds = %67, %66
  store ptr %8, ptr %63, align 8, !tbaa !379
  %70 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 2
  %71 = tail call i32 @pthread_cond_broadcast(ptr noundef nonnull %70) #45
  br i1 %42, label %72, label %81

72:                                               ; preds = %69
  %73 = load i32, ptr %62, align 8, !tbaa !391
  %74 = icmp eq i32 %73, 2
  br i1 %74, label %79, label %75

75:                                               ; preds = %72, %75
  %76 = tail call i32 @pthread_cond_wait(ptr noundef nonnull %70, ptr noundef nonnull %58) #45
  %77 = load i32, ptr %62, align 8, !tbaa !391
  %78 = icmp eq i32 %77, 2
  br i1 %78, label %79, label %75, !llvm.loop !396

79:                                               ; preds = %75, %72
  store ptr null, ptr %1, align 8, !tbaa !10
  %80 = getelementptr inbounds i8, ptr %1, i64 40
  store atomic i32 3, ptr %80 release, align 8
  br label %81

81:                                               ; preds = %79, %69
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %82, label %84

82:                                               ; preds = %81
  %83 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %58) #45
  br label %84

84:                                               ; preds = %82, %81, %29, %39
  ret void
}

; Function Attrs: optsize
declare extern_weak i32 @pthread_cond_wait(ptr noundef, ptr noundef) local_unnamed_addr #3

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define void @fa_call_resume(ptr noundef %0) local_unnamed_addr #37 {
  store ptr null, ptr %0, align 8, !tbaa !10
  tail call void @fa_task_finish(ptr noundef nonnull %0) #44
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_call_destroy(ptr noundef %0) local_unnamed_addr #2 {
  %2 = getelementptr inbounds i8, ptr %0, i64 -144
  %3 = getelementptr inbounds i8, ptr %0, i64 -136
  %4 = load ptr, ptr %3, align 8, !tbaa !395
  %5 = icmp eq ptr %4, null
  br i1 %5, label %10, label %6

6:                                                ; preds = %1
  %7 = getelementptr inbounds %struct.FaWorker, ptr %4, i64 0, i32 8
  %8 = load i32, ptr %7, align 8, !tbaa !374
  %9 = icmp eq i32 %8, 0
  br i1 %9, label %10, label %13

10:                                               ; preds = %6, %1
  %11 = getelementptr inbounds i8, ptr %0, i64 -104
  %12 = load ptr, ptr %11, align 8, !tbaa !394
  tail call void %12(ptr noundef nonnull %0) #45
  tail call void @fa_shared_release(ptr noundef %4) #44
  br label %80

13:                                               ; preds = %6
  %14 = getelementptr inbounds %struct.FaWorker, ptr %4, i64 0, i32 1
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %15, label %17

15:                                               ; preds = %13
  %16 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %14) #45
  br label %17

17:                                               ; preds = %13, %15
  %18 = getelementptr inbounds i8, ptr %0, i64 -80
  %19 = load i32, ptr %18, align 8, !tbaa !391
  switch i32 %19, label %57 [
    i32 0, label %20
    i32 1, label %48
  ]

20:                                               ; preds = %17
  %21 = getelementptr inbounds %struct.FaWorker, ptr %4, i64 0, i32 3
  br label %22

22:                                               ; preds = %22, %20
  %23 = phi ptr [ null, %20 ], [ %25, %22 ]
  %24 = phi ptr [ %21, %20 ], [ %25, %22 ]
  %25 = load ptr, ptr %24, align 8, !tbaa !10
  %26 = icmp eq ptr %25, %2
  br i1 %26, label %27, label %22, !llvm.loop !397

27:                                               ; preds = %22
  %28 = load ptr, ptr %2, align 8, !tbaa !376
  store ptr %28, ptr %24, align 8, !tbaa !10
  %29 = getelementptr inbounds %struct.FaWorker, ptr %4, i64 0, i32 4
  %30 = load ptr, ptr %29, align 8, !tbaa !379
  %31 = icmp eq ptr %30, %2
  br i1 %31, label %32, label %33

32:                                               ; preds = %27
  store ptr %23, ptr %29, align 8, !tbaa !379
  br label %33

33:                                               ; preds = %32, %27
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %34, label %36

34:                                               ; preds = %33
  %35 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %14) #45
  br label %36

36:                                               ; preds = %33, %34
  %37 = getelementptr inbounds i8, ptr %0, i64 -112
  %38 = load ptr, ptr %37, align 8, !tbaa !382
  %39 = getelementptr inbounds i8, ptr %0, i64 -96
  %40 = load ptr, ptr %39, align 8, !tbaa !383
  tail call void %38(ptr noundef %40) #45
  %41 = getelementptr inbounds i8, ptr %0, i64 -128
  %42 = load ptr, ptr %41, align 8, !tbaa !381
  %43 = getelementptr inbounds i8, ptr %0, i64 -88
  %44 = load i64, ptr %43, align 8, !tbaa !384
  tail call void @fa_free(ptr noundef nonnull %2, i64 noundef %44, i64 noundef 16) #45
  %45 = icmp eq ptr %42, null
  br i1 %45, label %47, label %46

46:                                               ; preds = %36
  tail call void @fa_remote_source_end(ptr noundef nonnull %42) #44
  br label %47

47:                                               ; preds = %46, %36
  tail call void @fa_shared_release(ptr noundef nonnull %4) #44
  br label %80

48:                                               ; preds = %17
  store i32 3, ptr %18, align 8, !tbaa !391
  %49 = getelementptr inbounds i8, ptr %0, i64 -128
  %50 = load ptr, ptr %49, align 8, !tbaa !381
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %51, label %53

51:                                               ; preds = %48
  %52 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %14) #45
  br label %53

53:                                               ; preds = %48, %51
  %54 = icmp eq ptr %50, null
  br i1 %54, label %56, label %55

55:                                               ; preds = %53
  tail call void @fa_remote_source_end(ptr noundef nonnull %50) #44
  br label %56

56:                                               ; preds = %55, %53
  tail call void @fa_shared_release(ptr noundef nonnull %4) #44
  br label %80

57:                                               ; preds = %17
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %58, label %60

58:                                               ; preds = %57
  %59 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %14) #45
  br label %60

60:                                               ; preds = %57, %58
  %61 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @current_executor)
  %62 = load ptr, ptr %61, align 8, !tbaa !10
  %63 = getelementptr inbounds i8, ptr %0, i64 16
  %64 = icmp eq ptr %62, null
  br i1 %64, label %77, label %65

65:                                               ; preds = %60
  %66 = getelementptr inbounds i8, ptr %0, i64 -128
  %67 = load ptr, ptr %66, align 8, !tbaa !381
  %68 = icmp eq ptr %67, %62
  br i1 %68, label %69, label %77

69:                                               ; preds = %65
  %70 = getelementptr inbounds i8, ptr %0, i64 40
  %71 = load atomic i32, ptr %70 acquire, align 8
  %72 = icmp eq i32 %71, 5
  br i1 %72, label %73, label %77

73:                                               ; preds = %69
  tail call fastcc void @drain_remote(ptr noundef nonnull %62) #44
  %74 = load atomic i32, ptr %70 acquire, align 8
  %75 = icmp eq i32 %74, 1
  br i1 %75, label %76, label %77

76:                                               ; preds = %73
  tail call fastcc void @ready_unlink(ptr noundef nonnull %62, ptr noundef nonnull %63) #44
  br label %77

77:                                               ; preds = %73, %76, %69, %65, %60
  %78 = getelementptr inbounds i8, ptr %0, i64 -104
  %79 = load ptr, ptr %78, align 8, !tbaa !394
  tail call void %79(ptr noundef nonnull %0) #45
  tail call void @fa_shared_release(ptr noundef nonnull %4) #44
  br label %80

80:                                               ; preds = %77, %56, %47, %10
  ret void
}

; Function Attrs: nounwind optsize
define ptr @fa_workers_new(i64 noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3, ptr noundef %4, ptr noundef %5) local_unnamed_addr #2 {
  %7 = tail call i64 @llvm.smax.i64(i64 %0, i64 1)
  %8 = add i64 %3, 384
  %9 = tail call fastcc ptr @shared_new(i64 noundef %8, ptr noundef nonnull @workers_drop) #44
  %10 = getelementptr inbounds %struct.FaWorkerPool, ptr %9, i64 0, i32 13
  store ptr %4, ptr %10, align 8, !tbaa !398
  %11 = getelementptr inbounds %struct.FaWorkerPool, ptr %9, i64 0, i32 12
  store ptr %5, ptr %11, align 16, !tbaa !400
  %12 = tail call i64 @llvm.smax.i64(i64 %1, i64 0)
  %13 = getelementptr inbounds %struct.FaWorkerPool, ptr %9, i64 0, i32 10
  store i64 %12, ptr %13, align 16, !tbaa !401
  %14 = tail call i64 @llvm.smax.i64(i64 %2, i64 0)
  %15 = getelementptr inbounds %struct.FaWorkerPool, ptr %9, i64 0, i32 9
  store i64 %14, ptr %15, align 8, !tbaa !402
  %16 = shl i64 %7, 3
  %17 = tail call ptr @fa_alloc(i64 noundef %16, i64 noundef 8) #45
  %18 = getelementptr inbounds %struct.FaWorkerPool, ptr %9, i64 0, i32 4
  store ptr %17, ptr %18, align 16, !tbaa !403
  %19 = getelementptr inbounds %struct.FaWorkerPool, ptr %9, i64 0, i32 5
  store i64 %7, ptr %19, align 8, !tbaa !404
  ret ptr %9
}

; Function Attrs: nounwind optsize
define internal void @workers_drop(ptr noundef %0) #2 {
  tail call fastcc void @workers_shut(ptr noundef %0) #44
  %2 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i64 0, i32 5
  %3 = load i64, ptr %2, align 8, !tbaa !404
  %4 = icmp sgt i64 %3, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i64 0, i32 4
  br label %15

7:                                                ; preds = %15, %1
  %8 = phi i64 [ %3, %1 ], [ %21, %15 ]
  %9 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i64 0, i32 4
  %10 = load ptr, ptr %9, align 16, !tbaa !403
  %11 = shl i64 %8, 3
  tail call void @fa_free(ptr noundef %10, i64 noundef %11, i64 noundef 8) #45
  %12 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i64 0, i32 13
  %13 = load ptr, ptr %12, align 8, !tbaa !398
  %14 = icmp eq ptr %13, null
  br i1 %14, label %24, label %23

15:                                               ; preds = %5, %15
  %16 = phi i64 [ 0, %5 ], [ %20, %15 ]
  %17 = load ptr, ptr %6, align 16, !tbaa !403
  %18 = getelementptr inbounds ptr, ptr %17, i64 %16
  %19 = load ptr, ptr %18, align 8, !tbaa !10
  tail call void @fa_shared_release(ptr noundef %19) #44
  %20 = add nuw nsw i64 %16, 1
  %21 = load i64, ptr %2, align 8, !tbaa !404
  %22 = icmp slt i64 %20, %21
  br i1 %22, label %15, label %7, !llvm.loop !405

23:                                               ; preds = %7
  tail call void %13(ptr noundef nonnull %0) #45
  br label %24

24:                                               ; preds = %23, %7
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_workers_fill(ptr noundef %0) local_unnamed_addr #2 {
  %2 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i64 0, i32 5
  %3 = load i64, ptr %2, align 8, !tbaa !404
  %4 = icmp sgt i64 %3, 0
  br i1 %4, label %5, label %8

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i64 0, i32 12
  %7 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i64 0, i32 4
  br label %9

8:                                                ; preds = %9, %1
  ret void

9:                                                ; preds = %5, %9
  %10 = phi i64 [ 0, %5 ], [ %15, %9 ]
  %11 = load ptr, ptr %6, align 16, !tbaa !400
  %12 = tail call ptr %11(ptr noundef nonnull %0) #45
  %13 = load ptr, ptr %7, align 16, !tbaa !403
  %14 = getelementptr inbounds ptr, ptr %13, i64 %10
  store ptr %12, ptr %14, align 8, !tbaa !10
  %15 = add nuw nsw i64 %10, 1
  %16 = load i64, ptr %2, align 8, !tbaa !404
  %17 = icmp slt i64 %15, %16
  br i1 %17, label %9, label %8, !llvm.loop !406
}

; Function Attrs: nounwind optsize
define noundef i32 @fa_workers_reserve(ptr noundef %0, ptr noundef %1, ptr noundef %2) local_unnamed_addr #2 {
  %4 = tail call fastcc ptr @executor() #44
  %5 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i64 0, i32 1
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %6, label %8

6:                                                ; preds = %3
  %7 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %5) #45
  br label %8

8:                                                ; preds = %3, %6
  %9 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i64 0, i32 9
  %10 = load i64, ptr %9, align 8, !tbaa !402
  %11 = icmp eq i64 %10, 0
  %12 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i64 0, i32 8
  %13 = load i64, ptr %12, align 16, !tbaa !407
  %14 = icmp slt i64 %13, %10
  %15 = select i1 %11, i1 true, i1 %14
  br i1 %15, label %16, label %19

16:                                               ; preds = %8
  %17 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i64 0, i32 8
  %18 = add nsw i64 %13, 1
  store i64 %18, ptr %17, align 16, !tbaa !407
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %21, label %24

19:                                               ; preds = %8
  %20 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i64 0, i32 11
  tail call fastcc void @line_push(ptr noundef nonnull %20, ptr noundef %1, ptr noundef %2, ptr noundef nonnull @.str.31) #44
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %21, label %24

21:                                               ; preds = %19, %16
  %22 = phi i32 [ 0, %16 ], [ 1, %19 ]
  %23 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %5) #45
  br label %24

24:                                               ; preds = %21, %19, %16
  %25 = phi i32 [ 0, %16 ], [ 1, %19 ], [ %22, %21 ]
  ret i32 %25
}

; Function Attrs: nounwind optsize
define internal fastcc void @line_push(ptr nocapture noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef %3) unnamed_addr #2 {
  %5 = tail call fastcc ptr @executor() #44
  store ptr %2, ptr %1, align 8, !tbaa !286
  %6 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 1
  store ptr %5, ptr %6, align 8, !tbaa !288
  %7 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 5
  store ptr %3, ptr %7, align 8, !tbaa !408
  %8 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 9
  store i32 0, ptr %8, align 8, !tbaa !309
  %9 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 10
  store i32 0, ptr %9, align 4, !tbaa !310
  %10 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 8
  store i64 -1, ptr %10, align 8, !tbaa !289
  %11 = load ptr, ptr %0, align 8, !tbaa !10
  %12 = icmp eq ptr %11, null
  %13 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 2
  br i1 %12, label %14, label %16

14:                                               ; preds = %4
  store ptr %1, ptr %13, align 8, !tbaa !409
  %15 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 3
  br label %21

16:                                               ; preds = %4
  store ptr %11, ptr %13, align 8, !tbaa !409
  %17 = getelementptr inbounds %struct.FaWait, ptr %11, i64 0, i32 3
  %18 = load ptr, ptr %17, align 8, !tbaa !410
  %19 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 3
  store ptr %18, ptr %19, align 8, !tbaa !410
  %20 = getelementptr inbounds %struct.FaWait, ptr %18, i64 0, i32 2
  br label %21

21:                                               ; preds = %16, %14
  %22 = phi ptr [ %20, %16 ], [ %15, %14 ]
  %23 = phi ptr [ %17, %16 ], [ %0, %14 ]
  store ptr %1, ptr %22, align 8, !tbaa !10
  store ptr %1, ptr %23, align 8, !tbaa !10
  %24 = getelementptr inbounds i8, ptr %2, i64 40
  store atomic i32 2, ptr %24 release, align 8
  %25 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 26
  store ptr %3, ptr %25, align 8, !tbaa !411
  %26 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 10
  %27 = load ptr, ptr %26, align 8, !tbaa !275
  %28 = icmp eq ptr %27, %2
  %29 = icmp eq ptr %27, null
  %30 = or i1 %28, %29
  br i1 %30, label %31, label %33

31:                                               ; preds = %21
  %32 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 25
  store ptr %3, ptr %32, align 8, !tbaa !276
  br label %33

33:                                               ; preds = %21, %31
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_workers_reserve_now(ptr noundef %0) local_unnamed_addr #2 {
  %2 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i64 0, i32 1
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %3, label %5

3:                                                ; preds = %1
  %4 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %2) #45
  br label %5

5:                                                ; preds = %1, %3
  %6 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i64 0, i32 8
  %7 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i64 0, i32 9
  %8 = load i64, ptr %7, align 8, !tbaa !402
  %9 = icmp sgt i64 %8, 0
  br i1 %9, label %10, label %20

10:                                               ; preds = %5
  %11 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i64 0, i32 3
  br label %12

12:                                               ; preds = %10, %16
  %13 = phi i64 [ %8, %10 ], [ %18, %16 ]
  %14 = load i64, ptr %6, align 16, !tbaa !407
  %15 = icmp slt i64 %14, %13
  br i1 %15, label %20, label %16

16:                                               ; preds = %12
  %17 = tail call i32 @pthread_cond_wait(ptr noundef nonnull %11, ptr noundef nonnull %2) #45
  %18 = load i64, ptr %7, align 8, !tbaa !402
  %19 = icmp sgt i64 %18, 0
  br i1 %19, label %12, label %20, !llvm.loop !412

20:                                               ; preds = %12, %16, %5
  %21 = load i64, ptr %6, align 16, !tbaa !407
  %22 = add nsw i64 %21, 1
  store i64 %22, ptr %6, align 16, !tbaa !407
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %23, label %25

23:                                               ; preds = %20
  %24 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %2) #45
  br label %25

25:                                               ; preds = %20, %23
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_workers_unreserve(ptr noundef %0, ptr noundef %1) local_unnamed_addr #2 {
  %3 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i64 0, i32 1
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %4, label %6

4:                                                ; preds = %2
  %5 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %3) #45
  br label %6

6:                                                ; preds = %2, %4
  %7 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 9
  %8 = load i32, ptr %7, align 8, !tbaa !309
  %9 = icmp eq i32 %8, 1
  br i1 %9, label %10, label %11

10:                                               ; preds = %6
  tail call fastcc void @workers_room(ptr noundef %0) #44
  br label %28

11:                                               ; preds = %6
  %12 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i64 0, i32 11
  %13 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 2
  %14 = load ptr, ptr %13, align 8, !tbaa !409
  %15 = icmp eq ptr %14, %1
  br i1 %15, label %24, label %16

16:                                               ; preds = %11
  %17 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 3
  %18 = load ptr, ptr %17, align 8, !tbaa !410
  %19 = getelementptr inbounds %struct.FaWait, ptr %18, i64 0, i32 2
  store ptr %14, ptr %19, align 8, !tbaa !409
  %20 = load ptr, ptr %13, align 8, !tbaa !409
  %21 = getelementptr inbounds %struct.FaWait, ptr %20, i64 0, i32 3
  store ptr %18, ptr %21, align 8, !tbaa !410
  %22 = load ptr, ptr %12, align 8, !tbaa !10
  %23 = icmp eq ptr %22, %1
  br i1 %23, label %24, label %26

24:                                               ; preds = %16, %11
  %25 = phi ptr [ null, %11 ], [ %20, %16 ]
  store ptr %25, ptr %12, align 8, !tbaa !10
  br label %26

26:                                               ; preds = %16, %24
  store ptr null, ptr %13, align 8, !tbaa !409
  %27 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 3
  store ptr null, ptr %27, align 8, !tbaa !410
  br label %28

28:                                               ; preds = %26, %10
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %29, label %31

29:                                               ; preds = %28
  %30 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %3) #45
  br label %31

31:                                               ; preds = %28, %29
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @workers_room(ptr noundef %0) unnamed_addr #2 {
  %2 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i64 0, i32 11
  %3 = load ptr, ptr %2, align 8, !tbaa !10
  %4 = icmp eq ptr %3, null
  br i1 %4, label %22, label %5

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaWait, ptr %3, i64 0, i32 2
  %7 = load ptr, ptr %6, align 8, !tbaa !409
  %8 = icmp eq ptr %7, %3
  br i1 %8, label %17, label %9

9:                                                ; preds = %5
  %10 = getelementptr inbounds %struct.FaWait, ptr %3, i64 0, i32 3
  %11 = load ptr, ptr %10, align 8, !tbaa !410
  %12 = getelementptr inbounds %struct.FaWait, ptr %11, i64 0, i32 2
  store ptr %7, ptr %12, align 8, !tbaa !409
  %13 = load ptr, ptr %6, align 8, !tbaa !409
  %14 = getelementptr inbounds %struct.FaWait, ptr %13, i64 0, i32 3
  store ptr %11, ptr %14, align 8, !tbaa !410
  %15 = load ptr, ptr %2, align 8, !tbaa !10
  %16 = icmp eq ptr %15, %3
  br i1 %16, label %17, label %19

17:                                               ; preds = %9, %5
  %18 = phi ptr [ null, %5 ], [ %13, %9 ]
  store ptr %18, ptr %2, align 8, !tbaa !10
  br label %19

19:                                               ; preds = %17, %9
  store ptr null, ptr %6, align 8, !tbaa !409
  %20 = getelementptr inbounds %struct.FaWait, ptr %3, i64 0, i32 3
  store ptr null, ptr %20, align 8, !tbaa !410
  %21 = getelementptr inbounds %struct.FaWait, ptr %3, i64 0, i32 9
  store i32 1, ptr %21, align 8, !tbaa !309
  tail call fastcc void @wake(ptr noundef nonnull %3) #44
  br label %28

22:                                               ; preds = %1
  %23 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i64 0, i32 8
  %24 = load i64, ptr %23, align 16, !tbaa !407
  %25 = add nsw i64 %24, -1
  store i64 %25, ptr %23, align 16, !tbaa !407
  %26 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i64 0, i32 3
  %27 = tail call i32 @pthread_cond_broadcast(ptr noundef nonnull %26) #45
  br label %28

28:                                               ; preds = %22, %19
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_worker_start_in_pool(ptr noundef %0, ptr noundef %1) local_unnamed_addr #2 {
  %3 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 12
  store ptr %1, ptr %3, align 8, !tbaa !375
  br i1 icmp eq (ptr @pthread_create, ptr null), label %9, label %4

4:                                                ; preds = %2
  %5 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 8
  store i32 1, ptr %5, align 8, !tbaa !374
  %6 = atomicrmw add ptr @workers_alive, i64 1 acq_rel, align 8
  %7 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 7
  %8 = tail call i32 @pthread_create(ptr noundef nonnull %7, ptr noundef null, ptr noundef nonnull @pool_worker_thread, ptr noundef nonnull %0) #45
  br label %9

9:                                                ; preds = %2, %4
  ret void
}

; Function Attrs: nounwind optsize
define internal noundef ptr @pool_worker_thread(ptr noundef %0) #2 {
  %2 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 12
  %3 = load ptr, ptr %2, align 8, !tbaa !375
  %4 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @current_worker)
  store ptr %0, ptr %4, align 8, !tbaa !10
  %5 = tail call align 4 ptr @llvm.threadlocal.address.p0(ptr align 4 @fa_blocking_depth)
  store i32 1, ptr %5, align 4, !tbaa !25
  %6 = getelementptr inbounds %struct.FaWorkerPool, ptr %3, i64 0, i32 1
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %7, label %9

7:                                                ; preds = %1
  %8 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %6) #45
  br label %9

9:                                                ; preds = %1, %7
  %10 = getelementptr inbounds %struct.FaWorkerPool, ptr %3, i64 0, i32 6
  %11 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 9
  %12 = getelementptr inbounds %struct.FaWorkerPool, ptr %3, i64 0, i32 14
  %13 = getelementptr inbounds %struct.FaWorkerPool, ptr %3, i64 0, i32 2
  %14 = getelementptr inbounds %struct.FaWorkerPool, ptr %3, i64 0, i32 7
  br label %15

15:                                               ; preds = %90, %9
  %16 = load ptr, ptr %10, align 16, !tbaa !413
  %17 = icmp eq ptr %16, null
  br i1 %17, label %18, label %28

18:                                               ; preds = %15, %24
  %19 = load i32, ptr %11, align 4, !tbaa !380
  %20 = icmp eq i32 %19, 0
  br i1 %20, label %21, label %91

21:                                               ; preds = %18
  %22 = load i32, ptr %12, align 16, !tbaa !414
  %23 = icmp eq i32 %22, 0
  br i1 %23, label %24, label %91

24:                                               ; preds = %21
  %25 = tail call i32 @pthread_cond_wait(ptr noundef nonnull %13, ptr noundef nonnull %6) #45
  %26 = load ptr, ptr %10, align 16, !tbaa !413
  %27 = icmp eq ptr %26, null
  br i1 %27, label %18, label %28, !llvm.loop !415

28:                                               ; preds = %24, %15
  %29 = phi ptr [ %16, %15 ], [ %26, %24 ]
  %30 = load i32, ptr %11, align 4, !tbaa !380
  %31 = icmp eq i32 %30, 0
  br i1 %31, label %32, label %91

32:                                               ; preds = %28
  %33 = load ptr, ptr %29, align 8, !tbaa !376
  store ptr %33, ptr %10, align 16, !tbaa !413
  %34 = icmp eq ptr %33, null
  br i1 %34, label %35, label %36

35:                                               ; preds = %32
  store ptr null, ptr %14, align 8, !tbaa !416
  br label %36

36:                                               ; preds = %35, %32
  %37 = getelementptr inbounds %struct.FaCall, ptr %29, i64 0, i32 8
  store i32 1, ptr %37, align 8, !tbaa !391
  %38 = getelementptr inbounds %struct.FaCall, ptr %29, i64 0, i32 1
  store ptr %0, ptr %38, align 8, !tbaa !395
  %39 = tail call i64 @fa_clock() #45
  %40 = getelementptr inbounds %struct.FaCall, ptr %29, i64 0, i32 11
  store i64 %39, ptr %40, align 8, !tbaa !417
  tail call fastcc void @workers_room(ptr noundef nonnull %3) #44
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %41, label %43

41:                                               ; preds = %36
  %42 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %6) #45
  br label %43

43:                                               ; preds = %36, %41
  %44 = getelementptr inbounds %struct.FaCall, ptr %29, i64 0, i32 3
  %45 = load ptr, ptr %44, align 8, !tbaa !393
  %46 = getelementptr inbounds %struct.FaCall, ptr %29, i64 0, i32 6
  %47 = load ptr, ptr %46, align 8, !tbaa !383
  %48 = getelementptr inbounds %struct.FaCall, ptr %29, i64 0, i32 12
  %49 = load ptr, ptr %48, align 8, !tbaa !418
  tail call void %45(ptr noundef nonnull %0, ptr noundef %47, ptr noundef %49) #45
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %50, label %52

50:                                               ; preds = %43
  %51 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %6) #45
  br label %52

52:                                               ; preds = %43, %50
  %53 = load i32, ptr %37, align 8, !tbaa !391
  %54 = icmp eq i32 %53, 3
  br i1 %54, label %55, label %72

55:                                               ; preds = %52
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %56, label %58

56:                                               ; preds = %55
  %57 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %6) #45
  br label %58

58:                                               ; preds = %55, %56
  %59 = getelementptr inbounds %struct.FaCall, ptr %29, i64 0, i32 16
  %60 = load ptr, ptr %59, align 8, !tbaa !419
  %61 = load ptr, ptr %48, align 8, !tbaa !418
  tail call void %60(ptr noundef %61) #45
  %62 = getelementptr inbounds %struct.FaCall, ptr %29, i64 0, i32 9
  %63 = atomicrmw sub ptr %62, i32 1 acq_rel, align 4
  %64 = icmp eq i32 %63, 1
  br i1 %64, label %65, label %69

65:                                               ; preds = %58
  %66 = getelementptr inbounds %struct.FaCall, ptr %29, i64 0, i32 5
  %67 = load ptr, ptr %66, align 8, !tbaa !394
  %68 = getelementptr inbounds i8, ptr %29, i64 144
  tail call void %67(ptr noundef nonnull %68) #45
  br label %69

69:                                               ; preds = %58, %65
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %70, label %90

70:                                               ; preds = %69
  %71 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %6) #45
  br label %90

72:                                               ; preds = %52
  %73 = getelementptr inbounds i8, ptr %29, i64 192
  %74 = load ptr, ptr %48, align 8, !tbaa !418
  %75 = getelementptr inbounds %struct.FaCall, ptr %29, i64 0, i32 13
  %76 = load i64, ptr %75, align 8, !tbaa !420
  tail call fastcc void @copy_bytes(ptr noundef nonnull %73, ptr noundef %74, i64 noundef %76) #44
  %77 = getelementptr inbounds i8, ptr %29, i64 160
  %78 = getelementptr inbounds i8, ptr %29, i64 188
  %79 = load i32, ptr %78, align 4, !tbaa !267
  %80 = or i32 %79, 2
  store i32 %80, ptr %78, align 4, !tbaa !267
  store i32 2, ptr %37, align 8, !tbaa !391
  %81 = getelementptr inbounds %struct.FaCall, ptr %29, i64 0, i32 2
  %82 = load ptr, ptr %81, align 8, !tbaa !381
  tail call fastcc void @remote_push(ptr noundef %82, ptr noundef nonnull %77) #44
  tail call void @fa_remote_source_end(ptr noundef %82) #44
  %83 = getelementptr inbounds %struct.FaCall, ptr %29, i64 0, i32 9
  %84 = atomicrmw sub ptr %83, i32 1 acq_rel, align 4
  %85 = icmp eq i32 %84, 1
  br i1 %85, label %86, label %90

86:                                               ; preds = %72
  %87 = getelementptr inbounds i8, ptr %29, i64 144
  %88 = getelementptr inbounds %struct.FaCall, ptr %29, i64 0, i32 5
  %89 = load ptr, ptr %88, align 8, !tbaa !394
  tail call void %89(ptr noundef nonnull %87) #45
  br label %90

90:                                               ; preds = %86, %72, %70, %69
  br label %15

91:                                               ; preds = %28, %21, %18
  %92 = getelementptr inbounds %struct.FaWorker, ptr %0, i64 0, i32 11
  %93 = load i32, ptr %92, align 4, !tbaa !387
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %94, label %96

94:                                               ; preds = %91
  %95 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %6) #45
  br label %96

96:                                               ; preds = %91, %94
  tail call void @fa_release_builder() #44
  %97 = icmp eq i32 %93, 0
  br i1 %97, label %100, label %98

98:                                               ; preds = %96
  %99 = atomicrmw sub ptr @workers_alive, i64 1 acq_rel, align 8
  tail call void @fa_shared_release(ptr noundef nonnull %0) #44
  tail call void @fa_shared_release(ptr noundef nonnull %3) #44
  br label %100

100:                                              ; preds = %98, %96
  ret ptr null
}

; Function Attrs: nounwind optsize
define nonnull ptr @fa_job_new(i64 noundef %0, i64 noundef %1) local_unnamed_addr #2 {
  %3 = add i64 %0, 15
  %4 = and i64 %3, -16
  %5 = add i64 %1, 15
  %6 = and i64 %5, -16
  %7 = add i64 %4, 232
  %8 = add i64 %7, %6
  %9 = tail call ptr @fa_alloc(i64 noundef %8, i64 noundef 16) #45
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(192) %9, i8 0, i64 192, i1 false)
  %10 = getelementptr inbounds %struct.FaCall, ptr %9, i64 0, i32 7
  store i64 %8, ptr %10, align 8, !tbaa !384
  %11 = getelementptr inbounds i8, ptr %9, i64 144
  %12 = getelementptr inbounds i8, ptr %11, i64 %4
  %13 = getelementptr inbounds %struct.FaCall, ptr %9, i64 0, i32 12
  store ptr %12, ptr %13, align 8, !tbaa !418
  %14 = getelementptr inbounds i8, ptr %12, i64 %6
  %15 = getelementptr inbounds %struct.FaCall, ptr %9, i64 0, i32 14
  store ptr %14, ptr %15, align 8, !tbaa !421
  %16 = getelementptr inbounds %struct.FaWait, ptr %14, i64 0, i32 8
  store i64 -1, ptr %16, align 8, !tbaa !289
  %17 = getelementptr inbounds %struct.FaWait, ptr %14, i64 0, i32 9
  store i32 2, ptr %17, align 8, !tbaa !309
  %18 = getelementptr inbounds %struct.FaCall, ptr %9, i64 0, i32 13
  store i64 %1, ptr %18, align 8, !tbaa !420
  %19 = getelementptr inbounds %struct.FaCall, ptr %9, i64 0, i32 9
  store i32 2, ptr %19, align 4, !tbaa !422
  ret ptr %11
}

; Function Attrs: nounwind optsize
define void @fa_workers_enqueue(ptr noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef %3, ptr noundef %4, ptr noundef %5, ptr noundef %6, ptr noundef %7) local_unnamed_addr #2 {
  %9 = getelementptr inbounds i8, ptr %1, i64 -144
  %10 = tail call fastcc ptr @executor() #44
  %11 = getelementptr inbounds i8, ptr %1, i64 -72
  store ptr %0, ptr %11, align 8, !tbaa !423
  %12 = getelementptr inbounds i8, ptr %1, i64 -120
  store ptr %2, ptr %12, align 8, !tbaa !393
  %13 = getelementptr inbounds i8, ptr %1, i64 -96
  store ptr %3, ptr %13, align 8, !tbaa !383
  %14 = getelementptr inbounds i8, ptr %1, i64 -112
  store ptr %4, ptr %14, align 8, !tbaa !382
  %15 = getelementptr inbounds i8, ptr %1, i64 -104
  store ptr %5, ptr %15, align 8, !tbaa !394
  %16 = getelementptr inbounds i8, ptr %1, i64 -32
  store ptr %6, ptr %16, align 8, !tbaa !424
  %17 = getelementptr inbounds i8, ptr %1, i64 -24
  store ptr %7, ptr %17, align 8, !tbaa !419
  %18 = getelementptr inbounds i8, ptr %1, i64 -128
  store ptr %10, ptr %18, align 8, !tbaa !381
  %19 = icmp eq ptr %0, null
  br i1 %19, label %22, label %20

20:                                               ; preds = %8
  %21 = atomicrmw add ptr %0, i64 1 monotonic, align 8
  br label %22

22:                                               ; preds = %8, %20
  br i1 icmp eq (ptr @pthread_create, ptr null), label %23, label %42

23:                                               ; preds = %22
  %24 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i64 0, i32 4
  %25 = load ptr, ptr %24, align 16, !tbaa !403
  %26 = load ptr, ptr %25, align 8, !tbaa !10
  %27 = getelementptr inbounds i8, ptr %1, i64 -56
  %28 = load ptr, ptr %27, align 8, !tbaa !418
  tail call void %2(ptr noundef %26, ptr noundef %3, ptr noundef %28) #45
  %29 = getelementptr inbounds i8, ptr %1, i64 48
  %30 = load ptr, ptr %27, align 8, !tbaa !418
  %31 = getelementptr inbounds i8, ptr %1, i64 -48
  %32 = load i64, ptr %31, align 8, !tbaa !420
  tail call fastcc void @copy_bytes(ptr noundef nonnull %29, ptr noundef %30, i64 noundef %32) #44
  %33 = getelementptr inbounds i8, ptr %1, i64 44
  %34 = load i32, ptr %33, align 4, !tbaa !267
  %35 = or i32 %34, 2
  store i32 %35, ptr %33, align 4, !tbaa !267
  %36 = getelementptr inbounds i8, ptr %1, i64 -80
  store i32 2, ptr %36, align 8, !tbaa !391
  store ptr null, ptr %1, align 8, !tbaa !10
  %37 = getelementptr inbounds i8, ptr %1, i64 40
  store atomic i32 3, ptr %37 release, align 8
  %38 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i64 0, i32 8
  %39 = load i64, ptr %38, align 16, !tbaa !407
  %40 = add nsw i64 %39, -1
  store i64 %40, ptr %38, align 16, !tbaa !407
  %41 = getelementptr inbounds i8, ptr %1, i64 -76
  store i32 1, ptr %41, align 4, !tbaa !422
  br label %77

42:                                               ; preds = %22
  %43 = getelementptr inbounds %struct.FaExecutor, ptr %10, i64 0, i32 12
  %44 = atomicrmw add ptr %43, i64 1 acq_rel, align 8
  %45 = getelementptr inbounds i8, ptr %1, i64 40
  store atomic i32 2, ptr %45 release, align 8
  %46 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i64 0, i32 1
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %47, label %49

47:                                               ; preds = %42
  %48 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %46) #45
  br label %49

49:                                               ; preds = %42, %47
  %50 = getelementptr inbounds i8, ptr %1, i64 -80
  store i32 0, ptr %50, align 8, !tbaa !391
  store ptr null, ptr %9, align 8, !tbaa !376
  %51 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i64 0, i32 7
  %52 = load ptr, ptr %51, align 8, !tbaa !416
  %53 = icmp eq ptr %52, null
  br i1 %53, label %55, label %54

54:                                               ; preds = %49
  store ptr %9, ptr %52, align 8, !tbaa !376
  br label %57

55:                                               ; preds = %49
  %56 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i64 0, i32 6
  store ptr %9, ptr %56, align 16, !tbaa !413
  br label %57

57:                                               ; preds = %55, %54
  store ptr %9, ptr %51, align 8, !tbaa !416
  %58 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i64 0, i32 2
  %59 = tail call i32 @pthread_cond_signal(ptr noundef nonnull %58) #45
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %60, label %62

60:                                               ; preds = %57
  %61 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %46) #45
  br label %62

62:                                               ; preds = %57, %60
  %63 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i64 0, i32 10
  %64 = load i64, ptr %63, align 16, !tbaa !401
  %65 = icmp sgt i64 %64, 0
  br i1 %65, label %66, label %77

66:                                               ; preds = %62
  %67 = tail call i64 @fa_clock() #45
  %68 = load i64, ptr %63, align 16, !tbaa !401
  %69 = mul nsw i64 %68, 1000000
  %70 = add nsw i64 %69, %67
  %71 = getelementptr inbounds i8, ptr %1, i64 -40
  %72 = load ptr, ptr %71, align 8, !tbaa !421
  store ptr %1, ptr %72, align 8, !tbaa !286
  %73 = getelementptr inbounds %struct.FaWait, ptr %72, i64 0, i32 4
  store ptr %9, ptr %73, align 8, !tbaa !425
  %74 = getelementptr inbounds %struct.FaWait, ptr %72, i64 0, i32 11
  store ptr @job_timer_fired, ptr %74, align 8, !tbaa !290
  %75 = getelementptr inbounds %struct.FaWait, ptr %72, i64 0, i32 9
  store i32 0, ptr %75, align 8, !tbaa !309
  %76 = load ptr, ptr %18, align 8, !tbaa !381
  tail call fastcc void @timer_insert(ptr noundef %76, ptr noundef nonnull %72, i64 noundef %70) #44
  br label %77

77:                                               ; preds = %62, %66, %23
  ret void
}

; Function Attrs: nofree norecurse nounwind optsize memory(readwrite, inaccessiblemem: none)
define void @fa_job_resume(ptr noundef %0) local_unnamed_addr #38 {
  %2 = getelementptr inbounds i8, ptr %0, i64 -144
  tail call fastcc void @job_disarm(ptr noundef nonnull %2) #44
  store ptr null, ptr %0, align 8, !tbaa !10
  tail call void @fa_task_finish(ptr noundef nonnull %0) #44
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc void @job_disarm(ptr nocapture noundef readonly %0) unnamed_addr #17 {
  %2 = getelementptr inbounds %struct.FaCall, ptr %0, i64 0, i32 14
  %3 = load ptr, ptr %2, align 8, !tbaa !421
  %4 = getelementptr inbounds %struct.FaWait, ptr %3, i64 0, i32 9
  %5 = load i32, ptr %4, align 8, !tbaa !309
  %6 = icmp eq i32 %5, 0
  br i1 %6, label %7, label %14

7:                                                ; preds = %1
  %8 = getelementptr inbounds %struct.FaWait, ptr %3, i64 0, i32 8
  %9 = load i64, ptr %8, align 8, !tbaa !289
  %10 = icmp sgt i64 %9, -1
  br i1 %10, label %11, label %14

11:                                               ; preds = %7
  %12 = getelementptr inbounds %struct.FaWait, ptr %3, i64 0, i32 1
  %13 = load ptr, ptr %12, align 8, !tbaa !288
  tail call fastcc void @timer_remove(ptr noundef %13, i64 noundef %9) #44
  br label %14

14:                                               ; preds = %11, %7, %1
  store i32 2, ptr %4, align 8, !tbaa !309
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_job_destroy(ptr noundef %0) local_unnamed_addr #2 {
  %2 = getelementptr inbounds i8, ptr %0, i64 -144
  %3 = getelementptr inbounds i8, ptr %0, i64 -72
  %4 = load ptr, ptr %3, align 8, !tbaa !423
  tail call fastcc void @job_disarm(ptr noundef nonnull %2) #44
  %5 = getelementptr inbounds %struct.FaWorkerPool, ptr %4, i64 0, i32 1
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %6, label %8

6:                                                ; preds = %1
  %7 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %5) #45
  br label %8

8:                                                ; preds = %1, %6
  %9 = getelementptr inbounds i8, ptr %0, i64 -80
  %10 = load i32, ptr %9, align 8, !tbaa !391
  switch i32 %10, label %46 [
    i32 0, label %11
    i32 1, label %40
  ]

11:                                               ; preds = %8
  %12 = getelementptr inbounds %struct.FaWorkerPool, ptr %4, i64 0, i32 6
  br label %13

13:                                               ; preds = %13, %11
  %14 = phi ptr [ null, %11 ], [ %16, %13 ]
  %15 = phi ptr [ %12, %11 ], [ %16, %13 ]
  %16 = load ptr, ptr %15, align 8, !tbaa !10
  %17 = icmp eq ptr %16, %2
  br i1 %17, label %18, label %13, !llvm.loop !426

18:                                               ; preds = %13
  %19 = load ptr, ptr %2, align 8, !tbaa !376
  store ptr %19, ptr %15, align 8, !tbaa !10
  %20 = getelementptr inbounds %struct.FaWorkerPool, ptr %4, i64 0, i32 7
  %21 = load ptr, ptr %20, align 8, !tbaa !416
  %22 = icmp eq ptr %21, %2
  br i1 %22, label %23, label %24

23:                                               ; preds = %18
  store ptr %14, ptr %20, align 8, !tbaa !416
  br label %24

24:                                               ; preds = %23, %18
  tail call fastcc void @workers_room(ptr noundef nonnull %4) #44
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %25, label %27

25:                                               ; preds = %24
  %26 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %5) #45
  br label %27

27:                                               ; preds = %24, %25
  %28 = getelementptr inbounds i8, ptr %0, i64 -112
  %29 = load ptr, ptr %28, align 8, !tbaa !382
  %30 = getelementptr inbounds i8, ptr %0, i64 -96
  %31 = load ptr, ptr %30, align 8, !tbaa !383
  tail call void %29(ptr noundef %31) #45
  %32 = getelementptr inbounds i8, ptr %0, i64 -128
  %33 = load ptr, ptr %32, align 8, !tbaa !381
  tail call void @fa_remote_source_end(ptr noundef %33) #44
  %34 = getelementptr inbounds i8, ptr %0, i64 -76
  %35 = atomicrmw sub ptr %34, i32 1 acq_rel, align 4
  %36 = icmp eq i32 %35, 1
  br i1 %36, label %37, label %66

37:                                               ; preds = %27
  %38 = getelementptr inbounds i8, ptr %0, i64 -104
  %39 = load ptr, ptr %38, align 8, !tbaa !394
  tail call void %39(ptr noundef nonnull %0) #45
  br label %66

40:                                               ; preds = %8
  store i32 3, ptr %9, align 8, !tbaa !391
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %41, label %43

41:                                               ; preds = %40
  %42 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %5) #45
  br label %43

43:                                               ; preds = %40, %41
  %44 = getelementptr inbounds i8, ptr %0, i64 -128
  %45 = load ptr, ptr %44, align 8, !tbaa !381
  tail call void @fa_remote_source_end(ptr noundef %45) #44
  br label %66

46:                                               ; preds = %8
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %47, label %49

47:                                               ; preds = %46
  %48 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %5) #45
  br label %49

49:                                               ; preds = %46, %47
  %50 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @current_executor)
  %51 = load ptr, ptr %50, align 8, !tbaa !10
  %52 = getelementptr inbounds i8, ptr %0, i64 16
  %53 = icmp eq ptr %51, null
  br i1 %53, label %66, label %54

54:                                               ; preds = %49
  %55 = getelementptr inbounds i8, ptr %0, i64 -128
  %56 = load ptr, ptr %55, align 8, !tbaa !381
  %57 = icmp eq ptr %56, %51
  br i1 %57, label %58, label %66

58:                                               ; preds = %54
  %59 = getelementptr inbounds i8, ptr %0, i64 40
  %60 = load atomic i32, ptr %59 acquire, align 8
  %61 = icmp eq i32 %60, 5
  br i1 %61, label %62, label %66

62:                                               ; preds = %58
  tail call fastcc void @drain_remote(ptr noundef nonnull %51) #44
  %63 = load atomic i32, ptr %59 acquire, align 8
  %64 = icmp eq i32 %63, 1
  br i1 %64, label %65, label %66

65:                                               ; preds = %62
  tail call fastcc void @ready_unlink(ptr noundef nonnull %51, ptr noundef nonnull %52) #44
  br label %66

66:                                               ; preds = %37, %27, %49, %54, %58, %65, %62, %43
  %67 = getelementptr inbounds i8, ptr %0, i64 -76
  %68 = atomicrmw sub ptr %67, i32 1 acq_rel, align 4
  %69 = icmp eq i32 %68, 1
  br i1 %69, label %70, label %73

70:                                               ; preds = %66
  %71 = getelementptr inbounds i8, ptr %0, i64 -104
  %72 = load ptr, ptr %71, align 8, !tbaa !394
  tail call void %72(ptr noundef nonnull %0) #45
  br label %73

73:                                               ; preds = %66, %70
  tail call void @fa_shared_release(ptr noundef %4) #44
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_workers_shutdown(ptr noundef %0) local_unnamed_addr #2 {
  tail call fastcc void @workers_shut(ptr noundef %0) #44
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @workers_shut(ptr noundef %0) unnamed_addr #2 {
  %2 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i64 0, i32 14
  %3 = load i32, ptr %2, align 16, !tbaa !414
  %4 = icmp eq i32 %3, 0
  br i1 %4, label %5, label %28

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i64 0, i32 1
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %7, label %9

7:                                                ; preds = %5
  %8 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %6) #45
  br label %9

9:                                                ; preds = %5, %7
  store i32 1, ptr %2, align 16, !tbaa !414
  %10 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i64 0, i32 2
  %11 = tail call i32 @pthread_cond_broadcast(ptr noundef nonnull %10) #45
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %12, label %14

12:                                               ; preds = %9
  %13 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %6) #45
  br label %14

14:                                               ; preds = %9, %12
  %15 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i64 0, i32 5
  %16 = load i64, ptr %15, align 8, !tbaa !404
  %17 = icmp sgt i64 %16, 0
  br i1 %17, label %18, label %28

18:                                               ; preds = %14
  %19 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i64 0, i32 4
  br label %20

20:                                               ; preds = %18, %20
  %21 = phi i64 [ 0, %18 ], [ %25, %20 ]
  %22 = load ptr, ptr %19, align 16, !tbaa !403
  %23 = getelementptr inbounds ptr, ptr %22, i64 %21
  %24 = load ptr, ptr %23, align 8, !tbaa !10
  tail call void @fa_worker_stop(ptr noundef %24) #44
  %25 = add nuw nsw i64 %21, 1
  %26 = load i64, ptr %15, align 8, !tbaa !404
  %27 = icmp slt i64 %25, %26
  br i1 %27, label %20, label %28, !llvm.loop !427

28:                                               ; preds = %20, %14, %1
  ret void
}

; Function Attrs: cold mustprogress nofree noinline norecurse nosync nounwind optsize willreturn memory(none)
define noundef nonnull ptr @fa_error_worker_timeout() local_unnamed_addr #32 {
  ret ptr @worker_timeout
}

; Function Attrs: nounwind optsize
define ptr @fa_mutex_new(i64 noundef %0, ptr noundef %1) local_unnamed_addr #2 {
  %3 = add i64 %0, 176
  %4 = tail call fastcc ptr @shared_new(i64 noundef %3, ptr noundef %1) #44
  ret ptr %4
}

; Function Attrs: nounwind optsize
define noundef ptr @fa_mutex_lock(ptr noundef returned %0) local_unnamed_addr #2 {
  %2 = icmp eq ptr %0, null
  br i1 %2, label %5, label %3

3:                                                ; preds = %1
  %4 = atomicrmw add ptr %0, i64 1 monotonic, align 8
  br label %5

5:                                                ; preds = %1, %3
  %6 = getelementptr inbounds %struct.FaMutex, ptr %0, i64 0, i32 1
  %7 = atomicrmw xchg ptr %6, i32 1 acquire, align 16
  %8 = icmp eq i32 %7, 0
  br i1 %8, label %42, label %9

9:                                                ; preds = %5
  br i1 icmp eq (ptr @pthread_create, ptr null), label %20, label %10

10:                                               ; preds = %9
  %11 = tail call align 4 ptr @llvm.threadlocal.address.p0(ptr align 4 @in_parallel)
  %12 = load i32, ptr %11, align 4, !tbaa !25
  %13 = icmp eq i32 %12, 0
  br i1 %13, label %14, label %21

14:                                               ; preds = %10
  %15 = load atomic i32, ptr getelementptr inbounds (%struct.anon.0, ptr @pool, i64 0, i32 5) acquire, align 4
  %16 = icmp eq i32 %15, 0
  br i1 %16, label %17, label %21

17:                                               ; preds = %14
  %18 = load atomic i64, ptr @workers_alive acquire, align 8
  %19 = icmp slt i64 %18, 1
  br i1 %19, label %20, label %21

20:                                               ; preds = %9, %17
  tail call fastcc void @deadlock(ptr noundef nonnull @.str.32) #49
  unreachable

21:                                               ; preds = %10, %14, %17
  %22 = getelementptr inbounds %struct.FaMutex, ptr %0, i64 0, i32 4
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %23, label %25

23:                                               ; preds = %21
  %24 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %22) #45
  br label %25

25:                                               ; preds = %21, %23
  %26 = getelementptr inbounds %struct.FaMutex, ptr %0, i64 0, i32 2
  %27 = load i32, ptr %26, align 4, !tbaa !25
  %28 = add nsw i32 %27, 1
  store i32 %28, ptr %26, align 4, !tbaa !25
  %29 = atomicrmw xchg ptr %6, i32 1 acquire, align 16
  %30 = icmp eq i32 %29, 0
  br i1 %30, label %37, label %31

31:                                               ; preds = %25
  %32 = getelementptr inbounds %struct.FaMutex, ptr %0, i64 0, i32 5
  br label %33

33:                                               ; preds = %31, %33
  %34 = tail call i32 @pthread_cond_wait(ptr noundef nonnull %32, ptr noundef nonnull %22) #45
  %35 = atomicrmw xchg ptr %6, i32 1 acquire, align 16
  %36 = icmp eq i32 %35, 0
  br i1 %36, label %37, label %33, !llvm.loop !428

37:                                               ; preds = %33, %25
  %38 = load i32, ptr %26, align 4, !tbaa !25
  %39 = add nsw i32 %38, -1
  store i32 %39, ptr %26, align 4, !tbaa !25
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %40, label %42

40:                                               ; preds = %37
  %41 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %22) #45
  br label %42

42:                                               ; preds = %40, %37, %5
  ret ptr %0
}

; Function Attrs: nounwind optsize
define noundef i32 @fa_mutex_lock_async(ptr noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef %3) local_unnamed_addr #2 {
  %5 = icmp eq ptr %0, null
  br i1 %5, label %8, label %6

6:                                                ; preds = %4
  %7 = atomicrmw add ptr %0, i64 1 monotonic, align 8
  br label %8

8:                                                ; preds = %4, %6
  %9 = getelementptr inbounds %struct.FaMutex, ptr %0, i64 0, i32 4
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %10, label %12

10:                                               ; preds = %8
  %11 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %9) #45
  br label %12

12:                                               ; preds = %8, %10
  %13 = getelementptr inbounds %struct.FaMutex, ptr %0, i64 0, i32 3
  %14 = load ptr, ptr %13, align 8, !tbaa !10
  %15 = icmp eq ptr %14, null
  br i1 %15, label %16, label %55

16:                                               ; preds = %12
  %17 = getelementptr inbounds %struct.FaMutex, ptr %0, i64 0, i32 1
  %18 = atomicrmw xchg ptr %17, i32 1 acquire, align 16
  %19 = icmp eq i32 %18, 0
  br i1 %19, label %20, label %55

20:                                               ; preds = %16
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %21, label %23

21:                                               ; preds = %20
  %22 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %9) #45
  br label %23

23:                                               ; preds = %20, %21
  %24 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 9
  store i32 1, ptr %24, align 8, !tbaa !309
  %25 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 10
  store i32 0, ptr %25, align 4, !tbaa !310
  %26 = tail call align 4 ptr @llvm.threadlocal.address.p0(ptr align 4 @fa_coop_budget)
  %27 = load i32, ptr %26, align 4, !tbaa !25
  %28 = add nsw i32 %27, -1
  store i32 %28, ptr %26, align 4, !tbaa !25
  %29 = icmp sgt i32 %27, 1
  br i1 %29, label %58, label %30

30:                                               ; preds = %23
  %31 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @current_executor)
  %32 = load ptr, ptr %31, align 8, !tbaa !10
  %33 = icmp eq ptr %32, null
  br i1 %33, label %36, label %34

34:                                               ; preds = %30
  %35 = getelementptr inbounds %struct.FaExecutor, ptr %32, i64 0, i32 23
  store i32 1, ptr %35, align 8, !tbaa !284
  br label %36

36:                                               ; preds = %34, %30
  %37 = tail call fastcc ptr @executor() #44
  %38 = getelementptr inbounds %struct.FaExecutor, ptr %37, i64 0, i32 23
  store i32 1, ptr %38, align 8, !tbaa !284
  %39 = getelementptr inbounds i8, ptr %2, i64 16
  %40 = getelementptr inbounds i8, ptr %2, i64 44
  %41 = load i32, ptr %40, align 4, !tbaa !267
  %42 = or i32 %41, 4
  store i32 %42, ptr %40, align 4, !tbaa !267
  %43 = getelementptr inbounds %struct.FaExecutor, ptr %37, i64 0, i32 2
  %44 = getelementptr inbounds %struct.FaExecutor, ptr %37, i64 0, i32 3
  %45 = getelementptr inbounds i8, ptr %2, i64 24
  store ptr null, ptr %45, align 8, !tbaa !269
  %46 = load ptr, ptr %44, align 8, !tbaa !10
  %47 = getelementptr inbounds i8, ptr %2, i64 32
  store ptr %46, ptr %47, align 8, !tbaa !270
  %48 = icmp eq ptr %46, null
  %49 = getelementptr inbounds %struct.FaTask, ptr %46, i64 0, i32 1
  %50 = select i1 %48, ptr %43, ptr %49
  store ptr %39, ptr %50, align 8, !tbaa !10
  store ptr %39, ptr %44, align 8, !tbaa !10
  %51 = getelementptr inbounds %struct.FaExecutor, ptr %37, i64 0, i32 4
  %52 = load i64, ptr %51, align 8, !tbaa !283
  %53 = add nsw i64 %52, 1
  store i64 %53, ptr %51, align 8, !tbaa !283
  %54 = getelementptr inbounds i8, ptr %2, i64 40
  store atomic i32 1, ptr %54 release, align 8
  br label %58

55:                                               ; preds = %16, %12
  tail call fastcc void @line_push(ptr noundef nonnull %13, ptr noundef %1, ptr noundef %2, ptr noundef %3) #44
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %56, label %58

56:                                               ; preds = %55
  %57 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %9) #45
  br label %58

58:                                               ; preds = %56, %55, %23, %36
  %59 = phi i32 [ 2, %36 ], [ 0, %23 ], [ 1, %55 ], [ 1, %56 ]
  ret i32 %59
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(argmem: readwrite)
define noundef ptr @fa_mutex_try_lock(ptr noundef %0) local_unnamed_addr #36 {
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
define void @fa_mutex_unlock(ptr noundef %0) local_unnamed_addr #2 {
  %2 = icmp eq ptr %0, null
  br i1 %2, label %40, label %3

3:                                                ; preds = %1
  %4 = getelementptr inbounds %struct.FaMutex, ptr %0, i64 0, i32 4
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %5, label %7

5:                                                ; preds = %3
  %6 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %4) #45
  br label %7

7:                                                ; preds = %3, %5
  %8 = getelementptr inbounds %struct.FaMutex, ptr %0, i64 0, i32 3
  %9 = load ptr, ptr %8, align 8, !tbaa !10
  %10 = icmp eq ptr %9, null
  br i1 %10, label %28, label %11

11:                                               ; preds = %7
  %12 = getelementptr inbounds %struct.FaWait, ptr %9, i64 0, i32 2
  %13 = load ptr, ptr %12, align 8, !tbaa !409
  %14 = icmp eq ptr %13, %9
  br i1 %14, label %23, label %15

15:                                               ; preds = %11
  %16 = getelementptr inbounds %struct.FaWait, ptr %9, i64 0, i32 3
  %17 = load ptr, ptr %16, align 8, !tbaa !410
  %18 = getelementptr inbounds %struct.FaWait, ptr %17, i64 0, i32 2
  store ptr %13, ptr %18, align 8, !tbaa !409
  %19 = load ptr, ptr %12, align 8, !tbaa !409
  %20 = getelementptr inbounds %struct.FaWait, ptr %19, i64 0, i32 3
  store ptr %17, ptr %20, align 8, !tbaa !410
  %21 = load ptr, ptr %8, align 8, !tbaa !10
  %22 = icmp eq ptr %21, %9
  br i1 %22, label %23, label %25

23:                                               ; preds = %15, %11
  %24 = phi ptr [ null, %11 ], [ %19, %15 ]
  store ptr %24, ptr %8, align 8, !tbaa !10
  br label %25

25:                                               ; preds = %23, %15
  store ptr null, ptr %12, align 8, !tbaa !409
  %26 = getelementptr inbounds %struct.FaWait, ptr %9, i64 0, i32 3
  store ptr null, ptr %26, align 8, !tbaa !410
  %27 = getelementptr inbounds %struct.FaWait, ptr %9, i64 0, i32 9
  store i32 1, ptr %27, align 8, !tbaa !309
  tail call fastcc void @wake(ptr noundef nonnull %9) #44
  br label %36

28:                                               ; preds = %7
  %29 = getelementptr inbounds %struct.FaMutex, ptr %0, i64 0, i32 1
  store atomic i32 0, ptr %29 release, align 16
  %30 = getelementptr inbounds %struct.FaMutex, ptr %0, i64 0, i32 2
  %31 = load i32, ptr %30, align 4, !tbaa !25
  %32 = icmp sgt i32 %31, 0
  br i1 %32, label %33, label %36

33:                                               ; preds = %28
  %34 = getelementptr inbounds %struct.FaMutex, ptr %0, i64 0, i32 5
  %35 = tail call i32 @pthread_cond_signal(ptr noundef nonnull %34) #45
  br label %36

36:                                               ; preds = %28, %33, %25
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %37, label %39

37:                                               ; preds = %36
  %38 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %4) #45
  br label %39

39:                                               ; preds = %36, %37
  tail call void @fa_shared_release(ptr noundef nonnull %0) #44
  br label %40

40:                                               ; preds = %1, %39
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @wake(ptr nocapture noundef %0) unnamed_addr #2 {
  %2 = load ptr, ptr %0, align 8, !tbaa !286
  %3 = getelementptr inbounds i8, ptr %2, i64 16
  %4 = getelementptr inbounds %struct.FaWait, ptr %0, i64 0, i32 1
  %5 = load ptr, ptr %4, align 8, !tbaa !288
  %6 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @current_executor)
  %7 = load ptr, ptr %6, align 8, !tbaa !10
  %8 = icmp eq ptr %5, %7
  br i1 %8, label %9, label %24

9:                                                ; preds = %1
  %10 = getelementptr inbounds i8, ptr %2, i64 44
  %11 = load i32, ptr %10, align 4, !tbaa !267
  %12 = and i32 %11, -5
  store i32 %12, ptr %10, align 4, !tbaa !267
  %13 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 1
  %14 = getelementptr inbounds i8, ptr %2, i64 24
  store ptr null, ptr %14, align 8, !tbaa !269
  %15 = load ptr, ptr %13, align 8, !tbaa !10
  %16 = getelementptr inbounds i8, ptr %2, i64 32
  store ptr %15, ptr %16, align 8, !tbaa !270
  %17 = icmp eq ptr %15, null
  %18 = getelementptr inbounds %struct.FaTask, ptr %15, i64 0, i32 1
  %19 = select i1 %17, ptr %5, ptr %18
  store ptr %3, ptr %19, align 8, !tbaa !10
  store ptr %3, ptr %13, align 8, !tbaa !10
  %20 = getelementptr inbounds %struct.FaExecutor, ptr %5, i64 0, i32 4
  %21 = load <2 x i64>, ptr %20, align 8, !tbaa !8
  %22 = add nsw <2 x i64> %21, <i64 1, i64 1>
  store <2 x i64> %22, ptr %20, align 8, !tbaa !8
  %23 = getelementptr inbounds i8, ptr %2, i64 40
  store atomic i32 1, ptr %23 release, align 8
  br label %26

24:                                               ; preds = %1
  %25 = getelementptr inbounds %struct.FaWait, ptr %0, i64 0, i32 10
  store i32 1, ptr %25, align 4, !tbaa !310
  tail call fastcc void @remote_push(ptr noundef %5, ptr noundef nonnull %3) #44
  br label %26

26:                                               ; preds = %24, %9
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_mutex_cancel(ptr noundef %0, ptr noundef %1) local_unnamed_addr #2 {
  %3 = getelementptr inbounds %struct.FaMutex, ptr %0, i64 0, i32 4
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %4, label %6

4:                                                ; preds = %2
  %5 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %3) #45
  br label %6

6:                                                ; preds = %2, %4
  %7 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 9
  %8 = load i32, ptr %7, align 8, !tbaa !309
  %9 = icmp eq i32 %8, 0
  br i1 %9, label %10, label %30

10:                                               ; preds = %6
  %11 = getelementptr inbounds %struct.FaMutex, ptr %0, i64 0, i32 3
  %12 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 2
  %13 = load ptr, ptr %12, align 8, !tbaa !409
  %14 = icmp eq ptr %13, %1
  br i1 %14, label %23, label %15

15:                                               ; preds = %10
  %16 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 3
  %17 = load ptr, ptr %16, align 8, !tbaa !410
  %18 = getelementptr inbounds %struct.FaWait, ptr %17, i64 0, i32 2
  store ptr %13, ptr %18, align 8, !tbaa !409
  %19 = load ptr, ptr %12, align 8, !tbaa !409
  %20 = getelementptr inbounds %struct.FaWait, ptr %19, i64 0, i32 3
  store ptr %17, ptr %20, align 8, !tbaa !410
  %21 = load ptr, ptr %11, align 8, !tbaa !10
  %22 = icmp eq ptr %21, %1
  br i1 %22, label %23, label %25

23:                                               ; preds = %15, %10
  %24 = phi ptr [ null, %10 ], [ %19, %15 ]
  store ptr %24, ptr %11, align 8, !tbaa !10
  br label %25

25:                                               ; preds = %15, %23
  store ptr null, ptr %12, align 8, !tbaa !409
  %26 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 3
  store ptr null, ptr %26, align 8, !tbaa !410
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %27, label %29

27:                                               ; preds = %25
  %28 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %3) #45
  br label %29

29:                                               ; preds = %25, %27
  tail call void @fa_shared_release(ptr noundef nonnull %0) #44
  br label %34

30:                                               ; preds = %6
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %31, label %33

31:                                               ; preds = %30
  %32 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %3) #45
  br label %33

33:                                               ; preds = %30, %31
  tail call fastcc void @forget_remote_wake(ptr noundef nonnull %1) #44
  tail call void @fa_mutex_unlock(ptr noundef %0) #44
  br label %34

34:                                               ; preds = %33, %29
  ret void
}

; Function Attrs: nounwind optsize
define ptr @fa_channel_new(i64 noundef %0, i64 noundef %1, ptr noundef %2) local_unnamed_addr #2 {
  %4 = tail call fastcc ptr @shared_new(i64 noundef 240, ptr noundef nonnull @channel_drop) #44
  %5 = tail call i64 @llvm.umin.i64(i64 %0, i64 9223372036854775807)
  %6 = getelementptr inbounds %struct.FaChannel, ptr %4, i64 0, i32 3
  store i64 %5, ptr %6, align 16, !tbaa !429
  %7 = tail call i64 @llvm.smax.i64(i64 %1, i64 1)
  %8 = getelementptr inbounds %struct.FaChannel, ptr %4, i64 0, i32 4
  store i64 %7, ptr %8, align 8, !tbaa !431
  %9 = getelementptr inbounds %struct.FaChannel, ptr %4, i64 0, i32 11
  store ptr %2, ptr %9, align 8, !tbaa !432
  ret ptr %4
}

; Function Attrs: nounwind optsize
define internal void @channel_drop(ptr nocapture noundef readonly %0) #2 {
  %2 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 7
  %3 = load i64, ptr %2, align 16, !tbaa !433
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
  %13 = load i64, ptr %12, align 8, !tbaa !434
  %14 = icmp sgt i64 %13, 0
  br i1 %14, label %34, label %40

15:                                               ; preds = %5, %30
  %16 = phi i64 [ %3, %5 ], [ %31, %30 ]
  %17 = phi i64 [ 0, %5 ], [ %32, %30 ]
  %18 = load ptr, ptr %6, align 8, !tbaa !432
  %19 = icmp eq ptr %18, null
  br i1 %19, label %30, label %20

20:                                               ; preds = %15
  %21 = load i64, ptr %7, align 8, !tbaa !435
  %22 = add nsw i64 %21, %17
  %23 = load i64, ptr %8, align 8, !tbaa !434
  %24 = srem i64 %22, %23
  %25 = load ptr, ptr %9, align 16, !tbaa !436
  %26 = load i64, ptr %10, align 8, !tbaa !431
  %27 = mul nsw i64 %26, %24
  %28 = getelementptr inbounds i8, ptr %25, i64 %27
  tail call void %18(ptr noundef %28) #45
  %29 = load i64, ptr %2, align 16, !tbaa !433
  br label %30

30:                                               ; preds = %20, %15
  %31 = phi i64 [ %29, %20 ], [ %16, %15 ]
  %32 = add nuw nsw i64 %17, 1
  %33 = icmp slt i64 %32, %31
  br i1 %33, label %15, label %11, !llvm.loop !437

34:                                               ; preds = %11
  %35 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 5
  %36 = load ptr, ptr %35, align 16, !tbaa !436
  %37 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 4
  %38 = load i64, ptr %37, align 8, !tbaa !431
  %39 = mul nsw i64 %38, %13
  tail call void @fa_free(ptr noundef %36, i64 noundef %39, i64 noundef 16) #45
  br label %40

40:                                               ; preds = %34, %11
  ret void
}

; Function Attrs: nounwind optsize
define noundef i32 @fa_channel_send(ptr noundef %0, ptr nocapture noundef readonly %1, i32 noundef %2) local_unnamed_addr #2 {
  %4 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 1
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %5, label %7

5:                                                ; preds = %3
  %6 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %4) #45
  br label %7

7:                                                ; preds = %3, %5
  %8 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 9
  %9 = load i32, ptr %8, align 16, !tbaa !438
  %10 = icmp eq i32 %9, 0
  br i1 %10, label %11, label %16

11:                                               ; preds = %7
  %12 = icmp eq i32 %2, 0
  %13 = tail call align 4 ptr @llvm.threadlocal.address.p0(ptr align 4 @in_parallel)
  %14 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 10
  %15 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 2
  br label %17

16:                                               ; preds = %34, %7
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %42, label %45

17:                                               ; preds = %11, %34
  %18 = tail call fastcc i32 @channel_send_now(ptr noundef nonnull %0, ptr noundef %1) #44, !range !60
  %19 = icmp eq i32 %18, 0
  br i1 %19, label %21, label %20

20:                                               ; preds = %17
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %42, label %45

21:                                               ; preds = %17
  br i1 %12, label %22, label %23

22:                                               ; preds = %21
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %42, label %45

23:                                               ; preds = %21
  br i1 icmp eq (ptr @pthread_create, ptr null), label %33, label %24

24:                                               ; preds = %23
  %25 = load i32, ptr %13, align 4, !tbaa !25
  %26 = icmp eq i32 %25, 0
  br i1 %26, label %27, label %34

27:                                               ; preds = %24
  %28 = load atomic i32, ptr getelementptr inbounds (%struct.anon.0, ptr @pool, i64 0, i32 5) acquire, align 4
  %29 = icmp eq i32 %28, 0
  br i1 %29, label %30, label %34

30:                                               ; preds = %27
  %31 = load atomic i64, ptr @workers_alive acquire, align 8
  %32 = icmp slt i64 %31, 1
  br i1 %32, label %33, label %34

33:                                               ; preds = %23, %30
  tail call fastcc void @deadlock(ptr noundef nonnull @.str.33) #49
  unreachable

34:                                               ; preds = %24, %27, %30
  %35 = load i32, ptr %14, align 4, !tbaa !439
  %36 = add nsw i32 %35, 1
  store i32 %36, ptr %14, align 4, !tbaa !439
  %37 = tail call i32 @pthread_cond_wait(ptr noundef nonnull %15, ptr noundef nonnull %4) #45
  %38 = load i32, ptr %14, align 4, !tbaa !439
  %39 = add nsw i32 %38, -1
  store i32 %39, ptr %14, align 4, !tbaa !439
  %40 = load i32, ptr %8, align 16, !tbaa !438
  %41 = icmp eq i32 %40, 0
  br i1 %41, label %17, label %16

42:                                               ; preds = %22, %20, %16
  %43 = phi i32 [ 0, %16 ], [ 1, %20 ], [ 0, %22 ]
  %44 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %4) #45
  br label %45

45:                                               ; preds = %42, %22, %20, %16
  %46 = phi i32 [ 0, %16 ], [ 1, %20 ], [ 0, %22 ], [ %43, %42 ]
  ret i32 %46
}

; Function Attrs: nounwind optsize
define internal fastcc noundef i32 @channel_send_now(ptr noundef %0, ptr nocapture noundef readonly %1) unnamed_addr #2 {
  %3 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 13
  %4 = load ptr, ptr %3, align 8, !tbaa !10
  %5 = icmp eq ptr %4, null
  br i1 %5, label %27, label %6

6:                                                ; preds = %2
  %7 = getelementptr inbounds %struct.FaWait, ptr %4, i64 0, i32 2
  %8 = load ptr, ptr %7, align 8, !tbaa !409
  %9 = icmp eq ptr %8, %4
  br i1 %9, label %18, label %10

10:                                               ; preds = %6
  %11 = getelementptr inbounds %struct.FaWait, ptr %4, i64 0, i32 3
  %12 = load ptr, ptr %11, align 8, !tbaa !410
  %13 = getelementptr inbounds %struct.FaWait, ptr %12, i64 0, i32 2
  store ptr %8, ptr %13, align 8, !tbaa !409
  %14 = load ptr, ptr %7, align 8, !tbaa !409
  %15 = getelementptr inbounds %struct.FaWait, ptr %14, i64 0, i32 3
  store ptr %12, ptr %15, align 8, !tbaa !410
  %16 = load ptr, ptr %3, align 8, !tbaa !10
  %17 = icmp eq ptr %16, %4
  br i1 %17, label %18, label %20

18:                                               ; preds = %10, %6
  %19 = phi ptr [ null, %6 ], [ %14, %10 ]
  store ptr %19, ptr %3, align 8, !tbaa !10
  br label %20

20:                                               ; preds = %18, %10
  store ptr null, ptr %7, align 8, !tbaa !409
  %21 = getelementptr inbounds %struct.FaWait, ptr %4, i64 0, i32 3
  store ptr null, ptr %21, align 8, !tbaa !410
  %22 = getelementptr inbounds %struct.FaWait, ptr %4, i64 0, i32 4
  %23 = load ptr, ptr %22, align 8, !tbaa !425
  %24 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 4
  %25 = load i64, ptr %24, align 8, !tbaa !431
  tail call fastcc void @copy_bytes(ptr noundef %23, ptr noundef %1, i64 noundef %25) #44
  %26 = getelementptr inbounds %struct.FaWait, ptr %4, i64 0, i32 9
  store i32 1, ptr %26, align 8, !tbaa !309
  tail call fastcc void @wake(ptr noundef nonnull %4) #44
  br label %40

27:                                               ; preds = %2
  %28 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 7
  %29 = load i64, ptr %28, align 16, !tbaa !433
  %30 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 3
  %31 = load i64, ptr %30, align 16, !tbaa !429
  %32 = icmp slt i64 %29, %31
  br i1 %32, label %33, label %40

33:                                               ; preds = %27
  tail call fastcc void @channel_push(ptr noundef nonnull %0, ptr noundef %1) #44
  %34 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 10
  %35 = load i32, ptr %34, align 4, !tbaa !439
  %36 = icmp sgt i32 %35, 0
  br i1 %36, label %37, label %40

37:                                               ; preds = %33
  %38 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 2
  %39 = tail call i32 @pthread_cond_broadcast(ptr noundef nonnull %38) #45
  br label %40

40:                                               ; preds = %37, %33, %27, %20
  %41 = phi i32 [ 1, %20 ], [ 0, %27 ], [ 1, %33 ], [ 1, %37 ]
  ret i32 %41
}

; Function Attrs: nounwind optsize
define noundef i32 @fa_channel_send_async(ptr noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef %3, ptr noundef %4) local_unnamed_addr #2 {
  %6 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 1
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %7, label %9

7:                                                ; preds = %5
  %8 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %6) #45
  br label %9

9:                                                ; preds = %5, %7
  %10 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 9
  %11 = load i32, ptr %10, align 16, !tbaa !438
  %12 = icmp eq i32 %11, 0
  br i1 %12, label %16, label %13

13:                                               ; preds = %9
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %14, label %59

14:                                               ; preds = %13
  %15 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %6) #45
  br label %59

16:                                               ; preds = %9
  %17 = tail call fastcc i32 @channel_send_now(ptr noundef nonnull %0, ptr noundef %1) #44, !range !60
  %18 = icmp eq i32 %17, 0
  br i1 %18, label %54, label %19

19:                                               ; preds = %16
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %20, label %22

20:                                               ; preds = %19
  %21 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %6) #45
  br label %22

22:                                               ; preds = %19, %20
  %23 = getelementptr inbounds %struct.FaWait, ptr %2, i64 0, i32 9
  store i32 1, ptr %23, align 8, !tbaa !309
  %24 = getelementptr inbounds %struct.FaWait, ptr %2, i64 0, i32 10
  store i32 0, ptr %24, align 4, !tbaa !310
  %25 = tail call align 4 ptr @llvm.threadlocal.address.p0(ptr align 4 @fa_coop_budget)
  %26 = load i32, ptr %25, align 4, !tbaa !25
  %27 = add nsw i32 %26, -1
  store i32 %27, ptr %25, align 4, !tbaa !25
  %28 = icmp sgt i32 %26, 1
  br i1 %28, label %59, label %29

29:                                               ; preds = %22
  %30 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @current_executor)
  %31 = load ptr, ptr %30, align 8, !tbaa !10
  %32 = icmp eq ptr %31, null
  br i1 %32, label %35, label %33

33:                                               ; preds = %29
  %34 = getelementptr inbounds %struct.FaExecutor, ptr %31, i64 0, i32 23
  store i32 1, ptr %34, align 8, !tbaa !284
  br label %35

35:                                               ; preds = %33, %29
  %36 = tail call fastcc ptr @executor() #44
  %37 = getelementptr inbounds %struct.FaExecutor, ptr %36, i64 0, i32 23
  store i32 1, ptr %37, align 8, !tbaa !284
  %38 = getelementptr inbounds i8, ptr %3, i64 16
  %39 = getelementptr inbounds i8, ptr %3, i64 44
  %40 = load i32, ptr %39, align 4, !tbaa !267
  %41 = or i32 %40, 4
  store i32 %41, ptr %39, align 4, !tbaa !267
  %42 = getelementptr inbounds %struct.FaExecutor, ptr %36, i64 0, i32 2
  %43 = getelementptr inbounds %struct.FaExecutor, ptr %36, i64 0, i32 3
  %44 = getelementptr inbounds i8, ptr %3, i64 24
  store ptr null, ptr %44, align 8, !tbaa !269
  %45 = load ptr, ptr %43, align 8, !tbaa !10
  %46 = getelementptr inbounds i8, ptr %3, i64 32
  store ptr %45, ptr %46, align 8, !tbaa !270
  %47 = icmp eq ptr %45, null
  %48 = getelementptr inbounds %struct.FaTask, ptr %45, i64 0, i32 1
  %49 = select i1 %47, ptr %42, ptr %48
  store ptr %38, ptr %49, align 8, !tbaa !10
  store ptr %38, ptr %43, align 8, !tbaa !10
  %50 = getelementptr inbounds %struct.FaExecutor, ptr %36, i64 0, i32 4
  %51 = load i64, ptr %50, align 8, !tbaa !283
  %52 = add nsw i64 %51, 1
  store i64 %52, ptr %50, align 8, !tbaa !283
  %53 = getelementptr inbounds i8, ptr %3, i64 40
  store atomic i32 1, ptr %53 release, align 8
  br label %59

54:                                               ; preds = %16
  %55 = getelementptr inbounds %struct.FaWait, ptr %2, i64 0, i32 4
  store ptr %1, ptr %55, align 8, !tbaa !425
  %56 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 12
  tail call fastcc void @line_push(ptr noundef nonnull %56, ptr noundef %2, ptr noundef %3, ptr noundef %4) #44
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %57, label %59

57:                                               ; preds = %54
  %58 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %6) #45
  br label %59

59:                                               ; preds = %57, %54, %22, %14, %13, %35
  %60 = phi i32 [ 2, %35 ], [ 3, %13 ], [ 3, %14 ], [ 0, %22 ], [ 1, %54 ], [ 1, %57 ]
  ret i32 %60
}

; Function Attrs: nounwind optsize
define noundef i32 @fa_channel_receive(ptr noundef %0, ptr nocapture noundef writeonly %1, i32 noundef %2) local_unnamed_addr #2 {
  %4 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 1
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %5, label %7

5:                                                ; preds = %3
  %6 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %4) #45
  br label %7

7:                                                ; preds = %3, %5
  %8 = tail call fastcc i32 @channel_receive_now(ptr noundef %0, ptr noundef %1) #44, !range !60
  %9 = icmp eq i32 %8, 0
  br i1 %9, label %10, label %16

10:                                               ; preds = %7
  %11 = icmp eq i32 %2, 0
  %12 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 9
  %13 = tail call align 4 ptr @llvm.threadlocal.address.p0(ptr align 4 @in_parallel)
  %14 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 10
  %15 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 2
  br i1 %11, label %20, label %17

16:                                               ; preds = %32, %7
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %40, label %43

17:                                               ; preds = %10, %32
  %18 = load i32, ptr %12, align 16, !tbaa !438
  %19 = icmp eq i32 %18, 0
  br i1 %19, label %21, label %20

20:                                               ; preds = %17, %10
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %40, label %43

21:                                               ; preds = %17
  br i1 icmp eq (ptr @pthread_create, ptr null), label %31, label %22

22:                                               ; preds = %21
  %23 = load i32, ptr %13, align 4, !tbaa !25
  %24 = icmp eq i32 %23, 0
  br i1 %24, label %25, label %32

25:                                               ; preds = %22
  %26 = load atomic i32, ptr getelementptr inbounds (%struct.anon.0, ptr @pool, i64 0, i32 5) acquire, align 4
  %27 = icmp eq i32 %26, 0
  br i1 %27, label %28, label %32

28:                                               ; preds = %25
  %29 = load atomic i64, ptr @workers_alive acquire, align 8
  %30 = icmp slt i64 %29, 1
  br i1 %30, label %31, label %32

31:                                               ; preds = %21, %28
  tail call fastcc void @deadlock(ptr noundef nonnull @.str.34) #49
  unreachable

32:                                               ; preds = %22, %25, %28
  %33 = load i32, ptr %14, align 4, !tbaa !439
  %34 = add nsw i32 %33, 1
  store i32 %34, ptr %14, align 4, !tbaa !439
  %35 = tail call i32 @pthread_cond_wait(ptr noundef nonnull %15, ptr noundef nonnull %4) #45
  %36 = load i32, ptr %14, align 4, !tbaa !439
  %37 = add nsw i32 %36, -1
  store i32 %37, ptr %14, align 4, !tbaa !439
  %38 = tail call fastcc i32 @channel_receive_now(ptr noundef nonnull %0, ptr noundef %1) #44, !range !60
  %39 = icmp eq i32 %38, 0
  br i1 %39, label %17, label %16

40:                                               ; preds = %20, %16
  %41 = phi i32 [ 1, %16 ], [ 0, %20 ]
  %42 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %4) #45
  br label %43

43:                                               ; preds = %40, %20, %16
  %44 = phi i32 [ 1, %16 ], [ 0, %20 ], [ %41, %40 ]
  ret i32 %44
}

; Function Attrs: nounwind optsize
define internal fastcc noundef i32 @channel_receive_now(ptr noundef %0, ptr nocapture noundef writeonly %1) unnamed_addr #2 {
  %3 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 7
  %4 = load i64, ptr %3, align 16, !tbaa !433
  %5 = icmp sgt i64 %4, 0
  br i1 %5, label %6, label %48

6:                                                ; preds = %2
  %7 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 5
  %8 = load ptr, ptr %7, align 16, !tbaa !436
  %9 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 6
  %10 = load i64, ptr %9, align 8, !tbaa !435
  %11 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 4
  %12 = load i64, ptr %11, align 8, !tbaa !431
  %13 = mul nsw i64 %12, %10
  %14 = getelementptr inbounds i8, ptr %8, i64 %13
  tail call fastcc void @copy_bytes(ptr noundef %1, ptr noundef %14, i64 noundef %12) #44
  %15 = load i64, ptr %9, align 8, !tbaa !435
  %16 = add nsw i64 %15, 1
  %17 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 8
  %18 = load i64, ptr %17, align 8, !tbaa !434
  %19 = srem i64 %16, %18
  store i64 %19, ptr %9, align 8, !tbaa !435
  %20 = load i64, ptr %3, align 16, !tbaa !433
  %21 = add nsw i64 %20, -1
  store i64 %21, ptr %3, align 16, !tbaa !433
  %22 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 12
  %23 = load ptr, ptr %22, align 8, !tbaa !10
  %24 = icmp eq ptr %23, null
  br i1 %24, label %44, label %25

25:                                               ; preds = %6
  %26 = getelementptr inbounds %struct.FaWait, ptr %23, i64 0, i32 2
  %27 = load ptr, ptr %26, align 8, !tbaa !409
  %28 = icmp eq ptr %27, %23
  br i1 %28, label %37, label %29

29:                                               ; preds = %25
  %30 = getelementptr inbounds %struct.FaWait, ptr %23, i64 0, i32 3
  %31 = load ptr, ptr %30, align 8, !tbaa !410
  %32 = getelementptr inbounds %struct.FaWait, ptr %31, i64 0, i32 2
  store ptr %27, ptr %32, align 8, !tbaa !409
  %33 = load ptr, ptr %26, align 8, !tbaa !409
  %34 = getelementptr inbounds %struct.FaWait, ptr %33, i64 0, i32 3
  store ptr %31, ptr %34, align 8, !tbaa !410
  %35 = load ptr, ptr %22, align 8, !tbaa !10
  %36 = icmp eq ptr %35, %23
  br i1 %36, label %37, label %39

37:                                               ; preds = %29, %25
  %38 = phi ptr [ null, %25 ], [ %33, %29 ]
  store ptr %38, ptr %22, align 8, !tbaa !10
  br label %39

39:                                               ; preds = %37, %29
  store ptr null, ptr %26, align 8, !tbaa !409
  %40 = getelementptr inbounds %struct.FaWait, ptr %23, i64 0, i32 3
  store ptr null, ptr %40, align 8, !tbaa !410
  %41 = getelementptr inbounds %struct.FaWait, ptr %23, i64 0, i32 4
  %42 = load ptr, ptr %41, align 8, !tbaa !425
  tail call fastcc void @channel_push(ptr noundef nonnull %0, ptr noundef %42) #44
  %43 = getelementptr inbounds %struct.FaWait, ptr %23, i64 0, i32 9
  store i32 1, ptr %43, align 8, !tbaa !309
  tail call fastcc void @wake(ptr noundef nonnull %23) #44
  br label %44

44:                                               ; preds = %6, %39
  %45 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 10
  %46 = load i32, ptr %45, align 4, !tbaa !439
  %47 = icmp sgt i32 %46, 0
  br i1 %47, label %76, label %79

48:                                               ; preds = %2
  %49 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 12
  %50 = load ptr, ptr %49, align 8, !tbaa !10
  %51 = icmp eq ptr %50, null
  br i1 %51, label %79, label %52

52:                                               ; preds = %48
  %53 = getelementptr inbounds %struct.FaWait, ptr %50, i64 0, i32 2
  %54 = load ptr, ptr %53, align 8, !tbaa !409
  %55 = icmp eq ptr %54, %50
  br i1 %55, label %64, label %56

56:                                               ; preds = %52
  %57 = getelementptr inbounds %struct.FaWait, ptr %50, i64 0, i32 3
  %58 = load ptr, ptr %57, align 8, !tbaa !410
  %59 = getelementptr inbounds %struct.FaWait, ptr %58, i64 0, i32 2
  store ptr %54, ptr %59, align 8, !tbaa !409
  %60 = load ptr, ptr %53, align 8, !tbaa !409
  %61 = getelementptr inbounds %struct.FaWait, ptr %60, i64 0, i32 3
  store ptr %58, ptr %61, align 8, !tbaa !410
  %62 = load ptr, ptr %49, align 8, !tbaa !10
  %63 = icmp eq ptr %62, %50
  br i1 %63, label %64, label %66

64:                                               ; preds = %56, %52
  %65 = phi ptr [ null, %52 ], [ %60, %56 ]
  store ptr %65, ptr %49, align 8, !tbaa !10
  br label %66

66:                                               ; preds = %64, %56
  store ptr null, ptr %53, align 8, !tbaa !409
  %67 = getelementptr inbounds %struct.FaWait, ptr %50, i64 0, i32 3
  store ptr null, ptr %67, align 8, !tbaa !410
  %68 = getelementptr inbounds %struct.FaWait, ptr %50, i64 0, i32 4
  %69 = load ptr, ptr %68, align 8, !tbaa !425
  %70 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 4
  %71 = load i64, ptr %70, align 8, !tbaa !431
  tail call fastcc void @copy_bytes(ptr noundef %1, ptr noundef %69, i64 noundef %71) #44
  %72 = getelementptr inbounds %struct.FaWait, ptr %50, i64 0, i32 9
  store i32 1, ptr %72, align 8, !tbaa !309
  tail call fastcc void @wake(ptr noundef nonnull %50) #44
  %73 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 10
  %74 = load i32, ptr %73, align 4, !tbaa !439
  %75 = icmp sgt i32 %74, 0
  br i1 %75, label %76, label %79

76:                                               ; preds = %66, %44
  %77 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 2
  %78 = tail call i32 @pthread_cond_broadcast(ptr noundef nonnull %77) #45
  br label %79

79:                                               ; preds = %76, %48, %66, %44
  %80 = phi i32 [ 1, %44 ], [ 1, %66 ], [ 0, %48 ], [ 1, %76 ]
  ret i32 %80
}

; Function Attrs: nounwind optsize
define noundef i32 @fa_channel_receive_async(ptr noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef %3, ptr noundef %4) local_unnamed_addr #2 {
  %6 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 1
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %7, label %9

7:                                                ; preds = %5
  %8 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %6) #45
  br label %9

9:                                                ; preds = %5, %7
  %10 = tail call fastcc i32 @channel_receive_now(ptr noundef %0, ptr noundef %1) #44, !range !60
  %11 = icmp eq i32 %10, 0
  br i1 %11, label %47, label %12

12:                                               ; preds = %9
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %13, label %15

13:                                               ; preds = %12
  %14 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %6) #45
  br label %15

15:                                               ; preds = %12, %13
  %16 = getelementptr inbounds %struct.FaWait, ptr %2, i64 0, i32 9
  store i32 1, ptr %16, align 8, !tbaa !309
  %17 = getelementptr inbounds %struct.FaWait, ptr %2, i64 0, i32 10
  store i32 0, ptr %17, align 4, !tbaa !310
  %18 = tail call align 4 ptr @llvm.threadlocal.address.p0(ptr align 4 @fa_coop_budget)
  %19 = load i32, ptr %18, align 4, !tbaa !25
  %20 = add nsw i32 %19, -1
  store i32 %20, ptr %18, align 4, !tbaa !25
  %21 = icmp sgt i32 %19, 1
  br i1 %21, label %59, label %22

22:                                               ; preds = %15
  %23 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @current_executor)
  %24 = load ptr, ptr %23, align 8, !tbaa !10
  %25 = icmp eq ptr %24, null
  br i1 %25, label %28, label %26

26:                                               ; preds = %22
  %27 = getelementptr inbounds %struct.FaExecutor, ptr %24, i64 0, i32 23
  store i32 1, ptr %27, align 8, !tbaa !284
  br label %28

28:                                               ; preds = %26, %22
  %29 = tail call fastcc ptr @executor() #44
  %30 = getelementptr inbounds %struct.FaExecutor, ptr %29, i64 0, i32 23
  store i32 1, ptr %30, align 8, !tbaa !284
  %31 = getelementptr inbounds i8, ptr %3, i64 16
  %32 = getelementptr inbounds i8, ptr %3, i64 44
  %33 = load i32, ptr %32, align 4, !tbaa !267
  %34 = or i32 %33, 4
  store i32 %34, ptr %32, align 4, !tbaa !267
  %35 = getelementptr inbounds %struct.FaExecutor, ptr %29, i64 0, i32 2
  %36 = getelementptr inbounds %struct.FaExecutor, ptr %29, i64 0, i32 3
  %37 = getelementptr inbounds i8, ptr %3, i64 24
  store ptr null, ptr %37, align 8, !tbaa !269
  %38 = load ptr, ptr %36, align 8, !tbaa !10
  %39 = getelementptr inbounds i8, ptr %3, i64 32
  store ptr %38, ptr %39, align 8, !tbaa !270
  %40 = icmp eq ptr %38, null
  %41 = getelementptr inbounds %struct.FaTask, ptr %38, i64 0, i32 1
  %42 = select i1 %40, ptr %35, ptr %41
  store ptr %31, ptr %42, align 8, !tbaa !10
  store ptr %31, ptr %36, align 8, !tbaa !10
  %43 = getelementptr inbounds %struct.FaExecutor, ptr %29, i64 0, i32 4
  %44 = load i64, ptr %43, align 8, !tbaa !283
  %45 = add nsw i64 %44, 1
  store i64 %45, ptr %43, align 8, !tbaa !283
  %46 = getelementptr inbounds i8, ptr %3, i64 40
  store atomic i32 1, ptr %46 release, align 8
  br label %59

47:                                               ; preds = %9
  %48 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 9
  %49 = load i32, ptr %48, align 16, !tbaa !438
  %50 = icmp eq i32 %49, 0
  br i1 %50, label %54, label %51

51:                                               ; preds = %47
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %52, label %59

52:                                               ; preds = %51
  %53 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %6) #45
  br label %59

54:                                               ; preds = %47
  %55 = getelementptr inbounds %struct.FaWait, ptr %2, i64 0, i32 4
  store ptr %1, ptr %55, align 8, !tbaa !425
  %56 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 13
  tail call fastcc void @line_push(ptr noundef nonnull %56, ptr noundef %2, ptr noundef %3, ptr noundef %4) #44
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %57, label %59

57:                                               ; preds = %54
  %58 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %6) #45
  br label %59

59:                                               ; preds = %57, %54, %52, %51, %15, %28
  %60 = phi i32 [ 2, %28 ], [ 0, %15 ], [ 3, %51 ], [ 3, %52 ], [ 1, %54 ], [ 1, %57 ]
  ret i32 %60
}

; Function Attrs: nounwind optsize
define i32 @fa_channel_cancel(ptr noundef %0, ptr noundef %1) local_unnamed_addr #2 {
  %3 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 1
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %4, label %6

4:                                                ; preds = %2
  %5 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %3) #45
  br label %6

6:                                                ; preds = %2, %4
  %7 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 9
  %8 = load i32, ptr %7, align 8, !tbaa !309
  %9 = icmp eq i32 %8, 0
  br i1 %9, label %10, label %40

10:                                               ; preds = %6
  %11 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 12
  %12 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 13
  %13 = load ptr, ptr %12, align 8, !tbaa !440
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
  %21 = load ptr, ptr %20, align 8, !tbaa !409
  %22 = icmp eq ptr %21, %13
  br i1 %22, label %23, label %14, !llvm.loop !441

23:                                               ; preds = %19, %17, %14
  %24 = phi ptr [ %11, %14 ], [ %12, %17 ], [ %11, %19 ]
  %25 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 2
  %26 = load ptr, ptr %25, align 8, !tbaa !409
  %27 = icmp eq ptr %26, %1
  br i1 %27, label %36, label %28

28:                                               ; preds = %23
  %29 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 3
  %30 = load ptr, ptr %29, align 8, !tbaa !410
  %31 = getelementptr inbounds %struct.FaWait, ptr %30, i64 0, i32 2
  store ptr %26, ptr %31, align 8, !tbaa !409
  %32 = load ptr, ptr %25, align 8, !tbaa !409
  %33 = getelementptr inbounds %struct.FaWait, ptr %32, i64 0, i32 3
  store ptr %30, ptr %33, align 8, !tbaa !410
  %34 = load ptr, ptr %24, align 8, !tbaa !10
  %35 = icmp eq ptr %34, %1
  br i1 %35, label %36, label %38

36:                                               ; preds = %28, %23
  %37 = phi ptr [ null, %23 ], [ %32, %28 ]
  store ptr %37, ptr %24, align 8, !tbaa !10
  br label %38

38:                                               ; preds = %28, %36
  store ptr null, ptr %25, align 8, !tbaa !409
  %39 = getelementptr inbounds %struct.FaWait, ptr %1, i64 0, i32 3
  store ptr null, ptr %39, align 8, !tbaa !410
  br label %40

40:                                               ; preds = %38, %6
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %41, label %43

41:                                               ; preds = %40
  %42 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %3) #45
  br label %43

43:                                               ; preds = %40, %41
  br i1 %9, label %45, label %44

44:                                               ; preds = %43
  tail call fastcc void @forget_remote_wake(ptr noundef nonnull %1) #44
  br label %45

45:                                               ; preds = %44, %43
  ret i32 %8
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(argmem: readwrite)
define i32 @fa_wait_status(ptr nocapture noundef readonly %0) local_unnamed_addr #36 {
  %2 = getelementptr inbounds %struct.FaWait, ptr %0, i64 0, i32 9
  %3 = load atomic i32, ptr %2 acquire, align 8
  ret i32 %3
}

; Function Attrs: nounwind optsize
define void @fa_channel_close(ptr noundef %0) local_unnamed_addr #2 {
  %2 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 1
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %3, label %5

3:                                                ; preds = %1
  %4 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %2) #45
  br label %5

5:                                                ; preds = %1, %3
  %6 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 9
  store i32 1, ptr %6, align 16, !tbaa !438
  %7 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 13
  %8 = load ptr, ptr %7, align 8, !tbaa !10
  %9 = icmp eq ptr %8, null
  br i1 %9, label %10, label %14

10:                                               ; preds = %29, %5
  %11 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 12
  %12 = load ptr, ptr %11, align 8, !tbaa !10
  %13 = icmp eq ptr %12, null
  br i1 %13, label %54, label %34

14:                                               ; preds = %5, %29
  %15 = phi ptr [ %32, %29 ], [ %8, %5 ]
  %16 = getelementptr inbounds %struct.FaWait, ptr %15, i64 0, i32 2
  %17 = load ptr, ptr %16, align 8, !tbaa !409
  %18 = icmp eq ptr %17, %15
  br i1 %18, label %27, label %19

19:                                               ; preds = %14
  %20 = getelementptr inbounds %struct.FaWait, ptr %15, i64 0, i32 3
  %21 = load ptr, ptr %20, align 8, !tbaa !410
  %22 = getelementptr inbounds %struct.FaWait, ptr %21, i64 0, i32 2
  store ptr %17, ptr %22, align 8, !tbaa !409
  %23 = load ptr, ptr %16, align 8, !tbaa !409
  %24 = getelementptr inbounds %struct.FaWait, ptr %23, i64 0, i32 3
  store ptr %21, ptr %24, align 8, !tbaa !410
  %25 = load ptr, ptr %7, align 8, !tbaa !10
  %26 = icmp eq ptr %25, %15
  br i1 %26, label %27, label %29

27:                                               ; preds = %19, %14
  %28 = phi ptr [ null, %14 ], [ %23, %19 ]
  store ptr %28, ptr %7, align 8, !tbaa !10
  br label %29

29:                                               ; preds = %19, %27
  store ptr null, ptr %16, align 8, !tbaa !409
  %30 = getelementptr inbounds %struct.FaWait, ptr %15, i64 0, i32 3
  store ptr null, ptr %30, align 8, !tbaa !410
  %31 = getelementptr inbounds %struct.FaWait, ptr %15, i64 0, i32 9
  store i32 2, ptr %31, align 8, !tbaa !309
  tail call fastcc void @wake(ptr noundef nonnull %15) #44
  %32 = load ptr, ptr %7, align 8, !tbaa !10
  %33 = icmp eq ptr %32, null
  br i1 %33, label %10, label %14, !llvm.loop !442

34:                                               ; preds = %10, %49
  %35 = phi ptr [ %52, %49 ], [ %12, %10 ]
  %36 = getelementptr inbounds %struct.FaWait, ptr %35, i64 0, i32 2
  %37 = load ptr, ptr %36, align 8, !tbaa !409
  %38 = icmp eq ptr %37, %35
  br i1 %38, label %47, label %39

39:                                               ; preds = %34
  %40 = getelementptr inbounds %struct.FaWait, ptr %35, i64 0, i32 3
  %41 = load ptr, ptr %40, align 8, !tbaa !410
  %42 = getelementptr inbounds %struct.FaWait, ptr %41, i64 0, i32 2
  store ptr %37, ptr %42, align 8, !tbaa !409
  %43 = load ptr, ptr %36, align 8, !tbaa !409
  %44 = getelementptr inbounds %struct.FaWait, ptr %43, i64 0, i32 3
  store ptr %41, ptr %44, align 8, !tbaa !410
  %45 = load ptr, ptr %11, align 8, !tbaa !10
  %46 = icmp eq ptr %45, %35
  br i1 %46, label %47, label %49

47:                                               ; preds = %39, %34
  %48 = phi ptr [ null, %34 ], [ %43, %39 ]
  store ptr %48, ptr %11, align 8, !tbaa !10
  br label %49

49:                                               ; preds = %47, %39
  store ptr null, ptr %36, align 8, !tbaa !409
  %50 = getelementptr inbounds %struct.FaWait, ptr %35, i64 0, i32 3
  store ptr null, ptr %50, align 8, !tbaa !410
  %51 = getelementptr inbounds %struct.FaWait, ptr %35, i64 0, i32 9
  store i32 2, ptr %51, align 8, !tbaa !309
  tail call fastcc void @wake(ptr noundef nonnull %35) #44
  %52 = load ptr, ptr %11, align 8, !tbaa !10
  %53 = icmp eq ptr %52, null
  br i1 %53, label %54, label %34, !llvm.loop !443

54:                                               ; preds = %49, %10
  %55 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 10
  %56 = load i32, ptr %55, align 4, !tbaa !439
  %57 = icmp sgt i32 %56, 0
  br i1 %57, label %58, label %61

58:                                               ; preds = %54
  %59 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 2
  %60 = tail call i32 @pthread_cond_broadcast(ptr noundef nonnull %59) #45
  br label %61

61:                                               ; preds = %54, %58
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %62, label %64

62:                                               ; preds = %61
  %63 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %2) #45
  br label %64

64:                                               ; preds = %61, %62
  ret void
}

; Function Attrs: nounwind optsize
define ptr @fa_parallel_for(i64 noundef %0, ptr noundef %1, ptr noundef %2, i64 noundef %3, ptr noundef %4) local_unnamed_addr #2 {
  %6 = alloca i64, align 8
  %7 = alloca i64, align 8
  %8 = alloca %struct.ParallelJob, align 8
  %9 = icmp slt i64 %0, 1
  br i1 %9, label %75, label %10

10:                                               ; preds = %5
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %6) #40
  store i64 0, ptr %6, align 8, !tbaa !8
  %11 = icmp eq i64 %0, 1
  %12 = or i1 %11, icmp eq (ptr @pthread_create, ptr null)
  br i1 %12, label %17, label %13

13:                                               ; preds = %10
  %14 = tail call align 4 ptr @llvm.threadlocal.address.p0(ptr align 4 @in_parallel)
  %15 = load i32, ptr %14, align 4, !tbaa !25
  %16 = icmp eq i32 %15, 0
  br i1 %16, label %19, label %17

17:                                               ; preds = %13, %10
  %18 = call ptr %1(ptr noundef %2, i64 noundef 0, i64 noundef %0, ptr noundef nonnull %6) #45
  br label %73

19:                                               ; preds = %13
  br i1 icmp ne (ptr @sysconf, ptr null), label %20, label %22

20:                                               ; preds = %19
  %21 = tail call i64 @sysconf(i32 noundef 84) #45
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
  %30 = atomicrmw xchg ptr getelementptr inbounds (%struct.anon.0, ptr @pool, i64 0, i32 5), i32 1 acquire, align 4
  %31 = icmp eq i32 %30, 0
  br i1 %31, label %34, label %32

32:                                               ; preds = %29, %22
  %33 = call ptr %1(ptr noundef %2, i64 noundef 0, i64 noundef %0, ptr noundef nonnull %6) #45
  br label %73

34:                                               ; preds = %29
  %35 = load i32, ptr getelementptr inbounds (%struct.anon.0, ptr @pool, i64 0, i32 6), align 8, !tbaa !444
  %36 = icmp eq i32 %35, 0
  br i1 %36, label %37, label %44

37:                                               ; preds = %34
  store i32 1, ptr getelementptr inbounds (%struct.anon.0, ptr @pool, i64 0, i32 6), align 8, !tbaa !444
  %38 = icmp sgt i64 %23, 1
  br i1 %38, label %39, label %44

39:                                               ; preds = %37, %39
  %40 = phi i64 [ %42, %39 ], [ 1, %37 ]
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %7) #40
  %41 = call i32 @pthread_create(ptr noundef nonnull %7, ptr noundef null, ptr noundef nonnull @pool_thread, ptr noundef null) #45
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %7) #40
  %42 = add nuw nsw i64 %40, 1
  %43 = icmp eq i64 %42, %23
  br i1 %43, label %44, label %39, !llvm.loop !446

44:                                               ; preds = %39, %37, %34
  call void @llvm.lifetime.start.p0(i64 80, ptr nonnull %8) #40
  %45 = getelementptr inbounds i8, ptr %8, i64 32
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(80) %45, i8 0, i64 40, i1 false)
  store ptr %1, ptr %8, align 8, !tbaa !447
  %46 = getelementptr inbounds %struct.ParallelJob, ptr %8, i64 0, i32 1
  store ptr %2, ptr %46, align 8, !tbaa !449
  %47 = getelementptr inbounds %struct.ParallelJob, ptr %8, i64 0, i32 2
  store i64 %0, ptr %47, align 8, !tbaa !450
  %48 = trunc i64 %27 to i32
  %49 = add i32 %48, -1
  %50 = getelementptr inbounds %struct.ParallelJob, ptr %8, i64 0, i32 6
  store i32 %49, ptr %50, align 4, !tbaa !451
  %51 = shl nsw i64 %27, 4
  %52 = udiv i64 %0, %51
  %53 = getelementptr inbounds %struct.ParallelJob, ptr %8, i64 0, i32 3
  %54 = call i64 @llvm.smax.i64(i64 %52, i64 1)
  store i64 %54, ptr %53, align 8, !tbaa !452
  %55 = getelementptr inbounds %struct.ParallelJob, ptr %8, i64 0, i32 11
  store ptr %4, ptr %55, align 8, !tbaa !453
  store i32 1, ptr %14, align 4, !tbaa !25
  %56 = ptrtoint ptr %8 to i64
  store atomic i64 %56, ptr getelementptr inbounds (%struct.anon.0, ptr @pool, i64 0, i32 2) seq_cst, align 16
  %57 = call i32 @pthread_mutex_lock(ptr noundef nonnull @pool) #45
  %58 = load i64, ptr getelementptr inbounds (%struct.anon.0, ptr @pool, i64 0, i32 3), align 8, !tbaa !454
  %59 = add nsw i64 %58, 1
  store i64 %59, ptr getelementptr inbounds (%struct.anon.0, ptr @pool, i64 0, i32 3), align 8, !tbaa !454
  %60 = call i32 @pthread_cond_broadcast(ptr noundef nonnull getelementptr inbounds (%struct.anon.0, ptr @pool, i64 0, i32 1)) #45
  %61 = call i32 @pthread_mutex_unlock(ptr noundef nonnull @pool) #45
  call fastcc void @run_chunks(ptr noundef nonnull %8) #44
  store atomic i64 0, ptr getelementptr inbounds (%struct.anon.0, ptr @pool, i64 0, i32 2) seq_cst, align 16
  %62 = load atomic i32, ptr getelementptr inbounds (%struct.anon.0, ptr @pool, i64 0, i32 4) seq_cst, align 16
  %63 = icmp eq i32 %62, 0
  br i1 %63, label %70, label %64

64:                                               ; preds = %44, %67
  br i1 icmp ne (ptr @sched_yield, ptr null), label %65, label %67

65:                                               ; preds = %64
  %66 = call i32 @sched_yield() #45
  br label %67

67:                                               ; preds = %65, %64
  %68 = load atomic i32, ptr getelementptr inbounds (%struct.anon.0, ptr @pool, i64 0, i32 4) seq_cst, align 16
  %69 = icmp eq i32 %68, 0
  br i1 %69, label %70, label %64, !llvm.loop !455

70:                                               ; preds = %67, %44
  store i32 0, ptr %14, align 4, !tbaa !25
  store atomic i32 0, ptr getelementptr inbounds (%struct.anon.0, ptr @pool, i64 0, i32 5) release, align 4
  %71 = getelementptr inbounds %struct.ParallelJob, ptr %8, i64 0, i32 9
  %72 = load ptr, ptr %71, align 8, !tbaa !456
  call void @llvm.lifetime.end.p0(i64 80, ptr nonnull %8) #40
  br label %73

73:                                               ; preds = %32, %70, %17
  %74 = phi ptr [ %18, %17 ], [ %33, %32 ], [ %72, %70 ]
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %6) #40
  br label %75

75:                                               ; preds = %5, %73
  %76 = phi ptr [ %74, %73 ], [ null, %5 ]
  ret ptr %76
}

; Function Attrs: noreturn nounwind optsize
define internal noalias noundef nonnull ptr @pool_thread(ptr nocapture readnone %0) #4 {
  %2 = tail call align 4 ptr @llvm.threadlocal.address.p0(ptr align 4 @in_parallel)
  store i32 1, ptr %2, align 4, !tbaa !25
  br label %3

3:                                                ; preds = %26, %1
  %4 = phi i64 [ 0, %1 ], [ %13, %26 ]
  %5 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull @pool) #45
  %6 = load i64, ptr getelementptr inbounds (%struct.anon.0, ptr @pool, i64 0, i32 3), align 8, !tbaa !454
  %7 = icmp eq i64 %6, %4
  br i1 %7, label %8, label %12

8:                                                ; preds = %3, %8
  %9 = tail call i32 @pthread_cond_wait(ptr noundef nonnull getelementptr inbounds (%struct.anon.0, ptr @pool, i64 0, i32 1), ptr noundef nonnull @pool) #45
  %10 = load i64, ptr getelementptr inbounds (%struct.anon.0, ptr @pool, i64 0, i32 3), align 8, !tbaa !454
  %11 = icmp eq i64 %10, %4
  br i1 %11, label %8, label %12, !llvm.loop !457

12:                                               ; preds = %8, %3
  %13 = phi i64 [ %6, %3 ], [ %10, %8 ]
  %14 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull @pool) #45
  %15 = atomicrmw add ptr getelementptr inbounds (%struct.anon.0, ptr @pool, i64 0, i32 4), i32 1 seq_cst, align 16
  %16 = load atomic i64, ptr getelementptr inbounds (%struct.anon.0, ptr @pool, i64 0, i32 2) seq_cst, align 16
  %17 = inttoptr i64 %16 to ptr
  %18 = icmp eq i64 %16, 0
  br i1 %18, label %26, label %19

19:                                               ; preds = %12
  %20 = getelementptr inbounds %struct.ParallelJob, ptr %17, i64 0, i32 7
  %21 = atomicrmw add ptr %20, i32 1 monotonic, align 8
  %22 = getelementptr inbounds %struct.ParallelJob, ptr %17, i64 0, i32 6
  %23 = load i32, ptr %22, align 4, !tbaa !451
  %24 = icmp slt i32 %21, %23
  br i1 %24, label %25, label %26

25:                                               ; preds = %19
  tail call fastcc void @run_chunks(ptr noundef nonnull %17) #44
  tail call void @fa_release_builder() #44
  br label %26

26:                                               ; preds = %25, %19, %12
  %27 = atomicrmw sub ptr getelementptr inbounds (%struct.anon.0, ptr @pool, i64 0, i32 4), i32 1 seq_cst, align 16
  br label %3
}

; Function Attrs: optsize
declare extern_weak i32 @pthread_mutex_lock(ptr noundef) #3

; Function Attrs: optsize
declare extern_weak i32 @pthread_mutex_unlock(ptr noundef) #3

; Function Attrs: nounwind optsize
define internal fastcc void @run_chunks(ptr nocapture noundef %0) unnamed_addr #2 {
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
  %16 = load i64, ptr %4, align 8, !tbaa !452
  %17 = atomicrmw add ptr %3, i64 %16 monotonic, align 8
  %18 = load i64, ptr %9, align 8, !tbaa !450
  %19 = icmp slt i64 %17, %18
  br i1 %19, label %20, label %59

20:                                               ; preds = %15
  %21 = sub nsw i64 %18, %17
  %22 = icmp slt i64 %21, %16
  %23 = add nsw i64 %17, %16
  %24 = select i1 %22, i64 %18, i64 %23
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %2) #40
  store i64 %17, ptr %2, align 8, !tbaa !8
  %25 = load ptr, ptr %0, align 8, !tbaa !447
  %26 = load ptr, ptr %10, align 8, !tbaa !449
  %27 = call ptr %25(ptr noundef %26, i64 noundef %17, i64 noundef %24, ptr noundef nonnull %2) #45
  %28 = icmp eq ptr %27, null
  br i1 %28, label %56, label %29

29:                                               ; preds = %20
  %30 = load i64, ptr %2, align 8, !tbaa !8
  store atomic i32 1, ptr %5 monotonic, align 8
  %31 = atomicrmw xchg ptr %11, i32 1 acquire, align 4
  %32 = icmp eq i32 %31, 0
  br i1 %32, label %42, label %36

33:                                               ; preds = %39, %36
  %34 = atomicrmw xchg ptr %11, i32 1 acquire, align 4
  %35 = icmp eq i32 %34, 0
  br i1 %35, label %42, label %36, !llvm.loop !5

36:                                               ; preds = %29, %33
  %37 = load atomic i32, ptr %11 monotonic, align 4
  %38 = icmp eq i32 %37, 0
  br i1 %38, label %33, label %39

39:                                               ; preds = %36, %39
  call void @llvm.x86.sse2.pause()
  %40 = load atomic i32, ptr %11 monotonic, align 4
  %41 = icmp eq i32 %40, 0
  br i1 %41, label %33, label %39, !llvm.loop !7

42:                                               ; preds = %33, %29
  %43 = load ptr, ptr %12, align 8, !tbaa !456
  %44 = icmp eq ptr %43, null
  br i1 %44, label %48, label %45

45:                                               ; preds = %42
  %46 = load i64, ptr %13, align 8, !tbaa !458
  %47 = icmp sgt i64 %46, %30
  br i1 %47, label %48, label %49

48:                                               ; preds = %45, %42
  store ptr %27, ptr %12, align 8, !tbaa !456
  store i64 %30, ptr %13, align 8, !tbaa !458
  br label %49

49:                                               ; preds = %48, %45
  %50 = phi ptr [ %43, %48 ], [ %27, %45 ]
  store atomic i32 0, ptr %11 release, align 4
  %51 = icmp eq ptr %50, null
  br i1 %51, label %56, label %52

52:                                               ; preds = %49
  %53 = load ptr, ptr %14, align 8, !tbaa !453
  %54 = icmp eq ptr %53, null
  br i1 %54, label %56, label %55

55:                                               ; preds = %52
  call void %53(ptr noundef nonnull %50) #45
  br label %56

56:                                               ; preds = %20, %49, %52, %55
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %2) #40
  %57 = load atomic i32, ptr %5 monotonic, align 8
  %58 = icmp eq i32 %57, 0
  br i1 %58, label %15, label %59

59:                                               ; preds = %56, %15, %1
  ret void
}

; Function Attrs: nounwind optsize
define weak { i64, i64 } @__multi3(i64 noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3) local_unnamed_addr #2 {
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
define weak { i64, i64 } @__udivti3(i64 noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3) local_unnamed_addr #2 {
  %5 = alloca i128, align 16
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %5) #40
  %6 = call fastcc { i64, i64 } @divide_u128(i64 noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3, ptr noundef nonnull %5) #44
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %5) #40
  ret { i64, i64 } %6
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: write)
define internal fastcc { i64, i64 } @divide_u128(i64 noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3, ptr nocapture noundef writeonly %4) unnamed_addr #39 {
  br label %13

6:                                                ; preds = %45
  %7 = zext i64 %46 to i128
  %8 = shl nuw i128 %7, 64
  %9 = zext i64 %47 to i128
  %10 = or disjoint i128 %8, %9
  store i128 %10, ptr %4, align 16, !tbaa !459
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
  br i1 %51, label %6, label %13, !llvm.loop !461
}

; Function Attrs: nounwind optsize
define weak { i64, i64 } @__umodti3(i64 noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3) local_unnamed_addr #2 {
  %5 = alloca i128, align 16
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %5) #40
  %6 = call fastcc { i64, i64 } @divide_u128(i64 noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3, ptr noundef nonnull %5) #44
  %7 = load i128, ptr %5, align 16, !tbaa !459
  %8 = trunc i128 %7 to i64
  %9 = lshr i128 %7, 64
  %10 = trunc i128 %9 to i64
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %5) #40
  %11 = insertvalue { i64, i64 } poison, i64 %8, 0
  %12 = insertvalue { i64, i64 } %11, i64 %10, 1
  ret { i64, i64 } %12
}

; Function Attrs: nounwind optsize
define weak { i64, i64 } @__divti3(i64 noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3) local_unnamed_addr #2 {
  %5 = alloca i128, align 16
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %5) #40
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
  %22 = call fastcc { i64, i64 } @divide_u128(i64 noundef %11, i64 noundef %13, i64 noundef %19, i64 noundef %21, ptr noundef nonnull %5) #44
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
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %5) #40
  %36 = insertvalue { i64, i64 } poison, i64 %33, 0
  %37 = insertvalue { i64, i64 } %36, i64 %35, 1
  ret { i64, i64 } %37
}

; Function Attrs: nounwind optsize
define weak { i64, i64 } @__modti3(i64 noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3) local_unnamed_addr #2 {
  %5 = alloca i128, align 16
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %5) #40
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
  %22 = call fastcc { i64, i64 } @divide_u128(i64 noundef %11, i64 noundef %13, i64 noundef %19, i64 noundef %21, ptr noundef nonnull %5) #44
  %23 = load i128, ptr %5, align 16
  %24 = sub i128 0, %23
  %25 = icmp slt i64 %1, 0
  %26 = select i1 %25, i128 %24, i128 %23
  %27 = trunc i128 %26 to i64
  %28 = lshr i128 %26, 64
  %29 = trunc i128 %28 to i64
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %5) #40
  %30 = insertvalue { i64, i64 } poison, i64 %27, 0
  %31 = insertvalue { i64, i64 } %30, i64 %29, 1
  ret { i64, i64 } %31
}

; Function Attrs: nounwind
declare void @llvm.x86.sse2.pause() #40

; Function Attrs: optsize
declare i32 @snprintf(ptr noundef, i64 noundef, ptr noundef, ...) local_unnamed_addr #3

; Function Attrs: optsize
declare double @strtod(ptr noundef, ptr noundef) local_unnamed_addr #3

; Function Attrs: noinline nounwind optsize
define internal fastcc void @string_grow(ptr nocapture noundef %0, ptr nocapture noundef readonly %1, i64 noundef %2, i64 noundef %3) unnamed_addr #14 {
  %5 = getelementptr inbounds %struct.FaString, ptr %0, i64 0, i32 2
  %6 = load i64, ptr %5, align 8, !tbaa !22
  %7 = and i64 %6, 4611686018427387904
  %8 = icmp eq i64 %7, 0
  br i1 %8, label %12, label %9

9:                                                ; preds = %4
  %10 = lshr i64 %6, 56
  %11 = and i64 %10, 31
  br label %15

12:                                               ; preds = %4
  %13 = getelementptr inbounds %struct.FaString, ptr %0, i64 0, i32 1
  %14 = load i64, ptr %13, align 8, !tbaa !21
  br label %15

15:                                               ; preds = %9, %12
  %16 = phi i64 [ %11, %9 ], [ %14, %12 ]
  %17 = add i64 %16, %2
  %18 = icmp ult i64 %17, %16
  %19 = icmp ugt i64 %17, 4611686018427387902
  %20 = or i1 %18, %19
  br i1 %20, label %21, label %22

21:                                               ; preds = %15
  tail call fastcc void @out_of_memory() #47
  unreachable

22:                                               ; preds = %15
  %23 = and i64 %6, 4611686018427387903
  %24 = icmp ne i64 %23, 0
  %25 = icmp ne i64 %23, 4611686018427387903
  %26 = and i1 %24, %25
  %27 = and i1 %8, %26
  %28 = shl nuw nsw i64 %23, 1
  %29 = tail call i64 @llvm.umax.i64(i64 %28, i64 %17)
  %30 = select i1 %27, i64 %29, i64 %17
  %31 = tail call i64 @llvm.umin.i64(i64 %30, i64 4611686018427387902)
  %32 = tail call i64 @llvm.umax.i64(i64 %31, i64 16)
  %33 = tail call ptr @fa_alloc(i64 noundef %32, i64 noundef 1) #45
  %34 = load i64, ptr %5, align 8, !tbaa !22
  %35 = and i64 %34, 4611686018427387904
  %36 = icmp eq i64 %35, 0
  br i1 %36, label %37, label %39

37:                                               ; preds = %22
  %38 = load ptr, ptr %0, align 8, !tbaa !16
  br label %39

39:                                               ; preds = %22, %37
  %40 = phi ptr [ %38, %37 ], [ %0, %22 ]
  tail call fastcc void @copy_bytes(ptr noundef %33, ptr noundef %40, i64 noundef %16) #44
  %41 = getelementptr inbounds i8, ptr %33, i64 %16
  tail call fastcc void @copy_bytes(ptr noundef %41, ptr noundef %1, i64 noundef %2) #44
  br i1 %8, label %42, label %45

42:                                               ; preds = %39
  switch i64 %23, label %43 [
    i64 4611686018427387903, label %45
    i64 0, label %45
  ]

43:                                               ; preds = %42
  %44 = load ptr, ptr %0, align 8, !tbaa !16
  tail call void @fa_free(ptr noundef %44, i64 noundef %23, i64 noundef 1) #45
  br label %45

45:                                               ; preds = %42, %42, %39, %43
  store ptr %33, ptr %0, align 8, !tbaa !16
  %46 = getelementptr inbounds %struct.FaString, ptr %0, i64 0, i32 1
  store i64 %17, ptr %46, align 8, !tbaa !21
  %47 = or i64 %32, %3
  store i64 %47, ptr %5, align 8, !tbaa !22
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(none)
define internal fastcc i32 @break_kind(i32 noundef %0) unnamed_addr #41 {
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
  %59 = load i32, ptr %58, align 8, !tbaa !120
  %60 = icmp ugt i32 %59, %0
  br i1 %60, label %52, label %61, !llvm.loop !462

61:                                               ; preds = %55
  %62 = getelementptr inbounds %struct.Range, ptr @EXTEND, i64 %57, i32 1
  %63 = load i32, ptr %62, align 4, !tbaa !122
  %64 = icmp ult i32 %63, %0
  %65 = add nuw i64 %57, 1
  br i1 %64, label %49, label %83, !llvm.loop !462

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
  %76 = load i32, ptr %75, align 8, !tbaa !120
  %77 = icmp ugt i32 %76, %0
  br i1 %77, label %69, label %78, !llvm.loop !462

78:                                               ; preds = %72
  %79 = getelementptr inbounds %struct.Range, ptr @PICTOGRAPHIC, i64 %74, i32 1
  %80 = load i32, ptr %79, align 4, !tbaa !122
  %81 = icmp ult i32 %80, %0
  %82 = add nuw i64 %74, 1
  br i1 %81, label %66, label %83, !llvm.loop !462

83:                                               ; preds = %61, %78, %69, %35, %29, %23, %20, %18, %3, %5, %1, %44, %13, %2
  %84 = phi i32 [ 2, %2 ], [ %17, %13 ], [ %48, %44 ], [ 1, %1 ], [ 3, %5 ], [ 3, %3 ], [ 5, %18 ], [ 6, %20 ], [ 8, %23 ], [ 9, %29 ], [ 10, %35 ], [ 0, %69 ], [ 7, %78 ], [ 4, %61 ]
  ret i32 %84
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: read)
define internal fastcc i64 @next_byte(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2, i8 noundef zeroext %3) unnamed_addr #12 {
  %5 = zext i8 %3 to i64
  %6 = mul nuw i64 %5, 72340172838076673
  br label %7

7:                                                ; preds = %13, %4
  %8 = phi i64 [ %2, %4 ], [ %9, %13 ]
  %9 = add i64 %8, 8
  %10 = icmp ugt i64 %9, %1
  br i1 %10, label %11, label %13

11:                                               ; preds = %7
  %12 = icmp ult i64 %8, %1
  br i1 %12, label %22, label %34

13:                                               ; preds = %7
  %14 = getelementptr inbounds i8, ptr %0, i64 %8
  %15 = load i64, ptr %14, align 1
  %16 = xor i64 %15, %6
  %17 = add i64 %16, -72340172838076673
  %18 = and i64 %16, -9187201950435737472
  %19 = xor i64 %18, -9187201950435737472
  %20 = and i64 %19, %17
  %21 = icmp eq i64 %20, 0
  br i1 %21, label %7, label %30, !llvm.loop !463

22:                                               ; preds = %11, %27
  %23 = phi i64 [ %28, %27 ], [ %8, %11 ]
  %24 = getelementptr inbounds i8, ptr %0, i64 %23
  %25 = load i8, ptr %24, align 1, !tbaa !2
  %26 = icmp eq i8 %25, %3
  br i1 %26, label %34, label %27

27:                                               ; preds = %22
  %28 = add nuw i64 %23, 1
  %29 = icmp ult i64 %28, %1
  br i1 %29, label %22, label %34, !llvm.loop !464

30:                                               ; preds = %13
  %31 = tail call i64 @llvm.cttz.i64(i64 %20, i1 true), !range !465
  %32 = lshr i64 %31, 3
  %33 = add i64 %32, %8
  br label %34

34:                                               ; preds = %22, %27, %11, %30
  %35 = phi i64 [ %33, %30 ], [ %1, %11 ], [ %23, %22 ], [ %1, %27 ]
  ret i64 %35
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc i32 @at_boundary(ptr nocapture noundef %0, i64 noundef %1) unnamed_addr #17 {
  %3 = load ptr, ptr %0, align 8, !tbaa !55
  %4 = getelementptr inbounds %struct.Boundaries, ptr %0, i64 0, i32 3
  %5 = load i64, ptr %4, align 8, !tbaa !59
  %6 = icmp ne i64 %5, 0
  %7 = icmp eq i64 %1, 0
  %8 = or i1 %7, %6
  br i1 %8, label %37, label %9

9:                                                ; preds = %2
  %10 = getelementptr inbounds %struct.Boundaries, ptr %0, i64 0, i32 1
  %11 = load i64, ptr %10, align 8, !tbaa !57
  %12 = icmp ugt i64 %11, %1
  br i1 %12, label %13, label %37

13:                                               ; preds = %9
  %14 = getelementptr inbounds i8, ptr %3, i64 %1
  %15 = load i8, ptr %14, align 1, !tbaa !2
  %16 = icmp sgt i8 %15, -1
  br i1 %16, label %17, label %25

17:                                               ; preds = %13
  %18 = getelementptr i8, ptr %14, i64 -1
  %19 = load i8, ptr %18, align 1, !tbaa !2
  %20 = icmp sgt i8 %19, -1
  br i1 %20, label %21, label %25

21:                                               ; preds = %17
  %22 = icmp eq i8 %19, 13
  %23 = icmp eq i8 %15, 10
  %24 = and i1 %23, %22
  br i1 %24, label %25, label %37

25:                                               ; preds = %21, %17, %13
  %26 = getelementptr inbounds %struct.Boundaries, ptr %0, i64 0, i32 2
  %27 = load i64, ptr %26, align 8, !tbaa !58
  %28 = icmp ult i64 %27, %1
  br i1 %28, label %29, label %33

29:                                               ; preds = %25, %29
  %30 = phi i64 [ %31, %29 ], [ %27, %25 ]
  %31 = tail call fastcc i64 @next_boundary(ptr noundef %3, i64 noundef %11, i64 noundef %30) #44
  store i64 %31, ptr %26, align 8, !tbaa !58
  %32 = icmp ult i64 %31, %1
  br i1 %32, label %29, label %33, !llvm.loop !466

33:                                               ; preds = %29, %25
  %34 = phi i64 [ %27, %25 ], [ %31, %29 ]
  %35 = icmp eq i64 %34, %1
  %36 = zext i1 %35 to i32
  br label %37

37:                                               ; preds = %2, %9, %21, %33
  %38 = phi i32 [ %36, %33 ], [ 1, %21 ], [ 1, %9 ], [ 1, %2 ]
  ret i32 %38
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.ctlz.i64(i64, i1 immarg) #7

; Function Attrs: noinline nounwind optsize
define internal fastcc ptr @take_pages(i64 noundef %0) unnamed_addr #14 {
  %2 = atomicrmw xchg ptr @spans_lock, i32 1 acquire, align 4
  %3 = icmp eq i32 %2, 0
  br i1 %3, label %4, label %8

4:                                                ; preds = %5, %1
  br label %14

5:                                                ; preds = %11, %8
  %6 = atomicrmw xchg ptr @spans_lock, i32 1 acquire, align 4
  %7 = icmp eq i32 %6, 0
  br i1 %7, label %4, label %8, !llvm.loop !5

8:                                                ; preds = %1, %5
  %9 = load atomic i32, ptr @spans_lock monotonic, align 4
  %10 = icmp eq i32 %9, 0
  br i1 %10, label %5, label %11

11:                                               ; preds = %8, %11
  tail call void @llvm.x86.sse2.pause()
  %12 = load atomic i32, ptr @spans_lock monotonic, align 4
  %13 = icmp eq i32 %12, 0
  br i1 %13, label %5, label %11, !llvm.loop !7

14:                                               ; preds = %4, %18
  %15 = phi ptr [ %16, %18 ], [ @free_spans, %4 ]
  %16 = load ptr, ptr %15, align 8, !tbaa !10
  %17 = icmp eq ptr %16, null
  br i1 %17, label %34, label %18

18:                                               ; preds = %14
  %19 = getelementptr inbounds %struct.FreeSpan, ptr %16, i64 0, i32 1
  %20 = load i64, ptr %19, align 8, !tbaa !124
  %21 = icmp ult i64 %20, %0
  br i1 %21, label %14, label %22

22:                                               ; preds = %18
  %23 = icmp eq i64 %20, %0
  br i1 %23, label %24, label %26

24:                                               ; preds = %22
  %25 = load ptr, ptr %16, align 8, !tbaa !127
  br label %32

26:                                               ; preds = %22
  %27 = shl i64 %0, 16
  %28 = getelementptr inbounds i8, ptr %16, i64 %27
  %29 = load ptr, ptr %16, align 8, !tbaa !127
  store ptr %29, ptr %28, align 8, !tbaa !127
  %30 = sub i64 %20, %0
  %31 = getelementptr inbounds %struct.FreeSpan, ptr %28, i64 0, i32 1
  store i64 %30, ptr %31, align 8, !tbaa !124
  br label %32

32:                                               ; preds = %26, %24
  %33 = phi ptr [ %28, %26 ], [ %25, %24 ]
  store ptr %33, ptr %15, align 8, !tbaa !10
  br label %52

34:                                               ; preds = %14
  %35 = tail call i64 @llvm.umax.i64(i64 %0, i64 32)
  %36 = tail call ptr @fa_pages(i64 noundef %35) #45
  %37 = icmp eq ptr %36, null
  br i1 %37, label %52, label %38

38:                                               ; preds = %34
  %39 = icmp ult i64 %0, 32
  br i1 %39, label %40, label %52

40:                                               ; preds = %38
  %41 = shl nuw nsw i64 %0, 16
  %42 = getelementptr inbounds i8, ptr %36, i64 %41
  %43 = sub nuw nsw i64 %35, %0
  %44 = getelementptr inbounds %struct.FreeSpan, ptr %42, i64 0, i32 1
  store i64 %43, ptr %44, align 8, !tbaa !124
  store ptr null, ptr %42, align 8, !tbaa !127
  br label %45

45:                                               ; preds = %45, %40
  %46 = phi ptr [ @free_spans, %40 ], [ %47, %45 ]
  %47 = load ptr, ptr %46, align 8, !tbaa !10
  %48 = icmp ne ptr %47, null
  %49 = icmp ult ptr %47, %42
  %50 = and i1 %48, %49
  br i1 %50, label %45, label %51, !llvm.loop !467

51:                                               ; preds = %45
  store ptr %47, ptr %42, align 8, !tbaa !127
  store ptr %42, ptr %46, align 8, !tbaa !10
  br label %52

52:                                               ; preds = %32, %34, %38, %51
  %53 = phi ptr [ null, %34 ], [ %36, %51 ], [ %36, %38 ], [ %16, %32 ]
  store atomic i32 0, ptr @spans_lock release, align 4
  ret ptr %53
}

; Function Attrs: optsize
declare ptr @fa_pages(i64 noundef) local_unnamed_addr #3

; Function Attrs: nounwind optsize
define internal fastcc void @mark(ptr noundef %0) unnamed_addr #2 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 17
  %3 = load i32, ptr %2, align 8, !tbaa !217
  %4 = icmp eq i32 %3, 0
  br i1 %4, label %20, label %5

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 14
  %7 = load i32, ptr %6, align 4, !tbaa !225
  %8 = icmp eq i32 %7, 0
  br i1 %8, label %9, label %38

9:                                                ; preds = %5
  %10 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 15
  %11 = load i32, ptr %10, align 8, !tbaa !219
  %12 = icmp eq i32 %11, 0
  br i1 %12, label %13, label %38

13:                                               ; preds = %9
  store i32 1, ptr %6, align 4, !tbaa !225
  %14 = load ptr, ptr @reactive_pending, align 8, !tbaa !10
  %15 = load i64, ptr @reactive_pending_count, align 8, !tbaa !8
  %16 = tail call fastcc ptr @grow_items(ptr noundef %14, i64 noundef %15, ptr noundef nonnull @reactive_pending_capacity, i64 noundef 8) #44
  store ptr %16, ptr @reactive_pending, align 8, !tbaa !10
  %17 = load i64, ptr @reactive_pending_count, align 8, !tbaa !8
  %18 = getelementptr inbounds ptr, ptr %16, i64 %17
  store ptr %0, ptr %18, align 8, !tbaa !10
  %19 = add nsw i64 %17, 1
  store i64 %19, ptr @reactive_pending_count, align 8, !tbaa !8
  br label %38

20:                                               ; preds = %1
  %21 = getelementptr inbounds %struct.FaObserver, ptr %0, i64 0, i32 11
  %22 = load i32, ptr %21, align 8, !tbaa !206
  %23 = icmp eq i32 %22, 0
  br i1 %23, label %24, label %38

24:                                               ; preds = %20
  store i32 1, ptr %21, align 8, !tbaa !206
  %25 = getelementptr inbounds %struct.FaSource, ptr %0, i64 0, i32 3
  %26 = load i64, ptr %25, align 8, !tbaa !468
  %27 = icmp sgt i64 %26, 0
  br i1 %27, label %28, label %38

28:                                               ; preds = %24
  %29 = getelementptr inbounds %struct.FaSource, ptr %0, i64 0, i32 2
  br label %30

30:                                               ; preds = %28, %30
  %31 = phi i64 [ 0, %28 ], [ %35, %30 ]
  %32 = load ptr, ptr %29, align 8, !tbaa !469
  %33 = getelementptr inbounds ptr, ptr %32, i64 %31
  %34 = load ptr, ptr %33, align 8, !tbaa !10
  tail call fastcc void @mark(ptr noundef %34) #44
  %35 = add nuw nsw i64 %31, 1
  %36 = load i64, ptr %25, align 8, !tbaa !468
  %37 = icmp slt i64 %35, %36
  br i1 %37, label %30, label %38, !llvm.loop !470

38:                                               ; preds = %30, %24, %20, %5, %9, %13
  ret void
}

; Function Attrs: noreturn optsize
declare void @_exit(i32 noundef) local_unnamed_addr #5

; Function Attrs: optsize
declare extern_weak i32 @epoll_create1(i32 noundef) #3

; Function Attrs: optsize
declare extern_weak i32 @eventfd(i32 noundef, i32 noundef) #3

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc void @timer_up(ptr nocapture noundef readonly %0, i64 noundef %1) unnamed_addr #17 {
  %3 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 6
  %4 = load ptr, ptr %3, align 8, !tbaa !293
  %5 = getelementptr inbounds %struct.FaTimer, ptr %4, i64 %1
  %6 = load i64, ptr %5, align 8, !tbaa.struct !189
  %7 = getelementptr inbounds i8, ptr %5, i64 8
  %8 = load ptr, ptr %7, align 8, !tbaa.struct !190
  %9 = getelementptr inbounds %struct.FaWait, ptr %8, i64 0, i32 7
  %10 = icmp sgt i64 %1, 0
  br i1 %10, label %11, label %35

11:                                               ; preds = %2, %28
  %12 = phi i64 [ %14, %28 ], [ %1, %2 ]
  %13 = add nsw i64 %12, -1
  %14 = lshr i64 %13, 2
  %15 = load ptr, ptr %3, align 8, !tbaa !293
  %16 = getelementptr inbounds %struct.FaTimer, ptr %15, i64 %14
  %17 = load i64, ptr %16, align 8
  %18 = getelementptr inbounds { i64, ptr }, ptr %16, i64 0, i32 1
  %19 = load ptr, ptr %18, align 8
  %20 = icmp slt i64 %6, %17
  br i1 %20, label %28, label %21

21:                                               ; preds = %11
  %22 = icmp eq i64 %6, %17
  br i1 %22, label %23, label %35

23:                                               ; preds = %21
  %24 = load i64, ptr %9, align 8, !tbaa !297
  %25 = getelementptr inbounds %struct.FaWait, ptr %19, i64 0, i32 7
  %26 = load i64, ptr %25, align 8, !tbaa !297
  %27 = icmp slt i64 %24, %26
  br i1 %27, label %28, label %35

28:                                               ; preds = %11, %23
  %29 = getelementptr inbounds %struct.FaTimer, ptr %15, i64 %12
  store i64 %17, ptr %29, align 8, !tbaa.struct !189
  %30 = getelementptr inbounds i8, ptr %29, i64 8
  store ptr %19, ptr %30, align 8, !tbaa.struct !190
  %31 = getelementptr inbounds %struct.FaWait, ptr %19, i64 0, i32 8
  store i64 %12, ptr %31, align 8, !tbaa !289
  %32 = icmp ult i64 %13, 4
  br i1 %32, label %33, label %11

33:                                               ; preds = %28
  %34 = load ptr, ptr %3, align 8, !tbaa !293
  br label %35

35:                                               ; preds = %23, %21, %33, %2
  %36 = phi ptr [ %4, %2 ], [ %34, %33 ], [ %15, %21 ], [ %15, %23 ]
  %37 = phi i64 [ %1, %2 ], [ %14, %33 ], [ %12, %21 ], [ %12, %23 ]
  %38 = getelementptr inbounds %struct.FaTimer, ptr %36, i64 %37
  store i64 %6, ptr %38, align 8, !tbaa.struct !189
  %39 = getelementptr inbounds i8, ptr %38, i64 8
  store ptr %8, ptr %39, align 8, !tbaa.struct !190
  %40 = getelementptr inbounds %struct.FaWait, ptr %8, i64 0, i32 8
  store i64 %37, ptr %40, align 8, !tbaa !289
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @fire_timers(ptr nocapture noundef %0, i64 noundef %1) unnamed_addr #2 {
  %3 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 6
  %4 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 7
  %5 = load i64, ptr %4, align 8, !tbaa !291
  %6 = icmp sgt i64 %5, 0
  br i1 %6, label %7, label %39

7:                                                ; preds = %2
  %8 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 1
  %9 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 4
  br label %10

10:                                               ; preds = %7, %36
  %11 = load ptr, ptr %3, align 8, !tbaa !293
  %12 = load i64, ptr %11, align 8, !tbaa !322
  %13 = icmp sgt i64 %12, %1
  br i1 %13, label %39, label %14

14:                                               ; preds = %10
  %15 = getelementptr inbounds %struct.FaTimer, ptr %11, i64 0, i32 1
  %16 = load ptr, ptr %15, align 8, !tbaa !298
  tail call fastcc void @timer_remove(ptr noundef nonnull %0, i64 noundef 0) #44
  %17 = getelementptr inbounds %struct.FaWait, ptr %16, i64 0, i32 11
  %18 = load ptr, ptr %17, align 8, !tbaa !290
  %19 = icmp eq ptr %18, null
  br i1 %19, label %21, label %20

20:                                               ; preds = %14
  tail call void %18(ptr noundef nonnull %16) #45
  br label %36

21:                                               ; preds = %14
  %22 = load ptr, ptr %16, align 8, !tbaa !286
  %23 = getelementptr inbounds i8, ptr %22, i64 16
  %24 = getelementptr inbounds i8, ptr %22, i64 44
  %25 = load i32, ptr %24, align 4, !tbaa !267
  %26 = and i32 %25, -5
  store i32 %26, ptr %24, align 4, !tbaa !267
  %27 = getelementptr inbounds i8, ptr %22, i64 24
  store ptr null, ptr %27, align 8, !tbaa !269
  %28 = load ptr, ptr %8, align 8, !tbaa !10
  %29 = getelementptr inbounds i8, ptr %22, i64 32
  store ptr %28, ptr %29, align 8, !tbaa !270
  %30 = icmp eq ptr %28, null
  %31 = getelementptr inbounds %struct.FaTask, ptr %28, i64 0, i32 1
  %32 = select i1 %30, ptr %0, ptr %31
  store ptr %23, ptr %32, align 8, !tbaa !10
  store ptr %23, ptr %8, align 8, !tbaa !10
  %33 = load <2 x i64>, ptr %9, align 8, !tbaa !8
  %34 = add nsw <2 x i64> %33, <i64 1, i64 1>
  store <2 x i64> %34, ptr %9, align 8, !tbaa !8
  %35 = getelementptr inbounds i8, ptr %22, i64 40
  store atomic i32 1, ptr %35 release, align 8
  br label %36

36:                                               ; preds = %21, %20
  %37 = load i64, ptr %4, align 8, !tbaa !291
  %38 = icmp sgt i64 %37, 0
  br i1 %38, label %10, label %39, !llvm.loop !471

39:                                               ; preds = %10, %36, %2
  ret void
}

; Function Attrs: cold noinline noreturn nounwind optsize
define internal fastcc void @deadlock_stop(ptr nocapture noundef readonly %0) unnamed_addr #31 {
  tail call void @fa_flush() #44
  %2 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %2, align 8, !tbaa !8
  tail call fastcc void @append_c(ptr noundef nonnull @.str.50) #44
  %3 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 25
  %4 = load ptr, ptr %3, align 8, !tbaa !276
  %5 = icmp eq ptr %4, null
  br i1 %5, label %6, label %10

6:                                                ; preds = %1
  %7 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 26
  %8 = load ptr, ptr %7, align 8, !tbaa !411
  %9 = icmp eq ptr %8, null
  br i1 %9, label %14, label %10

10:                                               ; preds = %6, %1
  %11 = phi ptr [ @.str.51, %1 ], [ @.str.53, %6 ]
  %12 = phi ptr [ %3, %1 ], [ %7, %6 ]
  tail call fastcc void @append_c(ptr noundef nonnull %11) #44
  %13 = load ptr, ptr %12, align 8, !tbaa !10
  tail call fastcc void @append_c(ptr noundef %13) #44
  tail call fastcc void @append_c(ptr noundef nonnull @.str.52) #44
  br label %14

14:                                               ; preds = %10, %6
  tail call void @fa_b_line(i32 noundef 2) #44
  tail call void @fa_exit(i32 noundef 101) #46
  unreachable
}

; Function Attrs: optsize
declare void @fa_resume(ptr noundef) local_unnamed_addr #3

; Function Attrs: optsize
declare extern_weak i32 @epoll_wait(i32 noundef, ptr noundef, i32 noundef, i32 noundef) local_unnamed_addr #3

; Function Attrs: optsize
declare extern_weak i64 @read(i32 noundef, ptr noundef, i64 noundef) local_unnamed_addr #3

; Function Attrs: optsize
declare extern_weak i32 @pthread_cond_timedwait(ptr noundef, ptr noundef, ptr noundef) #3

; Function Attrs: optsize
declare extern_weak i32 @clock_gettime(i32 noundef, ptr noundef) #3

; Function Attrs: nounwind optsize
define internal void @socket_drop(ptr nocapture noundef %0) #2 {
  tail call fastcc void @socket_close(ptr noundef %0) #44
  ret void
}

; Function Attrs: optsize
declare extern_weak i32 @poll(ptr noundef, i64 noundef, i32 noundef) #3

; Function Attrs: optsize
declare extern_weak ptr @__errno_location() #3

; Function Attrs: nounwind optsize
define internal fastcc noundef ptr @net_error(i32 noundef %0, ptr nocapture noundef readonly %1) unnamed_addr #2 {
  %3 = alloca %struct.FaString, align 8
  %4 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder_length)
  store i64 0, ptr %4, align 8, !tbaa !8
  tail call fastcc void @append_c(ptr noundef %1) #44
  tail call fastcc void @append_c(ptr noundef nonnull @.str.56) #44
  tail call fastcc void @append_error(i32 noundef %0) #44
  %5 = tail call ptr @fa_alloc(i64 noundef 32, i64 noundef 8) #45
  store i64 11, ptr %5, align 8, !tbaa !232
  %6 = getelementptr inbounds %struct.MessageError, ptr %5, i64 0, i32 1
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %3) #40
  %7 = tail call align 8 ptr @llvm.threadlocal.address.p0(ptr align 8 @builder)
  %8 = load ptr, ptr %7, align 8, !tbaa !10
  %9 = load i64, ptr %4, align 8, !tbaa !8
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 8 %3, ptr noundef %8, i64 noundef %9, i64 noundef 0) #44
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %6, ptr noundef nonnull align 8 dereferenceable(24) %3, i64 24, i1 false), !tbaa.struct !13
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %3) #40
  store i64 0, ptr %4, align 8, !tbaa !8
  ret ptr %5
}

; Function Attrs: optsize
declare extern_weak i32 @accept4(i32 noundef, ptr noundef, ptr noundef, i32 noundef) local_unnamed_addr #3

; Function Attrs: nounwind optsize
define internal fastcc void @append_error(i32 noundef %0) unnamed_addr #2 {
  switch i32 %0, label %6 [
    i32 -1, label %2
    i32 38, label %3
    i32 9, label %3
  ]

2:                                                ; preds = %1
  tail call fastcc void @append_c(ptr noundef nonnull @.str.57) #44
  br label %14

3:                                                ; preds = %1, %1
  %4 = icmp eq i32 %0, 38
  %5 = select i1 %4, ptr @.str.58, ptr @.str.59
  tail call fastcc void @append_c(ptr noundef nonnull %5) #44
  br label %14

6:                                                ; preds = %1
  br i1 icmp ne (ptr @strerror, ptr null), label %7, label %9

7:                                                ; preds = %6
  %8 = tail call ptr @strerror(i32 noundef %0) #45
  tail call fastcc void @append_c(ptr noundef %8) #44
  br label %14

9:                                                ; preds = %6
  tail call fastcc void @append_c(ptr noundef nonnull @.str.60) #44
  %10 = sext i32 %0 to i64
  %11 = tail call i64 @llvm.abs.i64(i64 %10, i1 true)
  %12 = lshr i64 %10, 63
  %13 = trunc i64 %12 to i32
  tail call fastcc void @append_u64(i64 noundef %11, i32 noundef %13) #44
  br label %14

14:                                               ; preds = %9, %7, %3, %2
  ret void
}

; Function Attrs: optsize
declare extern_weak ptr @strerror(i32 noundef) #3

; Function Attrs: optsize
declare extern_weak i32 @connect(i32 noundef, ptr noundef, i32 noundef) local_unnamed_addr #3

; Function Attrs: optsize
declare extern_weak i32 @getsockopt(i32 noundef, i32 noundef, i32 noundef, ptr noundef, ptr noundef) local_unnamed_addr #3

; Function Attrs: optsize
declare extern_weak i64 @send(i32 noundef, ptr noundef, i64 noundef, i32 noundef) local_unnamed_addr #3

; Function Attrs: optsize
declare extern_weak i32 @open(ptr noundef, i32 noundef, ...) #3

; Function Attrs: nounwind optsize
define internal fastcc void @remote_push(ptr nocapture noundef %0, ptr noundef %1) unnamed_addr #2 {
  %3 = alloca i64, align 8
  %4 = getelementptr inbounds %struct.FaTask, ptr %1, i64 0, i32 3
  store atomic i32 5, ptr %4 release, align 8
  %5 = getelementptr inbounds %struct.FaTask, ptr %1, i64 0, i32 1
  store atomic i64 0, ptr %5 monotonic, align 8
  %6 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 16
  %7 = ptrtoint ptr %1 to i64
  %8 = atomicrmw xchg ptr %6, i64 %7 acq_rel, align 8
  %9 = inttoptr i64 %8 to ptr
  %10 = getelementptr inbounds %struct.FaTask, ptr %9, i64 0, i32 1
  store atomic i64 %7, ptr %10 release, align 8
  %11 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 18
  %12 = atomicrmw xchg ptr %11, i32 0 seq_cst, align 8
  %13 = icmp eq i32 %12, 0
  br i1 %13, label %20, label %14

14:                                               ; preds = %2
  %15 = getelementptr inbounds %struct.FaExecutor, ptr %0, i64 0, i32 20
  %16 = load i32, ptr %15, align 8, !tbaa !264
  %17 = icmp sgt i32 %16, -1
  br i1 %17, label %18, label %20

18:                                               ; preds = %14
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %3) #40
  store i64 1, ptr %3, align 8, !tbaa !8
  %19 = call i64 @write(i32 noundef %16, ptr noundef nonnull %3, i64 noundef 8) #45
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %3) #40
  br label %20

20:                                               ; preds = %18, %14, %2
  ret void
}

; Function Attrs: nounwind optsize
define internal void @job_timer_fired(ptr nocapture noundef %0) #2 {
  %2 = getelementptr inbounds %struct.FaWait, ptr %0, i64 0, i32 4
  %3 = load ptr, ptr %2, align 8, !tbaa !425
  %4 = getelementptr inbounds %struct.FaCall, ptr %3, i64 0, i32 10
  %5 = load ptr, ptr %4, align 8, !tbaa !423
  %6 = tail call i64 @fa_clock() #45
  %7 = getelementptr inbounds %struct.FaWorkerPool, ptr %5, i64 0, i32 10
  %8 = load i64, ptr %7, align 16, !tbaa !401
  %9 = mul nsw i64 %8, 1000000
  %10 = getelementptr inbounds %struct.FaWait, ptr %0, i64 0, i32 9
  store i32 2, ptr %10, align 8, !tbaa !309
  %11 = getelementptr inbounds %struct.FaWorkerPool, ptr %5, i64 0, i32 1
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %12, label %14

12:                                               ; preds = %1
  %13 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %11) #45
  br label %14

14:                                               ; preds = %1, %12
  %15 = getelementptr inbounds %struct.FaCall, ptr %3, i64 0, i32 8
  %16 = load i32, ptr %15, align 8, !tbaa !391
  switch i32 %16, label %30 [
    i32 0, label %17
    i32 1, label %33
  ]

17:                                               ; preds = %14
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %18, label %20

18:                                               ; preds = %17
  %19 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %11) #45
  br label %20

20:                                               ; preds = %17, %18
  %21 = add nsw i64 %9, %6
  %22 = getelementptr inbounds %struct.FaCall, ptr %3, i64 0, i32 14
  %23 = load ptr, ptr %22, align 8, !tbaa !421
  %24 = getelementptr inbounds i8, ptr %3, i64 144
  store ptr %24, ptr %23, align 8, !tbaa !286
  %25 = getelementptr inbounds %struct.FaWait, ptr %23, i64 0, i32 4
  store ptr %3, ptr %25, align 8, !tbaa !425
  %26 = getelementptr inbounds %struct.FaWait, ptr %23, i64 0, i32 11
  store ptr @job_timer_fired, ptr %26, align 8, !tbaa !290
  %27 = getelementptr inbounds %struct.FaWait, ptr %23, i64 0, i32 9
  store i32 0, ptr %27, align 8, !tbaa !309
  %28 = getelementptr inbounds %struct.FaCall, ptr %3, i64 0, i32 2
  %29 = load ptr, ptr %28, align 8, !tbaa !381
  tail call fastcc void @timer_insert(ptr noundef %29, ptr noundef nonnull %23, i64 noundef %21) #44
  br label %122

30:                                               ; preds = %14
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %31, label %122

31:                                               ; preds = %30
  %32 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %11) #45
  br label %122

33:                                               ; preds = %14
  %34 = getelementptr inbounds %struct.FaCall, ptr %3, i64 0, i32 11
  %35 = load i64, ptr %34, align 8, !tbaa !417
  %36 = sub nsw i64 %6, %35
  %37 = icmp slt i64 %36, %9
  br i1 %37, label %38, label %53

38:                                               ; preds = %33
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %39, label %42

39:                                               ; preds = %38
  %40 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %11) #45
  %41 = load i64, ptr %34, align 8, !tbaa !417
  br label %42

42:                                               ; preds = %38, %39
  %43 = phi i64 [ %35, %38 ], [ %41, %39 ]
  %44 = add nsw i64 %43, %9
  %45 = getelementptr inbounds %struct.FaCall, ptr %3, i64 0, i32 14
  %46 = load ptr, ptr %45, align 8, !tbaa !421
  %47 = getelementptr inbounds i8, ptr %3, i64 144
  store ptr %47, ptr %46, align 8, !tbaa !286
  %48 = getelementptr inbounds %struct.FaWait, ptr %46, i64 0, i32 4
  store ptr %3, ptr %48, align 8, !tbaa !425
  %49 = getelementptr inbounds %struct.FaWait, ptr %46, i64 0, i32 11
  store ptr @job_timer_fired, ptr %49, align 8, !tbaa !290
  %50 = getelementptr inbounds %struct.FaWait, ptr %46, i64 0, i32 9
  store i32 0, ptr %50, align 8, !tbaa !309
  %51 = getelementptr inbounds %struct.FaCall, ptr %3, i64 0, i32 2
  %52 = load ptr, ptr %51, align 8, !tbaa !381
  tail call fastcc void @timer_insert(ptr noundef %52, ptr noundef nonnull %46, i64 noundef %44) #44
  br label %122

53:                                               ; preds = %33
  store i32 3, ptr %15, align 8, !tbaa !391
  %54 = getelementptr inbounds %struct.FaCall, ptr %3, i64 0, i32 1
  %55 = load ptr, ptr %54, align 8, !tbaa !395
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %56, label %58

56:                                               ; preds = %53
  %57 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %11) #45
  br label %58

58:                                               ; preds = %53, %56
  %59 = getelementptr inbounds i8, ptr %3, i64 144
  %60 = getelementptr inbounds %struct.FaCall, ptr %3, i64 0, i32 15
  %61 = load ptr, ptr %60, align 8, !tbaa !424
  tail call void %61(ptr noundef nonnull %59) #45
  %62 = getelementptr inbounds i8, ptr %3, i64 160
  %63 = getelementptr inbounds i8, ptr %3, i64 188
  %64 = load i32, ptr %63, align 4, !tbaa !267
  %65 = or i32 %64, 2
  store i32 %65, ptr %63, align 4, !tbaa !267
  %66 = getelementptr inbounds %struct.FaCall, ptr %3, i64 0, i32 2
  %67 = load ptr, ptr %66, align 8, !tbaa !381
  tail call void @fa_remote_source_end(ptr noundef %67) #44
  %68 = getelementptr inbounds %struct.FaWorkerPool, ptr %5, i64 0, i32 12
  %69 = load ptr, ptr %68, align 16, !tbaa !400
  %70 = tail call ptr %69(ptr noundef nonnull %5) #45
  %71 = getelementptr inbounds %struct.FaWorkerPool, ptr %5, i64 0, i32 5
  %72 = load i64, ptr %71, align 8, !tbaa !404
  %73 = icmp sgt i64 %72, 0
  br i1 %73, label %74, label %76

74:                                               ; preds = %58
  %75 = getelementptr inbounds %struct.FaWorkerPool, ptr %5, i64 0, i32 4
  br label %93

76:                                               ; preds = %100, %58
  %77 = icmp eq ptr %55, null
  br i1 %77, label %80, label %78

78:                                               ; preds = %76
  %79 = atomicrmw add ptr %55, i64 1 monotonic, align 8
  br label %80

80:                                               ; preds = %76, %78
  %81 = atomicrmw add ptr %5, i64 1 monotonic, align 8
  br i1 icmp ne (ptr @pthread_mutex_lock, ptr null), label %82, label %84

82:                                               ; preds = %80
  %83 = tail call i32 @pthread_mutex_lock(ptr noundef nonnull %11) #45
  br label %84

84:                                               ; preds = %80, %82
  %85 = getelementptr inbounds %struct.FaWorker, ptr %55, i64 0, i32 11
  store i32 1, ptr %85, align 4, !tbaa !387
  %86 = getelementptr inbounds %struct.FaWorker, ptr %55, i64 0, i32 9
  store i32 1, ptr %86, align 4, !tbaa !380
  %87 = getelementptr inbounds %struct.FaWorkerPool, ptr %5, i64 0, i32 2
  %88 = tail call i32 @pthread_cond_broadcast(ptr noundef nonnull %87) #45
  br i1 icmp ne (ptr @pthread_mutex_unlock, ptr null), label %89, label %91

89:                                               ; preds = %84
  %90 = tail call i32 @pthread_mutex_unlock(ptr noundef nonnull %11) #45
  br label %91

91:                                               ; preds = %84, %89
  %92 = getelementptr inbounds %struct.FaWorker, ptr %55, i64 0, i32 10
  store i32 1, ptr %92, align 16, !tbaa !372
  br i1 icmp ne (ptr @pthread_detach, ptr null), label %103, label %107

93:                                               ; preds = %74, %100
  %94 = phi i64 [ 0, %74 ], [ %101, %100 ]
  %95 = load ptr, ptr %75, align 16, !tbaa !403
  %96 = getelementptr inbounds ptr, ptr %95, i64 %94
  %97 = load ptr, ptr %96, align 8, !tbaa !10
  %98 = icmp eq ptr %97, %55
  br i1 %98, label %99, label %100

99:                                               ; preds = %93
  store ptr %70, ptr %96, align 8, !tbaa !10
  br label %100

100:                                              ; preds = %93, %99
  %101 = add nuw nsw i64 %94, 1
  %102 = icmp eq i64 %101, %72
  br i1 %102, label %76, label %93, !llvm.loop !472

103:                                              ; preds = %91
  %104 = getelementptr inbounds %struct.FaWorker, ptr %55, i64 0, i32 7
  %105 = load i64, ptr %104, align 16, !tbaa !386
  %106 = tail call i32 @pthread_detach(i64 noundef %105) #45
  br label %107

107:                                              ; preds = %103, %91
  tail call void @fa_shared_release(ptr noundef nonnull %55) #44
  %108 = load ptr, ptr %66, align 8, !tbaa !381
  %109 = load i32, ptr %63, align 4, !tbaa !267
  %110 = and i32 %109, -5
  store i32 %110, ptr %63, align 4, !tbaa !267
  %111 = getelementptr inbounds %struct.FaExecutor, ptr %108, i64 0, i32 1
  %112 = getelementptr inbounds i8, ptr %3, i64 168
  store ptr null, ptr %112, align 8, !tbaa !269
  %113 = load ptr, ptr %111, align 8, !tbaa !10
  %114 = getelementptr inbounds i8, ptr %3, i64 176
  store ptr %113, ptr %114, align 8, !tbaa !270
  %115 = icmp eq ptr %113, null
  %116 = getelementptr inbounds %struct.FaTask, ptr %113, i64 0, i32 1
  %117 = select i1 %115, ptr %108, ptr %116
  store ptr %62, ptr %117, align 8, !tbaa !10
  store ptr %62, ptr %111, align 8, !tbaa !10
  %118 = getelementptr inbounds %struct.FaExecutor, ptr %108, i64 0, i32 4
  %119 = load <2 x i64>, ptr %118, align 8, !tbaa !8
  %120 = add nsw <2 x i64> %119, <i64 1, i64 1>
  store <2 x i64> %120, ptr %118, align 8, !tbaa !8
  %121 = getelementptr inbounds i8, ptr %3, i64 184
  store atomic i32 1, ptr %121 release, align 8
  br label %122

122:                                              ; preds = %31, %30, %107, %42, %20
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @channel_push(ptr nocapture noundef %0, ptr nocapture noundef readonly %1) unnamed_addr #2 {
  %3 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 7
  %4 = load i64, ptr %3, align 16, !tbaa !433
  %5 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 8
  %6 = load i64, ptr %5, align 8, !tbaa !434
  %7 = icmp eq i64 %4, %6
  br i1 %7, label %13, label %8

8:                                                ; preds = %2
  %9 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 6
  %10 = load i64, ptr %9, align 8, !tbaa !435
  %11 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 5
  %12 = load ptr, ptr %11, align 16, !tbaa !436
  br label %58

13:                                               ; preds = %2
  %14 = icmp slt i64 %4, 4
  %15 = shl nuw nsw i64 %4, 1
  %16 = select i1 %14, i64 4, i64 %15
  %17 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 4
  %18 = load i64, ptr %17, align 8, !tbaa !431
  %19 = mul nsw i64 %18, %16
  %20 = tail call ptr @fa_alloc(i64 noundef %19, i64 noundef 16) #45
  %21 = icmp eq ptr %20, null
  br i1 %21, label %28, label %22

22:                                               ; preds = %13
  %23 = load i64, ptr %3, align 16, !tbaa !433
  %24 = icmp sgt i64 %23, 0
  br i1 %24, label %25, label %29

25:                                               ; preds = %22
  %26 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 6
  %27 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 5
  br label %33

28:                                               ; preds = %13
  tail call fastcc void @out_of_memory() #47
  unreachable

29:                                               ; preds = %33, %22
  %30 = phi i64 [ %23, %22 ], [ %46, %33 ]
  %31 = load i64, ptr %5, align 8, !tbaa !434
  %32 = icmp sgt i64 %31, 0
  br i1 %32, label %48, label %54

33:                                               ; preds = %25, %33
  %34 = phi i64 [ 0, %25 ], [ %45, %33 ]
  %35 = load i64, ptr %26, align 8, !tbaa !435
  %36 = add nsw i64 %35, %34
  %37 = load i64, ptr %5, align 8, !tbaa !434
  %38 = srem i64 %36, %37
  %39 = load i64, ptr %17, align 8, !tbaa !431
  %40 = mul nsw i64 %39, %34
  %41 = getelementptr inbounds i8, ptr %20, i64 %40
  %42 = load ptr, ptr %27, align 16, !tbaa !436
  %43 = mul nsw i64 %39, %38
  %44 = getelementptr inbounds i8, ptr %42, i64 %43
  tail call fastcc void @copy_bytes(ptr noundef nonnull %41, ptr noundef %44, i64 noundef %39) #44
  %45 = add nuw nsw i64 %34, 1
  %46 = load i64, ptr %3, align 16, !tbaa !433
  %47 = icmp slt i64 %45, %46
  br i1 %47, label %33, label %29, !llvm.loop !473

48:                                               ; preds = %29
  %49 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 5
  %50 = load ptr, ptr %49, align 16, !tbaa !436
  %51 = load i64, ptr %17, align 8, !tbaa !431
  %52 = mul nsw i64 %51, %31
  tail call void @fa_free(ptr noundef %50, i64 noundef %52, i64 noundef 16) #45
  %53 = load i64, ptr %3, align 16, !tbaa !433
  br label %54

54:                                               ; preds = %48, %29
  %55 = phi i64 [ %53, %48 ], [ %30, %29 ]
  %56 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 5
  store ptr %20, ptr %56, align 16, !tbaa !436
  %57 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 6
  store i64 0, ptr %57, align 8, !tbaa !435
  store i64 %16, ptr %5, align 8, !tbaa !434
  br label %58

58:                                               ; preds = %8, %54
  %59 = phi ptr [ %12, %8 ], [ %20, %54 ]
  %60 = phi i64 [ %6, %8 ], [ %16, %54 ]
  %61 = phi i64 [ %4, %8 ], [ %55, %54 ]
  %62 = phi i64 [ %10, %8 ], [ 0, %54 ]
  %63 = add nsw i64 %61, %62
  %64 = srem i64 %63, %60
  %65 = getelementptr inbounds %struct.FaChannel, ptr %0, i64 0, i32 4
  %66 = load i64, ptr %65, align 8, !tbaa !431
  %67 = mul nsw i64 %66, %64
  %68 = getelementptr inbounds i8, ptr %59, i64 %67
  tail call fastcc void @copy_bytes(ptr noundef %68, ptr noundef %1, i64 noundef %66) #44
  %69 = load i64, ptr %3, align 16, !tbaa !433
  %70 = add nsw i64 %69, 1
  store i64 %70, ptr %3, align 16, !tbaa !433
  ret void
}

; Function Attrs: optsize
declare extern_weak i64 @sysconf(i32 noundef) #3

; Function Attrs: nocallback nofree nosync nounwind willreturn memory(inaccessiblemem: write)
declare void @llvm.assume(i1 noundef) #42

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.abs.i64(i64, i1 immarg) #43

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.umin.i64(i64, i64) #43

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.umax.i64(i64, i64) #43

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.smin.i64(i64, i64) #43

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.smax.i64(i64, i64) #43

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i16 @llvm.bswap.i16(i16) #43

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.fshl.i64(i64, i64, i64) #43

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i128 @llvm.abs.i128(i128, i1 immarg) #43

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.smin.i32(i32, i32) #43

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.abs.i32(i32, i1 immarg) #43

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.usub.sat.i64(i64, i64) #43

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.vector.reduce.or.v4i32(<4 x i32>) #43

attributes #0 = { alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: readwrite) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #1 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: readwrite) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #2 = { nounwind optsize "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #3 = { optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #4 = { noreturn nounwind optsize "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #5 = { noreturn optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #6 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, inaccessiblemem: none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #7 = { mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #8 = { mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite) }
attributes #9 = { mustprogress nocallback nofree nounwind willreturn memory(argmem: readwrite) }
attributes #10 = { nofree norecurse nosync nounwind optsize memory(argmem: readwrite) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #11 = { mustprogress nocallback nofree nounwind willreturn memory(argmem: write) }
attributes #12 = { nofree norecurse nosync nounwind optsize memory(argmem: read) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #13 = { alwaysinline nounwind optsize "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #14 = { noinline nounwind optsize "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #15 = { alwaysinline nofree norecurse nosync nounwind optsize memory(argmem: read) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #16 = { nofree noinline norecurse nosync nounwind optsize memory(argmem: read) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #17 = { nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #18 = { nofree noinline norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #19 = { cold noinline nounwind optsize "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #20 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #21 = { alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(readwrite, inaccessiblemem: none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #22 = { mustprogress nofree noinline norecurse nosync nounwind optsize willreturn memory(none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #23 = { alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #24 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(readwrite, inaccessiblemem: none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #25 = { nofree norecurse nosync nounwind optsize memory(read, inaccessiblemem: none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #26 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(read, inaccessiblemem: none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #27 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, argmem: none, inaccessiblemem: none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #28 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(read, argmem: none, inaccessiblemem: none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #29 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #30 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: write) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #31 = { cold noinline noreturn nounwind optsize "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #32 = { cold mustprogress nofree noinline norecurse nosync nounwind optsize willreturn memory(none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #33 = { alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(read, inaccessiblemem: none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #34 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, argmem: readwrite, inaccessiblemem: none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #35 = { alwaysinline nofree norecurse nosync nounwind optsize memory(read, inaccessiblemem: none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #36 = { mustprogress nofree norecurse nounwind optsize willreturn memory(argmem: readwrite) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #37 = { mustprogress nofree norecurse nounwind optsize willreturn memory(readwrite, inaccessiblemem: none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #38 = { nofree norecurse nounwind optsize memory(readwrite, inaccessiblemem: none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #39 = { nofree norecurse nosync nounwind optsize memory(argmem: write) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #40 = { nounwind }
attributes #41 = { nofree norecurse nosync nounwind optsize memory(none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #42 = { nocallback nofree nosync nounwind willreturn memory(inaccessiblemem: write) }
attributes #43 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #44 = { optsize }
attributes #45 = { nounwind optsize }
attributes #46 = { noreturn nounwind optsize }
attributes #47 = { noreturn optsize }
attributes #48 = { cold optsize }
attributes #49 = { cold noreturn optsize }

!llvm.module.flags = !{!0, !1}

!0 = !{i32 1, !"wchar_size", i32 4}
!1 = !{i32 8, !"PIC Level", i32 2}
!2 = !{!3, !3, i64 0}
!3 = !{!"omnipotent char", !4, i64 0}
!4 = !{!"Simple C/C++ TBAA"}
!5 = distinct !{!5, !6}
!6 = !{!"llvm.loop.mustprogress"}
!7 = distinct !{!7, !6}
!8 = !{!9, !9, i64 0}
!9 = !{!"long", !3, i64 0}
!10 = !{!11, !11, i64 0}
!11 = !{!"any pointer", !3, i64 0}
!12 = distinct !{!12, !6}
!13 = !{i64 0, i64 8, !10, i64 8, i64 8, !8, i64 16, i64 8, !8}
!14 = !{i64 0, i64 8, !8, i64 8, i64 8, !8}
!15 = !{i64 0, i64 8, !8}
!16 = !{!17, !11, i64 0}
!17 = !{!"", !11, i64 0, !9, i64 8, !9, i64 16}
!18 = !{!19}
!19 = distinct !{!19, !20, !"ascii_string: argument 0"}
!20 = distinct !{!20, !"ascii_string"}
!21 = !{!17, !9, i64 8}
!22 = !{!17, !9, i64 16}
!23 = !{i64 0, i64 -9223372036854775807}
!24 = distinct !{!24, !6}
!25 = !{!26, !26, i64 0}
!26 = !{!"int", !3, i64 0}
!27 = distinct !{!27, !6}
!28 = distinct !{!28, !6}
!29 = distinct !{!29, !6}
!30 = distinct !{!30, !6}
!31 = distinct !{!31, !6}
!32 = distinct !{!32, !6}
!33 = distinct !{!33, !6}
!34 = distinct !{!34, !6}
!35 = distinct !{!35, !6}
!36 = distinct !{!36, !6}
!37 = distinct !{!37, !6}
!38 = distinct !{!38, !6}
!39 = distinct !{!39, !6}
!40 = distinct !{!40, !6}
!41 = distinct !{!41, !6}
!42 = distinct !{!42, !6}
!43 = distinct !{!43, !6}
!44 = distinct !{!44, !6}
!45 = distinct !{!45, !6}
!46 = !{!"branch_weights", i32 4000000, i32 4001}
!47 = !{!"branch_weights", i32 1, i32 2000}
!48 = distinct !{!48, !6}
!49 = distinct !{!49, !6}
!50 = !{i32 0, i32 13}
!51 = distinct !{!51, !6}
!52 = distinct !{!52, !6}
!53 = distinct !{!53, !6}
!54 = distinct !{!54, !6}
!55 = !{!56, !11, i64 0}
!56 = !{!"", !11, i64 0, !9, i64 8, !9, i64 16, !9, i64 24}
!57 = !{!56, !9, i64 8}
!58 = !{!56, !9, i64 16}
!59 = !{!56, !9, i64 24}
!60 = !{i32 0, i32 2}
!61 = distinct !{!61, !6}
!62 = distinct !{!62, !6}
!63 = distinct !{!63, !6}
!64 = distinct !{!64, !6}
!65 = !{!66, !11, i64 0}
!66 = !{!"", !11, i64 0, !9, i64 8, !9, i64 16, !11, i64 24, !9, i64 32, !26, i64 40, !26, i64 44}
!67 = !{!66, !9, i64 8}
!68 = !{!66, !9, i64 16}
!69 = !{!66, !11, i64 24}
!70 = !{!66, !9, i64 32}
!71 = !{!66, !26, i64 40}
!72 = !{!66, !26, i64 44}
!73 = !{ptr @replace_one, ptr @split_one}
!74 = distinct !{!74, !6}
!75 = distinct !{!75, !6}
!76 = distinct !{!76, !6}
!77 = !{!"branch_weights", i32 2000, i32 1}
!78 = distinct !{!78, !6}
!79 = !{!80, !11, i64 0}
!80 = !{!"", !11, i64 0, !9, i64 8, !9, i64 16, !11, i64 24, !9, i64 32}
!81 = !{!80, !9, i64 8}
!82 = !{!80, !9, i64 16}
!83 = !{!80, !11, i64 24}
!84 = !{!80, !9, i64 32}
!85 = distinct !{!85, !6}
!86 = distinct !{!86, !6}
!87 = distinct !{!87, !6}
!88 = distinct !{!88, !6}
!89 = distinct !{!89, !6}
!90 = distinct !{!90, !6}
!91 = distinct !{!91, !6}
!92 = distinct !{!92, !6}
!93 = distinct !{!93, !6}
!94 = !{!95, !9, i64 0}
!95 = !{!"", !9, i64 0, !9, i64 8, !9, i64 16}
!96 = !{!95, !9, i64 8}
!97 = !{!95, !9, i64 16}
!98 = distinct !{!98, !6}
!99 = distinct !{!99, !6}
!100 = distinct !{!100, !6}
!101 = distinct !{!101, !6}
!102 = distinct !{!102, !6}
!103 = distinct !{!103, !6}
!104 = !{i64 0, i64 1, !8}
!105 = distinct !{!105, !6}
!106 = distinct !{!106, !6}
!107 = distinct !{!107, !6}
!108 = distinct !{!108, !6}
!109 = distinct !{!109, !6}
!110 = distinct !{!110, !6}
!111 = !{i64 0, i64 65}
!112 = !{!113, !9, i64 0}
!113 = !{!"", !9, i64 0, !9, i64 8, !3, i64 16}
!114 = !{!113, !9, i64 8}
!115 = distinct !{!115, !6}
!116 = distinct !{!116, !6}
!117 = distinct !{!117, !6}
!118 = !{!119, !11, i64 0}
!119 = !{!"FreeBlock", !11, i64 0}
!120 = !{!121, !26, i64 0}
!121 = !{!"", !26, i64 0, !26, i64 4}
!122 = !{!121, !26, i64 4}
!123 = distinct !{!123, !6}
!124 = !{!125, !9, i64 8}
!125 = !{!"FreeSpan", !11, i64 0, !9, i64 8}
!126 = distinct !{!126, !6}
!127 = !{!125, !11, i64 0}
!128 = !{!129, !11, i64 0}
!129 = !{!"FaArena", !11, i64 0, !11, i64 8, !11, i64 16, !11, i64 24, !11, i64 32}
!130 = !{!131, !11, i64 8}
!131 = !{!"FaArenaCleanup", !11, i64 0, !11, i64 8, !11, i64 16}
!132 = !{!131, !11, i64 16}
!133 = distinct !{!133, !6}
!134 = !{!129, !11, i64 8}
!135 = distinct !{!135, !6}
!136 = !{!129, !11, i64 16}
!137 = !{!129, !11, i64 24}
!138 = !{!129, !11, i64 32}
!139 = !{!131, !11, i64 0}
!140 = !{!141, !26, i64 40}
!141 = !{!"", !9, i64 0, !11, i64 8, !11, i64 16, !11, i64 24, !26, i64 32, !26, i64 36, !26, i64 40, !26, i64 44, !26, i64 48, !26, i64 52}
!142 = distinct !{!142, !6}
!143 = !{!141, !26, i64 44}
!144 = !{!141, !26, i64 48}
!145 = !{!141, !26, i64 52}
!146 = !{!141, !26, i64 32}
!147 = !{!141, !11, i64 16}
!148 = distinct !{!148, !6}
!149 = !{!141, !11, i64 8}
!150 = !{!141, !9, i64 0}
!151 = !{!141, !11, i64 24}
!152 = distinct !{!152, !6}
!153 = !{!141, !26, i64 36}
!154 = distinct !{!154, !6}
!155 = distinct !{!155, !6}
!156 = distinct !{!156, !6}
!157 = !{!158, !11, i64 32}
!158 = !{!"", !9, i64 0, !11, i64 8, !9, i64 16, !9, i64 24, !11, i64 32, !9, i64 40}
!159 = !{!158, !9, i64 16}
!160 = !{!158, !11, i64 8}
!161 = distinct !{!161, !6}
!162 = !{!158, !9, i64 40}
!163 = !{!158, !9, i64 24}
!164 = !{!158, !9, i64 0}
!165 = distinct !{!165, !6}
!166 = distinct !{!166, !6}
!167 = distinct !{!167, !6}
!168 = distinct !{!168, !6}
!169 = distinct !{!169, !6}
!170 = distinct !{!170, !6}
!171 = distinct !{!171, !6}
!172 = distinct !{!172, !6}
!173 = !{!174, !9, i64 16}
!174 = !{!"", !11, i64 0, !11, i64 8, !9, i64 16}
!175 = !{!174, !11, i64 0}
!176 = !{!174, !11, i64 8}
!177 = distinct !{!177, !6}
!178 = distinct !{!178, !6}
!179 = !{!180, !9, i64 56}
!180 = !{!"FaSource", !9, i64 0, !9, i64 8, !11, i64 16, !9, i64 24, !9, i64 32, !11, i64 40, !11, i64 48, !9, i64 56, !9, i64 64}
!181 = !{!180, !11, i64 48}
!182 = !{!183, !9, i64 0}
!183 = !{!"", !9, i64 0, !9, i64 8, !11, i64 16}
!184 = !{!183, !9, i64 8}
!185 = distinct !{!185, !6}
!186 = !{!183, !11, i64 16}
!187 = !{!180, !11, i64 40}
!188 = !{i64 0, i64 8, !8, i64 8, i64 8, !8, i64 16, i64 8, !10}
!189 = !{i64 0, i64 8, !8, i64 8, i64 8, !10}
!190 = !{i64 0, i64 8, !10}
!191 = distinct !{!191, !6}
!192 = !{!193, !9, i64 128}
!193 = !{!"FaObserver", !180, i64 0, !11, i64 72, !11, i64 80, !11, i64 88, !11, i64 96, !11, i64 104, !11, i64 112, !11, i64 120, !9, i64 128, !9, i64 136, !9, i64 144, !26, i64 152, !26, i64 156, !26, i64 160, !26, i64 164, !26, i64 168, !26, i64 172, !26, i64 176, !26, i64 180, !9, i64 184, !3, i64 192}
!194 = !{!193, !11, i64 112}
!195 = distinct !{!195, !6}
!196 = !{!193, !9, i64 136}
!197 = !{!193, !11, i64 120}
!198 = !{!180, !9, i64 8}
!199 = !{!180, !9, i64 24}
!200 = !{!180, !11, i64 16}
!201 = distinct !{!201, !6}
!202 = distinct !{!202, !6}
!203 = distinct !{!203, !6}
!204 = distinct !{!204, !6}
!205 = !{!193, !9, i64 0}
!206 = !{!193, !26, i64 152}
!207 = !{!193, !11, i64 88}
!208 = !{!193, !11, i64 96}
!209 = !{!193, !11, i64 104}
!210 = !{!193, !11, i64 80}
!211 = !{!193, !11, i64 72}
!212 = !{!193, !9, i64 184}
!213 = !{!193, !26, i64 156}
!214 = !{!193, !26, i64 160}
!215 = !{!193, !26, i64 172}
!216 = !{!193, !9, i64 8}
!217 = !{!193, !26, i64 176}
!218 = !{!193, !9, i64 144}
!219 = !{!193, !26, i64 168}
!220 = distinct !{!220, !6}
!221 = distinct !{!221, !6}
!222 = distinct !{!222, !6}
!223 = distinct !{!223, !6}
!224 = distinct !{!224, !6}
!225 = !{!193, !26, i64 164}
!226 = distinct !{!226, !6}
!227 = distinct !{!227, !6}
!228 = !{!180, !9, i64 0}
!229 = !{!180, !9, i64 32}
!230 = !{!180, !9, i64 64}
!231 = distinct !{!231, !6}
!232 = !{!233, !9, i64 0}
!233 = !{!"", !9, i64 0, !17, i64 8}
!234 = distinct !{!234, !6}
!235 = distinct !{!235, !6}
!236 = distinct !{!236, !6}
!237 = !{!238, !9, i64 0}
!238 = !{!"", !9, i64 0, !17, i64 8, !17, i64 32}
!239 = !{!240, !11, i64 0}
!240 = !{!"", !11, i64 0, !9, i64 8, !9, i64 16, !9, i64 24, !11, i64 32, !9, i64 40, !9, i64 48, !9, i64 56, !9, i64 64}
!241 = !{!240, !9, i64 8}
!242 = !{!240, !9, i64 16}
!243 = !{!240, !9, i64 24}
!244 = !{!240, !11, i64 32}
!245 = !{!240, !9, i64 48}
!246 = !{!240, !9, i64 56}
!247 = distinct !{!247, !6}
!248 = !{!240, !9, i64 40}
!249 = !{!240, !9, i64 64}
!250 = distinct !{!250, !6}
!251 = distinct !{!251, !6}
!252 = distinct !{!252, !6}
!253 = distinct !{!253, !6}
!254 = distinct !{!254, !6}
!255 = distinct !{!255, !6}
!256 = distinct !{!256, !6}
!257 = distinct !{!257, !6}
!258 = !{!259, !11, i64 16}
!259 = !{!"", !9, i64 0, !9, i64 8, !11, i64 16, !9, i64 24}
!260 = !{!259, !9, i64 8}
!261 = !{!262, !9, i64 32}
!262 = !{!"", !259, i64 0, !9, i64 32}
!263 = !{!259, !9, i64 0}
!264 = !{!265, !26, i64 176}
!265 = !{!"FaExecutor", !11, i64 0, !11, i64 8, !11, i64 16, !11, i64 24, !9, i64 32, !9, i64 40, !11, i64 48, !9, i64 56, !9, i64 64, !9, i64 72, !11, i64 80, !9, i64 88, !9, i64 96, !9, i64 104, !9, i64 112, !11, i64 120, !11, i64 128, !266, i64 136, !26, i64 168, !26, i64 172, !26, i64 176, !26, i64 180, !9, i64 184, !26, i64 192, !9, i64 200, !11, i64 208, !11, i64 216, !3, i64 224, !26, i64 480}
!266 = !{!"FaTask", !11, i64 0, !11, i64 8, !11, i64 16, !26, i64 24, !26, i64 28}
!267 = !{!266, !26, i64 28}
!268 = !{!265, !9, i64 88}
!269 = !{!266, !11, i64 8}
!270 = !{!266, !11, i64 16}
!271 = !{!265, !11, i64 120}
!272 = !{!265, !11, i64 128}
!273 = !{!265, !26, i64 172}
!274 = !{!266, !11, i64 0}
!275 = !{!265, !11, i64 80}
!276 = !{!265, !11, i64 208}
!277 = !{!278, !11, i64 24}
!278 = !{!"", !11, i64 0, !9, i64 8, !11, i64 16, !11, i64 24}
!279 = !{!278, !9, i64 8}
!280 = !{!278, !11, i64 0}
!281 = distinct !{!281, !6}
!282 = !{!265, !9, i64 40}
!283 = !{!265, !9, i64 32}
!284 = !{!265, !26, i64 192}
!285 = !{!265, !9, i64 200}
!286 = !{!287, !11, i64 0}
!287 = !{!"FaWait", !11, i64 0, !11, i64 8, !11, i64 16, !11, i64 24, !11, i64 32, !11, i64 40, !9, i64 48, !9, i64 56, !9, i64 64, !26, i64 72, !26, i64 76, !11, i64 80}
!288 = !{!287, !11, i64 8}
!289 = !{!287, !9, i64 64}
!290 = !{!287, !11, i64 80}
!291 = !{!265, !9, i64 56}
!292 = !{!265, !9, i64 64}
!293 = !{!265, !11, i64 48}
!294 = distinct !{!294, !6}
!295 = !{!287, !9, i64 48}
!296 = !{!265, !9, i64 72}
!297 = !{!287, !9, i64 56}
!298 = !{!299, !11, i64 8}
!299 = !{!"", !9, i64 0, !11, i64 8}
!300 = distinct !{!300, !6}
!301 = !{!278, !11, i64 16}
!302 = !{!303, !26, i64 0}
!303 = !{!"FaIo", !26, i64 0, !26, i64 4, !11, i64 8, !11, i64 16, !11, i64 24}
!304 = !{!303, !26, i64 4}
!305 = !{!303, !11, i64 24}
!306 = !{!307, !26, i64 0}
!307 = !{!"", !26, i64 0, !9, i64 4}
!308 = !{!307, !9, i64 4}
!309 = !{!287, !26, i64 72}
!310 = !{!287, !26, i64 76}
!311 = !{!265, !9, i64 104}
!312 = !{!303, !11, i64 8}
!313 = !{!303, !11, i64 16}
!314 = !{!265, !26, i64 180}
!315 = !{!265, !11, i64 16}
!316 = !{!265, !11, i64 0}
!317 = distinct !{!317, !6}
!318 = !{!265, !26, i64 480}
!319 = distinct !{!319, !6}
!320 = !{!265, !9, i64 184}
!321 = distinct !{!321, !6}
!322 = !{!299, !9, i64 0}
!323 = distinct !{!323, !6}
!324 = distinct !{!324, !6}
!325 = distinct !{!325, !6}
!326 = !{!327, !11, i64 0}
!327 = !{!"FaJob", !11, i64 0, !11, i64 8, !11, i64 16, !11, i64 24, !11, i64 32, !26, i64 40}
!328 = !{!327, !26, i64 40}
!329 = !{!327, !11, i64 16}
!330 = !{!327, !11, i64 24}
!331 = !{!327, !11, i64 32}
!332 = !{!333, !11, i64 136}
!333 = !{!"", !334, i64 0, !334, i64 64, !11, i64 128, !11, i64 136, !26, i64 144, !26, i64 148}
!334 = !{!"", !3, i64 0}
!335 = !{!333, !11, i64 128}
!336 = !{!333, !26, i64 148}
!337 = !{!333, !26, i64 144}
!338 = !{!339, !9, i64 0}
!339 = !{!"", !9, i64 0, !9, i64 8}
!340 = !{!327, !11, i64 8}
!341 = distinct !{!341, !6}
!342 = !{!343, !11, i64 48}
!343 = !{!"", !259, i64 0, !26, i64 32, !26, i64 36, !9, i64 40, !11, i64 48}
!344 = !{!343, !26, i64 32}
!345 = !{!343, !26, i64 36}
!346 = !{!343, !9, i64 40}
!347 = distinct !{!347, !6}
!348 = distinct !{!348, !6}
!349 = !{!350, !26, i64 0}
!350 = !{!"", !26, i64 0, !351, i64 4, !351, i64 6}
!351 = !{!"short", !3, i64 0}
!352 = !{!350, !351, i64 4}
!353 = !{!350, !351, i64 6}
!354 = distinct !{!354, !6}
!355 = !{i32 0, i32 3}
!356 = distinct !{!356, !6}
!357 = distinct !{!357, !6}
!358 = distinct !{!358, !6}
!359 = distinct !{!359, !6}
!360 = distinct !{!360, !6}
!361 = !{!362, !11, i64 0}
!362 = !{!"", !327, i64 0, !26, i64 48, !26, i64 52, !11, i64 56, !9, i64 64, !11, i64 72, !9, i64 80, !9, i64 88}
!363 = !{!362, !11, i64 8}
!364 = !{!362, !26, i64 48}
!365 = !{!362, !11, i64 56}
!366 = !{!362, !9, i64 64}
!367 = !{!362, !11, i64 72}
!368 = !{!362, !9, i64 80}
!369 = !{!362, !9, i64 88}
!370 = !{!362, !26, i64 52}
!371 = distinct !{!371, !6}
!372 = !{!373, !26, i64 208}
!373 = !{!"FaWorker", !259, i64 0, !334, i64 32, !334, i64 96, !11, i64 160, !11, i64 168, !11, i64 176, !11, i64 184, !9, i64 192, !26, i64 200, !26, i64 204, !26, i64 208, !26, i64 212, !11, i64 216}
!374 = !{!373, !26, i64 200}
!375 = !{!373, !11, i64 216}
!376 = !{!377, !11, i64 0}
!377 = !{!"FaCall", !11, i64 0, !11, i64 8, !11, i64 16, !11, i64 24, !11, i64 32, !11, i64 40, !11, i64 48, !9, i64 56, !26, i64 64, !26, i64 68, !11, i64 72, !9, i64 80, !11, i64 88, !9, i64 96, !11, i64 104, !11, i64 112, !11, i64 120}
!378 = distinct !{!378, !6}
!379 = !{!373, !11, i64 168}
!380 = !{!373, !26, i64 204}
!381 = !{!377, !11, i64 16}
!382 = !{!377, !11, i64 32}
!383 = !{!377, !11, i64 48}
!384 = !{!377, !9, i64 56}
!385 = distinct !{!385, !6}
!386 = !{!373, !9, i64 192}
!387 = !{!373, !26, i64 212}
!388 = !{!373, !11, i64 184}
!389 = !{!373, !11, i64 160}
!390 = distinct !{!390, !6}
!391 = !{!377, !26, i64 64}
!392 = !{!373, !11, i64 176}
!393 = !{!377, !11, i64 24}
!394 = !{!377, !11, i64 40}
!395 = !{!377, !11, i64 8}
!396 = distinct !{!396, !6}
!397 = distinct !{!397, !6}
!398 = !{!399, !11, i64 296}
!399 = !{!"FaWorkerPool", !259, i64 0, !334, i64 32, !334, i64 96, !334, i64 160, !11, i64 224, !9, i64 232, !11, i64 240, !11, i64 248, !9, i64 256, !9, i64 264, !9, i64 272, !11, i64 280, !11, i64 288, !11, i64 296, !26, i64 304}
!400 = !{!399, !11, i64 288}
!401 = !{!399, !9, i64 272}
!402 = !{!399, !9, i64 264}
!403 = !{!399, !11, i64 224}
!404 = !{!399, !9, i64 232}
!405 = distinct !{!405, !6}
!406 = distinct !{!406, !6}
!407 = !{!399, !9, i64 256}
!408 = !{!287, !11, i64 40}
!409 = !{!287, !11, i64 16}
!410 = !{!287, !11, i64 24}
!411 = !{!265, !11, i64 216}
!412 = distinct !{!412, !6}
!413 = !{!399, !11, i64 240}
!414 = !{!399, !26, i64 304}
!415 = distinct !{!415, !6}
!416 = !{!399, !11, i64 248}
!417 = !{!377, !9, i64 80}
!418 = !{!377, !11, i64 88}
!419 = !{!377, !11, i64 120}
!420 = !{!377, !9, i64 96}
!421 = !{!377, !11, i64 104}
!422 = !{!377, !26, i64 68}
!423 = !{!377, !11, i64 72}
!424 = !{!377, !11, i64 112}
!425 = !{!287, !11, i64 32}
!426 = distinct !{!426, !6}
!427 = distinct !{!427, !6}
!428 = distinct !{!428, !6}
!429 = !{!430, !9, i64 160}
!430 = !{!"", !259, i64 0, !334, i64 32, !334, i64 96, !9, i64 160, !9, i64 168, !11, i64 176, !9, i64 184, !9, i64 192, !9, i64 200, !26, i64 208, !26, i64 212, !11, i64 216, !11, i64 224, !11, i64 232}
!431 = !{!430, !9, i64 168}
!432 = !{!430, !11, i64 216}
!433 = !{!430, !9, i64 192}
!434 = !{!430, !9, i64 200}
!435 = !{!430, !9, i64 184}
!436 = !{!430, !11, i64 176}
!437 = distinct !{!437, !6}
!438 = !{!430, !26, i64 208}
!439 = !{!430, !26, i64 212}
!440 = !{!430, !11, i64 232}
!441 = distinct !{!441, !6}
!442 = distinct !{!442, !6}
!443 = distinct !{!443, !6}
!444 = !{!445, !26, i64 152}
!445 = !{!"", !334, i64 0, !334, i64 64, !11, i64 128, !9, i64 136, !26, i64 144, !26, i64 148, !26, i64 152}
!446 = distinct !{!446, !6}
!447 = !{!448, !11, i64 0}
!448 = !{!"", !11, i64 0, !11, i64 8, !9, i64 16, !9, i64 24, !9, i64 32, !26, i64 40, !26, i64 44, !26, i64 48, !26, i64 52, !11, i64 56, !9, i64 64, !11, i64 72}
!449 = !{!448, !11, i64 8}
!450 = !{!448, !9, i64 16}
!451 = !{!448, !26, i64 44}
!452 = !{!448, !9, i64 24}
!453 = !{!448, !11, i64 72}
!454 = !{!445, !9, i64 136}
!455 = distinct !{!455, !6}
!456 = !{!448, !11, i64 56}
!457 = distinct !{!457, !6}
!458 = !{!448, !9, i64 64}
!459 = !{!460, !460, i64 0}
!460 = !{!"__int128", !3, i64 0}
!461 = distinct !{!461, !6}
!462 = distinct !{!462, !6}
!463 = distinct !{!463, !6}
!464 = distinct !{!464, !6}
!465 = !{i64 7, i64 65}
!466 = distinct !{!466, !6}
!467 = distinct !{!467, !6}
!468 = !{!193, !9, i64 24}
!469 = !{!193, !11, i64 16}
!470 = distinct !{!470, !6}
!471 = distinct !{!471, !6}
!472 = distinct !{!472, !6}
!473 = distinct !{!473, !6}
