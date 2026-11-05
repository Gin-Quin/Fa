source_filename = "fa_runtime.c"

%struct.BareError = type { i64 }
%struct.Range = type { i32, i32 }
%struct.FaString = type { ptr, i32, i32 }
%struct.Boundaries = type { ptr, i32, i32, i32 }
%struct.Replacing = type { ptr, i32, i32, ptr, i32, i32, i32 }
%struct.FaArrayHeader = type { ptr, i64, i64 }
%struct.Splitting = type { ptr, i32, i32, ptr, i32, i32 }
%struct.IndexError = type { i64, i64, i64 }
%struct.FaSegmented = type { i64, i64, [48 x ptr] }
%struct.PageHeader = type { i32, i32 }
%struct.FreeSpan = type { ptr, i32 }
%struct.FaArena = type { ptr, ptr, ptr, ptr, ptr }
%struct.FaArenaCleanup = type { ptr, ptr, ptr }
%struct.FaPool = type { i64, ptr, ptr, ptr, i32, i32, i32, i32, i32, i32 }
%struct.PoolChunk = type { ptr, i32, i32, i32, i32, [0 x i64] }
%struct.FaLinks = type { i64, ptr, i64, i64, ptr, i64 }
%struct.FaChild = type { i64, i64, ptr }
%struct.FaAddressMap = type { ptr, ptr, i64 }
%struct.FaSource = type { i64, i64, ptr, i64, i64, ptr, ptr, i64, i64 }
%struct.FaObserver = type { %struct.FaSource, ptr, ptr, ptr, ptr, ptr, ptr, ptr, i64, i64, i64, i32, i32, i32, i32, i32, i32, i32, i32, i64, [2 x ptr] }
%struct.MessageError = type { i64, %struct.FaString }
%struct.ParseError = type { i64, %struct.FaString, %struct.FaString }
%struct.FaTable = type { ptr, i64, i64, i64, ptr, i64, i64, i64, ptr, i64 }
%struct.TableChunk = type { ptr, i32, i32 }
%struct.WideSlot = type { i64, i64 }
%struct.FaShared = type { i64, i64, ptr, i64 }
%struct.FaAtomicBlock = type { %struct.FaShared, i64 }
%struct.FaExecutor = type { ptr, ptr, ptr, ptr, i64, i64, ptr, i64, i64, i64, ptr, i64, i64, i64, i64, ptr, ptr, %struct.FaTask, i32, i32, i32, i32, i64, i32, i64, ptr, ptr, [32 x ptr], i32 }
%struct.FaTask = type { ptr, ptr, ptr, i32, i32 }
%struct.FaJoin = type { ptr, i64, ptr, ptr }
%struct.FaWait = type { ptr, ptr, ptr, ptr, ptr, ptr, i64, i64, i64, i32, i32, ptr }
%struct.FaTimer = type { i64, ptr }
%struct.FaIo = type { i32, i32, ptr, ptr, ptr }
%struct.FaJob = type { ptr, ptr, ptr, ptr, ptr, i32 }
%struct.FaSocket = type { %struct.FaShared, i32, i32, i64, ptr }
%struct.FaFileJob = type { %struct.FaJob, i32, i32, ptr, i32, ptr, i32, i32 }
%struct.FileErrorBox = type { i64, %struct.FaString, %struct.FaString }
%struct.FaJsonDocument = type { %struct.FaJson, ptr, ptr, i32 }
%struct.FaJson = type { i32, i32, i64, double, ptr, i32, ptr, i32, ptr, i32 }
%struct.JsonChunk = type { ptr, i32 }
%struct.JsonReader = type { ptr, i32, i32, ptr, ptr, i32, i32, i32, ptr, ptr, i32, i8 }
%struct.JsonCheck = type { ptr, i32, i32, ptr }
%struct.JsonMember = type { ptr, i32, i8, ptr }
%struct.FaWorker = type { %struct.FaShared, %struct.FaSystemLock, %struct.FaSystemLock, ptr, ptr, ptr, ptr, i32, i32, i32, i32, i32, ptr, [8 x i8] }
%struct.FaSystemLock = type { [64 x i8] }
%struct.FaCall = type { ptr, ptr, ptr, ptr, ptr, ptr, ptr, i64, i32, i32, ptr, i64, ptr, i64, ptr, ptr, ptr }
%struct.FaWorkerPool = type { %struct.FaShared, %struct.FaSystemLock, %struct.FaSystemLock, %struct.FaSystemLock, ptr, i64, ptr, ptr, i64, i64, i64, ptr, ptr, ptr, i32 }
%struct.FaMutex = type { %struct.FaShared, i32, i32, ptr, [4 x i8], %struct.FaSystemLock, %struct.FaSystemLock, [0 x i8] }
%struct.FaChannel = type { %struct.FaShared, %struct.FaSystemLock, %struct.FaSystemLock, i64, i64, ptr, i64, i64, i64, i32, i32, ptr, ptr, ptr, [12 x i8] }

@output_length = internal unnamed_addr global i32 0, align 4
@output = internal global [8192 x i8] zeroinitializer, align 16
@builder_length = internal unnamed_addr global i32 0, align 4
@builder_simple = internal unnamed_addr global i32 0, align 4
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
@.str.31 = private unnamed_addr constant [28 x i8] c"Unable to parse JSON string\00", align 1
@.str.32 = private unnamed_addr constant [10 x i8] c"the value\00", align 1
@fa_b_json_string.hex = internal unnamed_addr constant [17 x i8] c"0123456789abcdef\00", align 16
@.str.33 = private unnamed_addr constant [3 x i8] c"\\b\00", align 1
@.str.34 = private unnamed_addr constant [3 x i8] c"\\f\00", align 1
@.str.35 = private unnamed_addr constant [5 x i8] c"\\u00\00", align 1
@.str.36 = private unnamed_addr constant [5 x i8] c"null\00", align 1
@workers_alive = internal global i64 0, align 8
@fa_blocking_depth = hidden local_unnamed_addr global i32 0, align 4
@.str.37 = private unnamed_addr constant [38 x i8] c"WorkerPool.submit (the queue is full)\00", align 1
@worker_timeout = internal constant %struct.BareError { i64 8 }, align 8
@.str.38 = private unnamed_addr constant [38 x i8] c"`lock()` on a mutex this thread holds\00", align 1
@.str.39 = private unnamed_addr constant [60 x i8] c"`send` on a full channel that no other thread receives from\00", align 1
@.str.40 = private unnamed_addr constant [60 x i8] c"`receive` on an empty channel that no other thread sends to\00", align 1
@fa_turn_site = hidden local_unnamed_addr global ptr null, align 4
@.str.41 = private unnamed_addr constant [1 x i8] zeroinitializer, align 1
@decimal_u64.pairs = internal unnamed_addr constant [201 x i8] c"00010203040506070809101112131415161718192021222324252627282930313233343536373839404142434445464748495051525354555657585960616263646566676869707172737475767778798081828384858687888990919293949596979899\00", align 16
@out_of_memory.message = internal constant [21 x i8] c"stop: out of memory\0A\00", align 16
@EXTEND = internal unnamed_addr constant [153 x %struct.Range] [%struct.Range { i32 768, i32 879 }, %struct.Range { i32 1155, i32 1161 }, %struct.Range { i32 1425, i32 1469 }, %struct.Range { i32 1471, i32 1471 }, %struct.Range { i32 1473, i32 1474 }, %struct.Range { i32 1476, i32 1477 }, %struct.Range { i32 1479, i32 1479 }, %struct.Range { i32 1552, i32 1562 }, %struct.Range { i32 1611, i32 1631 }, %struct.Range { i32 1648, i32 1648 }, %struct.Range { i32 1750, i32 1756 }, %struct.Range { i32 1759, i32 1764 }, %struct.Range { i32 1767, i32 1768 }, %struct.Range { i32 1770, i32 1773 }, %struct.Range { i32 1809, i32 1809 }, %struct.Range { i32 1840, i32 1866 }, %struct.Range { i32 1958, i32 1968 }, %struct.Range { i32 2027, i32 2035 }, %struct.Range { i32 2070, i32 2093 }, %struct.Range { i32 2137, i32 2139 }, %struct.Range { i32 2200, i32 2207 }, %struct.Range { i32 2250, i32 2273 }, %struct.Range { i32 2275, i32 2307 }, %struct.Range { i32 2362, i32 2364 }, %struct.Range { i32 2366, i32 2383 }, %struct.Range { i32 2385, i32 2391 }, %struct.Range { i32 2402, i32 2403 }, %struct.Range { i32 2433, i32 2435 }, %struct.Range { i32 2492, i32 2492 }, %struct.Range { i32 2494, i32 2509 }, %struct.Range { i32 2519, i32 2519 }, %struct.Range { i32 2530, i32 2531 }, %struct.Range { i32 2561, i32 2563 }, %struct.Range { i32 2620, i32 2641 }, %struct.Range { i32 2672, i32 2673 }, %struct.Range { i32 2677, i32 2677 }, %struct.Range { i32 2689, i32 2691 }, %struct.Range { i32 2748, i32 2748 }, %struct.Range { i32 2750, i32 2765 }, %struct.Range { i32 2786, i32 2787 }, %struct.Range { i32 2817, i32 2819 }, %struct.Range { i32 2876, i32 2876 }, %struct.Range { i32 2878, i32 2903 }, %struct.Range { i32 2914, i32 2915 }, %struct.Range { i32 2946, i32 2946 }, %struct.Range { i32 3006, i32 3021 }, %struct.Range { i32 3031, i32 3031 }, %struct.Range { i32 3072, i32 3076 }, %struct.Range { i32 3132, i32 3132 }, %struct.Range { i32 3134, i32 3158 }, %struct.Range { i32 3170, i32 3171 }, %struct.Range { i32 3201, i32 3203 }, %struct.Range { i32 3260, i32 3260 }, %struct.Range { i32 3262, i32 3286 }, %struct.Range { i32 3298, i32 3299 }, %struct.Range { i32 3328, i32 3331 }, %struct.Range { i32 3387, i32 3388 }, %struct.Range { i32 3390, i32 3405 }, %struct.Range { i32 3415, i32 3415 }, %struct.Range { i32 3426, i32 3427 }, %struct.Range { i32 3457, i32 3459 }, %struct.Range { i32 3530, i32 3551 }, %struct.Range { i32 3570, i32 3571 }, %struct.Range { i32 3633, i32 3633 }, %struct.Range { i32 3635, i32 3642 }, %struct.Range { i32 3655, i32 3662 }, %struct.Range { i32 3761, i32 3761 }, %struct.Range { i32 3763, i32 3772 }, %struct.Range { i32 3784, i32 3790 }, %struct.Range { i32 3864, i32 3865 }, %struct.Range { i32 3893, i32 3893 }, %struct.Range { i32 3895, i32 3895 }, %struct.Range { i32 3897, i32 3897 }, %struct.Range { i32 3902, i32 3903 }, %struct.Range { i32 3953, i32 3972 }, %struct.Range { i32 3974, i32 3975 }, %struct.Range { i32 3981, i32 4028 }, %struct.Range { i32 4038, i32 4038 }, %struct.Range { i32 4139, i32 4158 }, %struct.Range { i32 4182, i32 4185 }, %struct.Range { i32 4190, i32 4192 }, %struct.Range { i32 4194, i32 4196 }, %struct.Range { i32 4199, i32 4205 }, %struct.Range { i32 4209, i32 4212 }, %struct.Range { i32 4226, i32 4237 }, %struct.Range { i32 4239, i32 4239 }, %struct.Range { i32 4250, i32 4253 }, %struct.Range { i32 4957, i32 4959 }, %struct.Range { i32 5906, i32 5909 }, %struct.Range { i32 5938, i32 5940 }, %struct.Range { i32 5970, i32 5971 }, %struct.Range { i32 6002, i32 6003 }, %struct.Range { i32 6068, i32 6099 }, %struct.Range { i32 6109, i32 6109 }, %struct.Range { i32 6155, i32 6157 }, %struct.Range { i32 6159, i32 6159 }, %struct.Range { i32 6277, i32 6278 }, %struct.Range { i32 6313, i32 6313 }, %struct.Range { i32 6432, i32 6459 }, %struct.Range { i32 6679, i32 6683 }, %struct.Range { i32 6741, i32 6783 }, %struct.Range { i32 6832, i32 6911 }, %struct.Range { i32 6912, i32 6916 }, %struct.Range { i32 6964, i32 6980 }, %struct.Range { i32 7019, i32 7027 }, %struct.Range { i32 7040, i32 7042 }, %struct.Range { i32 7073, i32 7085 }, %struct.Range { i32 7142, i32 7155 }, %struct.Range { i32 7204, i32 7223 }, %struct.Range { i32 7376, i32 7378 }, %struct.Range { i32 7380, i32 7400 }, %struct.Range { i32 7405, i32 7405 }, %struct.Range { i32 7412, i32 7412 }, %struct.Range { i32 7415, i32 7417 }, %struct.Range { i32 7616, i32 7679 }, %struct.Range { i32 8204, i32 8204 }, %struct.Range { i32 8400, i32 8432 }, %struct.Range { i32 11503, i32 11505 }, %struct.Range { i32 11647, i32 11647 }, %struct.Range { i32 11744, i32 11775 }, %struct.Range { i32 12330, i32 12335 }, %struct.Range { i32 12441, i32 12442 }, %struct.Range { i32 42607, i32 42610 }, %struct.Range { i32 42612, i32 42621 }, %struct.Range { i32 42654, i32 42655 }, %struct.Range { i32 42736, i32 42737 }, %struct.Range { i32 43010, i32 43010 }, %struct.Range { i32 43014, i32 43014 }, %struct.Range { i32 43019, i32 43019 }, %struct.Range { i32 43043, i32 43047 }, %struct.Range { i32 43052, i32 43052 }, %struct.Range { i32 43136, i32 43137 }, %struct.Range { i32 43188, i32 43205 }, %struct.Range { i32 43232, i32 43249 }, %struct.Range { i32 43263, i32 43263 }, %struct.Range { i32 43302, i32 43309 }, %struct.Range { i32 43335, i32 43347 }, %struct.Range { i32 43392, i32 43395 }, %struct.Range { i32 43443, i32 43456 }, %struct.Range { i32 43561, i32 43574 }, %struct.Range { i32 43587, i32 43587 }, %struct.Range { i32 43596, i32 43597 }, %struct.Range { i32 43755, i32 43759 }, %struct.Range { i32 43765, i32 43766 }, %struct.Range { i32 44003, i32 44010 }, %struct.Range { i32 44012, i32 44013 }, %struct.Range { i32 64286, i32 64286 }, %struct.Range { i32 65024, i32 65039 }, %struct.Range { i32 65056, i32 65071 }, %struct.Range { i32 65438, i32 65439 }, %struct.Range { i32 127995, i32 127999 }, %struct.Range { i32 917536, i32 917631 }, %struct.Range { i32 917760, i32 917999 }], align 16
@PICTOGRAPHIC = internal unnamed_addr constant [22 x %struct.Range] [%struct.Range { i32 169, i32 169 }, %struct.Range { i32 174, i32 174 }, %struct.Range { i32 8252, i32 8252 }, %struct.Range { i32 8265, i32 8265 }, %struct.Range { i32 8482, i32 8482 }, %struct.Range { i32 8505, i32 8505 }, %struct.Range { i32 8596, i32 8618 }, %struct.Range { i32 8986, i32 9215 }, %struct.Range { i32 9410, i32 9410 }, %struct.Range { i32 9642, i32 9726 }, %struct.Range { i32 9728, i32 10175 }, %struct.Range { i32 10548, i32 10549 }, %struct.Range { i32 11013, i32 11093 }, %struct.Range { i32 12336, i32 12336 }, %struct.Range { i32 12349, i32 12349 }, %struct.Range { i32 12951, i32 12951 }, %struct.Range { i32 12953, i32 12953 }, %struct.Range { i32 126976, i32 127231 }, %struct.Range { i32 127245, i32 127405 }, %struct.Range { i32 127489, i32 127994 }, %struct.Range { i32 128000, i32 129791 }, %struct.Range { i32 130048, i32 131069 }], align 16
@ascii_table = internal constant [128 x i8] c"\00\01\02\03\04\05\06\07\08\09\0A\0B\0C\0D\0E\0F\10\11\12\13\14\15\16\17\18\19\1A\1B\1C\1D\1E\1F !\22#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\7F", align 16
@free_spans = internal unnamed_addr global ptr null, align 4
@arena_refill.message = internal constant [21 x i8] c"stop: out of memory\0A\00", align 16
@.str.42 = private unnamed_addr constant [34 x i8] c"a derived value depends on itself\00", align 1
@.str.43 = private unnamed_addr constant [7 x i8] c"stop: \00", align 1
@.str.44 = private unnamed_addr constant [7 x i8] c"\0A  at \00", align 1
@.str.45 = private unnamed_addr constant [17 x i8] c"stop: deadlock: \00", align 1
@.str.46 = private unnamed_addr constant [37 x i8] c"warning: a task kept the thread for \00", align 1
@.str.47 = private unnamed_addr constant [6 x i8] c" ms (\00", align 1
@.str.48 = private unnamed_addr constant [7 x i8] c"a task\00", align 1
@.str.49 = private unnamed_addr constant [51 x i8] c"): await Task.yield() or move the work to a worker\00", align 1
@.str.50 = private unnamed_addr constant [33 x i8] c"stop: deadlock: every task waits\00", align 1
@.str.51 = private unnamed_addr constant [17 x i8] c" (main waits on \00", align 1
@.str.52 = private unnamed_addr constant [2 x i8] c")\00", align 1
@.str.53 = private unnamed_addr constant [19 x i8] c" (a task waits on \00", align 1
@.str.54 = private unnamed_addr constant [13 x i8] c"can't listen\00", align 1
@.str.55 = private unnamed_addr constant [7 x i8] c"accept\00", align 1
@.str.56 = private unnamed_addr constant [3 x i8] c": \00", align 1
@.str.57 = private unnamed_addr constant [7 x i8] c"error \00", align 1
@.str.58 = private unnamed_addr constant [8 x i8] c"connect\00", align 1
@.str.59 = private unnamed_addr constant [5 x i8] c"read\00", align 1
@.str.60 = private unnamed_addr constant [6 x i8] c"write\00", align 1
@.str.61 = private unnamed_addr constant [15 x i8] c"Unexpected EOF\00", align 1
@.str.62 = private unnamed_addr constant [27 x i8] c"Exceeded the maximum depth\00", align 1
@.str.63 = private unnamed_addr constant [18 x i8] c"Unexpected token \00", align 1
@.str.64 = private unnamed_addr constant [23 x i8] c"Unexpected identifier \00", align 1
@.str.65 = private unnamed_addr constant [20 x i8] c"Unrecognized token \00", align 1
@.str.66 = private unnamed_addr constant [13 x i8] c"Expected '}'\00", align 1
@.str.67 = private unnamed_addr constant [56 x i8] c"Expected ':' before value in object property definition\00", align 1
@.str.68 = private unnamed_addr constant [39 x i8] c"Property name must be a string literal\00", align 1
@.str.69 = private unnamed_addr constant [48 x i8] c"Unexpected comma at the end of array expression\00", align 1
@.str.70 = private unnamed_addr constant [13 x i8] c"Expected ']'\00", align 1
@.str.71 = private unnamed_addr constant [20 x i8] c"Unterminated string\00", align 1
@.str.72 = private unnamed_addr constant [36 x i8] c"\\u must be followed by 4 hex digits\00", align 1
@.str.73 = private unnamed_addr constant [26 x i8] c"Invalid escape character \00", align 1
@.str.74 = private unnamed_addr constant [15 x i8] c"Invalid number\00", align 1
@.str.75 = private unnamed_addr constant [35 x i8] c"Invalid digits after decimal point\00", align 1
@.str.76 = private unnamed_addr constant [19 x i8] c"JSON Parse error: \00", align 1
@.str.77 = private unnamed_addr constant [10 x i8] c"a boolean\00", align 1
@.str.78 = private unnamed_addr constant [9 x i8] c"a string\00", align 1
@.str.79 = private unnamed_addr constant [9 x i8] c"a number\00", align 1
@.str.80 = private unnamed_addr constant [11 x i8] c"an integer\00", align 1
@.str.81 = private unnamed_addr constant [27 x i8] c" doesn't fit in an Integer\00", align 1
@.str.82 = private unnamed_addr constant [21 x i8] c" should be at least \00", align 1
@.str.83 = private unnamed_addr constant [20 x i8] c" should be at most \00", align 1
@.str.84 = private unnamed_addr constant [7 x i8] c", not \00", align 1
@.str.85 = private unnamed_addr constant [9 x i8] c"an array\00", align 1
@.str.86 = private unnamed_addr constant [2 x i8] c"[\00", align 1
@.str.87 = private unnamed_addr constant [2 x i8] c"]\00", align 1
@.str.88 = private unnamed_addr constant [10 x i8] c"an object\00", align 1
@.str.89 = private unnamed_addr constant [2 x i8] c".\00", align 1
@.str.90 = private unnamed_addr constant [16 x i8] c" has no field `\00", align 1
@.str.91 = private unnamed_addr constant [2 x i8] c"`\00", align 1
@.str.92 = private unnamed_addr constant [32 x i8] c"one of the members of its union\00", align 1
@.str.93 = private unnamed_addr constant [19 x i8] c" should be one of \00", align 1
@.str.94 = private unnamed_addr constant [24 x i8] c".type should be one of \00", align 1
@.str.95 = private unnamed_addr constant [6 x i8] c"value\00", align 1
@.str.96 = private unnamed_addr constant [22 x i8] c" has no field `value`\00", align 1
@.str.97 = private unnamed_addr constant [7 x i8] c".value\00", align 1
@.str.98 = private unnamed_addr constant [32 x i8] c" has a type JSON can't describe\00", align 1
@.str.99 = private unnamed_addr constant [12 x i8] c" should be \00", align 1
@.str.100 = private unnamed_addr constant [8 x i8] c"boolean\00", align 1
@.str.101 = private unnamed_addr constant [7 x i8] c"number\00", align 1
@.str.102 = private unnamed_addr constant [7 x i8] c"string\00", align 1
@.str.103 = private unnamed_addr constant [7 x i8] c"object\00", align 1
@.str.104 = private unnamed_addr constant [5 x i8] c"type\00", align 1
@llvm.used = appending global [1 x ptr] [ptr @fa_flush], section "llvm.metadata"
@switch.table.json_mismatch = private unnamed_addr constant [5 x ptr] [ptr @.str.36, ptr @.str.100, ptr @.str.101, ptr @.str.102, ptr @.str.85], align 4

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: readwrite)
define hidden void @fa_copy_bytes(ptr nocapture noundef writeonly %0, ptr nocapture noundef readonly %1, i32 noundef %2) local_unnamed_addr #0 {
  tail call fastcc void @copy_bytes(ptr noundef %0, ptr noundef %1, i32 noundef %2) #61
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
  tail call void @fa_write(i32 noundef 1, ptr noundef nonnull @output, i32 noundef %1) #62
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
  tail call void @fa_write(i32 noundef 1, ptr noundef nonnull @output, i32 noundef %5) #62
  store i32 0, ptr @output_length, align 4, !tbaa !4
  br label %9

9:                                                ; preds = %6, %8
  tail call void @fa_write(i32 noundef %0, ptr noundef %1, i32 noundef %2) #62
  br label %23

10:                                               ; preds = %3
  %11 = add i32 %5, %2
  %12 = icmp ugt i32 %11, 8192
  br i1 %12, label %13, label %19

13:                                               ; preds = %10
  %14 = icmp eq i32 %5, 0
  br i1 %14, label %16, label %15

15:                                               ; preds = %13
  tail call void @fa_write(i32 noundef 1, ptr noundef nonnull @output, i32 noundef %5) #62
  store i32 0, ptr @output_length, align 4, !tbaa !4
  br label %16

16:                                               ; preds = %13, %15
  %17 = icmp ugt i32 %2, 8192
  br i1 %17, label %18, label %19

18:                                               ; preds = %16
  tail call void @fa_write(i32 noundef 1, ptr noundef %1, i32 noundef %2) #62
  br label %23

19:                                               ; preds = %16, %10
  %20 = phi i32 [ %5, %10 ], [ 0, %16 ]
  %21 = getelementptr inbounds i8, ptr @output, i32 %20
  tail call fastcc void @copy_bytes(ptr noundef nonnull %21, ptr noundef %1, i32 noundef %2) #61
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
  tail call void @fa_flush() #61
  tail call void @fa_exit(i32 noundef %0) #63
  unreachable
}

; Function Attrs: noreturn optsize
declare void @fa_exit(i32 noundef) local_unnamed_addr #6

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, argmem: none, inaccessiblemem: none)
define hidden void @fa_b_reset() local_unnamed_addr #7 {
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  store i32 0, ptr @builder_simple, align 4, !tbaa !4
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_bytes(ptr nocapture noundef readonly %0, i32 noundef %1) local_unnamed_addr #3 {
  tail call fastcc void @reserve(i32 noundef %1) #61
  %3 = load ptr, ptr @builder, align 4, !tbaa !6
  %4 = load i32, ptr @builder_length, align 4, !tbaa !4
  %5 = getelementptr inbounds i8, ptr %3, i32 %4
  tail call fastcc void @copy_bytes(ptr noundef %5, ptr noundef %0, i32 noundef %1) #61
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
  br i1 %5, label %6, label %22

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
  %15 = tail call ptr @fa_alloc(i32 noundef %11, i32 noundef 1) #62
  %16 = load ptr, ptr @builder, align 4, !tbaa !6
  %17 = load i32, ptr @builder_length, align 4, !tbaa !4
  tail call fastcc void @copy_bytes(ptr noundef %15, ptr noundef %16, i32 noundef %17) #61
  %18 = icmp eq ptr %16, null
  br i1 %18, label %21, label %19

19:                                               ; preds = %14
  %20 = load i32, ptr @builder_capacity, align 4, !tbaa !4
  tail call void @fa_free(ptr noundef nonnull %16, i32 noundef %20, i32 noundef 1) #62
  br label %21

21:                                               ; preds = %19, %14
  store ptr %15, ptr @builder, align 4, !tbaa !6
  store i32 %11, ptr @builder_capacity, align 4, !tbaa !4
  br label %22

22:                                               ; preds = %1, %21
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_string(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2) local_unnamed_addr #3 {
  %4 = icmp sgt i32 %2, -1
  br i1 %4, label %6, label %5

5:                                                ; preds = %3
  tail call fastcc void @append_simple(ptr noundef %0, i32 noundef %1) #61
  br label %11

6:                                                ; preds = %3
  tail call fastcc void @reserve(i32 noundef %1) #61
  %7 = load ptr, ptr @builder, align 4, !tbaa !6
  %8 = load i32, ptr @builder_length, align 4, !tbaa !4
  %9 = getelementptr inbounds i8, ptr %7, i32 %8
  tail call fastcc void @copy_bytes(ptr noundef %9, ptr noundef %0, i32 noundef %1) #61
  %10 = add i32 %8, %1
  store i32 %10, ptr @builder_length, align 4, !tbaa !4
  br label %11

11:                                               ; preds = %6, %5
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @append_simple(ptr nocapture noundef readonly %0, i32 noundef %1) unnamed_addr #3 {
  %3 = load i32, ptr @builder_simple, align 4, !tbaa !4
  %4 = load i32, ptr @builder_length, align 4, !tbaa !4
  %5 = icmp eq i32 %3, %4
  tail call fastcc void @reserve(i32 noundef %1) #61
  %6 = load ptr, ptr @builder, align 4, !tbaa !6
  %7 = load i32, ptr @builder_length, align 4, !tbaa !4
  %8 = getelementptr inbounds i8, ptr %6, i32 %7
  tail call fastcc void @copy_bytes(ptr noundef %8, ptr noundef %0, i32 noundef %1) #61
  %9 = add i32 %7, %1
  store i32 %9, ptr @builder_length, align 4, !tbaa !4
  br i1 %5, label %10, label %11

10:                                               ; preds = %2
  store i32 %9, ptr @builder_simple, align 4, !tbaa !4
  br label %11

11:                                               ; preds = %10, %2
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_line(i32 noundef %0) local_unnamed_addr #3 {
  tail call fastcc void @reserve(i32 noundef 1) #61
  %2 = load ptr, ptr @builder, align 4, !tbaa !6
  %3 = load i32, ptr @builder_length, align 4, !tbaa !4
  %4 = getelementptr inbounds i8, ptr %2, i32 %3
  store i8 10, ptr %4, align 1, !tbaa !1
  %5 = add i32 %3, 1
  store i32 %5, ptr @builder_length, align 4, !tbaa !4
  tail call void @fa_out(i32 noundef %0, ptr noundef %2, i32 noundef %5) #61
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  store i32 0, ptr @builder_simple, align 4, !tbaa !4
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_string_own(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef writeonly %3) local_unnamed_addr #3 {
  %5 = alloca %struct.FaString, align 4
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %5) #64
  %6 = and i32 %2, -2147483648
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %5, ptr noundef %0, i32 noundef %1, i32 noundef %6) #61
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %3, ptr noundef nonnull align 4 dereferenceable(12) %5, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %5) #64
  ret void
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.start.p0(i64 immarg, ptr nocapture) #8

; Function Attrs: nounwind optsize
define internal fastcc void @owned_copy_with(ptr dead_on_unwind noalias nocapture writable writeonly sret(%struct.FaString) align 4 %0, ptr nocapture noundef readonly %1, i32 noundef %2, i32 noundef %3) unnamed_addr #3 {
  %5 = alloca i32, align 4
  %6 = icmp ugt i32 %2, 1073741822
  br i1 %6, label %7, label %8

7:                                                ; preds = %4
  tail call fastcc void @out_of_memory() #65
  unreachable

8:                                                ; preds = %4
  switch i32 %2, label %22 [
    i32 0, label %9
    i32 1, label %12
  ]

9:                                                ; preds = %8
  store ptr @.str.41, ptr %0, align 4, !tbaa.struct !10
  %10 = getelementptr inbounds i8, ptr %0, i32 4
  store i32 0, ptr %10, align 4, !tbaa.struct !11
  %11 = getelementptr inbounds i8, ptr %0, i32 8
  store i32 -2147483648, ptr %11, align 4, !tbaa.struct !12
  br label %56

12:                                               ; preds = %8
  %13 = load i8, ptr %1, align 1, !tbaa !1
  %14 = icmp sgt i8 %13, -1
  br i1 %14, label %15, label %24

15:                                               ; preds = %12
  %16 = zext nneg i8 %13 to i32
  %17 = getelementptr inbounds i8, ptr @ascii_table, i32 %16
  store ptr %17, ptr %0, align 4, !tbaa !13, !alias.scope !15
  %18 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 1
  store i32 1, ptr %18, align 4, !tbaa !18, !alias.scope !15
  %19 = icmp eq i8 %13, 13
  %20 = select i1 %19, i32 0, i32 -2147483648
  %21 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 2
  store i32 %20, ptr %21, align 4, !tbaa !19, !alias.scope !15
  br label %56

22:                                               ; preds = %8
  %23 = icmp ult i32 %2, 12
  br i1 %23, label %24, label %35

24:                                               ; preds = %12, %22
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %0, i8 0, i32 12, i1 false)
  tail call fastcc void @copy_bytes(ptr noundef %0, ptr noundef %1, i32 noundef %2) #61
  %25 = icmp eq i32 %3, 0
  br i1 %25, label %26, label %30

26:                                               ; preds = %24
  %27 = tail call fastcc i32 @simple_flag(ptr noundef %1, i32 noundef %2) #61, !range !20
  %28 = icmp eq i32 %27, 0
  %29 = select i1 %28, i32 64, i32 192
  br label %30

30:                                               ; preds = %24, %26
  %31 = phi i32 [ %29, %26 ], [ 192, %24 ]
  %32 = or i32 %31, %2
  %33 = trunc i32 %32 to i8
  %34 = getelementptr inbounds i8, ptr %0, i32 11
  store i8 %33, ptr %34, align 1, !tbaa !1
  br label %56

35:                                               ; preds = %22
  %36 = icmp ult i32 %2, 129
  br i1 %36, label %37, label %40

37:                                               ; preds = %35
  %38 = add nuw nsw i32 %2, 7
  %39 = and i32 %38, 504
  br label %45

40:                                               ; preds = %35
  %41 = icmp ugt i32 %2, 8192
  br i1 %41, label %45, label %42

42:                                               ; preds = %40
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %5) #64
  %43 = call fastcc i32 @size_class(i32 noundef %2, ptr noundef nonnull %5) #61
  %44 = load i32, ptr %5, align 4, !tbaa !4
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %5) #64
  br label %45

45:                                               ; preds = %37, %40, %42
  %46 = phi i32 [ %39, %37 ], [ %44, %42 ], [ %2, %40 ]
  %47 = tail call ptr @fa_alloc(i32 noundef %46, i32 noundef 1) #62
  tail call fastcc void @copy_bytes(ptr noundef %47, ptr noundef %1, i32 noundef %2) #61
  %48 = icmp eq i32 %3, 0
  br i1 %48, label %49, label %51

49:                                               ; preds = %45
  %50 = tail call fastcc i32 @simple_flag(ptr noundef %47, i32 noundef %2) #61, !range !20
  br label %51

51:                                               ; preds = %45, %49
  %52 = phi i32 [ %50, %49 ], [ %3, %45 ]
  %53 = or i32 %52, %46
  store ptr %47, ptr %0, align 4, !tbaa.struct !10
  %54 = getelementptr inbounds i8, ptr %0, i32 4
  store i32 %2, ptr %54, align 4, !tbaa.struct !11
  %55 = getelementptr inbounds i8, ptr %0, i32 8
  store i32 %53, ptr %55, align 4, !tbaa.struct !12
  br label %56

56:                                               ; preds = %51, %30, %15, %9
  ret void
}

; Function Attrs: mustprogress nocallback nofree nounwind willreturn memory(argmem: readwrite)
declare void @llvm.memcpy.p0.p0.i32(ptr noalias nocapture writeonly, ptr noalias nocapture readonly, i32, i1 immarg) #9

; Function Attrs: mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.end.p0(i64 immarg, ptr nocapture) #8

; Function Attrs: nounwind optsize
define hidden void @fa_b_take(ptr nocapture noundef writeonly %0) local_unnamed_addr #3 {
  %2 = alloca %struct.FaString, align 4
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %2) #64
  %3 = load ptr, ptr @builder, align 4, !tbaa !6
  %4 = load i32, ptr @builder_length, align 4, !tbaa !4
  %5 = load i32, ptr @builder_simple, align 4, !tbaa !4
  %6 = icmp eq i32 %5, %4
  br i1 %6, label %11, label %7

7:                                                ; preds = %1
  %8 = getelementptr inbounds i8, ptr %3, i32 %5
  %9 = sub i32 %4, %5
  %10 = tail call fastcc i32 @simple_flag(ptr noundef %8, i32 noundef %9) #61, !range !20
  br label %11

11:                                               ; preds = %1, %7
  %12 = phi i32 [ %10, %7 ], [ -2147483648, %1 ]
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %2, ptr noundef %3, i32 noundef %4, i32 noundef %12) #61
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %0, ptr noundef nonnull align 4 dereferenceable(12) %2, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %2) #64
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  store i32 0, ptr @builder_simple, align 4, !tbaa !4
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
  tail call fastcc void @string_u64(i64 noundef %3, i32 noundef %5, ptr noundef %1) #61
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @string_u64(i64 noundef %0, i32 noundef %1, ptr nocapture noundef writeonly %2) unnamed_addr #3 {
  %4 = alloca [24 x i8], align 16
  %5 = alloca %struct.FaString, align 4
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %4) #64
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
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %5) #64
  %89 = getelementptr inbounds i8, ptr %4, i32 %87
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %5, ptr noundef nonnull %89, i32 noundef %88, i32 noundef -2147483648) #61
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %2, ptr noundef nonnull align 4 dereferenceable(12) %5, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %5) #64
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %4) #64
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_string_u64(i64 noundef %0, ptr nocapture noundef writeonly %1) local_unnamed_addr #3 {
  tail call fastcc void @string_u64(i64 noundef %0, i32 noundef 0, ptr noundef %1) #61
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_i64(i64 noundef %0) local_unnamed_addr #3 {
  %2 = tail call i64 @llvm.abs.i64(i64 %0, i1 false)
  %3 = lshr i64 %0, 63
  %4 = trunc i64 %3 to i32
  tail call fastcc void @append_u64(i64 noundef %2, i32 noundef %4) #61
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @append_u64(i64 noundef %0, i32 noundef %1) unnamed_addr #3 {
  %3 = alloca [24 x i8], align 16
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %3) #64
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
  call fastcc void @append_simple(ptr noundef nonnull %86, i32 noundef %87) #61
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %3) #64
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_u64(i64 noundef %0) local_unnamed_addr #3 {
  tail call fastcc void @append_u64(i64 noundef %0, i32 noundef 0) #61
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
  tail call fastcc void @append_u128(i128 noundef %9, i32 noundef %11) #61
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
  tail call fastcc void @append_u64(i64 noundef %7, i32 noundef %1) #61
  br label %76

8:                                                ; preds = %2
  %9 = lshr i128 %0, 64
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %3) #64
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
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %4) #64
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
  call fastcc void @append_simple(ptr noundef nonnull %74, i32 noundef %75) #61
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %4) #64
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %3) #64
  br label %76

76:                                               ; preds = %72, %6
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_u128(i64 noundef %0, i64 noundef %1) local_unnamed_addr #3 {
  %3 = zext i64 %1 to i128
  %4 = shl nuw i128 %3, 64
  %5 = zext i64 %0 to i128
  %6 = or disjoint i128 %4, %5
  tail call fastcc void @append_u128(i128 noundef %6, i32 noundef 0) #61
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_bool(i32 noundef %0) local_unnamed_addr #3 {
  %2 = icmp eq i32 %0, 0
  %3 = select i1 %2, ptr @.str.1, ptr @.str
  %4 = select i1 %2, i32 5, i32 4
  tail call fastcc void @append_simple(ptr noundef nonnull %3, i32 noundef %4) #61
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_f64(double noundef %0) local_unnamed_addr #3 {
  tail call fastcc void @reserve(i32 noundef 40) #61
  %2 = load ptr, ptr @builder, align 4, !tbaa !6
  %3 = load i32, ptr @builder_length, align 4, !tbaa !4
  %4 = getelementptr inbounds i8, ptr %2, i32 %3
  %5 = tail call i32 @fa_host_format_float(double noundef %0, i32 noundef 0, ptr noundef %4) #62
  %6 = load i32, ptr @builder_length, align 4, !tbaa !4
  %7 = add i32 %6, %5
  store i32 %7, ptr @builder_length, align 4, !tbaa !4
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_f32(float noundef %0) local_unnamed_addr #3 {
  %2 = fpext float %0 to double
  tail call fastcc void @reserve(i32 noundef 40) #61
  %3 = load ptr, ptr @builder, align 4, !tbaa !6
  %4 = load i32, ptr @builder_length, align 4, !tbaa !4
  %5 = getelementptr inbounds i8, ptr %3, i32 %4
  %6 = tail call i32 @fa_host_format_float(double noundef %2, i32 noundef 1, ptr noundef %5) #62
  %7 = load i32, ptr @builder_length, align 4, !tbaa !4
  %8 = add i32 %7, %6
  store i32 %8, ptr @builder_length, align 4, !tbaa !4
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_percentage(double noundef %0) local_unnamed_addr #3 {
  tail call fastcc void @reserve(i32 noundef 40) #61
  %2 = load ptr, ptr @builder, align 4, !tbaa !6
  %3 = load i32, ptr @builder_length, align 4, !tbaa !4
  %4 = getelementptr inbounds i8, ptr %2, i32 %3
  %5 = tail call i32 @fa_host_format_float(double noundef %0, i32 noundef 2, ptr noundef %4) #62
  %6 = load i32, ptr @builder_length, align 4, !tbaa !4
  %7 = add i32 %6, %5
  store i32 %7, ptr @builder_length, align 4, !tbaa !4
  tail call fastcc void @reserve(i32 noundef 1) #61
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
  tail call fastcc void @reserve(i32 noundef 1) #61
  %3 = load ptr, ptr @builder, align 4, !tbaa !6
  %4 = load i32, ptr @builder_length, align 4, !tbaa !4
  %5 = getelementptr inbounds i8, ptr %3, i32 %4
  store i8 34, ptr %5, align 1, !tbaa !1
  %6 = add i32 %4, 1
  store i32 %6, ptr @builder_length, align 4, !tbaa !4
  %7 = icmp eq i32 %1, 0
  br i1 %7, label %8, label %13

8:                                                ; preds = %50, %2
  tail call fastcc void @reserve(i32 noundef 1) #61
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
  tail call fastcc void @append_c(ptr noundef nonnull @.str.2) #61
  br label %50

19:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.3) #61
  br label %50

20:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.4) #61
  br label %50

21:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.5) #61
  br label %50

22:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.6) #61
  br label %50

23:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.7) #61
  br label %50

24:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.8) #61
  br label %50

25:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.9) #61
  br label %50

26:                                               ; preds = %13
  %27 = icmp ult i8 %16, 32
  %28 = icmp eq i8 %16, 127
  %29 = or i1 %27, %28
  br i1 %29, label %30, label %45

30:                                               ; preds = %26
  tail call fastcc void @append_c(ptr noundef nonnull @.str.10) #61
  %31 = lshr i32 %17, 4
  %32 = getelementptr inbounds [17 x i8], ptr @fa_b_quoted.hex, i32 0, i32 %31
  %33 = load i8, ptr %32, align 1, !tbaa !1
  tail call fastcc void @reserve(i32 noundef 1) #61
  %34 = load ptr, ptr @builder, align 4, !tbaa !6
  %35 = load i32, ptr @builder_length, align 4, !tbaa !4
  %36 = getelementptr inbounds i8, ptr %34, i32 %35
  store i8 %33, ptr %36, align 1, !tbaa !1
  %37 = add i32 %35, 1
  store i32 %37, ptr @builder_length, align 4, !tbaa !4
  %38 = and i32 %17, 15
  %39 = getelementptr inbounds [17 x i8], ptr @fa_b_quoted.hex, i32 0, i32 %38
  %40 = load i8, ptr %39, align 1, !tbaa !1
  tail call fastcc void @reserve(i32 noundef 1) #61
  %41 = load ptr, ptr @builder, align 4, !tbaa !6
  %42 = load i32, ptr @builder_length, align 4, !tbaa !4
  %43 = getelementptr inbounds i8, ptr %41, i32 %42
  store i8 %40, ptr %43, align 1, !tbaa !1
  %44 = add i32 %42, 1
  store i32 %44, ptr @builder_length, align 4, !tbaa !4
  br label %50

45:                                               ; preds = %26
  tail call fastcc void @reserve(i32 noundef 1) #61
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
  br i1 %52, label %8, label %13, !llvm.loop !27
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
  br i1 %6, label %8, label %2, !llvm.loop !28

8:                                                ; preds = %2
  tail call fastcc void @reserve(i32 noundef %3) #61
  %9 = load ptr, ptr @builder, align 4, !tbaa !6
  %10 = load i32, ptr @builder_length, align 4, !tbaa !4
  %11 = getelementptr inbounds i8, ptr %9, i32 %10
  tail call fastcc void @copy_bytes(ptr noundef %11, ptr noundef nonnull %0, i32 noundef %3) #61
  %12 = add i32 %10, %3
  store i32 %12, ptr @builder_length, align 4, !tbaa !4
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_string_array(ptr nocapture noundef readonly %0, i32 noundef %1) local_unnamed_addr #3 {
  tail call fastcc void @reserve(i32 noundef 1) #61
  %3 = load ptr, ptr @builder, align 4, !tbaa !6
  %4 = load i32, ptr @builder_length, align 4, !tbaa !4
  %5 = getelementptr inbounds i8, ptr %3, i32 %4
  store i8 91, ptr %5, align 1, !tbaa !1
  %6 = add i32 %4, 1
  store i32 %6, ptr @builder_length, align 4, !tbaa !4
  %7 = icmp eq i32 %1, 0
  br i1 %7, label %8, label %13

8:                                                ; preds = %30, %2
  tail call fastcc void @reserve(i32 noundef 1) #61
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
  tail call fastcc void @append_c(ptr noundef nonnull @.str.11) #61
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
  tail call void @fa_b_quoted(ptr noundef %31, i32 noundef %32) #61
  %33 = add nuw i32 %14, 1
  %34 = icmp eq i32 %33, %1
  br i1 %34, label %8, label %13, !llvm.loop !29
}

; Function Attrs: nounwind optsize
define hidden void @fa_concat(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef readonly %2, i32 noundef %3, ptr nocapture noundef writeonly %4) local_unnamed_addr #3 {
  %6 = alloca i32, align 4
  %7 = alloca %struct.FaString, align 4
  %8 = alloca %struct.FaString, align 4
  %9 = add i32 %3, %1
  %10 = icmp ult i32 %9, %1
  %11 = icmp ugt i32 %9, 1073741822
  %12 = or i1 %10, %11
  br i1 %12, label %13, label %14

13:                                               ; preds = %5
  tail call fastcc void @out_of_memory() #65
  unreachable

14:                                               ; preds = %5
  %15 = icmp ult i32 %9, 2
  br i1 %15, label %16, label %19

16:                                               ; preds = %14
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %7) #64
  %17 = icmp eq i32 %1, 0
  %18 = select i1 %17, ptr %2, ptr %0
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %7, ptr noundef %18, i32 noundef %9, i32 noundef 0) #61
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %4, ptr noundef nonnull align 4 dereferenceable(12) %7, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %7) #64
  br label %47

19:                                               ; preds = %14
  %20 = icmp ult i32 %9, 12
  br i1 %20, label %21, label %29

21:                                               ; preds = %19
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %8) #64
  call void @llvm.memset.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %8, i8 0, i32 12, i1 false)
  call fastcc void @copy_bytes(ptr noundef nonnull %8, ptr noundef %0, i32 noundef %1) #61
  %22 = getelementptr inbounds i8, ptr %8, i32 %1
  call fastcc void @copy_bytes(ptr noundef nonnull %22, ptr noundef %2, i32 noundef %3) #61
  %23 = call fastcc i32 @simple_flag(ptr noundef nonnull %8, i32 noundef %9) #61, !range !20
  %24 = icmp eq i32 %23, 0
  %25 = select i1 %24, i32 64, i32 192
  %26 = or disjoint i32 %25, %9
  %27 = trunc i32 %26 to i8
  %28 = getelementptr inbounds i8, ptr %8, i32 11
  store i8 %27, ptr %28, align 1, !tbaa !1
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %4, ptr noundef nonnull align 4 dereferenceable(12) %8, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %8) #64
  br label %47

29:                                               ; preds = %19
  %30 = icmp ult i32 %9, 129
  br i1 %30, label %31, label %34

31:                                               ; preds = %29
  %32 = add nuw nsw i32 %9, 7
  %33 = and i32 %32, 504
  br label %39

34:                                               ; preds = %29
  %35 = icmp ugt i32 %9, 8192
  br i1 %35, label %39, label %36

36:                                               ; preds = %34
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %6) #64
  %37 = call fastcc i32 @size_class(i32 noundef %9, ptr noundef nonnull %6) #61
  %38 = load i32, ptr %6, align 4, !tbaa !4
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %6) #64
  br label %39

39:                                               ; preds = %31, %34, %36
  %40 = phi i32 [ %33, %31 ], [ %38, %36 ], [ %9, %34 ]
  %41 = tail call ptr @fa_alloc(i32 noundef %40, i32 noundef 1) #62
  tail call fastcc void @copy_bytes(ptr noundef %41, ptr noundef %0, i32 noundef %1) #61
  %42 = getelementptr inbounds i8, ptr %41, i32 %1
  tail call fastcc void @copy_bytes(ptr noundef %42, ptr noundef %2, i32 noundef %3) #61
  store ptr %41, ptr %4, align 4, !tbaa !13
  %43 = getelementptr inbounds %struct.FaString, ptr %4, i32 0, i32 1
  store i32 %9, ptr %43, align 4, !tbaa !18
  %44 = tail call fastcc i32 @simple_flag(ptr noundef %41, i32 noundef %9) #61, !range !20
  %45 = or i32 %44, %40
  %46 = getelementptr inbounds %struct.FaString, ptr %4, i32 0, i32 2
  store i32 %45, ptr %46, align 4, !tbaa !19
  br label %47

47:                                               ; preds = %39, %21, %16
  ret void
}

; Function Attrs: noreturn nounwind optsize
define internal fastcc void @out_of_memory() unnamed_addr #5 {
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @out_of_memory.message, i32 noundef 20) #62
  tail call void @fa_exit(i32 noundef 101) #63
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
  tail call fastcc void @out_of_memory() #65
  unreachable

6:                                                ; preds = %2
  %7 = add i32 %1, %0
  ret i32 %7
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_string_allocate(i32 noundef %0, i32 noundef %1, ptr noundef writeonly %2) local_unnamed_addr #3 {
  %4 = alloca i32, align 4
  store ptr null, ptr %2, align 4, !tbaa.struct !10
  %5 = getelementptr inbounds i8, ptr %2, i32 4
  store i32 0, ptr %5, align 4, !tbaa.struct !11
  %6 = getelementptr inbounds i8, ptr %2, i32 8
  store i32 0, ptr %6, align 4, !tbaa.struct !12
  %7 = icmp ult i32 %0, 12
  br i1 %7, label %8, label %14

8:                                                ; preds = %3
  %9 = icmp eq i32 %1, 0
  %10 = select i1 %9, i32 64, i32 192
  %11 = or disjoint i32 %10, %0
  %12 = trunc i32 %11 to i8
  %13 = getelementptr inbounds i8, ptr %2, i32 11
  store i8 %12, ptr %13, align 1, !tbaa !1
  br label %28

14:                                               ; preds = %3
  %15 = icmp ult i32 %0, 129
  br i1 %15, label %16, label %19

16:                                               ; preds = %14
  %17 = add nuw nsw i32 %0, 7
  %18 = and i32 %17, 504
  br label %24

19:                                               ; preds = %14
  %20 = icmp ugt i32 %0, 8192
  br i1 %20, label %24, label %21

21:                                               ; preds = %19
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %4) #64
  %22 = call fastcc i32 @size_class(i32 noundef %0, ptr noundef nonnull %4) #61
  %23 = load i32, ptr %4, align 4, !tbaa !4
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %4) #64
  br label %24

24:                                               ; preds = %16, %19, %21
  %25 = phi i32 [ %18, %16 ], [ %23, %21 ], [ %0, %19 ]
  %26 = tail call ptr @fa_alloc(i32 noundef %25, i32 noundef 1) #62
  store ptr %26, ptr %2, align 4, !tbaa !13
  store i32 %0, ptr %5, align 4, !tbaa !18
  %27 = or i32 %25, %1
  store i32 %27, ptr %6, align 4, !tbaa !19
  br label %28

28:                                               ; preds = %24, %8
  %29 = phi ptr [ %2, %8 ], [ %26, %24 ]
  ret ptr %29
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
  %21 = tail call ptr @fa_string_allocate(i32 noundef %0, i32 noundef %1, ptr noundef nonnull %2) #61
  br i1 %9, label %22, label %24

22:                                               ; preds = %20
  switch i32 %7, label %23 [
    i32 1073741823, label %24
    i32 0, label %24
  ]

23:                                               ; preds = %22
  tail call void @fa_free(ptr noundef %13, i32 noundef %7, i32 noundef 1) #62
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
  %17 = tail call fastcc i32 @simple_flag(ptr noundef %15, i32 noundef %16) #61, !range !20
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
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %4) #64
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
  tail call fastcc void @out_of_memory() #65
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
  %35 = tail call ptr @fa_alloc(i32 noundef %28, i32 noundef 1) #62
  br label %37

36:                                               ; preds = %3
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %4) #64
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
  call fastcc void @copy_bytes(ptr noundef %59, ptr noundef %58, i32 noundef %57) #61
  %60 = add i32 %57, %43
  %61 = add nuw i32 %42, 1
  %62 = icmp eq i32 %61, %1
  br i1 %62, label %39, label %41, !llvm.loop !33

63:                                               ; preds = %39
  %64 = call fastcc i32 @simple_flag(ptr noundef %38, i32 noundef %28) #61, !range !20
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
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %4) #64
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
  tail call fastcc void @copy_bytes(ptr noundef %20, ptr noundef %1, i32 noundef %2) #61
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
  tail call fastcc void @copy_bytes(ptr noundef %42, ptr noundef %1, i32 noundef %2) #61
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
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %5) #64
  call void @llvm.memset.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %5, i8 0, i32 12, i1 false)
  %52 = load ptr, ptr %0, align 4, !tbaa !13
  call fastcc void @copy_bytes(ptr noundef nonnull %5, ptr noundef %52, i32 noundef %30) #61
  %53 = getelementptr inbounds i8, ptr %5, i32 %30
  call fastcc void @copy_bytes(ptr noundef nonnull %53, ptr noundef %1, i32 noundef %2) #61
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
  tail call void @fa_free(ptr noundef %52, i32 noundef %59, i32 noundef 1) #62
  br label %61

61:                                               ; preds = %48, %48, %60
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %0, ptr noundef nonnull align 4 dereferenceable(12) %5, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %5) #64
  br label %63

62:                                               ; preds = %33, %28
  tail call fastcc void @string_append_slow(ptr noundef nonnull %0, ptr noundef %1, i32 noundef %2, i32 noundef %3) #61
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
  %29 = tail call fastcc i32 @simple_flag(ptr noundef %1, i32 noundef %2) #61, !range !20
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
  tail call fastcc void @copy_bytes(ptr noundef %38, ptr noundef %1, i32 noundef %2) #61
  %39 = icmp eq i32 %34, 0
  %40 = select i1 %39, i32 64, i32 192
  %41 = or disjoint i32 %40, %18
  %42 = trunc i32 %41 to i8
  %43 = getelementptr inbounds i8, ptr %0, i32 11
  store i8 %42, ptr %43, align 1, !tbaa !1
  br label %69

44:                                               ; preds = %36
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %5) #64
  call void @llvm.memset.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %5, i8 0, i32 12, i1 false)
  %45 = load ptr, ptr %0, align 4, !tbaa !13
  call fastcc void @copy_bytes(ptr noundef nonnull %5, ptr noundef %45, i32 noundef %17) #61
  %46 = getelementptr inbounds i8, ptr %5, i32 %17
  call fastcc void @copy_bytes(ptr noundef nonnull %46, ptr noundef %1, i32 noundef %2) #61
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
  tail call void @fa_free(ptr noundef %45, i32 noundef %52, i32 noundef 1) #62
  br label %54

54:                                               ; preds = %44, %44, %53
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %0, ptr noundef nonnull align 4 dereferenceable(12) %5, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %5) #64
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
  tail call fastcc void @copy_bytes(ptr noundef %65, ptr noundef %1, i32 noundef %2) #61
  %66 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 1
  store i32 %18, ptr %66, align 4, !tbaa !18
  %67 = or i32 %34, %56
  store i32 %67, ptr %6, align 4, !tbaa !19
  br label %69

68:                                               ; preds = %55
  tail call fastcc void @string_grow(ptr noundef nonnull %0, ptr noundef %1, i32 noundef %2, i32 noundef %34) #61
  br label %69

69:                                               ; preds = %63, %68, %54, %37
  ret void
}

; Function Attrs: alwaysinline nounwind optsize
define hidden void @fa_string_plus(ptr nocapture noundef %0, ptr nocapture noundef readonly %1, i32 noundef %2, i32 noundef %3) local_unnamed_addr #13 {
  %5 = alloca %struct.FaString, align 4
  %6 = alloca %struct.FaString, align 4
  %7 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 2
  %8 = load i32, ptr %7, align 4, !tbaa !19
  %9 = and i32 %8, 2147483647
  %10 = icmp slt i32 %3, 0
  %11 = icmp ult i32 %9, 1073741823
  %12 = select i1 %10, i1 %11, i1 false
  br i1 %12, label %13, label %23, !prof !34

13:                                               ; preds = %4
  %14 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 1
  %15 = load i32, ptr %14, align 4, !tbaa !18
  %16 = add i32 %15, %2
  %17 = icmp ugt i32 %16, %9
  br i1 %17, label %23, label %18, !prof !35

18:                                               ; preds = %13
  %19 = load ptr, ptr %0, align 4, !tbaa !13
  %20 = getelementptr inbounds i8, ptr %19, i32 %15
  tail call fastcc void @copy_bytes(ptr noundef %20, ptr noundef %1, i32 noundef %2) #61
  %21 = load i32, ptr %14, align 4, !tbaa !18
  %22 = add i32 %21, %2
  store i32 %22, ptr %14, align 4, !tbaa !18
  br label %84

23:                                               ; preds = %4, %13
  %24 = icmp eq i32 %2, 0
  br i1 %24, label %84, label %25

25:                                               ; preds = %23
  %26 = and i32 %8, 1073741824
  %27 = icmp eq i32 %26, 0
  br i1 %27, label %28, label %33

28:                                               ; preds = %25
  %29 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 1
  %30 = load i32, ptr %29, align 4, !tbaa !18
  %31 = add i32 %30, %2
  %32 = icmp ult i32 %31, 12
  br i1 %32, label %33, label %83

33:                                               ; preds = %28, %25
  %34 = and i32 %3, -2147483648
  %35 = icmp ne i32 %34, 0
  %36 = select i1 %35, i1 %11, i1 false
  br i1 %36, label %37, label %47, !prof !34

37:                                               ; preds = %33
  %38 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 1
  %39 = load i32, ptr %38, align 4, !tbaa !18
  %40 = add i32 %39, %2
  %41 = icmp ugt i32 %40, %9
  br i1 %41, label %47, label %42, !prof !35

42:                                               ; preds = %37
  %43 = load ptr, ptr %0, align 4, !tbaa !13
  %44 = getelementptr inbounds i8, ptr %43, i32 %39
  tail call fastcc void @copy_bytes(ptr noundef %44, ptr noundef %1, i32 noundef %2) #61
  %45 = load i32, ptr %38, align 4, !tbaa !18
  %46 = add i32 %45, %2
  store i32 %46, ptr %38, align 4, !tbaa !18
  br label %84

47:                                               ; preds = %33, %37
  br i1 %27, label %48, label %53

48:                                               ; preds = %47
  %49 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 1
  %50 = load i32, ptr %49, align 4, !tbaa !18
  %51 = add i32 %50, %2
  %52 = icmp ult i32 %51, 12
  br i1 %52, label %68, label %82

53:                                               ; preds = %47
  %54 = lshr i32 %8, 24
  %55 = and i32 %54, 31
  %56 = add i32 %55, %2
  %57 = icmp ult i32 %56, 12
  br i1 %57, label %58, label %82

58:                                               ; preds = %53
  %59 = icmp eq i32 %55, 0
  %60 = select i1 %59, i32 -2147483648, i32 %8
  %61 = and i32 %60, %34
  %62 = getelementptr inbounds i8, ptr %0, i32 %55
  tail call fastcc void @copy_bytes(ptr noundef %62, ptr noundef %1, i32 noundef %2) #61
  %63 = icmp eq i32 %61, 0
  %64 = select i1 %63, i32 64, i32 192
  %65 = or disjoint i32 %64, %56
  %66 = trunc i32 %65 to i8
  %67 = getelementptr inbounds i8, ptr %0, i32 11
  store i8 %66, ptr %67, align 1, !tbaa !1
  br label %84

68:                                               ; preds = %48
  %69 = icmp eq i32 %50, 0
  %70 = select i1 %69, i32 -2147483648, i32 %8
  %71 = and i32 %70, %34
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %5) #64
  call void @llvm.memset.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %5, i8 0, i32 12, i1 false)
  %72 = load ptr, ptr %0, align 4, !tbaa !13
  call fastcc void @copy_bytes(ptr noundef nonnull %5, ptr noundef %72, i32 noundef %50) #61
  %73 = getelementptr inbounds i8, ptr %5, i32 %50
  call fastcc void @copy_bytes(ptr noundef nonnull %73, ptr noundef %1, i32 noundef %2) #61
  %74 = icmp eq i32 %71, 0
  %75 = select i1 %74, i32 64, i32 192
  %76 = or disjoint i32 %75, %51
  %77 = trunc i32 %76 to i8
  %78 = getelementptr inbounds i8, ptr %5, i32 11
  store i8 %77, ptr %78, align 1, !tbaa !1
  %79 = and i32 %8, 1073741823
  switch i32 %79, label %80 [
    i32 1073741823, label %81
    i32 0, label %81
  ]

80:                                               ; preds = %68
  tail call void @fa_free(ptr noundef %72, i32 noundef %79, i32 noundef 1) #62
  br label %81

81:                                               ; preds = %68, %68, %80
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %0, ptr noundef nonnull align 4 dereferenceable(12) %5, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %5) #64
  br label %84

82:                                               ; preds = %53, %48
  tail call fastcc void @string_append_slow(ptr noundef nonnull %0, ptr noundef %1, i32 noundef %2, i32 noundef %3) #61
  br label %84

83:                                               ; preds = %28
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %6) #64
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %6, ptr noundef nonnull align 4 dereferenceable(12) %0, i32 12, i1 false), !tbaa.struct !10
  call fastcc void @string_plus_slow(ptr noundef nonnull %6, ptr noundef %1, i32 noundef %2) #61
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %0, ptr noundef nonnull align 4 dereferenceable(12) %6, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %6) #64
  br label %84

84:                                               ; preds = %82, %81, %58, %42, %23, %83, %18
  ret void
}

; Function Attrs: noinline nounwind optsize
define internal fastcc void @string_plus_slow(ptr nocapture noundef %0, ptr nocapture noundef readonly %1, i32 noundef %2) unnamed_addr #15 {
  %4 = alloca %struct.FaString, align 4
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %4) #64
  %5 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 2
  %6 = load i32, ptr %5, align 4, !tbaa !19
  %7 = and i32 %6, 1073741824
  %8 = icmp eq i32 %7, 0
  br i1 %8, label %12, label %9

9:                                                ; preds = %3
  %10 = lshr i32 %6, 24
  %11 = and i32 %10, 31
  br label %16

12:                                               ; preds = %3
  %13 = load ptr, ptr %0, align 4, !tbaa !13
  %14 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 1
  %15 = load i32, ptr %14, align 4, !tbaa !18
  br label %16

16:                                               ; preds = %9, %12
  %17 = phi ptr [ %0, %9 ], [ %13, %12 ]
  %18 = phi i32 [ %11, %9 ], [ %15, %12 ]
  call void @fa_concat(ptr noundef %17, i32 noundef %18, ptr noundef %1, i32 noundef %2, ptr noundef nonnull %4) #61
  %19 = load i32, ptr %5, align 4, !tbaa !19
  %20 = freeze i32 %19
  %21 = and i32 %20, 1073741823
  %22 = and i32 %20, 1073741824
  %23 = icmp eq i32 %22, 0
  br i1 %23, label %24, label %27

24:                                               ; preds = %16
  switch i32 %21, label %25 [
    i32 1073741823, label %27
    i32 0, label %27
  ]

25:                                               ; preds = %24
  %26 = load ptr, ptr %0, align 4, !tbaa !13
  tail call void @fa_free(ptr noundef %26, i32 noundef %21, i32 noundef 1) #62
  br label %27

27:                                               ; preds = %24, %24, %16, %25
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %0, ptr noundef nonnull align 4 dereferenceable(12) %4, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %4) #64
  ret void
}

; Function Attrs: alwaysinline nounwind optsize
define hidden void @fa_string_reserve(ptr nocapture noundef %0, i32 noundef %1) local_unnamed_addr #13 {
  %3 = alloca %struct.FaString, align 4
  %4 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 2
  %5 = load i32, ptr %4, align 4, !tbaa !19
  %6 = and i32 %5, 2147483647
  %7 = and i32 %5, 1073741824
  %8 = icmp eq i32 %7, 0
  br i1 %8, label %12, label %9

9:                                                ; preds = %2
  %10 = lshr i32 %5, 24
  %11 = and i32 %10, 31
  br label %15

12:                                               ; preds = %2
  %13 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 1
  %14 = load i32, ptr %13, align 4, !tbaa !18
  br label %15

15:                                               ; preds = %9, %12
  %16 = phi i32 [ %11, %9 ], [ %14, %12 ]
  %17 = add i32 %16, %1
  %18 = icmp ult i32 %17, %1
  br i1 %18, label %19, label %20

19:                                               ; preds = %15
  tail call fastcc void @out_of_memory() #65
  unreachable

20:                                               ; preds = %15
  %21 = icmp ult i32 %17, 12
  br i1 %21, label %27, label %22

22:                                               ; preds = %20
  %23 = icmp ult i32 %6, 1073741823
  %24 = icmp ule i32 %17, %6
  %25 = and i1 %23, %24
  br i1 %25, label %27, label %26, !prof !36

26:                                               ; preds = %22
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %3) #64
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %3, ptr noundef nonnull align 4 dereferenceable(12) %0, i32 12, i1 false), !tbaa.struct !10
  call fastcc void @string_reserve_slow(ptr noundef nonnull %3, i32 noundef %17) #61
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %0, ptr noundef nonnull align 4 dereferenceable(12) %3, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %3) #64
  br label %27

27:                                               ; preds = %20, %22, %26
  ret void
}

; Function Attrs: noinline nounwind optsize
define internal fastcc void @string_reserve_slow(ptr nocapture noundef %0, i32 noundef %1) unnamed_addr #15 {
  %3 = alloca i32, align 4
  %4 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 2
  %5 = load i32, ptr %4, align 4, !tbaa !19
  %6 = and i32 %5, 1073741824
  %7 = icmp eq i32 %6, 0
  br i1 %7, label %11, label %8

8:                                                ; preds = %2
  %9 = lshr i32 %5, 24
  %10 = and i32 %9, 31
  br label %14

11:                                               ; preds = %2
  %12 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 1
  %13 = load i32, ptr %12, align 4, !tbaa !18
  br label %14

14:                                               ; preds = %8, %11
  %15 = phi i32 [ %10, %8 ], [ %13, %11 ]
  %16 = and i32 %5, 1073741823
  %17 = icmp ne i32 %16, 0
  %18 = icmp ne i32 %16, 1073741823
  %19 = and i1 %17, %18
  %20 = and i1 %7, %19
  %21 = shl nuw nsw i32 %16, 1
  %22 = tail call i32 @llvm.umax.i32(i32 %21, i32 %1)
  %23 = select i1 %20, i32 %22, i32 %1
  %24 = icmp ugt i32 %23, 1073741822
  br i1 %24, label %25, label %26

25:                                               ; preds = %14
  tail call fastcc void @out_of_memory() #65
  unreachable

26:                                               ; preds = %14
  %27 = tail call i32 @llvm.umax.i32(i32 %23, i32 64)
  %28 = icmp ult i32 %23, 129
  br i1 %28, label %29, label %32

29:                                               ; preds = %26
  %30 = add nuw nsw i32 %27, 7
  %31 = and i32 %30, 504
  br label %37

32:                                               ; preds = %26
  %33 = icmp ugt i32 %23, 8192
  br i1 %33, label %37, label %34

34:                                               ; preds = %32
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %3) #64
  %35 = call fastcc i32 @size_class(i32 noundef %27, ptr noundef nonnull %3) #61
  %36 = load i32, ptr %3, align 4, !tbaa !4
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %3) #64
  br label %37

37:                                               ; preds = %29, %32, %34
  %38 = phi i32 [ %31, %29 ], [ %36, %34 ], [ %27, %32 ]
  %39 = tail call ptr @fa_alloc(i32 noundef %38, i32 noundef 1) #62
  %40 = load i32, ptr %4, align 4, !tbaa !19
  %41 = and i32 %40, 1073741824
  %42 = icmp eq i32 %41, 0
  br i1 %42, label %43, label %45

43:                                               ; preds = %37
  %44 = load ptr, ptr %0, align 4, !tbaa !13
  br label %45

45:                                               ; preds = %37, %43
  %46 = phi ptr [ %44, %43 ], [ %0, %37 ]
  tail call fastcc void @copy_bytes(ptr noundef %39, ptr noundef %46, i32 noundef %15) #61
  br i1 %7, label %47, label %50

47:                                               ; preds = %45
  switch i32 %16, label %48 [
    i32 1073741823, label %50
    i32 0, label %50
  ]

48:                                               ; preds = %47
  %49 = load ptr, ptr %0, align 4, !tbaa !13
  tail call void @fa_free(ptr noundef %49, i32 noundef %16, i32 noundef 1) #62
  br label %50

50:                                               ; preds = %47, %47, %45, %48
  store ptr %39, ptr %0, align 4, !tbaa !13
  %51 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 1
  store i32 %15, ptr %51, align 4, !tbaa !18
  %52 = icmp eq i32 %15, 0
  %53 = and i32 %5, -2147483648
  %54 = select i1 %52, i32 -2147483648, i32 %53
  %55 = or i32 %38, %54
  store i32 %55, ptr %4, align 4, !tbaa !19
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_string_reserve_owned(ptr nocapture noundef %0, i32 noundef %1) local_unnamed_addr #3 {
  %3 = alloca i32, align 4
  %4 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 2
  %5 = load i32, ptr %4, align 4, !tbaa !19
  %6 = and i32 %5, 1073741824
  %7 = icmp eq i32 %6, 0
  br i1 %7, label %11, label %8

8:                                                ; preds = %2
  %9 = lshr i32 %5, 24
  %10 = and i32 %9, 31
  br label %14

11:                                               ; preds = %2
  %12 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 1
  %13 = load i32, ptr %12, align 4, !tbaa !18
  br label %14

14:                                               ; preds = %8, %11
  %15 = phi i32 [ %10, %8 ], [ %13, %11 ]
  %16 = icmp eq i32 %1, 0
  br i1 %16, label %60, label %17

17:                                               ; preds = %14
  %18 = add i32 %15, %1
  %19 = and i32 %5, 1073741823
  %20 = icmp ne i32 %19, 0
  %21 = icmp ne i32 %19, 1073741823
  %22 = and i1 %20, %21
  %23 = and i1 %7, %22
  %24 = xor i1 %23, true
  %25 = icmp uge i32 %18, %1
  %26 = select i1 %23, i1 %25, i1 false
  %27 = icmp ule i32 %18, %19
  %28 = select i1 %26, i1 %27, i1 false
  br i1 %28, label %60, label %29, !prof !37

29:                                               ; preds = %17
  br i1 %7, label %33, label %30

30:                                               ; preds = %29
  %31 = lshr i32 %5, 24
  %32 = and i32 %31, 31
  br label %36

33:                                               ; preds = %29
  %34 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 1
  %35 = load i32, ptr %34, align 4, !tbaa !18
  br label %36

36:                                               ; preds = %30, %33
  %37 = phi i32 [ %32, %30 ], [ %35, %33 ]
  %38 = icmp eq i32 %37, 0
  %39 = icmp ult i32 %1, 1073741823
  %40 = and i1 %39, %24
  %41 = and i1 %40, %38
  br i1 %41, label %42, label %59

42:                                               ; preds = %36
  %43 = icmp ult i32 %1, 129
  br i1 %43, label %44, label %49

44:                                               ; preds = %42
  %45 = add nuw nsw i32 %1, 7
  %46 = and i32 %45, 504
  %47 = icmp ugt i32 %1, 8
  %48 = select i1 %47, i32 %46, i32 8
  br label %54

49:                                               ; preds = %42
  %50 = icmp ugt i32 %1, 8192
  br i1 %50, label %54, label %51

51:                                               ; preds = %49
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %3) #64
  %52 = call fastcc i32 @size_class(i32 noundef %1, ptr noundef nonnull %3) #61
  %53 = load i32, ptr %3, align 4, !tbaa !4
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %3) #64
  br label %54

54:                                               ; preds = %44, %49, %51
  %55 = phi i32 [ %48, %44 ], [ %53, %51 ], [ %1, %49 ]
  %56 = tail call ptr @fa_alloc(i32 noundef %55, i32 noundef 1) #62
  store ptr %56, ptr %0, align 4, !tbaa !13
  %57 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 1
  store i32 0, ptr %57, align 4, !tbaa !18
  %58 = or i32 %55, -2147483648
  store i32 %58, ptr %4, align 4, !tbaa !19
  br label %60

59:                                               ; preds = %36
  tail call fastcc void @string_reserve_owned_slow(ptr noundef nonnull %0, i32 noundef %1) #61
  br label %60

60:                                               ; preds = %14, %17, %59, %54
  ret void
}

; Function Attrs: noinline nounwind optsize
define internal fastcc void @string_reserve_owned_slow(ptr nocapture noundef %0, i32 noundef %1) unnamed_addr #15 {
  %3 = alloca i32, align 4
  %4 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 2
  %5 = load i32, ptr %4, align 4, !tbaa !19
  %6 = and i32 %5, 1073741824
  %7 = icmp eq i32 %6, 0
  br i1 %7, label %11, label %8

8:                                                ; preds = %2
  %9 = lshr i32 %5, 24
  %10 = and i32 %9, 31
  br label %14

11:                                               ; preds = %2
  %12 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 1
  %13 = load i32, ptr %12, align 4, !tbaa !18
  br label %14

14:                                               ; preds = %8, %11
  %15 = phi i32 [ %10, %8 ], [ %13, %11 ]
  %16 = add i32 %15, %1
  %17 = icmp ult i32 %16, %1
  %18 = icmp ugt i32 %16, 1073741822
  %19 = or i1 %17, %18
  br i1 %19, label %20, label %21

20:                                               ; preds = %14
  tail call fastcc void @out_of_memory() #65
  unreachable

21:                                               ; preds = %14
  %22 = and i32 %5, 1073741823
  %23 = icmp ne i32 %22, 0
  %24 = icmp ne i32 %22, 1073741823
  %25 = and i1 %23, %24
  %26 = and i1 %7, %25
  %27 = icmp ule i32 %16, %22
  %28 = and i1 %26, %27
  br i1 %28, label %63, label %29

29:                                               ; preds = %21
  %30 = shl nuw nsw i32 %22, 1
  %31 = tail call i32 @llvm.umax.i32(i32 %30, i32 %16)
  %32 = select i1 %26, i32 %31, i32 %16
  %33 = icmp ult i32 %32, 129
  br i1 %33, label %34, label %39

34:                                               ; preds = %29
  %35 = add nuw nsw i32 %32, 7
  %36 = and i32 %35, 504
  %37 = icmp ugt i32 %32, 8
  %38 = select i1 %37, i32 %36, i32 8
  br label %44

39:                                               ; preds = %29
  %40 = icmp ugt i32 %32, 8192
  br i1 %40, label %44, label %41

41:                                               ; preds = %39
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %3) #64
  %42 = call fastcc i32 @size_class(i32 noundef %32, ptr noundef nonnull %3) #61
  %43 = load i32, ptr %3, align 4, !tbaa !4
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %3) #64
  br label %44

44:                                               ; preds = %34, %39, %41
  %45 = phi i32 [ %38, %34 ], [ %43, %41 ], [ %32, %39 ]
  %46 = tail call ptr @fa_alloc(i32 noundef %45, i32 noundef 1) #62
  %47 = load i32, ptr %4, align 4, !tbaa !19
  %48 = and i32 %47, 1073741824
  %49 = icmp eq i32 %48, 0
  br i1 %49, label %50, label %52

50:                                               ; preds = %44
  %51 = load ptr, ptr %0, align 4, !tbaa !13
  br label %52

52:                                               ; preds = %44, %50
  %53 = phi ptr [ %51, %50 ], [ %0, %44 ]
  tail call fastcc void @copy_bytes(ptr noundef %46, ptr noundef %53, i32 noundef %15) #61
  br i1 %7, label %54, label %57

54:                                               ; preds = %52
  switch i32 %22, label %55 [
    i32 1073741823, label %57
    i32 0, label %57
  ]

55:                                               ; preds = %54
  %56 = load ptr, ptr %0, align 4, !tbaa !13
  tail call void @fa_free(ptr noundef %56, i32 noundef %22, i32 noundef 1) #62
  br label %57

57:                                               ; preds = %54, %54, %52, %55
  store ptr %46, ptr %0, align 4, !tbaa !13
  %58 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 1
  store i32 %15, ptr %58, align 4, !tbaa !18
  %59 = icmp eq i32 %15, 0
  %60 = and i32 %5, -2147483648
  %61 = select i1 %59, i32 -2147483648, i32 %60
  %62 = or i32 %45, %61
  store i32 %62, ptr %4, align 4, !tbaa !19
  br label %63

63:                                               ; preds = %21, %57
  ret void
}

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(none)
define hidden noundef i32 @fa_string_room(i32 noundef %0, i32 noundef %1) local_unnamed_addr #16 {
  %3 = and i32 %1, 1073741824
  %4 = icmp eq i32 %3, 0
  br i1 %4, label %5, label %11

5:                                                ; preds = %2
  %6 = and i32 %1, 1073741823
  %7 = icmp eq i32 %6, 0
  %8 = icmp eq i32 %6, 1073741823
  %9 = or i1 %7, %8
  %10 = select i1 %9, i32 %0, i32 %6
  br label %11

11:                                               ; preds = %2, %5
  %12 = phi i32 [ %10, %5 ], [ 11, %2 ]
  ret i32 %12
}

; Function Attrs: nounwind optsize
define hidden void @fa_string_from(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2, i32 noundef %3, ptr nocapture noundef writeonly %4) local_unnamed_addr #3 {
  %6 = alloca i32, align 4
  %7 = alloca %struct.FaString, align 4
  %8 = icmp ule i32 %3, %1
  %9 = icmp ult i32 %3, 12
  %10 = or i1 %8, %9
  br i1 %10, label %11, label %13

11:                                               ; preds = %5
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %7) #64
  %12 = and i32 %2, -2147483648
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %7, ptr noundef %0, i32 noundef %1, i32 noundef %12) #61
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %4, ptr noundef nonnull align 4 dereferenceable(12) %7, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %7) #64
  br label %35

13:                                               ; preds = %5
  %14 = icmp ugt i32 %3, 1073741822
  br i1 %14, label %15, label %16

15:                                               ; preds = %13
  tail call fastcc void @out_of_memory() #65
  unreachable

16:                                               ; preds = %13
  %17 = icmp ult i32 %3, 129
  br i1 %17, label %18, label %21

18:                                               ; preds = %16
  %19 = add nuw nsw i32 %3, 7
  %20 = and i32 %19, 504
  br label %26

21:                                               ; preds = %16
  %22 = icmp ugt i32 %3, 8192
  br i1 %22, label %26, label %23

23:                                               ; preds = %21
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %6) #64
  %24 = call fastcc i32 @size_class(i32 noundef %3, ptr noundef nonnull %6) #61
  %25 = load i32, ptr %6, align 4, !tbaa !4
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %6) #64
  br label %26

26:                                               ; preds = %18, %21, %23
  %27 = phi i32 [ %20, %18 ], [ %25, %23 ], [ %3, %21 ]
  %28 = tail call ptr @fa_alloc(i32 noundef %27, i32 noundef 1) #62
  tail call fastcc void @copy_bytes(ptr noundef %28, ptr noundef %0, i32 noundef %1) #61
  store ptr %28, ptr %4, align 4, !tbaa !13
  %29 = getelementptr inbounds %struct.FaString, ptr %4, i32 0, i32 1
  store i32 %1, ptr %29, align 4, !tbaa !18
  %30 = icmp eq i32 %1, 0
  %31 = and i32 %2, -2147483648
  %32 = select i1 %30, i32 -2147483648, i32 %31
  %33 = or i32 %27, %32
  %34 = getelementptr inbounds %struct.FaString, ptr %4, i32 0, i32 2
  store i32 %33, ptr %34, align 4, !tbaa !19
  br label %35

35:                                               ; preds = %26, %11
  ret void
}

; Function Attrs: alwaysinline nofree norecurse nosync nounwind optsize memory(argmem: read)
define hidden i32 @fa_string_equal(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef readonly %2, i32 noundef %3) local_unnamed_addr #17 {
  %5 = icmp eq i32 %1, %3
  br i1 %5, label %6, label %58

6:                                                ; preds = %4
  switch i32 %1, label %12 [
    i32 0, label %58
    i32 1, label %7
  ]

7:                                                ; preds = %6
  %8 = load i8, ptr %0, align 1, !tbaa !1
  %9 = load i8, ptr %2, align 1, !tbaa !1
  %10 = icmp eq i8 %8, %9
  %11 = zext i1 %10 to i32
  br label %58

12:                                               ; preds = %6
  %13 = icmp ult i32 %1, 17
  br i1 %13, label %14, label %56

14:                                               ; preds = %12
  %15 = icmp ugt i32 %1, 7
  br i1 %15, label %16, label %29

16:                                               ; preds = %14
  %17 = load i64, ptr %0, align 1
  %18 = load i64, ptr %2, align 1
  %19 = getelementptr inbounds i8, ptr %0, i32 %1
  %20 = getelementptr inbounds i8, ptr %19, i32 -8
  %21 = load i64, ptr %20, align 1
  %22 = getelementptr inbounds i8, ptr %2, i32 %1
  %23 = getelementptr inbounds i8, ptr %22, i32 -8
  %24 = load i64, ptr %23, align 1
  %25 = icmp eq i64 %17, %18
  %26 = icmp eq i64 %21, %24
  %27 = and i1 %25, %26
  %28 = zext i1 %27 to i32
  br label %58

29:                                               ; preds = %14
  %30 = icmp ugt i32 %1, 3
  br i1 %30, label %31, label %44

31:                                               ; preds = %29
  %32 = load i32, ptr %0, align 1
  %33 = load i32, ptr %2, align 1
  %34 = getelementptr inbounds i8, ptr %0, i32 %1
  %35 = getelementptr inbounds i8, ptr %34, i32 -4
  %36 = load i32, ptr %35, align 1
  %37 = getelementptr inbounds i8, ptr %2, i32 %1
  %38 = getelementptr inbounds i8, ptr %37, i32 -4
  %39 = load i32, ptr %38, align 1
  %40 = icmp eq i32 %32, %33
  %41 = icmp eq i32 %36, %39
  %42 = and i1 %40, %41
  %43 = zext i1 %42 to i32
  br label %58

44:                                               ; preds = %29
  %45 = load i16, ptr %0, align 1
  %46 = load i16, ptr %2, align 1
  %47 = icmp eq i16 %45, %46
  br i1 %47, label %48, label %58

48:                                               ; preds = %44
  %49 = add nsw i32 %1, -1
  %50 = getelementptr inbounds i8, ptr %0, i32 %49
  %51 = load i8, ptr %50, align 1, !tbaa !1
  %52 = getelementptr inbounds i8, ptr %2, i32 %49
  %53 = load i8, ptr %52, align 1, !tbaa !1
  %54 = icmp eq i8 %51, %53
  %55 = zext i1 %54 to i32
  br label %58

56:                                               ; preds = %12
  %57 = tail call fastcc i32 @string_equal_bytes(ptr noundef %0, ptr noundef %2, i32 noundef %1) #61
  br label %58

58:                                               ; preds = %44, %48, %6, %4, %56, %31, %16, %7
  %59 = phi i32 [ %11, %7 ], [ %28, %16 ], [ %43, %31 ], [ %57, %56 ], [ 0, %4 ], [ 1, %6 ], [ 0, %44 ], [ %55, %48 ]
  ret i32 %59
}

; Function Attrs: nofree noinline norecurse nosync nounwind optsize memory(argmem: read)
define internal fastcc noundef i32 @string_equal_bytes(ptr nocapture noundef readonly %0, ptr nocapture noundef readonly %1, i32 noundef %2) unnamed_addr #18 {
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
  br i1 %17, label %7, label %18, !llvm.loop !38

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
  br i1 %19, label %39, label %20, !llvm.loop !39

20:                                               ; preds = %16
  %21 = getelementptr inbounds i8, ptr %0, i32 %18
  %22 = load i8, ptr %21, align 1, !tbaa !1
  %23 = getelementptr inbounds i8, ptr %2, i32 %18
  %24 = load i8, ptr %23, align 1, !tbaa !1
  %25 = icmp eq i8 %22, %24
  br i1 %25, label %16, label %26, !llvm.loop !39

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
define hidden i32 @fa_next_grapheme(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2) local_unnamed_addr #19 {
  %4 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %2) #61
  ret i32 %4
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define internal fastcc i32 @next_boundary(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2) unnamed_addr #19 {
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
  %25 = call fastcc i32 @decode(ptr noundef nonnull %0, i32 noundef %1, ptr noundef nonnull %4) #61
  %26 = tail call fastcc i32 @break_kind(i32 noundef %25) #61, !range !40
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
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %5) #64
  store i32 %38, ptr %5, align 4, !tbaa !4
  %39 = call fastcc i32 @decode(ptr noundef %0, i32 noundef %1, ptr noundef nonnull %5) #61
  %40 = tail call fastcc i32 @break_kind(i32 noundef %39) #61, !range !40
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
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %5) #64
  br label %106

101:                                              ; preds = %74, %94, %88
  %102 = phi i32 [ %89, %88 ], [ %95, %94 ], [ %75, %74 ]
  %103 = phi i32 [ %92, %88 ], [ %96, %94 ], [ 0, %74 ]
  %104 = phi i32 [ 1, %88 ], [ %99, %94 ], [ 1, %74 ]
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %5) #64
  %105 = icmp ult i32 %102, %1
  br i1 %105, label %34, label %106

106:                                              ; preds = %101, %24, %100, %21
  %107 = phi i32 [ %23, %21 ], [ %38, %100 ], [ %27, %24 ], [ %102, %101 ]
  ret i32 %107
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define hidden i32 @fa_graphemes(ptr nocapture noundef readonly %0, i32 noundef %1) local_unnamed_addr #19 {
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
  %23 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %6) #61
  %24 = add i32 %5, 1
  br label %25

25:                                               ; preds = %22, %13
  %26 = phi i32 [ %7, %13 ], [ %23, %22 ]
  %27 = phi i32 [ %21, %13 ], [ %24, %22 ]
  %28 = icmp ult i32 %26, %1
  br i1 %28, label %4, label %29, !llvm.loop !41

29:                                               ; preds = %25, %2
  %30 = phi i32 [ 0, %2 ], [ %27, %25 ]
  ret i32 %30
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_append_into(ptr nocapture noundef %0) local_unnamed_addr #3 {
  %2 = alloca %struct.FaString, align 4
  %3 = load ptr, ptr @builder, align 4, !tbaa !6
  %4 = load i32, ptr @builder_length, align 4, !tbaa !4
  %5 = load i32, ptr @builder_simple, align 4, !tbaa !4
  %6 = icmp eq i32 %5, %4
  br i1 %6, label %11, label %7

7:                                                ; preds = %1
  %8 = getelementptr inbounds i8, ptr %3, i32 %5
  %9 = sub i32 %4, %5
  %10 = tail call fastcc i32 @simple_flag(ptr noundef %8, i32 noundef %9) #61, !range !20
  br label %11

11:                                               ; preds = %1, %7
  %12 = phi i32 [ %10, %7 ], [ -2147483648, %1 ]
  %13 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 2
  %14 = load i32, ptr %13, align 4, !tbaa !19
  %15 = and i32 %14, 2147483647
  %16 = and i32 %12, -2147483648
  %17 = icmp ne i32 %16, 0
  %18 = icmp ult i32 %15, 1073741823
  %19 = select i1 %17, i1 %18, i1 false
  br i1 %19, label %20, label %30, !prof !34

20:                                               ; preds = %11
  %21 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 1
  %22 = load i32, ptr %21, align 4, !tbaa !18
  %23 = add i32 %22, %4
  %24 = icmp ugt i32 %23, %15
  br i1 %24, label %30, label %25, !prof !35

25:                                               ; preds = %20
  %26 = load ptr, ptr %0, align 4, !tbaa !13
  %27 = getelementptr inbounds i8, ptr %26, i32 %22
  tail call fastcc void @copy_bytes(ptr noundef %27, ptr noundef %3, i32 noundef %4) #61
  %28 = load i32, ptr %21, align 4, !tbaa !18
  %29 = add i32 %28, %4
  store i32 %29, ptr %21, align 4, !tbaa !18
  br label %70

30:                                               ; preds = %20, %11
  %31 = icmp eq i32 %4, 0
  br i1 %31, label %70, label %32

32:                                               ; preds = %30
  %33 = and i32 %14, 1073741824
  %34 = icmp eq i32 %33, 0
  br i1 %34, label %35, label %40

35:                                               ; preds = %32
  %36 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 1
  %37 = load i32, ptr %36, align 4, !tbaa !18
  %38 = add i32 %37, %4
  %39 = icmp ult i32 %38, 12
  br i1 %39, label %55, label %69

40:                                               ; preds = %32
  %41 = lshr i32 %14, 24
  %42 = and i32 %41, 31
  %43 = add i32 %42, %4
  %44 = icmp ult i32 %43, 12
  br i1 %44, label %45, label %69

45:                                               ; preds = %40
  %46 = icmp eq i32 %42, 0
  %47 = select i1 %46, i32 -2147483648, i32 %14
  %48 = and i32 %47, %16
  %49 = getelementptr inbounds i8, ptr %0, i32 %42
  tail call fastcc void @copy_bytes(ptr noundef %49, ptr noundef %3, i32 noundef %4) #61
  %50 = icmp eq i32 %48, 0
  %51 = select i1 %50, i32 64, i32 192
  %52 = or disjoint i32 %51, %43
  %53 = trunc i32 %52 to i8
  %54 = getelementptr inbounds i8, ptr %0, i32 11
  store i8 %53, ptr %54, align 1, !tbaa !1
  br label %70

55:                                               ; preds = %35
  %56 = icmp eq i32 %37, 0
  %57 = select i1 %56, i32 -2147483648, i32 %14
  %58 = and i32 %57, %16
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %2) #64
  call void @llvm.memset.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %2, i8 0, i32 12, i1 false)
  %59 = load ptr, ptr %0, align 4, !tbaa !13
  call fastcc void @copy_bytes(ptr noundef nonnull %2, ptr noundef %59, i32 noundef %37) #61
  %60 = getelementptr inbounds i8, ptr %2, i32 %37
  call fastcc void @copy_bytes(ptr noundef nonnull %60, ptr noundef %3, i32 noundef %4) #61
  %61 = icmp eq i32 %58, 0
  %62 = select i1 %61, i32 64, i32 192
  %63 = or disjoint i32 %62, %38
  %64 = trunc i32 %63 to i8
  %65 = getelementptr inbounds i8, ptr %2, i32 11
  store i8 %64, ptr %65, align 1, !tbaa !1
  %66 = and i32 %14, 1073741823
  switch i32 %66, label %67 [
    i32 1073741823, label %68
    i32 0, label %68
  ]

67:                                               ; preds = %55
  tail call void @fa_free(ptr noundef %59, i32 noundef %66, i32 noundef 1) #62
  br label %68

68:                                               ; preds = %55, %55, %67
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %0, ptr noundef nonnull align 4 dereferenceable(12) %2, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %2) #64
  br label %70

69:                                               ; preds = %40, %35
  tail call fastcc void @string_append_slow(ptr noundef nonnull %0, ptr noundef %3, i32 noundef %4, i32 noundef %12) #61
  br label %70

70:                                               ; preds = %25, %30, %45, %68, %69
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  store i32 0, ptr @builder_simple, align 4, !tbaa !4
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden i32 @fa_string_includes(ptr noundef %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3, i32 noundef %4) local_unnamed_addr #20 {
  %6 = and i32 %2, -2147483648
  %7 = tail call fastcc i64 @find(ptr noundef %0, i32 noundef %1, i32 noundef %6, ptr noundef %3, i32 noundef %4, i32 noundef 0) #61
  %8 = icmp sgt i64 %7, -1
  %9 = zext i1 %8 to i32
  ret i32 %9
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc i64 @find(ptr noundef %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3, i32 noundef %4, i32 noundef %5) unnamed_addr #20 {
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
  %18 = tail call fastcc i32 @next_byte(ptr noundef %0, i32 noundef %1, i32 noundef 0, i8 noundef zeroext %17) #61
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
  br i1 %29, label %36, label %30, !llvm.loop !42

30:                                               ; preds = %26
  %31 = getelementptr i8, ptr %23, i32 %28
  %32 = load i8, ptr %31, align 1, !tbaa !1
  %33 = getelementptr inbounds i8, ptr %3, i32 %28
  %34 = load i8, ptr %33, align 1, !tbaa !1
  %35 = icmp eq i8 %32, %34
  br i1 %35, label %26, label %36, !llvm.loop !42

36:                                               ; preds = %30, %26
  %37 = icmp ult i32 %28, %4
  br i1 %37, label %40, label %38

38:                                               ; preds = %36
  %39 = zext i32 %22 to i64
  br label %120

40:                                               ; preds = %21, %36
  %41 = add i32 %22, 1
  %42 = tail call fastcc i32 @next_byte(ptr noundef nonnull %0, i32 noundef %1, i32 noundef %41, i8 noundef zeroext %17) #61
  %43 = add i32 %42, %4
  %44 = icmp ugt i32 %43, %1
  br i1 %44, label %120, label %21, !llvm.loop !43

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
  br i1 %63, label %70, label %64, !llvm.loop !42

64:                                               ; preds = %60
  %65 = getelementptr i8, ptr %54, i32 %62
  %66 = load i8, ptr %65, align 1, !tbaa !1
  %67 = getelementptr inbounds i8, ptr %3, i32 %62
  %68 = load i8, ptr %67, align 1, !tbaa !1
  %69 = icmp eq i8 %66, %68
  br i1 %69, label %60, label %70, !llvm.loop !42

70:                                               ; preds = %64, %60
  %71 = icmp ult i32 %62, %4
  br i1 %71, label %72, label %118

72:                                               ; preds = %50, %70
  %73 = add nuw i32 %51, 1
  %74 = icmp eq i32 %51, %48
  br i1 %74, label %120, label %50, !llvm.loop !44

75:                                               ; preds = %45
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %7) #64
  store ptr %0, ptr %7, align 4, !tbaa !45
  %76 = getelementptr inbounds %struct.Boundaries, ptr %7, i32 0, i32 1
  store i32 %1, ptr %76, align 4, !tbaa !47
  %77 = getelementptr inbounds %struct.Boundaries, ptr %7, i32 0, i32 2
  store i32 0, ptr %77, align 4, !tbaa !48
  %78 = getelementptr inbounds %struct.Boundaries, ptr %7, i32 0, i32 3
  store i32 %2, ptr %78, align 4, !tbaa !49
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %8) #64
  store ptr %0, ptr %8, align 4, !tbaa !45
  %79 = getelementptr inbounds %struct.Boundaries, ptr %8, i32 0, i32 1
  store i32 %1, ptr %79, align 4, !tbaa !47
  %80 = getelementptr inbounds %struct.Boundaries, ptr %8, i32 0, i32 2
  store i32 0, ptr %80, align 4, !tbaa !48
  %81 = getelementptr inbounds %struct.Boundaries, ptr %8, i32 0, i32 3
  store i32 %2, ptr %81, align 4, !tbaa !49
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
  br i1 %94, label %101, label %95, !llvm.loop !42

95:                                               ; preds = %91
  %96 = getelementptr i8, ptr %86, i32 %93
  %97 = load i8, ptr %96, align 1, !tbaa !1
  %98 = getelementptr inbounds i8, ptr %3, i32 %93
  %99 = load i8, ptr %98, align 1, !tbaa !1
  %100 = icmp eq i8 %97, %99
  br i1 %100, label %91, label %101, !llvm.loop !42

101:                                              ; preds = %95, %91
  %102 = icmp ult i32 %93, %4
  br i1 %102, label %111, label %103

103:                                              ; preds = %82, %101
  %104 = call fastcc i32 @at_boundary(ptr noundef nonnull %7, i32 noundef %84) #61, !range !50
  %105 = icmp eq i32 %104, 0
  br i1 %105, label %111, label %106

106:                                              ; preds = %103
  %107 = call fastcc i32 @at_boundary(ptr noundef nonnull %8, i32 noundef %83) #61, !range !50
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
  br i1 %115, label %116, label %82, !llvm.loop !51

116:                                              ; preds = %111, %109
  %117 = phi i64 [ %112, %111 ], [ %110, %109 ]
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %8) #64
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %7) #64
  br label %120

118:                                              ; preds = %70
  %119 = zext i32 %53 to i64
  br label %120

120:                                              ; preds = %40, %72, %16, %118, %46, %38, %6, %116
  %121 = phi i64 [ %117, %116 ], [ -1, %6 ], [ %39, %38 ], [ %119, %118 ], [ -1, %46 ], [ -1, %16 ], [ -1, %72 ], [ -1, %40 ]
  ret i64 %121
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define hidden i32 @fa_string_starts_with(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3, i32 noundef %4) local_unnamed_addr #19 {
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
  br i1 %16, label %23, label %17, !llvm.loop !42

17:                                               ; preds = %13
  %18 = getelementptr i8, ptr %0, i32 %15
  %19 = load i8, ptr %18, align 1, !tbaa !1
  %20 = getelementptr inbounds i8, ptr %3, i32 %15
  %21 = load i8, ptr %20, align 1, !tbaa !1
  %22 = icmp eq i8 %19, %21
  br i1 %22, label %13, label %23, !llvm.loop !42

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
  %31 = tail call fastcc i32 @next_boundary(ptr noundef nonnull %0, i32 noundef %1, i32 noundef %30) #61
  %32 = icmp ult i32 %31, %4
  br i1 %32, label %29, label %33, !llvm.loop !52

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
define hidden i32 @fa_string_ends_with(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3, i32 noundef %4) local_unnamed_addr #19 {
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
  br i1 %18, label %25, label %19, !llvm.loop !42

19:                                               ; preds = %15
  %20 = getelementptr i8, ptr %9, i32 %17
  %21 = load i8, ptr %20, align 1, !tbaa !1
  %22 = getelementptr inbounds i8, ptr %3, i32 %17
  %23 = load i8, ptr %22, align 1, !tbaa !1
  %24 = icmp eq i8 %21, %23
  br i1 %24, label %15, label %25, !llvm.loop !42

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
  %33 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %32) #61
  %34 = icmp ult i32 %33, %8
  br i1 %34, label %31, label %35, !llvm.loop !52

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
define hidden i64 @fa_string_index_of(ptr noundef %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3, i32 noundef %4, i32 noundef %5) local_unnamed_addr #20 {
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
  br i1 %27, label %30, label %18, !llvm.loop !53

28:                                               ; preds = %6
  %29 = tail call fastcc i64 @string_index_of(ptr noundef %0, i32 noundef %1, i32 noundef %2, ptr noundef %3, i32 noundef %4, i32 noundef %5) #61
  br label %30

30:                                               ; preds = %25, %14, %23, %28
  %31 = phi i64 [ %29, %28 ], [ %24, %23 ], [ -1, %14 ], [ -1, %25 ]
  ret i64 %31
}

; Function Attrs: nofree noinline norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc i64 @string_index_of(ptr noundef %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3, i32 noundef %4, i32 noundef %5) unnamed_addr #21 {
  %7 = and i32 %2, -2147483648
  %8 = tail call fastcc i64 @find(ptr noundef %0, i32 noundef %1, i32 noundef %7, ptr noundef %3, i32 noundef %4, i32 noundef %5) #61
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
  %19 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %16) #61
  %20 = icmp ult i32 %19, %13
  br i1 %20, label %15, label %21, !llvm.loop !54

21:                                               ; preds = %15
  %22 = zext i32 %18 to i64
  br label %23

23:                                               ; preds = %12, %21, %6
  %24 = phi i64 [ %8, %6 ], [ 0, %12 ], [ %22, %21 ]
  ret i64 %24
}

; Function Attrs: nounwind optsize
define hidden void @fa_string_replace(ptr noundef %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3, i32 noundef %4, ptr noundef %5, i32 noundef %6, i32 noundef %7, ptr nocapture noundef writeonly %8) local_unnamed_addr #3 {
  %10 = alloca %struct.Replacing, align 4
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  store i32 0, ptr @builder_simple, align 4, !tbaa !4
  call void @llvm.lifetime.start.p0(i64 28, ptr nonnull %10) #64
  store ptr %0, ptr %10, align 4, !tbaa !55
  %11 = getelementptr inbounds %struct.Replacing, ptr %10, i32 0, i32 1
  store i32 0, ptr %11, align 4, !tbaa !57
  %12 = getelementptr inbounds %struct.Replacing, ptr %10, i32 0, i32 2
  store i32 %4, ptr %12, align 4, !tbaa !58
  %13 = getelementptr inbounds %struct.Replacing, ptr %10, i32 0, i32 3
  store ptr %5, ptr %13, align 4, !tbaa !59
  %14 = getelementptr inbounds %struct.Replacing, ptr %10, i32 0, i32 4
  store i32 %6, ptr %14, align 4, !tbaa !60
  %15 = getelementptr inbounds %struct.Replacing, ptr %10, i32 0, i32 5
  store i32 %7, ptr %15, align 4, !tbaa !61
  %16 = getelementptr inbounds %struct.Replacing, ptr %10, i32 0, i32 6
  store i32 0, ptr %16, align 4, !tbaa !62
  %17 = and i32 %2, -2147483648
  call fastcc void @each_match(ptr noundef %0, i32 noundef %1, i32 noundef %17, ptr noundef %3, i32 noundef %4, ptr noundef nonnull @replace_one, ptr noundef nonnull %10) #61
  %18 = load i32, ptr %11, align 4, !tbaa !57
  %19 = getelementptr inbounds i8, ptr %0, i32 %18
  %20 = sub i32 %1, %18
  call fastcc void @reserve(i32 noundef %20) #61
  %21 = load ptr, ptr @builder, align 4, !tbaa !6
  %22 = load i32, ptr @builder_length, align 4, !tbaa !4
  %23 = getelementptr inbounds i8, ptr %21, i32 %22
  call fastcc void @copy_bytes(ptr noundef %23, ptr noundef %19, i32 noundef %20) #61
  %24 = add i32 %22, %20
  store i32 %24, ptr @builder_length, align 4, !tbaa !4
  call void @fa_b_take(ptr noundef %8) #61
  call void @llvm.lifetime.end.p0(i64 28, ptr nonnull %10) #64
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
  %15 = tail call fastcc i32 @next_byte(ptr noundef %0, i32 noundef %1, i32 noundef 0, i8 noundef zeroext %14) #61
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
  br i1 %28, label %35, label %29, !llvm.loop !42

29:                                               ; preds = %25
  %30 = getelementptr i8, ptr %22, i32 %27
  %31 = load i8, ptr %30, align 1, !tbaa !1
  %32 = getelementptr inbounds i8, ptr %3, i32 %27
  %33 = load i8, ptr %32, align 1, !tbaa !1
  %34 = icmp eq i8 %31, %33
  br i1 %34, label %25, label %35, !llvm.loop !42

35:                                               ; preds = %29, %25
  %36 = icmp ult i32 %27, %4
  br i1 %36, label %39, label %37

37:                                               ; preds = %35
  tail call void %5(ptr noundef %6, i32 noundef %21) #62, !callees !63
  %38 = load i8, ptr %3, align 1, !tbaa !1
  br label %41

39:                                               ; preds = %18, %35
  %40 = add i32 %21, 1
  br label %41

41:                                               ; preds = %39, %37
  %42 = phi i8 [ %38, %37 ], [ %19, %39 ]
  %43 = phi i32 [ %20, %37 ], [ %40, %39 ]
  %44 = tail call fastcc i32 @next_byte(ptr noundef nonnull %0, i32 noundef %1, i32 noundef %43, i8 noundef zeroext %42) #61
  %45 = add i32 %44, %4
  %46 = icmp ugt i32 %45, %1
  br i1 %46, label %91, label %18, !llvm.loop !64

47:                                               ; preds = %7
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %8) #64
  store ptr %0, ptr %8, align 4, !tbaa !45
  %48 = getelementptr inbounds %struct.Boundaries, ptr %8, i32 0, i32 1
  store i32 %1, ptr %48, align 4, !tbaa !47
  %49 = getelementptr inbounds %struct.Boundaries, ptr %8, i32 0, i32 2
  store i32 0, ptr %49, align 4, !tbaa !48
  %50 = getelementptr inbounds %struct.Boundaries, ptr %8, i32 0, i32 3
  store i32 %2, ptr %50, align 4, !tbaa !49
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %9) #64
  store ptr %0, ptr %9, align 4, !tbaa !45
  %51 = getelementptr inbounds %struct.Boundaries, ptr %9, i32 0, i32 1
  store i32 %1, ptr %51, align 4, !tbaa !47
  %52 = getelementptr inbounds %struct.Boundaries, ptr %9, i32 0, i32 2
  store i32 0, ptr %52, align 4, !tbaa !48
  %53 = getelementptr inbounds %struct.Boundaries, ptr %9, i32 0, i32 3
  store i32 %2, ptr %53, align 4, !tbaa !49
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
  br i1 %69, label %76, label %70, !llvm.loop !42

70:                                               ; preds = %66
  %71 = getelementptr i8, ptr %61, i32 %68
  %72 = load i8, ptr %71, align 1, !tbaa !1
  %73 = getelementptr inbounds i8, ptr %3, i32 %68
  %74 = load i8, ptr %73, align 1, !tbaa !1
  %75 = icmp eq i8 %72, %74
  br i1 %75, label %66, label %76, !llvm.loop !42

76:                                               ; preds = %70, %66
  %77 = icmp ult i32 %68, %4
  br i1 %77, label %85, label %78

78:                                               ; preds = %58, %76
  %79 = call fastcc i32 @at_boundary(ptr noundef nonnull %8, i32 noundef %60) #61, !range !50
  %80 = icmp eq i32 %79, 0
  br i1 %80, label %85, label %81

81:                                               ; preds = %78
  %82 = call fastcc i32 @at_boundary(ptr noundef nonnull %9, i32 noundef %59) #61, !range !50
  %83 = icmp eq i32 %82, 0
  br i1 %83, label %85, label %84

84:                                               ; preds = %81
  tail call void %5(ptr noundef %6, i32 noundef %60) #62, !callees !63
  br label %85

85:                                               ; preds = %62, %76, %78, %81, %84
  %86 = phi i32 [ %57, %84 ], [ 1, %81 ], [ 1, %78 ], [ 1, %76 ], [ 1, %62 ]
  %87 = add i32 %86, %60
  %88 = add i32 %87, %4
  %89 = icmp ugt i32 %88, %1
  br i1 %89, label %90, label %58, !llvm.loop !65

90:                                               ; preds = %85, %47
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %9) #64
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %8) #64
  br label %91

91:                                               ; preds = %41, %13, %90
  ret void
}

; Function Attrs: nounwind optsize
define internal void @replace_one(ptr nocapture noundef %0, i32 noundef %1) #3 {
  %3 = getelementptr inbounds %struct.Replacing, ptr %0, i32 0, i32 6
  %4 = load i32, ptr %3, align 4, !tbaa !62
  %5 = icmp eq i32 %4, 0
  br i1 %5, label %6, label %31

6:                                                ; preds = %2
  %7 = load ptr, ptr %0, align 4, !tbaa !55
  %8 = getelementptr inbounds %struct.Replacing, ptr %0, i32 0, i32 1
  %9 = load i32, ptr %8, align 4, !tbaa !57
  %10 = getelementptr inbounds i8, ptr %7, i32 %9
  %11 = sub i32 %1, %9
  tail call fastcc void @reserve(i32 noundef %11) #61
  %12 = load ptr, ptr @builder, align 4, !tbaa !6
  %13 = load i32, ptr @builder_length, align 4, !tbaa !4
  %14 = getelementptr inbounds i8, ptr %12, i32 %13
  tail call fastcc void @copy_bytes(ptr noundef %14, ptr noundef %10, i32 noundef %11) #61
  %15 = add i32 %13, %11
  store i32 %15, ptr @builder_length, align 4, !tbaa !4
  %16 = getelementptr inbounds %struct.Replacing, ptr %0, i32 0, i32 3
  %17 = load ptr, ptr %16, align 4, !tbaa !59
  %18 = getelementptr inbounds %struct.Replacing, ptr %0, i32 0, i32 4
  %19 = load i32, ptr %18, align 4, !tbaa !60
  tail call fastcc void @reserve(i32 noundef %19) #61
  %20 = load ptr, ptr @builder, align 4, !tbaa !6
  %21 = load i32, ptr @builder_length, align 4, !tbaa !4
  %22 = getelementptr inbounds i8, ptr %20, i32 %21
  tail call fastcc void @copy_bytes(ptr noundef %22, ptr noundef %17, i32 noundef %19) #61
  %23 = add i32 %21, %19
  store i32 %23, ptr @builder_length, align 4, !tbaa !4
  %24 = getelementptr inbounds %struct.Replacing, ptr %0, i32 0, i32 2
  %25 = load i32, ptr %24, align 4, !tbaa !58
  %26 = add i32 %25, %1
  store i32 %26, ptr %8, align 4, !tbaa !57
  %27 = getelementptr inbounds %struct.Replacing, ptr %0, i32 0, i32 5
  %28 = load i32, ptr %27, align 4, !tbaa !61
  %29 = icmp eq i32 %28, 0
  %30 = zext i1 %29 to i32
  store i32 %30, ptr %3, align 4, !tbaa !62
  br label %31

31:                                               ; preds = %2, %6
  ret void
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_string_characters(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2) local_unnamed_addr #3 {
  %4 = tail call fastcc ptr @characters_of(ptr noundef %0, i32 noundef %1, i32 noundef %2) #61
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
  %11 = tail call ptr @fa_array_new(i64 noundef %9, i64 noundef %9, i64 noundef 12) #61
  %12 = load ptr, ptr %11, align 8, !tbaa !66
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
  br i1 %24, label %41, label %14, !llvm.loop !69

25:                                               ; preds = %6
  %26 = tail call ptr @fa_array_new(i64 noundef %9, i64 noundef %9, i64 noundef 1) #61
  %27 = load ptr, ptr %26, align 8, !tbaa !66
  tail call fastcc void @copy_bytes(ptr noundef %27, ptr noundef %0, i32 noundef %1) #61
  %28 = getelementptr inbounds %struct.FaArrayHeader, ptr %26, i32 0, i32 2
  store i64 -1, ptr %28, align 8, !tbaa !70
  br label %41

29:                                               ; preds = %3
  %30 = tail call i32 @fa_graphemes(ptr noundef %0, i32 noundef %1) #61
  %31 = zext i32 %30 to i64
  %32 = tail call ptr @fa_array_new(i64 noundef 0, i64 noundef %31, i64 noundef 12) #61
  %33 = icmp eq i32 %1, 0
  br i1 %33, label %41, label %34

34:                                               ; preds = %29, %34
  %35 = phi i32 [ %36, %34 ], [ 0, %29 ]
  %36 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %35) #61
  %37 = tail call ptr @fa_array_push_slot(ptr noundef %32, i64 noundef 12) #61
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %4) #64
  %38 = getelementptr inbounds i8, ptr %0, i32 %35
  %39 = sub i32 %36, %35
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %4, ptr noundef %38, i32 noundef %39, i32 noundef 0) #61
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %37, ptr noundef nonnull align 4 dereferenceable(12) %4, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %4) #64
  %40 = icmp ult i32 %36, %1
  br i1 %40, label %34, label %41, !llvm.loop !71

41:                                               ; preds = %14, %34, %10, %29, %25
  %42 = phi ptr [ %26, %25 ], [ %32, %29 ], [ %11, %10 ], [ %32, %34 ], [ %11, %14 ]
  ret ptr %42
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_string_split(ptr noundef %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3, i32 noundef %4) local_unnamed_addr #3 {
  %6 = tail call fastcc ptr @split_text(ptr noundef %0, i32 noundef %1, i32 noundef %2, ptr noundef %3, i32 noundef %4, i32 noundef 0) #61
  ret ptr %6
}

; Function Attrs: nounwind optsize
define internal fastcc ptr @split_text(ptr noundef %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3, i32 noundef %4, i32 noundef %5) unnamed_addr #3 {
  %7 = alloca %struct.Splitting, align 4
  %8 = alloca %struct.FaString, align 4
  %9 = icmp eq i32 %4, 0
  br i1 %9, label %10, label %12

10:                                               ; preds = %6
  %11 = tail call fastcc ptr @characters_of(ptr noundef %0, i32 noundef %1, i32 noundef %2) #61
  br label %42

12:                                               ; preds = %6
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %7) #64
  store ptr %0, ptr %7, align 4, !tbaa !72
  %13 = getelementptr inbounds %struct.Splitting, ptr %7, i32 0, i32 1
  store i32 0, ptr %13, align 4, !tbaa !74
  %14 = getelementptr inbounds %struct.Splitting, ptr %7, i32 0, i32 2
  store i32 %4, ptr %14, align 4, !tbaa !75
  %15 = getelementptr inbounds %struct.Splitting, ptr %7, i32 0, i32 3
  %16 = tail call ptr @fa_alloc(i32 noundef 80, i32 noundef 16) #62
  %17 = getelementptr inbounds i8, ptr %16, i32 24
  store i64 1, ptr %17, align 1
  %18 = getelementptr inbounds i8, ptr %16, i32 32
  store ptr %18, ptr %16, align 8, !tbaa !66
  %19 = getelementptr inbounds %struct.FaArrayHeader, ptr %16, i32 0, i32 1
  store i64 0, ptr %19, align 8, !tbaa !76
  %20 = getelementptr inbounds %struct.FaArrayHeader, ptr %16, i32 0, i32 2
  store i64 4, ptr %20, align 8, !tbaa !70
  store ptr %16, ptr %15, align 4, !tbaa !77
  %21 = getelementptr inbounds %struct.Splitting, ptr %7, i32 0, i32 4
  %22 = and i32 %2, -2147483648
  store i32 %22, ptr %21, align 4, !tbaa !78
  %23 = getelementptr inbounds %struct.Splitting, ptr %7, i32 0, i32 5
  store i32 %5, ptr %23, align 4, !tbaa !79
  call fastcc void @each_match(ptr noundef %0, i32 noundef %1, i32 noundef %22, ptr noundef %3, i32 noundef %4, ptr noundef nonnull @split_one, ptr noundef nonnull %7) #61
  %24 = load ptr, ptr %15, align 4, !tbaa !77
  %25 = call ptr @fa_array_push_slot(ptr noundef %24, i64 noundef 12) #61
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %8) #64
  %26 = load i32, ptr %13, align 4, !tbaa !74
  call void @llvm.experimental.noalias.scope.decl(metadata !80)
  %27 = load i32, ptr %23, align 4, !tbaa !79, !noalias !80
  %28 = icmp eq i32 %27, 0
  %29 = load ptr, ptr %7, align 4, !tbaa !72, !noalias !80
  %30 = getelementptr inbounds i8, ptr %29, i32 %26
  br i1 %28, label %37, label %31

31:                                               ; preds = %12
  store ptr %30, ptr %8, align 4, !tbaa !13, !alias.scope !80
  %32 = getelementptr inbounds %struct.FaString, ptr %8, i32 0, i32 1
  %33 = sub i32 %1, %26
  store i32 %33, ptr %32, align 4, !tbaa !18, !alias.scope !80
  %34 = getelementptr inbounds %struct.FaString, ptr %8, i32 0, i32 2
  %35 = load i32, ptr %21, align 4, !tbaa !78, !noalias !80
  %36 = or i32 %35, 1073741823
  store i32 %36, ptr %34, align 4, !tbaa !19, !alias.scope !80
  br label %40

37:                                               ; preds = %12
  %38 = sub i32 %1, %26
  %39 = load i32, ptr %21, align 4, !tbaa !78, !noalias !80
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %8, ptr noundef %30, i32 noundef %38, i32 noundef %39) #61
  br label %40

40:                                               ; preds = %31, %37
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %25, ptr noundef nonnull align 4 dereferenceable(12) %8, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %8) #64
  %41 = load ptr, ptr %15, align 4, !tbaa !77
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %7) #64
  br label %42

42:                                               ; preds = %40, %10
  %43 = phi ptr [ %11, %10 ], [ %41, %40 ]
  ret ptr %43
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_string_split_views(ptr noundef %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3, i32 noundef %4) local_unnamed_addr #3 {
  %6 = tail call fastcc ptr @split_text(ptr noundef %0, i32 noundef %1, i32 noundef %2, ptr noundef %3, i32 noundef %4, i32 noundef 1) #61
  ret ptr %6
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
  %22 = tail call ptr @fa_array_new(i64 noundef %21, i64 noundef %21, i64 noundef 4) #61
  %23 = load ptr, ptr %22, align 8, !tbaa !66
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
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %4) #64
  store i32 0, ptr %4, align 4, !tbaa !4
  %36 = icmp eq i32 %1, 0
  br i1 %36, label %37, label %38

37:                                               ; preds = %38, %35
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %4) #64
  br label %45

38:                                               ; preds = %35, %38
  %39 = phi i32 [ %42, %38 ], [ 0, %35 ]
  %40 = call fastcc i32 @decode(ptr noundef %0, i32 noundef %1, ptr noundef nonnull %4) #61
  %41 = getelementptr inbounds i32, ptr %23, i32 %39
  store i32 %40, ptr %41, align 4, !tbaa !23
  %42 = add i32 %39, 1
  %43 = load i32, ptr %4, align 4, !tbaa !4
  %44 = icmp ult i32 %43, %1
  br i1 %44, label %38, label %37, !llvm.loop !85

45:                                               ; preds = %27, %25, %37
  ret ptr %22
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
  %10 = tail call ptr @fa_alloc(i32 noundef %9, i32 noundef 16) #62
  %11 = getelementptr inbounds i8, ptr %10, i32 24
  store i64 1, ptr %11, align 1
  %12 = getelementptr inbounds i8, ptr %10, i32 32
  store ptr %12, ptr %10, align 8, !tbaa !66
  br label %17

13:                                               ; preds = %3
  %14 = tail call ptr @fa_alloc(i32 noundef 32, i32 noundef 8) #62
  %15 = getelementptr inbounds i8, ptr %14, i32 24
  store i64 0, ptr %15, align 1
  %16 = tail call ptr @fa_alloc(i32 noundef %6, i32 noundef 16) #62
  store ptr %16, ptr %14, align 8, !tbaa !66
  br label %17

17:                                               ; preds = %13, %8
  %18 = phi ptr [ %14, %13 ], [ %10, %8 ]
  %19 = getelementptr inbounds %struct.FaArrayHeader, ptr %18, i32 0, i32 1
  store i64 %0, ptr %19, align 8, !tbaa !76
  %20 = getelementptr inbounds %struct.FaArrayHeader, ptr %18, i32 0, i32 2
  store i64 %4, ptr %20, align 8, !tbaa !70
  ret ptr %18
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define internal fastcc i32 @decode(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef %2) unnamed_addr #19 {
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
define hidden i64 @fa_string_code_point_at(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2, i64 noundef %3) local_unnamed_addr #19 {
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
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %5) #64
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
  %45 = call fastcc i32 @decode(ptr noundef %0, i32 noundef %1, ptr noundef nonnull %5) #61
  %46 = icmp eq i64 %44, 0
  br i1 %46, label %47, label %39

47:                                               ; preds = %43
  %48 = zext i32 %45 to i64
  br label %49

49:                                               ; preds = %39, %36, %47
  %50 = phi i64 [ %48, %47 ], [ -1, %36 ], [ -1, %39 ]
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %5) #64
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
define hidden i32 @fa_next_code_point(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef %2) local_unnamed_addr #19 {
  %4 = alloca i32, align 4
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %4) #64
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
  %15 = call fastcc i32 @decode(ptr noundef nonnull %0, i32 noundef %1, ptr noundef nonnull %4) #61
  %16 = load i32, ptr %4, align 4, !tbaa !4
  %17 = zext i32 %16 to i64
  br label %18

18:                                               ; preds = %14, %10
  %19 = phi i64 [ %17, %14 ], [ %13, %10 ]
  %20 = phi i32 [ %15, %14 ], [ %11, %10 ]
  store i64 %19, ptr %2, align 8, !tbaa !89
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %4) #64
  ret i32 %20
}

; Function Attrs: nounwind optsize
define hidden noundef i32 @fa_string_get(ptr noundef %0, i32 noundef %1, i32 noundef %2, i64 noundef %3, ptr nocapture noundef writeonly %4) local_unnamed_addr #3 {
  %6 = icmp sgt i32 %2, -1
  br i1 %6, label %27, label %7

7:                                                ; preds = %5
  %8 = icmp slt i64 %3, 0
  %9 = zext i32 %1 to i64
  %10 = select i1 %8, i64 %9, i64 0
  %11 = add nsw i64 %10, %3
  %12 = icmp ult i64 %11, %9
  br i1 %12, label %13, label %29

13:                                               ; preds = %7
  %14 = and i32 %2, 2147483647
  %15 = icmp eq i32 %14, 0
  %16 = trunc i64 %11 to i32
  %17 = getelementptr inbounds i8, ptr %0, i32 %16
  br i1 %15, label %18, label %21

18:                                               ; preds = %13
  store ptr %17, ptr %4, align 4, !tbaa !13
  %19 = getelementptr inbounds %struct.FaString, ptr %4, i32 0, i32 1
  store i32 1, ptr %19, align 4, !tbaa !18
  %20 = getelementptr inbounds %struct.FaString, ptr %4, i32 0, i32 2
  store i32 -2147483648, ptr %20, align 4, !tbaa !19
  br label %29

21:                                               ; preds = %13
  %22 = load i8, ptr %17, align 1, !tbaa !1
  %23 = zext i8 %22 to i32
  %24 = getelementptr inbounds i8, ptr @ascii_table, i32 %23
  store ptr %24, ptr %4, align 4, !tbaa.struct !10
  %25 = getelementptr inbounds i8, ptr %4, i32 4
  store i32 1, ptr %25, align 4, !tbaa.struct !11
  %26 = getelementptr inbounds i8, ptr %4, i32 8
  store i32 -2147483648, ptr %26, align 4, !tbaa !19
  br label %29

27:                                               ; preds = %5
  %28 = tail call fastcc i32 @string_get_slow(ptr noundef %0, i32 noundef %1, i32 noundef %2, i64 noundef %3, ptr noundef %4) #61
  br label %29

29:                                               ; preds = %7, %27, %21, %18
  %30 = phi i32 [ 1, %18 ], [ 1, %21 ], [ %28, %27 ], [ 0, %7 ]
  ret i32 %30
}

; Function Attrs: noinline nounwind optsize
define internal fastcc noundef i32 @string_get_slow(ptr noundef %0, i32 noundef %1, i32 noundef %2, i64 noundef %3, ptr nocapture noundef writeonly %4) unnamed_addr #15 {
  %6 = alloca %struct.FaString, align 4
  %7 = icmp slt i64 %3, 0
  br i1 %7, label %8, label %13

8:                                                ; preds = %5
  %9 = tail call i32 @fa_graphemes(ptr noundef %0, i32 noundef %1) #61
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
  %22 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %21) #61
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
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %6) #64
  %35 = getelementptr inbounds i8, ptr %0, i32 %21
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %6, ptr noundef %35, i32 noundef %27, i32 noundef 0) #61
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %4, ptr noundef nonnull align 4 dereferenceable(12) %6, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %6) #64
  br label %36

36:                                               ; preds = %16, %30, %34, %13, %8
  %37 = phi i32 [ 0, %8 ], [ 1, %34 ], [ 1, %30 ], [ 0, %13 ], [ 0, %16 ]
  ret i32 %37
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define hidden i32 @fa_string_length(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2) local_unnamed_addr #19 {
  %4 = icmp sgt i32 %2, -1
  br i1 %4, label %5, label %7

5:                                                ; preds = %3
  %6 = tail call i32 @fa_graphemes(ptr noundef %0, i32 noundef %1) #61
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
  %12 = tail call i32 @fa_graphemes(ptr noundef %0, i32 noundef %1) #61
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
  %25 = tail call ptr @fa_error_index(i64 noundef %3, i64 noundef %15) #61
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
  %52 = tail call ptr @fa_error_index(i64 noundef %4, i64 noundef %15) #61
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
  %66 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %65) #61
  %67 = add nuw i32 %64, 1
  %68 = icmp eq i32 %67, %56
  br i1 %68, label %60, label %63, !llvm.loop !91

69:                                               ; preds = %60, %69
  %70 = phi i32 [ %73, %69 ], [ %56, %60 ]
  %71 = phi i32 [ %72, %69 ], [ %61, %60 ]
  %72 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %71) #61
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
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %8) #64
  %88 = getelementptr inbounds i8, ptr %0, i32 %76
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %8, ptr noundef %88, i32 noundef %80, i32 noundef 0) #61
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %6, ptr noundef nonnull align 4 dereferenceable(12) %8, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %8) #64
  br label %89

89:                                               ; preds = %51, %83, %87, %24
  %90 = phi ptr [ %25, %24 ], [ %52, %51 ], [ null, %87 ], [ null, %83 ]
  ret ptr %90
}

; Function Attrs: cold noinline nounwind optsize
define hidden noundef ptr @fa_error_index(i64 noundef %0, i64 noundef %1) local_unnamed_addr #22 {
  %3 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #62
  store i64 1, ptr %3, align 8, !tbaa !93
  %4 = getelementptr inbounds %struct.IndexError, ptr %3, i32 0, i32 1
  store i64 %0, ptr %4, align 8, !tbaa !95
  %5 = getelementptr inbounds %struct.IndexError, ptr %3, i32 0, i32 2
  store i64 %1, ptr %5, align 8, !tbaa !96
  ret ptr %3
}

; Function Attrs: nounwind optsize
define hidden void @fa_string_case(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef writeonly %3) local_unnamed_addr #3 {
  %5 = alloca i32, align 4
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  store i32 0, ptr @builder_simple, align 4, !tbaa !4
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %5) #64
  store i32 0, ptr %5, align 4, !tbaa !4
  %6 = icmp eq i32 %1, 0
  br i1 %6, label %144, label %7

7:                                                ; preds = %4
  %8 = icmp eq i32 %2, 0
  br label %9

9:                                                ; preds = %7, %141
  %10 = call fastcc i32 @decode(ptr noundef %0, i32 noundef %1, ptr noundef nonnull %5) #61
  br i1 %8, label %38, label %11

11:                                               ; preds = %9
  %12 = add i32 %10, -97
  %13 = icmp ult i32 %12, 26
  br i1 %13, label %27, label %14

14:                                               ; preds = %11
  %15 = add i32 %10, -224
  %16 = icmp ult i32 %15, 31
  %17 = icmp ne i32 %10, 247
  %18 = and i1 %17, %16
  br i1 %18, label %27, label %19

19:                                               ; preds = %14
  %20 = add i32 %10, -945
  %21 = icmp ult i32 %20, 25
  %22 = icmp ne i32 %10, 962
  %23 = and i1 %22, %21
  %24 = add i32 %10, -1072
  %25 = icmp ult i32 %24, 32
  %26 = or i1 %25, %23
  br i1 %26, label %27, label %29

27:                                               ; preds = %19, %14, %11
  %28 = add nsw i32 %10, -32
  br label %63

29:                                               ; preds = %19
  %30 = and i32 %10, -16
  %31 = icmp eq i32 %30, 1104
  br i1 %31, label %32, label %34

32:                                               ; preds = %29
  %33 = add nsw i32 %10, -80
  br label %73

34:                                               ; preds = %29
  switch i32 %10, label %63 [
    i32 255, label %73
    i32 181, label %35
    i32 962, label %36
    i32 223, label %37
  ]

35:                                               ; preds = %34
  br label %73

36:                                               ; preds = %34
  br label %73

37:                                               ; preds = %34
  tail call fastcc void @append_c(ptr noundef nonnull @.str.12) #61
  br label %141, !llvm.loop !97

38:                                               ; preds = %9
  %39 = add i32 %10, -65
  %40 = icmp ult i32 %39, 26
  br i1 %40, label %54, label %41

41:                                               ; preds = %38
  %42 = add i32 %10, -192
  %43 = icmp ult i32 %42, 31
  %44 = icmp ne i32 %10, 215
  %45 = and i1 %44, %43
  br i1 %45, label %54, label %46

46:                                               ; preds = %41
  %47 = add i32 %10, -913
  %48 = icmp ult i32 %47, 25
  %49 = icmp ne i32 %10, 930
  %50 = and i1 %49, %48
  %51 = add i32 %10, -1040
  %52 = icmp ult i32 %51, 32
  %53 = or i1 %52, %50
  br i1 %53, label %54, label %56

54:                                               ; preds = %46, %41, %38
  %55 = add nuw nsw i32 %10, 32
  br label %63

56:                                               ; preds = %46
  %57 = and i32 %10, -16
  %58 = icmp eq i32 %57, 1024
  br i1 %58, label %59, label %61

59:                                               ; preds = %56
  %60 = or disjoint i32 %10, 80
  br label %73

61:                                               ; preds = %56
  %62 = icmp eq i32 %10, 376
  br i1 %62, label %73, label %63

63:                                               ; preds = %34, %61, %54, %27
  %64 = phi i32 [ %28, %27 ], [ %55, %54 ], [ %10, %61 ], [ %10, %34 ]
  %65 = icmp ult i32 %64, 128
  br i1 %65, label %66, label %71

66:                                               ; preds = %63
  %67 = trunc i32 %64 to i8
  tail call fastcc void @reserve(i32 noundef 1) #61
  %68 = load ptr, ptr @builder, align 4, !tbaa !6
  %69 = load i32, ptr @builder_length, align 4, !tbaa !4
  %70 = getelementptr inbounds i8, ptr %68, i32 %69
  store i8 %67, ptr %70, align 1, !tbaa !1
  br label %138

71:                                               ; preds = %63
  %72 = icmp ult i32 %64, 2048
  br i1 %72, label %73, label %88

73:                                               ; preds = %32, %35, %36, %59, %34, %61, %71
  %74 = phi i32 [ %64, %71 ], [ 376, %34 ], [ %60, %59 ], [ 931, %36 ], [ 924, %35 ], [ %33, %32 ], [ 255, %61 ]
  %75 = lshr i32 %74, 6
  %76 = trunc i32 %75 to i8
  %77 = or disjoint i8 %76, -64
  tail call fastcc void @reserve(i32 noundef 1) #61
  %78 = load ptr, ptr @builder, align 4, !tbaa !6
  %79 = load i32, ptr @builder_length, align 4, !tbaa !4
  %80 = getelementptr inbounds i8, ptr %78, i32 %79
  store i8 %77, ptr %80, align 1, !tbaa !1
  %81 = add i32 %79, 1
  store i32 %81, ptr @builder_length, align 4, !tbaa !4
  %82 = trunc i32 %74 to i8
  %83 = and i8 %82, 63
  %84 = or disjoint i8 %83, -128
  tail call fastcc void @reserve(i32 noundef 1) #61
  %85 = load ptr, ptr @builder, align 4, !tbaa !6
  %86 = load i32, ptr @builder_length, align 4, !tbaa !4
  %87 = getelementptr inbounds i8, ptr %85, i32 %86
  store i8 %84, ptr %87, align 1, !tbaa !1
  br label %138

88:                                               ; preds = %71
  %89 = icmp ult i32 %64, 65536
  tail call fastcc void @reserve(i32 noundef 1) #61
  %90 = load ptr, ptr @builder, align 4, !tbaa !6
  %91 = load i32, ptr @builder_length, align 4, !tbaa !4
  %92 = getelementptr inbounds i8, ptr %90, i32 %91
  %93 = add i32 %91, 1
  br i1 %89, label %94, label %112

94:                                               ; preds = %88
  %95 = lshr i32 %64, 12
  %96 = trunc i32 %95 to i8
  %97 = or disjoint i8 %96, -32
  store i8 %97, ptr %92, align 1, !tbaa !1
  store i32 %93, ptr @builder_length, align 4, !tbaa !4
  %98 = lshr i32 %64, 6
  %99 = trunc i32 %98 to i8
  %100 = and i8 %99, 63
  %101 = or disjoint i8 %100, -128
  tail call fastcc void @reserve(i32 noundef 1) #61
  %102 = load ptr, ptr @builder, align 4, !tbaa !6
  %103 = load i32, ptr @builder_length, align 4, !tbaa !4
  %104 = getelementptr inbounds i8, ptr %102, i32 %103
  store i8 %101, ptr %104, align 1, !tbaa !1
  %105 = add i32 %103, 1
  store i32 %105, ptr @builder_length, align 4, !tbaa !4
  %106 = trunc i32 %64 to i8
  %107 = and i8 %106, 63
  %108 = or disjoint i8 %107, -128
  tail call fastcc void @reserve(i32 noundef 1) #61
  %109 = load ptr, ptr @builder, align 4, !tbaa !6
  %110 = load i32, ptr @builder_length, align 4, !tbaa !4
  %111 = getelementptr inbounds i8, ptr %109, i32 %110
  store i8 %108, ptr %111, align 1, !tbaa !1
  br label %138

112:                                              ; preds = %88
  %113 = lshr i32 %64, 18
  %114 = trunc i32 %113 to i8
  %115 = or i8 %114, -16
  store i8 %115, ptr %92, align 1, !tbaa !1
  store i32 %93, ptr @builder_length, align 4, !tbaa !4
  %116 = lshr i32 %64, 12
  %117 = trunc i32 %116 to i8
  %118 = and i8 %117, 63
  %119 = or disjoint i8 %118, -128
  tail call fastcc void @reserve(i32 noundef 1) #61
  %120 = load ptr, ptr @builder, align 4, !tbaa !6
  %121 = load i32, ptr @builder_length, align 4, !tbaa !4
  %122 = getelementptr inbounds i8, ptr %120, i32 %121
  store i8 %119, ptr %122, align 1, !tbaa !1
  %123 = add i32 %121, 1
  store i32 %123, ptr @builder_length, align 4, !tbaa !4
  %124 = lshr i32 %64, 6
  %125 = trunc i32 %124 to i8
  %126 = and i8 %125, 63
  %127 = or disjoint i8 %126, -128
  tail call fastcc void @reserve(i32 noundef 1) #61
  %128 = load ptr, ptr @builder, align 4, !tbaa !6
  %129 = load i32, ptr @builder_length, align 4, !tbaa !4
  %130 = getelementptr inbounds i8, ptr %128, i32 %129
  store i8 %127, ptr %130, align 1, !tbaa !1
  %131 = add i32 %129, 1
  store i32 %131, ptr @builder_length, align 4, !tbaa !4
  %132 = trunc i32 %64 to i8
  %133 = and i8 %132, 63
  %134 = or disjoint i8 %133, -128
  tail call fastcc void @reserve(i32 noundef 1) #61
  %135 = load ptr, ptr @builder, align 4, !tbaa !6
  %136 = load i32, ptr @builder_length, align 4, !tbaa !4
  %137 = getelementptr inbounds i8, ptr %135, i32 %136
  store i8 %134, ptr %137, align 1, !tbaa !1
  br label %138

138:                                              ; preds = %66, %73, %94, %112
  %139 = phi i32 [ %86, %73 ], [ %136, %112 ], [ %110, %94 ], [ %69, %66 ]
  %140 = add i32 %139, 1
  store i32 %140, ptr @builder_length, align 4, !tbaa !4
  br label %141

141:                                              ; preds = %138, %37
  %142 = load i32, ptr %5, align 4, !tbaa !4
  %143 = icmp ult i32 %142, %1
  br i1 %143, label %9, label %144

144:                                              ; preds = %141, %4
  tail call void @fa_b_take(ptr noundef %3) #61
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %5) #64
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
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %5) #64
  store i32 %11, ptr %5, align 4, !tbaa !4
  %14 = call fastcc i32 @decode(ptr noundef %0, i32 noundef %1, ptr noundef nonnull %5) #61
  %15 = tail call fastcc i32 @is_space(i32 noundef %14) #61, !range !50
  %16 = icmp eq i32 %15, 0
  %17 = load i32, ptr %5, align 4
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %5) #64
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
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %6) #64
  store i32 %35, ptr %6, align 4, !tbaa !4
  %36 = call fastcc i32 @decode(ptr noundef %0, i32 noundef %23, ptr noundef nonnull %6) #61
  %37 = tail call fastcc i32 @is_space(i32 noundef %36) #61, !range !50
  %38 = icmp eq i32 %37, 0
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %6) #64
  br i1 %38, label %39, label %22

39:                                               ; preds = %34, %22, %18
  %40 = phi i32 [ %1, %18 ], [ %23, %22 ], [ %23, %34 ]
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %7) #64
  %41 = getelementptr inbounds i8, ptr %0, i32 %19
  %42 = sub i32 %40, %19
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %7, ptr noundef %41, i32 noundef %42, i32 noundef 0) #61
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %3, ptr noundef nonnull align 4 dereferenceable(12) %7, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %7) #64
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(none)
define internal fastcc noundef i32 @is_space(i32 noundef %0) unnamed_addr #23 {
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
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  store i32 0, ptr @builder_simple, align 4, !tbaa !4
  %5 = icmp sgt i64 %2, 0
  br i1 %5, label %7, label %6

6:                                                ; preds = %7, %4
  tail call void @fa_b_take(ptr noundef %3) #61
  ret void

7:                                                ; preds = %4, %7
  %8 = phi i64 [ %13, %7 ], [ 0, %4 ]
  tail call fastcc void @reserve(i32 noundef %1) #61
  %9 = load ptr, ptr @builder, align 4, !tbaa !6
  %10 = load i32, ptr @builder_length, align 4, !tbaa !4
  %11 = getelementptr inbounds i8, ptr %9, i32 %10
  tail call fastcc void @copy_bytes(ptr noundef %11, ptr noundef %0, i32 noundef %1) #61
  %12 = add i32 %10, %1
  store i32 %12, ptr @builder_length, align 4, !tbaa !4
  %13 = add nuw nsw i64 %8, 1
  %14 = icmp eq i64 %13, %2
  br i1 %14, label %6, label %7, !llvm.loop !99
}

; Function Attrs: nounwind optsize
define hidden void @fa_array_join(ptr noundef %0, ptr nocapture noundef readonly %1, i32 noundef %2, ptr nocapture noundef writeonly %3) local_unnamed_addr #3 {
  %5 = alloca %struct.FaString, align 4
  %6 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %7 = load i64, ptr %6, align 8, !tbaa !70
  %8 = icmp slt i64 %7, 0
  br i1 %8, label %9, label %12, !prof !35

9:                                                ; preds = %4
  %10 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %11 = load i64, ptr %10, align 8, !tbaa !76
  tail call fastcc void @materialize_characters(ptr noundef nonnull %0, i64 noundef %11) #61
  br label %12

12:                                               ; preds = %4, %9
  %13 = load ptr, ptr %0, align 8, !tbaa !66
  %14 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %15 = load i64, ptr %14, align 8, !tbaa !76
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
  tail call fastcc void @out_of_memory() #65
  unreachable

25:                                               ; preds = %12
  br i1 %18, label %26, label %32

26:                                               ; preds = %25
  %27 = add i32 %16, -1
  br label %28

28:                                               ; preds = %26, %20
  %29 = phi i32 [ %27, %26 ], [ %21, %20 ]
  %30 = mul i32 %29, %2
  %31 = tail call fastcc i32 @simple_flag(ptr noundef %1, i32 noundef %2) #61, !range !20
  br label %34

32:                                               ; preds = %25
  %33 = icmp eq i32 %16, 0
  br i1 %33, label %37, label %34

34:                                               ; preds = %28, %32
  %35 = phi i32 [ 0, %32 ], [ %30, %28 ]
  %36 = phi i32 [ -2147483648, %32 ], [ %31, %28 ]
  br label %40

37:                                               ; preds = %32
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %5) #64
  call void @llvm.memset.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %5, i8 0, i32 12, i1 false)
  br label %103

38:                                               ; preds = %59
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %5) #64
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
  tail call fastcc void @out_of_memory() #65
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
  %67 = tail call ptr @fa_alloc(i32 noundef %60, i32 noundef 1) #62
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
  call fastcc void @copy_bytes(ptr noundef %77, ptr noundef %1, i32 noundef %2) #61
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
  call fastcc void @copy_bytes(ptr noundef %96, ptr noundef %95, i32 noundef %94) #61
  %97 = add i32 %94, %80
  %98 = add nuw i32 %73, 1
  %99 = icmp eq i32 %98, %16
  br i1 %99, label %70, label %72, !llvm.loop !33

100:                                              ; preds = %70
  %101 = call fastcc i32 @simple_flag(ptr noundef %69, i32 noundef %60) #61, !range !20
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
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %5) #64
  ret void
}

; Function Attrs: alwaysinline nounwind optsize
define hidden void @fa_array_materialize(ptr noundef %0) local_unnamed_addr #13 {
  %2 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %3 = load i64, ptr %2, align 8, !tbaa !70
  %4 = icmp slt i64 %3, 0
  br i1 %4, label %5, label %8, !prof !35

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %7 = load i64, ptr %6, align 8, !tbaa !76
  tail call fastcc void @materialize_characters(ptr noundef nonnull %0, i64 noundef %7) #61
  br label %8

8:                                                ; preds = %5, %1
  ret void
}

; Function Attrs: nounwind optsize
define hidden i32 @fa_array_join_length(ptr noundef %0, ptr nocapture noundef readonly %1, i32 noundef %2) local_unnamed_addr #3 {
  %4 = alloca %struct.FaString, align 4
  %5 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %6 = load i64, ptr %5, align 8, !tbaa !70
  %7 = icmp slt i64 %6, 0
  br i1 %7, label %8, label %11, !prof !35

8:                                                ; preds = %3
  %9 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %10 = load i64, ptr %9, align 8, !tbaa !76
  tail call fastcc void @materialize_characters(ptr noundef nonnull %0, i64 noundef %10) #61
  br label %11

11:                                               ; preds = %3, %8
  %12 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %13 = load i64, ptr %12, align 8, !tbaa !76
  %14 = trunc i64 %13 to i32
  %15 = icmp ugt i32 %14, 1
  br i1 %15, label %16, label %29

16:                                               ; preds = %11
  %17 = tail call fastcc i32 @simple_flag(ptr noundef %1, i32 noundef %2) #61, !range !20
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
  tail call fastcc void @out_of_memory() #65
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
  %33 = load ptr, ptr %0, align 8, !tbaa !66
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
  tail call fastcc void @out_of_memory() #65
  unreachable

56:                                               ; preds = %52
  %57 = add i32 %48, %35
  %58 = add nuw i32 %36, 1
  %59 = icmp eq i32 %58, %14
  br i1 %59, label %82, label %34, !llvm.loop !100

60:                                               ; preds = %47, %16
  call void @fa_array_join(ptr noundef %0, ptr noundef %1, i32 noundef %2, ptr noundef nonnull %4) #61
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
  %75 = call i32 @fa_graphemes(ptr noundef %74, i32 noundef %70) #61
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
  tail call void @fa_free(ptr noundef %81, i32 noundef %78, i32 noundef 1) #62
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
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %9) #64
  %14 = and i32 %2, -2147483648
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %9, ptr noundef %0, i32 noundef %1, i32 noundef %14) #61
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %7, ptr noundef nonnull align 4 dereferenceable(12) %9, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %9) #64
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
  tail call fastcc void @string_pad_slow(ptr noundef %0, i32 noundef %1, i64 noundef %3, ptr noundef %4, i32 noundef %5, i32 noundef %6, ptr noundef %7) #61
  br label %57

25:                                               ; preds = %19
  %26 = zext i32 %1 to i64
  %27 = icmp ult i64 %26, %3
  br i1 %27, label %29, label %28

28:                                               ; preds = %25
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %10) #64
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %10, ptr noundef %0, i32 noundef %1, i32 noundef -2147483648) #61
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %7, ptr noundef nonnull align 4 dereferenceable(12) %10, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %10) #64
  br label %57

29:                                               ; preds = %25
  %30 = icmp ugt i64 %3, 1073741822
  br i1 %30, label %31, label %32

31:                                               ; preds = %29
  tail call fastcc void @out_of_memory() #65
  unreachable

32:                                               ; preds = %29
  %33 = trunc i64 %3 to i32
  %34 = sub i32 %33, %1
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %11) #64
  call void @llvm.memset.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %11, i8 0, i32 12, i1 false)
  %35 = icmp ult i32 %33, 12
  br i1 %35, label %38, label %36

36:                                               ; preds = %32
  %37 = tail call ptr @fa_alloc(i32 noundef %33, i32 noundef 1) #62
  br label %38

38:                                               ; preds = %32, %36
  %39 = phi ptr [ %37, %36 ], [ %11, %32 ]
  %40 = icmp eq i32 %6, 0
  br i1 %40, label %44, label %41

41:                                               ; preds = %38
  %42 = load i8, ptr %4, align 1, !tbaa !1
  call fastcc void @fill_bytes(ptr noundef %39, i8 noundef zeroext %42, i32 noundef %34) #61
  %43 = getelementptr inbounds i8, ptr %39, i32 %34
  call fastcc void @copy_bytes(ptr noundef %43, ptr noundef %0, i32 noundef %1) #61
  br label %47

44:                                               ; preds = %38
  call fastcc void @copy_bytes(ptr noundef %39, ptr noundef %0, i32 noundef %1) #61
  %45 = getelementptr inbounds i8, ptr %39, i32 %1
  %46 = load i8, ptr %4, align 1, !tbaa !1
  call fastcc void @fill_bytes(ptr noundef %45, i8 noundef zeroext %46, i32 noundef %34) #61
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
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %11) #64
  br label %57

57:                                               ; preds = %56, %28, %24, %13
  ret void
}

; Function Attrs: noinline nounwind optsize
define internal fastcc void @string_pad_slow(ptr nocapture noundef readonly %0, i32 noundef %1, i64 noundef %2, ptr nocapture noundef readonly %3, i32 noundef %4, i32 noundef %5, ptr nocapture noundef writeonly %6) unnamed_addr #15 {
  %8 = tail call i32 @fa_graphemes(ptr noundef %0, i32 noundef %1) #61
  %9 = zext i32 %8 to i64
  %10 = sub nsw i64 %2, %9
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  store i32 0, ptr @builder_simple, align 4, !tbaa !4
  %11 = icmp eq i32 %5, 0
  br i1 %11, label %12, label %17

12:                                               ; preds = %7
  tail call fastcc void @reserve(i32 noundef %1) #61
  %13 = load ptr, ptr @builder, align 4, !tbaa !6
  %14 = load i32, ptr @builder_length, align 4, !tbaa !4
  %15 = getelementptr inbounds i8, ptr %13, i32 %14
  tail call fastcc void @copy_bytes(ptr noundef %15, ptr noundef %0, i32 noundef %1) #61
  %16 = add i32 %14, %1
  store i32 %16, ptr @builder_length, align 4, !tbaa !4
  br label %17

17:                                               ; preds = %12, %7
  %18 = icmp ne i32 %4, 0
  %19 = icmp sgt i64 %10, 0
  %20 = and i1 %18, %19
  br i1 %20, label %22, label %21

21:                                               ; preds = %22, %17
  br i1 %11, label %41, label %36

22:                                               ; preds = %17, %22
  %23 = phi i32 [ %33, %22 ], [ 0, %17 ]
  %24 = phi i64 [ %34, %22 ], [ 0, %17 ]
  %25 = tail call fastcc i32 @next_boundary(ptr noundef %3, i32 noundef %4, i32 noundef %23) #61
  %26 = getelementptr inbounds i8, ptr %3, i32 %23
  %27 = sub i32 %25, %23
  tail call fastcc void @reserve(i32 noundef %27) #61
  %28 = load ptr, ptr @builder, align 4, !tbaa !6
  %29 = load i32, ptr @builder_length, align 4, !tbaa !4
  %30 = getelementptr inbounds i8, ptr %28, i32 %29
  tail call fastcc void @copy_bytes(ptr noundef %30, ptr noundef %26, i32 noundef %27) #61
  %31 = add i32 %29, %27
  store i32 %31, ptr @builder_length, align 4, !tbaa !4
  %32 = icmp eq i32 %25, %4
  %33 = select i1 %32, i32 0, i32 %25
  %34 = add nuw nsw i64 %24, 1
  %35 = icmp slt i64 %34, %10
  br i1 %35, label %22, label %21, !llvm.loop !101

36:                                               ; preds = %21
  tail call fastcc void @reserve(i32 noundef %1) #61
  %37 = load ptr, ptr @builder, align 4, !tbaa !6
  %38 = load i32, ptr @builder_length, align 4, !tbaa !4
  %39 = getelementptr inbounds i8, ptr %37, i32 %38
  tail call fastcc void @copy_bytes(ptr noundef %39, ptr noundef %0, i32 noundef %1) #61
  %40 = add i32 %38, %1
  store i32 %40, ptr @builder_length, align 4, !tbaa !4
  br label %41

41:                                               ; preds = %36, %21
  tail call void @fa_b_take(ptr noundef %6) #61
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: write)
define internal fastcc void @fill_bytes(ptr nocapture noundef writeonly %0, i8 noundef zeroext %1, i32 noundef %2) unnamed_addr #24 {
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
  %3 = tail call i32 @fa_host_arguments_size() #62
  %4 = tail call i32 @llvm.umax.i32(i32 %3, i32 1)
  %5 = tail call ptr @fa_alloc(i32 noundef %4, i32 noundef 1) #62
  store ptr %5, ptr @argument_bytes, align 4, !tbaa !6
  tail call void @fa_host_arguments_copy(ptr noundef %5) #62
  store i32 0, ptr @argument_count, align 4, !tbaa !4
  %6 = icmp eq i32 %3, 0
  br i1 %6, label %7, label %13

7:                                                ; preds = %2
  %8 = tail call ptr @fa_alloc(i32 noundef 12, i32 noundef 4) #62
  store ptr %8, ptr @argument_items, align 4, !tbaa !6
  br label %23

9:                                                ; preds = %13
  store i32 %20, ptr @argument_count, align 4, !tbaa !4
  %10 = mul i32 %20, 12
  %11 = add i32 %10, 12
  %12 = tail call ptr @fa_alloc(i32 noundef %11, i32 noundef 4) #62
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
  %38 = tail call fastcc i32 @simple_flag(ptr noundef %34, i32 noundef %36) #61, !range !20
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
declare i32 @fa_host_arguments_size() local_unnamed_addr #25

; Function Attrs: optsize
declare void @fa_host_arguments_copy(ptr noundef) local_unnamed_addr #26

; Function Attrs: noinline nounwind optsize
define internal fastcc void @materialize_characters(ptr noundef %0, i64 noundef %1) unnamed_addr #15 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %29, label %4

4:                                                ; preds = %2
  %5 = icmp ugt i64 %1, 357913941
  br i1 %5, label %6, label %7

6:                                                ; preds = %4
  tail call fastcc void @out_of_memory() #65
  unreachable

7:                                                ; preds = %4
  %8 = trunc i64 %1 to i32
  %9 = mul nuw i32 %8, 12
  %10 = tail call ptr @fa_alloc(i32 noundef %9, i32 noundef 16) #62
  %11 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %12 = load i64, ptr %11, align 8, !tbaa !76
  %13 = icmp sgt i64 %12, 0
  br i1 %13, label %15, label %14

14:                                               ; preds = %15, %7
  tail call fastcc void @free_array_items(ptr noundef nonnull %0, i64 noundef 1) #61
  store ptr %10, ptr %0, align 8, !tbaa !66
  br label %29

15:                                               ; preds = %7, %15
  %16 = phi i64 [ %26, %15 ], [ 0, %7 ]
  %17 = trunc i64 %16 to i32
  %18 = getelementptr inbounds %struct.FaString, ptr %10, i32 %17
  %19 = load ptr, ptr %0, align 8, !tbaa !66
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
  %27 = load i64, ptr %11, align 8, !tbaa !76
  %28 = icmp slt i64 %26, %27
  br i1 %28, label %15, label %14, !llvm.loop !104

29:                                               ; preds = %2, %14
  %30 = phi i64 [ %1, %14 ], [ 0, %2 ]
  %31 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  store i64 %30, ptr %31, align 8, !tbaa !70
  ret void
}

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define hidden noundef i32 @fa_string_array_get(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef writeonly %2) local_unnamed_addr #27 {
  %4 = icmp slt i64 %1, 0
  %5 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %6 = load i64, ptr %5, align 8, !tbaa !76
  %7 = select i1 %4, i64 %6, i64 0
  %8 = add nsw i64 %7, %1
  %9 = icmp ult i64 %8, %6
  br i1 %9, label %10, label %23

10:                                               ; preds = %3
  %11 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %12 = load i64, ptr %11, align 8, !tbaa !70
  %13 = icmp slt i64 %12, 0
  %14 = load ptr, ptr %0, align 8, !tbaa !66
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

; Function Attrs: nounwind optsize
define hidden ptr @fa_array_push_slot(ptr noundef %0, i64 noundef %1) local_unnamed_addr #3 {
  %3 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %4 = load i64, ptr %3, align 8, !tbaa !76
  %5 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %6 = load i64, ptr %5, align 8, !tbaa !70
  %7 = icmp slt i64 %4, %6
  br i1 %7, label %10, label %8, !prof !36

8:                                                ; preds = %2
  tail call fastcc void @array_grow(ptr noundef nonnull %0, i64 noundef %1) #61
  %9 = load i64, ptr %3, align 8, !tbaa !76
  br label %10

10:                                               ; preds = %8, %2
  %11 = phi i64 [ %9, %8 ], [ %4, %2 ]
  %12 = load ptr, ptr %0, align 8, !tbaa !66
  %13 = mul nsw i64 %11, %1
  %14 = trunc i64 %13 to i32
  %15 = getelementptr inbounds i8, ptr %12, i32 %14
  %16 = add nsw i64 %11, 1
  store i64 %16, ptr %3, align 8, !tbaa !76
  ret ptr %15
}

; Function Attrs: noinline nounwind optsize
define internal fastcc void @array_grow(ptr noundef %0, i64 noundef %1) unnamed_addr #15 {
  %3 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %4 = load i64, ptr %3, align 8, !tbaa !70
  %5 = icmp slt i64 %4, 0
  br i1 %5, label %6, label %15

6:                                                ; preds = %2
  %7 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %8 = load i64, ptr %7, align 8, !tbaa !76
  %9 = icmp sgt i64 %8, 4611686018427387903
  br i1 %9, label %10, label %11

10:                                               ; preds = %6
  tail call fastcc void @out_of_memory() #65
  unreachable

11:                                               ; preds = %6
  %12 = icmp slt i64 %8, 4
  %13 = shl nuw nsw i64 %8, 1
  %14 = select i1 %12, i64 4, i64 %13
  tail call fastcc void @materialize_characters(ptr noundef nonnull %0, i64 noundef %14) #61
  br label %27

15:                                               ; preds = %2
  %16 = icmp ult i64 %4, 4
  %17 = shl nuw nsw i64 %4, 1
  %18 = select i1 %16, i64 4, i64 %17
  %19 = mul nsw i64 %18, %1
  %20 = trunc i64 %19 to i32
  %21 = tail call ptr @fa_alloc(i32 noundef %20, i32 noundef 16) #62
  %22 = load ptr, ptr %0, align 8, !tbaa !66
  %23 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %24 = load i64, ptr %23, align 8, !tbaa !76
  %25 = mul nsw i64 %24, %1
  %26 = trunc i64 %25 to i32
  tail call fastcc void @copy_bytes(ptr noundef %21, ptr noundef %22, i32 noundef %26) #61
  tail call fastcc void @free_array_items(ptr noundef nonnull %0, i64 noundef %1) #61
  store ptr %21, ptr %0, align 8, !tbaa !66
  store i64 %18, ptr %3, align 8, !tbaa !70
  br label %27

27:                                               ; preds = %15, %11
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_array_append(ptr noundef %0, ptr noundef %1, i64 noundef %2) local_unnamed_addr #3 {
  %4 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %5 = load i64, ptr %4, align 8, !tbaa !70
  %6 = icmp slt i64 %5, 0
  br i1 %6, label %7, label %10, !prof !35

7:                                                ; preds = %3
  %8 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %9 = load i64, ptr %8, align 8, !tbaa !76
  tail call fastcc void @materialize_characters(ptr noundef nonnull %0, i64 noundef %9) #61
  br label %10

10:                                               ; preds = %3, %7
  %11 = getelementptr inbounds %struct.FaArrayHeader, ptr %1, i32 0, i32 2
  %12 = load i64, ptr %11, align 8, !tbaa !70
  %13 = icmp slt i64 %12, 0
  br i1 %13, label %14, label %17, !prof !35

14:                                               ; preds = %10
  %15 = getelementptr inbounds %struct.FaArrayHeader, ptr %1, i32 0, i32 1
  %16 = load i64, ptr %15, align 8, !tbaa !76
  tail call fastcc void @materialize_characters(ptr noundef nonnull %1, i64 noundef %16) #61
  br label %17

17:                                               ; preds = %10, %14
  %18 = getelementptr inbounds %struct.FaArrayHeader, ptr %1, i32 0, i32 1
  %19 = load i64, ptr %18, align 8, !tbaa !76
  %20 = icmp sgt i64 %19, 0
  br i1 %20, label %21, label %23

21:                                               ; preds = %17
  %22 = trunc i64 %2 to i32
  br label %24

23:                                               ; preds = %24, %17
  ret void

24:                                               ; preds = %21, %24
  %25 = phi i64 [ 0, %21 ], [ %31, %24 ]
  %26 = tail call ptr @fa_array_push_slot(ptr noundef %0, i64 noundef %2) #61
  %27 = load ptr, ptr %1, align 8, !tbaa !66
  %28 = mul nsw i64 %25, %2
  %29 = trunc i64 %28 to i32
  %30 = getelementptr inbounds i8, ptr %27, i32 %29
  tail call fastcc void @copy_bytes(ptr noundef %26, ptr noundef %30, i32 noundef %22) #61
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
  %8 = load i64, ptr %7, align 8, !tbaa !70
  %9 = icmp slt i64 %8, 0
  br i1 %9, label %10, label %13, !prof !35

10:                                               ; preds = %6
  %11 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %12 = load i64, ptr %11, align 8, !tbaa !76
  tail call fastcc void @materialize_characters(ptr noundef nonnull %0, i64 noundef %12) #61
  br label %13

13:                                               ; preds = %6, %10
  %14 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %15 = load i64, ptr %14, align 8, !tbaa !76
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
  %25 = tail call ptr @fa_error_index(i64 noundef %1, i64 noundef %15) #61
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
  %52 = tail call ptr @fa_error_index(i64 noundef %2, i64 noundef %15) #61
  br label %65

53:                                               ; preds = %45, %30
  %54 = phi i64 [ %15, %30 ], [ %50, %45 ]
  %55 = tail call i64 @llvm.smax.i64(i64 %54, i64 %31)
  %56 = sub nsw i64 %55, %31
  %57 = tail call ptr @fa_array_new(i64 noundef %56, i64 noundef %56, i64 noundef %4) #61
  %58 = load ptr, ptr %57, align 8, !tbaa !66
  %59 = load ptr, ptr %0, align 8, !tbaa !66
  %60 = mul nsw i64 %31, %4
  %61 = trunc i64 %60 to i32
  %62 = getelementptr inbounds i8, ptr %59, i32 %61
  %63 = mul nsw i64 %56, %4
  %64 = trunc i64 %63 to i32
  tail call fastcc void @copy_bytes(ptr noundef %58, ptr noundef %62, i32 noundef %64) #61
  store ptr %57, ptr %5, align 4, !tbaa !6
  br label %65

65:                                               ; preds = %51, %53, %24
  %66 = phi ptr [ %25, %24 ], [ null, %53 ], [ %52, %51 ]
  ret ptr %66
}

; Function Attrs: nounwind optsize
define hidden void @fa_array_reserve(ptr noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #3 {
  %4 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %5 = load i64, ptr %4, align 8, !tbaa !70
  %6 = icmp slt i64 %5, 0
  br i1 %6, label %7, label %12

7:                                                ; preds = %3
  %8 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %9 = load i64, ptr %8, align 8, !tbaa !76
  %10 = icmp slt i64 %9, %1
  br i1 %10, label %11, label %29

11:                                               ; preds = %7
  tail call fastcc void @materialize_characters(ptr noundef nonnull %0, i64 noundef %1) #61
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
  tail call fastcc void @out_of_memory() #65
  unreachable

20:                                               ; preds = %16, %14
  %21 = mul nsw i64 %2, %1
  %22 = trunc i64 %21 to i32
  %23 = tail call ptr @fa_alloc(i32 noundef %22, i32 noundef 16) #62
  %24 = load ptr, ptr %0, align 8, !tbaa !66
  %25 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %26 = load i64, ptr %25, align 8, !tbaa !76
  %27 = mul nsw i64 %26, %2
  %28 = trunc i64 %27 to i32
  tail call fastcc void @copy_bytes(ptr noundef %23, ptr noundef %24, i32 noundef %28) #61
  tail call fastcc void @free_array_items(ptr noundef nonnull %0, i64 noundef %2) #61
  store ptr %23, ptr %0, align 8, !tbaa !66
  store i64 %1, ptr %4, align 8, !tbaa !70
  br label %29

29:                                               ; preds = %12, %7, %11, %20
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @free_array_items(ptr noundef %0, i64 noundef %1) unnamed_addr #3 {
  %3 = getelementptr inbounds i8, ptr %0, i32 24
  %4 = load i64, ptr %3, align 1
  %5 = icmp ne i64 %4, 0
  %6 = load ptr, ptr %0, align 8, !tbaa !66
  %7 = getelementptr inbounds i8, ptr %0, i32 32
  %8 = icmp eq ptr %6, %7
  %9 = select i1 %5, i1 %8, i1 false
  br i1 %9, label %10, label %11

10:                                               ; preds = %2
  store i64 0, ptr %3, align 1
  br label %23

11:                                               ; preds = %2
  %12 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %13 = load i64, ptr %12, align 8, !tbaa !70
  %14 = icmp slt i64 %13, 0
  br i1 %14, label %15, label %18

15:                                               ; preds = %11
  %16 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %17 = load i64, ptr %16, align 8, !tbaa !76
  br label %20

18:                                               ; preds = %11
  %19 = mul nsw i64 %13, %1
  br label %20

20:                                               ; preds = %18, %15
  %21 = phi i64 [ %17, %15 ], [ %19, %18 ]
  %22 = trunc i64 %21 to i32
  tail call void @fa_free(ptr noundef %6, i32 noundef %22, i32 noundef 16) #62
  br label %23

23:                                               ; preds = %20, %10
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_array_assign(ptr noundef %0, ptr noundef %1, i64 noundef %2) local_unnamed_addr #3 {
  %4 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %5 = load i64, ptr %4, align 8, !tbaa !70
  %6 = icmp slt i64 %5, 0
  br i1 %6, label %7, label %10, !prof !35

7:                                                ; preds = %3
  %8 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %9 = load i64, ptr %8, align 8, !tbaa !76
  tail call fastcc void @materialize_characters(ptr noundef nonnull %0, i64 noundef %9) #61
  br label %10

10:                                               ; preds = %3, %7
  %11 = getelementptr inbounds %struct.FaArrayHeader, ptr %1, i32 0, i32 2
  %12 = load i64, ptr %11, align 8, !tbaa !70
  %13 = icmp slt i64 %12, 0
  br i1 %13, label %14, label %17, !prof !35

14:                                               ; preds = %10
  %15 = getelementptr inbounds %struct.FaArrayHeader, ptr %1, i32 0, i32 1
  %16 = load i64, ptr %15, align 8, !tbaa !76
  tail call fastcc void @materialize_characters(ptr noundef nonnull %1, i64 noundef %16) #61
  br label %17

17:                                               ; preds = %10, %14
  %18 = icmp eq ptr %0, %1
  br i1 %18, label %39, label %19

19:                                               ; preds = %17
  %20 = load i64, ptr %4, align 8, !tbaa !70
  %21 = getelementptr inbounds %struct.FaArrayHeader, ptr %1, i32 0, i32 1
  %22 = load i64, ptr %21, align 8, !tbaa !76
  %23 = icmp slt i64 %20, %22
  br i1 %23, label %26, label %24

24:                                               ; preds = %19
  %25 = load ptr, ptr %0, align 8, !tbaa !66
  br label %31

26:                                               ; preds = %19
  %27 = mul nsw i64 %22, %2
  %28 = trunc i64 %27 to i32
  %29 = tail call ptr @fa_alloc(i32 noundef %28, i32 noundef 16) #62
  tail call fastcc void @free_array_items(ptr noundef nonnull %0, i64 noundef %2) #61
  store ptr %29, ptr %0, align 8, !tbaa !66
  %30 = load i64, ptr %21, align 8, !tbaa !76
  store i64 %30, ptr %4, align 8, !tbaa !70
  br label %31

31:                                               ; preds = %24, %26
  %32 = phi i64 [ %22, %24 ], [ %30, %26 ]
  %33 = phi ptr [ %25, %24 ], [ %29, %26 ]
  %34 = load ptr, ptr %1, align 8, !tbaa !66
  %35 = mul nsw i64 %32, %2
  %36 = trunc i64 %35 to i32
  tail call fastcc void @copy_bytes(ptr noundef %33, ptr noundef %34, i32 noundef %36) #61
  %37 = load i64, ptr %21, align 8, !tbaa !76
  %38 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  store i64 %37, ptr %38, align 8, !tbaa !76
  br label %39

39:                                               ; preds = %17, %31
  ret void
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_array_copy(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #3 {
  %3 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %4 = load i64, ptr %3, align 8, !tbaa !70
  %5 = icmp slt i64 %4, 0
  %6 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %7 = load i64, ptr %6, align 8, !tbaa !76
  br i1 %5, label %8, label %15

8:                                                ; preds = %2
  %9 = tail call ptr @fa_array_new(i64 noundef %7, i64 noundef %7, i64 noundef 1) #61
  %10 = load ptr, ptr %9, align 8, !tbaa !66
  %11 = load ptr, ptr %0, align 8, !tbaa !66
  %12 = load i64, ptr %6, align 8, !tbaa !76
  %13 = trunc i64 %12 to i32
  tail call fastcc void @copy_bytes(ptr noundef %10, ptr noundef %11, i32 noundef %13) #61
  %14 = getelementptr inbounds %struct.FaArrayHeader, ptr %9, i32 0, i32 2
  store i64 -1, ptr %14, align 8, !tbaa !70
  br label %22

15:                                               ; preds = %2
  %16 = tail call ptr @fa_array_new(i64 noundef %7, i64 noundef %7, i64 noundef %1) #61
  %17 = load ptr, ptr %16, align 8, !tbaa !66
  %18 = load ptr, ptr %0, align 8, !tbaa !66
  %19 = load i64, ptr %6, align 8, !tbaa !76
  %20 = mul nsw i64 %19, %1
  %21 = trunc i64 %20 to i32
  tail call fastcc void @copy_bytes(ptr noundef %17, ptr noundef %18, i32 noundef %21) #61
  br label %22

22:                                               ; preds = %15, %8
  %23 = phi ptr [ %9, %8 ], [ %16, %15 ]
  ret ptr %23
}

; Function Attrs: nounwind optsize
define hidden void @fa_array_reverse(ptr noundef %0, i64 noundef %1) local_unnamed_addr #3 {
  %3 = alloca [64 x i8], align 16
  %4 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %5 = load i64, ptr %4, align 8, !tbaa !70
  %6 = icmp slt i64 %5, 0
  br i1 %6, label %7, label %10, !prof !35

7:                                                ; preds = %2
  %8 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %9 = load i64, ptr %8, align 8, !tbaa !76
  tail call fastcc void @materialize_characters(ptr noundef nonnull %0, i64 noundef %9) #61
  br label %10

10:                                               ; preds = %2, %7
  %11 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %12 = load i64, ptr %11, align 8, !tbaa !76
  %13 = icmp slt i64 %12, 2
  br i1 %13, label %43, label %14

14:                                               ; preds = %10
  call void @llvm.lifetime.start.p0(i64 64, ptr nonnull %3) #64
  %15 = add nuw i64 %12, 4294967295
  %16 = mul i64 %15, %1
  %17 = trunc i64 %16 to i32
  %18 = icmp sgt i32 %17, 0
  br i1 %18, label %19, label %42

19:                                               ; preds = %14
  %20 = load ptr, ptr %0, align 8, !tbaa !66
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
  call fastcc void @copy_bytes(ptr noundef nonnull %3, ptr noundef %38, i32 noundef %36) #61
  %39 = getelementptr inbounds i8, ptr %27, i32 %37
  tail call fastcc void @copy_bytes(ptr noundef %38, ptr noundef nonnull %39, i32 noundef %36) #61
  call fastcc void @copy_bytes(ptr noundef nonnull %39, ptr noundef nonnull %3, i32 noundef %36) #61
  %40 = add nuw nsw i64 %33, 64
  %41 = icmp slt i64 %40, %1
  br i1 %41, label %32, label %28, !llvm.loop !108

42:                                               ; preds = %28, %14
  call void @llvm.lifetime.end.p0(i64 64, ptr nonnull %3) #64
  br label %43

43:                                               ; preds = %10, %42
  ret void
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_array_insert_slot(ptr noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #3 {
  %4 = tail call ptr @fa_array_push_slot(ptr noundef %0, i64 noundef %2) #61
  %5 = load ptr, ptr %0, align 8, !tbaa !66
  %6 = mul nsw i64 %2, %1
  %7 = trunc i64 %6 to i32
  %8 = getelementptr inbounds i8, ptr %5, i32 %7
  %9 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %10 = load i64, ptr %9, align 8, !tbaa !76
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
  %5 = load i64, ptr %4, align 8, !tbaa !70
  %6 = icmp slt i64 %5, 0
  br i1 %6, label %7, label %10, !prof !35

7:                                                ; preds = %3
  %8 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %9 = load i64, ptr %8, align 8, !tbaa !76
  tail call fastcc void @materialize_characters(ptr noundef nonnull %0, i64 noundef %9) #61
  br label %10

10:                                               ; preds = %3, %7
  %11 = load ptr, ptr %0, align 8, !tbaa !66
  %12 = mul nsw i64 %2, %1
  %13 = trunc i64 %12 to i32
  %14 = getelementptr inbounds i8, ptr %11, i32 %13
  %15 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %16 = load i64, ptr %15, align 8, !tbaa !76
  %17 = xor i64 %1, -1
  %18 = add i64 %16, %17
  %19 = mul nsw i64 %18, %2
  %20 = trunc i64 %19 to i32
  %21 = trunc i64 %2 to i32
  %22 = getelementptr inbounds i8, ptr %14, i32 %21
  tail call void @llvm.memmove.p0.p0.i32(ptr align 1 %14, ptr align 1 %22, i32 %20, i1 false)
  %23 = load i64, ptr %15, align 8, !tbaa !76
  %24 = add nsw i64 %23, -1
  store i64 %24, ptr %15, align 8, !tbaa !76
  ret void
}

; Function Attrs: alwaysinline nounwind optsize
define hidden void @fa_array_sort(ptr noundef %0, i64 noundef %1, ptr nocapture noundef readonly %2, ptr noundef %3) local_unnamed_addr #13 {
  %5 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %6 = load i64, ptr %5, align 8, !tbaa !70
  %7 = icmp slt i64 %6, 0
  br i1 %7, label %8, label %11, !prof !35

8:                                                ; preds = %4
  %9 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %10 = load i64, ptr %9, align 8, !tbaa !76
  tail call fastcc void @materialize_characters(ptr noundef nonnull %0, i64 noundef %10) #61
  br label %11

11:                                               ; preds = %4, %8
  %12 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %13 = load i64, ptr %12, align 8, !tbaa !76
  %14 = icmp slt i64 %13, 2
  br i1 %14, label %93, label %15

15:                                               ; preds = %11
  %16 = mul nsw i64 %13, %1
  %17 = trunc i64 %16 to i32
  %18 = load ptr, ptr %0, align 8, !tbaa !66
  %19 = tail call ptr @fa_alloc(i32 noundef %17, i32 noundef 16) #62
  %20 = trunc i64 %1 to i32
  br label %21

21:                                               ; preds = %29, %15
  %22 = phi ptr [ %18, %15 ], [ %23, %29 ]
  %23 = phi ptr [ %19, %15 ], [ %22, %29 ]
  %24 = phi i64 [ 1, %15 ], [ %25, %29 ]
  %25 = shl nsw i64 %24, 1
  br label %31

26:                                               ; preds = %29
  %27 = load ptr, ptr %0, align 8, !tbaa !66
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
  %50 = tail call i32 %2(ptr noundef %3, ptr noundef %46, ptr noundef %49) #62
  %51 = icmp slt i32 %50, 0
  %52 = mul nsw i64 %41, %1
  %53 = trunc i64 %52 to i32
  %54 = getelementptr inbounds i8, ptr %23, i32 %53
  br i1 %51, label %55, label %57

55:                                               ; preds = %40
  tail call fastcc void @copy_bytes(ptr noundef %54, ptr noundef %46, i32 noundef %20) #61
  %56 = add nsw i64 %42, 1
  br label %59

57:                                               ; preds = %40
  tail call fastcc void @copy_bytes(ptr noundef %54, ptr noundef %49, i32 noundef %20) #61
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
  tail call fastcc void @copy_bytes(ptr noundef %72, ptr noundef %75, i32 noundef %78) #61
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
  tail call fastcc void @copy_bytes(ptr noundef %82, ptr noundef %85, i32 noundef %88) #61
  %89 = icmp slt i64 %35, %13
  br i1 %89, label %31, label %29, !llvm.loop !111

90:                                               ; preds = %26
  tail call fastcc void @copy_bytes(ptr noundef %27, ptr noundef %23, i32 noundef %17) #61
  br label %91

91:                                               ; preds = %26, %90
  %92 = phi ptr [ %23, %90 ], [ %22, %26 ]
  tail call void @fa_free(ptr noundef %92, i32 noundef %17, i32 noundef 16) #62
  br label %93

93:                                               ; preds = %91, %11
  ret void
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_arguments() local_unnamed_addr #3 {
  %1 = load i32, ptr @argument_count, align 4, !tbaa !4
  %2 = zext i32 %1 to i64
  %3 = tail call ptr @fa_array_new(i64 noundef %2, i64 noundef %2, i64 noundef 12) #61
  %4 = load ptr, ptr %3, align 8, !tbaa !66
  %5 = load ptr, ptr @argument_items, align 4, !tbaa !6
  %6 = mul i32 %1, 12
  tail call fastcc void @copy_bytes(ptr noundef %4, ptr noundef %5, i32 noundef %6) #61
  %7 = getelementptr inbounds %struct.FaArrayHeader, ptr %3, i32 0, i32 1
  store i64 %2, ptr %7, align 8, !tbaa !76
  %8 = getelementptr inbounds %struct.FaArrayHeader, ptr %3, i32 0, i32 2
  store i64 %2, ptr %8, align 8, !tbaa !70
  ret ptr %3
}

; Function Attrs: nounwind optsize
define hidden void @fa_array_free(ptr noundef %0, i64 noundef %1) local_unnamed_addr #3 {
  %3 = getelementptr inbounds i8, ptr %0, i32 24
  %4 = load i64, ptr %3, align 1
  %5 = icmp ne i64 %4, 0
  %6 = load ptr, ptr %0, align 8, !tbaa !66
  %7 = getelementptr inbounds i8, ptr %0, i32 32
  %8 = icmp eq ptr %6, %7
  %9 = select i1 %5, i1 %8, i1 false
  br i1 %9, label %22, label %10

10:                                               ; preds = %2
  %11 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %12 = load i64, ptr %11, align 8, !tbaa !70
  %13 = icmp slt i64 %12, 0
  br i1 %13, label %14, label %17

14:                                               ; preds = %10
  %15 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %16 = load i64, ptr %15, align 8, !tbaa !76
  br label %19

17:                                               ; preds = %10
  %18 = mul nsw i64 %12, %1
  br label %19

19:                                               ; preds = %17, %14
  %20 = phi i64 [ %16, %14 ], [ %18, %17 ]
  %21 = trunc i64 %20 to i32
  tail call void @fa_free(ptr noundef %6, i32 noundef %21, i32 noundef 16) #62
  br label %22

22:                                               ; preds = %2, %19
  tail call void @fa_free(ptr noundef nonnull %0, i32 noundef 32, i32 noundef 8) #62
  ret void
}

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read)
define hidden ptr @fa_segmented_at(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2, i32 noundef %3) local_unnamed_addr #29 {
  %5 = zext nneg i32 %3 to i64
  %6 = lshr i64 %1, %5
  %7 = icmp eq i64 %6, 0
  br i1 %7, label %16, label %8

8:                                                ; preds = %4
  %9 = tail call i64 @llvm.ctlz.i64(i64 %6, i1 true), !range !112
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
  %19 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 2, i32 %18
  %20 = load ptr, ptr %19, align 4, !tbaa !6
  %21 = add i64 %17, %1
  %22 = mul nsw i64 %21, %2
  %23 = trunc i64 %22 to i32
  %24 = getelementptr inbounds i8, ptr %20, i32 %23
  ret ptr %24
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_segmented_new() local_unnamed_addr #3 {
  %1 = tail call ptr @fa_alloc(i32 noundef 208, i32 noundef 8) #62
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(208) %1, i8 0, i32 208, i1 false)
  ret ptr %1
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_segmented_push_slot(ptr nocapture noundef %0, i64 noundef %1, i32 noundef %2) local_unnamed_addr #3 {
  %4 = load i64, ptr %0, align 8, !tbaa !113
  %5 = zext nneg i32 %2 to i64
  %6 = lshr i64 %4, %5
  %7 = icmp eq i64 %6, 0
  br i1 %7, label %15, label %8

8:                                                ; preds = %3
  %9 = tail call i64 @llvm.ctlz.i64(i64 %6, i1 true), !range !112
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
  %19 = load i64, ptr %18, align 8, !tbaa !115
  %20 = icmp slt i64 %17, %19
  br i1 %20, label %21, label %23

21:                                               ; preds = %15
  %22 = trunc i64 %17 to i32
  br label %33

23:                                               ; preds = %15
  %24 = sext i32 %2 to i64
  %25 = tail call i64 @llvm.usub.sat.i64(i64 %17, i64 1)
  %26 = add nsw i64 %25, %24
  %27 = shl i64 %1, %26
  %28 = trunc i64 %27 to i32
  %29 = tail call ptr @fa_alloc(i32 noundef %28, i32 noundef 16) #62
  %30 = trunc i64 %17 to i32
  %31 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 2, i32 %30
  store ptr %29, ptr %31, align 4, !tbaa !6
  %32 = add nuw nsw i64 %17, 1
  store i64 %32, ptr %18, align 8, !tbaa !115
  br label %33

33:                                               ; preds = %21, %23
  %34 = phi i32 [ %22, %21 ], [ %30, %23 ]
  %35 = add nsw i64 %4, 1
  store i64 %35, ptr %0, align 8, !tbaa !113
  %36 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 2, i32 %34
  %37 = load ptr, ptr %36, align 4, !tbaa !6
  %38 = add i64 %16, %4
  %39 = mul nsw i64 %38, %1
  %40 = trunc i64 %39 to i32
  %41 = getelementptr inbounds i8, ptr %37, i32 %40
  ret ptr %41
}

; Function Attrs: nounwind optsize
define hidden void @fa_segmented_free(ptr noundef %0, i64 noundef %1, i32 noundef %2) local_unnamed_addr #3 {
  %4 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 1
  %5 = load i64, ptr %4, align 8, !tbaa !115
  %6 = icmp sgt i64 %5, 0
  br i1 %6, label %7, label %9

7:                                                ; preds = %3
  %8 = sext i32 %2 to i64
  br label %10

9:                                                ; preds = %10, %3
  tail call void @fa_free(ptr noundef nonnull %0, i32 noundef 208, i32 noundef 8) #62
  ret void

10:                                               ; preds = %7, %10
  %11 = phi i64 [ 0, %7 ], [ %19, %10 ]
  %12 = trunc i64 %11 to i32
  %13 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 2, i32 %12
  %14 = load ptr, ptr %13, align 4, !tbaa !6
  %15 = tail call i64 @llvm.usub.sat.i64(i64 %11, i64 1)
  %16 = add i64 %15, %8
  %17 = shl i64 %1, %16
  %18 = trunc i64 %17 to i32
  tail call void @fa_free(ptr noundef %14, i32 noundef %18, i32 noundef 16) #62
  %19 = add nuw nsw i64 %11, 1
  %20 = load i64, ptr %4, align 8, !tbaa !115
  %21 = icmp slt i64 %19, %20
  br i1 %21, label %10, label %9, !llvm.loop !116
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_segmented_flatten(ptr nocapture noundef readonly %0, i64 noundef %1, i32 noundef %2) local_unnamed_addr #3 {
  %4 = load i64, ptr %0, align 8, !tbaa !113
  %5 = tail call ptr @fa_array_new(i64 noundef %4, i64 noundef %4, i64 noundef %1) #61
  %6 = load i64, ptr %0, align 8, !tbaa !113
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
  %20 = load ptr, ptr %5, align 8, !tbaa !66
  %21 = mul nsw i64 %14, %1
  %22 = trunc i64 %21 to i32
  %23 = getelementptr inbounds i8, ptr %20, i32 %22
  %24 = trunc i64 %13 to i32
  %25 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 2, i32 %24
  %26 = load ptr, ptr %25, align 4, !tbaa !6
  %27 = mul nsw i64 %19, %1
  %28 = trunc i64 %27 to i32
  tail call fastcc void @copy_bytes(ptr noundef %23, ptr noundef %26, i32 noundef %28) #61
  %29 = add nsw i64 %19, %14
  %30 = add nuw nsw i64 %13, 1
  %31 = load i64, ptr %0, align 8, !tbaa !113
  %32 = icmp slt i64 %29, %31
  br i1 %32, label %11, label %10, !llvm.loop !117
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_segmented_from_array(ptr noundef %0, i64 noundef %1, i32 noundef %2) local_unnamed_addr #3 {
  %4 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %5 = load i64, ptr %4, align 8, !tbaa !70
  %6 = icmp slt i64 %5, 0
  br i1 %6, label %7, label %10, !prof !35

7:                                                ; preds = %3
  %8 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %9 = load i64, ptr %8, align 8, !tbaa !76
  tail call fastcc void @materialize_characters(ptr noundef nonnull %0, i64 noundef %9) #61
  br label %10

10:                                               ; preds = %3, %7
  %11 = tail call noundef ptr @fa_alloc(i32 noundef 208, i32 noundef 8) #62
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(208) %11, i8 0, i32 208, i1 false)
  %12 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %13 = load i64, ptr %12, align 8, !tbaa !76
  %14 = icmp sgt i64 %13, 0
  br i1 %14, label %15, label %17

15:                                               ; preds = %10
  %16 = trunc i64 %1 to i32
  br label %18

17:                                               ; preds = %18, %10
  tail call void @fa_array_free(ptr noundef nonnull %0, i64 noundef %1) #61
  ret ptr %11

18:                                               ; preds = %15, %18
  %19 = phi i64 [ 0, %15 ], [ %25, %18 ]
  %20 = tail call ptr @fa_segmented_push_slot(ptr noundef %11, i64 noundef %1, i32 noundef %2) #61
  %21 = load ptr, ptr %0, align 8, !tbaa !66
  %22 = mul nsw i64 %19, %1
  %23 = trunc i64 %22 to i32
  %24 = getelementptr inbounds i8, ptr %21, i32 %23
  tail call fastcc void @copy_bytes(ptr noundef %20, ptr noundef %24, i32 noundef %16) #61
  %25 = add nuw nsw i64 %19, 1
  %26 = load i64, ptr %12, align 8, !tbaa !76
  %27 = icmp slt i64 %25, %26
  br i1 %27, label %18, label %17, !llvm.loop !118
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
  %20 = load ptr, ptr %17, align 4, !tbaa !119
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
  %37 = tail call fastcc ptr @heap_alloc_slow(i32 noundef %8) #61
  br label %38

38:                                               ; preds = %19, %34, %36
  %39 = phi ptr [ %37, %36 ], [ %17, %19 ], [ %23, %34 ]
  ret ptr %39
}

; Function Attrs: noinline nounwind optsize
define internal fastcc ptr @heap_alloc_slow(i32 noundef %0) unnamed_addr #15 {
  %2 = alloca i32, align 4
  %3 = icmp ugt i32 %0, 8192
  br i1 %3, label %4, label %14

4:                                                ; preds = %1
  %5 = add i32 %0, 65551
  %6 = lshr i32 %5, 16
  %7 = icmp ult i32 %5, 65536
  br i1 %7, label %48, label %8

8:                                                ; preds = %4
  %9 = tail call fastcc ptr @take_pages(i32 noundef %6) #61
  %10 = icmp eq ptr %9, null
  br i1 %10, label %48, label %11

11:                                               ; preds = %8
  store i32 65535, ptr %9, align 4, !tbaa !121
  %12 = getelementptr inbounds %struct.PageHeader, ptr %9, i32 0, i32 1
  store i32 %6, ptr %12, align 4, !tbaa !123
  %13 = getelementptr inbounds i8, ptr %9, i32 16
  br label %48

14:                                               ; preds = %1
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %2) #64
  %15 = call fastcc i32 @size_class(i32 noundef %0, ptr noundef nonnull %2) #61
  %16 = getelementptr inbounds [40 x ptr], ptr @free_blocks, i32 0, i32 %15
  %17 = load ptr, ptr %16, align 4, !tbaa !6
  %18 = icmp eq ptr %17, null
  br i1 %18, label %21, label %19

19:                                               ; preds = %14
  %20 = load ptr, ptr %17, align 4, !tbaa !119
  store ptr %20, ptr %16, align 4, !tbaa !6
  br label %46

21:                                               ; preds = %14
  %22 = getelementptr inbounds [40 x ptr], ptr @class_next, i32 0, i32 %15
  %23 = load ptr, ptr %22, align 4, !tbaa !6
  %24 = icmp eq ptr %23, null
  br i1 %24, label %33, label %25

25:                                               ; preds = %21
  %26 = getelementptr inbounds [40 x ptr], ptr @class_end, i32 0, i32 %15
  %27 = load ptr, ptr %26, align 4, !tbaa !6
  %28 = ptrtoint ptr %27 to i32
  %29 = ptrtoint ptr %23 to i32
  %30 = sub i32 %28, %29
  %31 = load i32, ptr %2, align 4, !tbaa !4
  %32 = icmp ult i32 %30, %31
  br i1 %32, label %33, label %42

33:                                               ; preds = %25, %21
  %34 = tail call fastcc ptr @take_pages(i32 noundef 1) #61
  %35 = icmp eq ptr %34, null
  br i1 %35, label %46, label %36

36:                                               ; preds = %33
  store i32 %15, ptr %34, align 4, !tbaa !121
  %37 = getelementptr inbounds %struct.PageHeader, ptr %34, i32 0, i32 1
  store i32 1, ptr %37, align 4, !tbaa !123
  %38 = getelementptr inbounds i8, ptr %34, i32 16
  %39 = getelementptr inbounds i8, ptr %34, i32 65536
  %40 = getelementptr inbounds [40 x ptr], ptr @class_end, i32 0, i32 %15
  store ptr %39, ptr %40, align 4, !tbaa !6
  %41 = load i32, ptr %2, align 4, !tbaa !4
  br label %42

42:                                               ; preds = %36, %25
  %43 = phi i32 [ %41, %36 ], [ %31, %25 ]
  %44 = phi ptr [ %38, %36 ], [ %23, %25 ]
  %45 = getelementptr inbounds i8, ptr %44, i32 %43
  store ptr %45, ptr %22, align 4, !tbaa !6
  br label %46

46:                                               ; preds = %33, %42, %19
  %47 = phi ptr [ %17, %19 ], [ %44, %42 ], [ null, %33 ]
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %2) #64
  br label %48

48:                                               ; preds = %4, %8, %11, %46
  %49 = phi ptr [ %47, %46 ], [ null, %4 ], [ %13, %11 ], [ null, %8 ]
  ret ptr %49
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden void @fa_heap_free(ptr noundef %0) local_unnamed_addr #20 {
  %2 = ptrtoint ptr %0 to i32
  %3 = and i32 %2, -65536
  %4 = inttoptr i32 %3 to ptr
  %5 = load i32, ptr %4, align 65536, !tbaa !121
  %6 = icmp eq i32 %5, 65535
  br i1 %6, label %7, label %10

7:                                                ; preds = %1
  %8 = getelementptr inbounds %struct.PageHeader, ptr %4, i32 0, i32 1
  %9 = load i32, ptr %8, align 4, !tbaa !123
  tail call fastcc void @give_pages(ptr noundef nonnull %4, i32 noundef %9) #61
  br label %13

10:                                               ; preds = %1
  %11 = getelementptr inbounds [40 x ptr], ptr @free_blocks, i32 0, i32 %5
  %12 = load ptr, ptr %11, align 4, !tbaa !6
  store ptr %12, ptr %0, align 4, !tbaa !119
  store ptr %0, ptr %11, align 4, !tbaa !6
  br label %13

13:                                               ; preds = %10, %7
  ret void
}

; Function Attrs: nofree noinline norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc void @give_pages(ptr noundef %0, i32 noundef %1) unnamed_addr #21 {
  %3 = getelementptr inbounds %struct.FreeSpan, ptr %0, i32 0, i32 1
  store i32 %1, ptr %3, align 4, !tbaa !124
  br label %4

4:                                                ; preds = %4, %2
  %5 = phi ptr [ null, %2 ], [ %7, %4 ]
  %6 = phi ptr [ @free_spans, %2 ], [ %7, %4 ]
  %7 = load ptr, ptr %6, align 4, !tbaa !6
  %8 = icmp ne ptr %7, null
  %9 = icmp ult ptr %7, %0
  %10 = and i1 %8, %9
  br i1 %10, label %4, label %11, !llvm.loop !126

11:                                               ; preds = %4
  store ptr %7, ptr %0, align 4, !tbaa !127
  br i1 %8, label %12, label %21

12:                                               ; preds = %11
  %13 = shl i32 %1, 16
  %14 = getelementptr inbounds i8, ptr %0, i32 %13
  %15 = icmp eq ptr %14, %7
  br i1 %15, label %16, label %21

16:                                               ; preds = %12
  %17 = getelementptr inbounds %struct.FreeSpan, ptr %7, i32 0, i32 1
  %18 = load i32, ptr %17, align 4, !tbaa !124
  %19 = add i32 %18, %1
  store i32 %19, ptr %3, align 4, !tbaa !124
  %20 = load ptr, ptr %7, align 4, !tbaa !127
  store ptr %20, ptr %0, align 4, !tbaa !127
  br label %21

21:                                               ; preds = %16, %12, %11
  %22 = phi ptr [ %20, %16 ], [ %7, %12 ], [ null, %11 ]
  %23 = phi i32 [ %19, %16 ], [ %1, %12 ], [ %1, %11 ]
  %24 = icmp eq ptr %5, null
  br i1 %24, label %34, label %25

25:                                               ; preds = %21
  %26 = getelementptr inbounds %struct.FreeSpan, ptr %5, i32 0, i32 1
  %27 = load i32, ptr %26, align 4, !tbaa !124
  %28 = shl i32 %27, 16
  %29 = getelementptr inbounds i8, ptr %5, i32 %28
  %30 = icmp eq ptr %29, %0
  br i1 %30, label %31, label %33

31:                                               ; preds = %25
  %32 = add i32 %27, %23
  store i32 %32, ptr %26, align 4, !tbaa !124
  store ptr %22, ptr %5, align 4, !tbaa !127
  br label %35

33:                                               ; preds = %25
  store ptr %0, ptr %5, align 4, !tbaa !127
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
  %10 = load ptr, ptr %9, align 65536, !tbaa !128
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
  %7 = load ptr, ptr %6, align 65536, !tbaa !128
  %8 = icmp eq ptr %7, null
  %9 = load ptr, ptr @tree_arena, align 4
  %10 = icmp eq ptr %7, %9
  %11 = select i1 %8, i1 true, i1 %10
  br i1 %11, label %13, label %12

12:                                               ; preds = %3
  tail call fastcc void @release_arena(ptr noundef nonnull %7) #61
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
  %12 = load ptr, ptr %11, align 4, !tbaa !130
  %13 = getelementptr inbounds %struct.FaArenaCleanup, ptr %10, i32 0, i32 2
  %14 = load ptr, ptr %13, align 4, !tbaa !132
  tail call void %12(ptr noundef %14) #62
  %15 = load ptr, ptr %10, align 4, !tbaa !6
  %16 = icmp eq ptr %15, null
  br i1 %16, label %6, label %9, !llvm.loop !133

17:                                               ; preds = %8, %17
  %18 = phi ptr [ %20, %17 ], [ %0, %8 ]
  %19 = getelementptr inbounds %struct.FaArena, ptr %18, i32 0, i32 1
  %20 = load ptr, ptr %19, align 4, !tbaa !134
  tail call fastcc void @give_pages(ptr noundef nonnull %18, i32 noundef 1) #61
  %21 = icmp eq ptr %20, null
  br i1 %21, label %22, label %17, !llvm.loop !135

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
  %14 = load ptr, ptr %13, align 65536, !tbaa !128
  br label %15

15:                                               ; preds = %8, %10
  %16 = phi ptr [ %14, %10 ], [ null, %8 ]
  %17 = icmp eq ptr %16, %6
  br i1 %17, label %19, label %18

18:                                               ; preds = %15
  tail call fastcc void @release_arena(ptr noundef nonnull %6) #61
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
  %10 = load ptr, ptr %9, align 4, !tbaa !136
  %11 = ptrtoint ptr %10 to i32
  %12 = sub i32 0, %11
  %13 = add i32 %1, -1
  %14 = and i32 %13, %12
  %15 = select i1 %8, i32 0, i32 %14
  %16 = getelementptr inbounds %struct.FaArena, ptr %5, i32 0, i32 3
  %17 = load ptr, ptr %16, align 4, !tbaa !137
  %18 = ptrtoint ptr %17 to i32
  %19 = sub i32 %18, %11
  %20 = icmp ule i32 %15, %19
  %21 = sub i32 %19, %15
  %22 = icmp ule i32 %4, %21
  %23 = and i1 %20, %22
  br i1 %23, label %24, label %28, !prof !36

24:                                               ; preds = %7
  %25 = getelementptr inbounds %struct.FaArena, ptr %5, i32 0, i32 2
  %26 = getelementptr inbounds i8, ptr %10, i32 %15
  %27 = getelementptr inbounds i8, ptr %26, i32 %4
  store ptr %27, ptr %25, align 4, !tbaa !136
  br label %30

28:                                               ; preds = %7, %2
  %29 = tail call fastcc ptr @arena_refill(i32 noundef %4, i32 noundef %1) #61
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
  %6 = tail call fastcc ptr @take_pages(i32 noundef 1) #61
  %7 = icmp eq ptr %6, null
  %8 = icmp ugt i32 %5, 65536
  %9 = or i1 %8, %7
  %10 = sub nuw nsw i32 65536, %5
  %11 = icmp ult i32 %10, %0
  %12 = select i1 %9, i1 true, i1 %11
  br i1 %12, label %13, label %14

13:                                               ; preds = %2
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @arena_refill.message, i32 noundef 20) #62
  tail call void @fa_exit(i32 noundef 101) #63
  unreachable

14:                                               ; preds = %2
  %15 = load ptr, ptr @tree_arena, align 4, !tbaa !6
  %16 = icmp eq ptr %15, null
  br i1 %16, label %17, label %21

17:                                               ; preds = %14
  %18 = getelementptr inbounds %struct.FaArena, ptr %6, i32 0, i32 1
  store ptr null, ptr %18, align 4, !tbaa !134
  %19 = getelementptr inbounds %struct.FaArena, ptr %6, i32 0, i32 4
  store ptr null, ptr %19, align 4, !tbaa !138
  store ptr %6, ptr @tree_arena, align 4, !tbaa !6
  %20 = atomicrmw add ptr @live_tree_arenas, i32 1 monotonic, align 4
  br label %25

21:                                               ; preds = %14
  %22 = getelementptr inbounds %struct.FaArena, ptr %15, i32 0, i32 1
  %23 = load ptr, ptr %22, align 4, !tbaa !134
  %24 = getelementptr inbounds %struct.FaArena, ptr %6, i32 0, i32 1
  store ptr %23, ptr %24, align 4, !tbaa !134
  store ptr %6, ptr %22, align 4, !tbaa !134
  br label %25

25:                                               ; preds = %21, %17
  %26 = phi ptr [ %6, %17 ], [ %15, %21 ]
  store ptr %26, ptr %6, align 4, !tbaa !128
  %27 = getelementptr inbounds i8, ptr %6, i32 %5
  %28 = getelementptr inbounds i8, ptr %27, i32 %0
  %29 = getelementptr inbounds %struct.FaArena, ptr %26, i32 0, i32 2
  store ptr %28, ptr %29, align 4, !tbaa !136
  %30 = getelementptr inbounds i8, ptr %6, i32 65536
  %31 = getelementptr inbounds %struct.FaArena, ptr %26, i32 0, i32 3
  store ptr %30, ptr %31, align 4, !tbaa !137
  ret ptr %27
}

; Function Attrs: cold noinline nounwind optsize
define hidden void @fa_arena_cleanup(ptr noundef %0, ptr noundef %1) local_unnamed_addr #22 {
  %3 = load ptr, ptr @tree_arena, align 4, !tbaa !6
  %4 = icmp eq ptr %3, null
  br i1 %4, label %16, label %5, !prof !35

5:                                                ; preds = %2
  %6 = getelementptr inbounds %struct.FaArena, ptr %3, i32 0, i32 3
  %7 = load ptr, ptr %6, align 4, !tbaa !137
  %8 = getelementptr inbounds %struct.FaArena, ptr %3, i32 0, i32 2
  %9 = load ptr, ptr %8, align 4, !tbaa !136
  %10 = ptrtoint ptr %7 to i32
  %11 = ptrtoint ptr %9 to i32
  %12 = sub i32 %10, %11
  %13 = icmp ugt i32 %12, 15
  br i1 %13, label %14, label %16, !prof !36

14:                                               ; preds = %5
  %15 = getelementptr inbounds i8, ptr %9, i32 16
  store ptr %15, ptr %8, align 4, !tbaa !136
  br label %19

16:                                               ; preds = %5, %2
  %17 = tail call fastcc ptr @arena_refill(i32 noundef 16, i32 noundef 4) #61
  %18 = load ptr, ptr @tree_arena, align 4, !tbaa !6
  br label %19

19:                                               ; preds = %14, %16
  %20 = phi ptr [ %18, %16 ], [ %3, %14 ]
  %21 = phi ptr [ %17, %16 ], [ %9, %14 ]
  %22 = getelementptr inbounds %struct.FaArena, ptr %20, i32 0, i32 4
  %23 = load ptr, ptr %22, align 4, !tbaa !138
  store ptr %23, ptr %21, align 4, !tbaa !139
  %24 = getelementptr inbounds %struct.FaArenaCleanup, ptr %21, i32 0, i32 1
  store ptr %1, ptr %24, align 4, !tbaa !130
  %25 = getelementptr inbounds %struct.FaArenaCleanup, ptr %21, i32 0, i32 2
  store ptr %0, ptr %25, align 4, !tbaa !132
  store ptr %21, ptr %22, align 4, !tbaa !138
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_release_builder() local_unnamed_addr #3 {
  %1 = load ptr, ptr @builder, align 4, !tbaa !6
  %2 = icmp eq ptr %1, null
  br i1 %2, label %5, label %3

3:                                                ; preds = %0
  %4 = load i32, ptr @builder_capacity, align 4, !tbaa !4
  tail call void @fa_free(ptr noundef nonnull %1, i32 noundef %4, i32 noundef 1) #62
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
  tail call void @fa_write(i32 noundef 1, ptr noundef nonnull @output, i32 noundef %1) #62
  store i32 0, ptr @output_length, align 4, !tbaa !4
  br label %4

4:                                                ; preds = %0, %3
  %5 = load ptr, ptr @builder, align 4, !tbaa !6
  %6 = icmp eq ptr %5, null
  br i1 %6, label %9, label %7

7:                                                ; preds = %4
  %8 = load i32, ptr @builder_capacity, align 4, !tbaa !4
  tail call void @fa_free(ptr noundef nonnull %5, i32 noundef %8, i32 noundef 1) #62
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
  tail call void @fa_free(ptr noundef nonnull %10, i32 noundef %15, i32 noundef 4) #62
  store ptr null, ptr @argument_items, align 4, !tbaa !6
  br label %16

16:                                               ; preds = %12, %9
  %17 = load ptr, ptr @argument_bytes, align 4, !tbaa !6
  %18 = icmp eq ptr %17, null
  br i1 %18, label %20, label %19

19:                                               ; preds = %16
  tail call void @fa_free(ptr noundef nonnull %17, i32 noundef 1, i32 noundef 1) #62
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
  store i32 0, ptr @builder_simple, align 4, !tbaa !4
  tail call fastcc void @append_c(ptr noundef nonnull @.str.13) #61
  %27 = load i32, ptr @fa_live_blocks, align 4, !tbaa !4
  %28 = load i32, ptr @live_tree_arenas, align 4, !tbaa !4
  %29 = add i32 %28, %27
  %30 = zext i32 %29 to i64
  tail call void @fa_b_u64(i64 noundef %30) #61
  tail call fastcc void @append_c(ptr noundef nonnull @.str.14) #61
  tail call void @fa_b_line(i32 noundef 2) #61
  tail call void @fa_exit(i32 noundef 102) #63
  unreachable

31:                                               ; preds = %20
  ret void
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_pool_new(i64 noundef %0) local_unnamed_addr #3 {
  %2 = tail call ptr @fa_alloc(i32 noundef 48, i32 noundef 8) #62
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(48) %2, i8 0, i32 48, i1 false)
  %3 = tail call i64 @llvm.smax.i64(i64 %0, i64 4)
  %4 = trunc i64 %3 to i32
  %5 = getelementptr inbounds %struct.FaPool, ptr %2, i32 0, i32 6
  store i32 %4, ptr %5, align 4, !tbaa !140
  %6 = shl i32 %4, 6
  %7 = add i32 %6, 104
  br label %8

8:                                                ; preds = %8, %1
  %9 = phi i32 [ 1, %1 ], [ %12, %8 ]
  %10 = shl i32 %9, 16
  %11 = icmp ult i32 %10, %7
  %12 = shl i32 %9, 1
  br i1 %11, label %8, label %13, !llvm.loop !142

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
  store i32 %20, ptr %31, align 8, !tbaa !143
  %32 = getelementptr inbounds %struct.FaPool, ptr %2, i32 0, i32 8
  store i32 %9, ptr %32, align 4, !tbaa !144
  %33 = getelementptr inbounds %struct.FaPool, ptr %2, i32 0, i32 9
  store i32 %25, ptr %33, align 8, !tbaa !145
  ret ptr %2
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_columnar_new(i64 noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #3 {
  %4 = tail call ptr @fa_alloc(i32 noundef 48, i32 noundef 8) #62
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(48) %4, i8 0, i32 48, i1 false)
  %5 = getelementptr inbounds %struct.FaPool, ptr %4, i32 0, i32 6
  store i32 4, ptr %5, align 4, !tbaa !140
  %6 = trunc i64 %0 to i32
  %7 = getelementptr inbounds %struct.FaPool, ptr %4, i32 0, i32 7
  store i32 %6, ptr %7, align 8, !tbaa !143
  %8 = trunc i64 %1 to i32
  %9 = getelementptr inbounds %struct.FaPool, ptr %4, i32 0, i32 8
  store i32 %8, ptr %9, align 4, !tbaa !144
  %10 = trunc i64 %2 to i32
  %11 = getelementptr inbounds %struct.FaPool, ptr %4, i32 0, i32 9
  store i32 %10, ptr %11, align 8, !tbaa !145
  ret ptr %4
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_pool_add_slot(ptr nocapture noundef %0) local_unnamed_addr #3 {
  %2 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 4
  %3 = load i32, ptr %2, align 4, !tbaa !146
  %4 = add i32 %3, 63
  %5 = lshr i32 %4, 6
  %6 = icmp ult i32 %4, 64
  br i1 %6, label %28, label %7

7:                                                ; preds = %1
  %8 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 2
  %9 = load ptr, ptr %8, align 4, !tbaa !147
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
  br i1 %17, label %28, label %10, !llvm.loop !148

18:                                               ; preds = %10
  %19 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 1
  %20 = load ptr, ptr %19, align 8, !tbaa !149
  %21 = shl nuw i32 %11, 6
  %22 = tail call i64 @llvm.cttz.i64(i64 %13, i1 true), !range !112
  %23 = trunc i64 %22 to i32
  %24 = or disjoint i32 %21, %23
  %25 = getelementptr inbounds ptr, ptr %20, i32 %24
  %26 = load ptr, ptr %25, align 4, !tbaa !6
  %27 = icmp eq ptr %26, null
  br i1 %27, label %28, label %30

28:                                               ; preds = %15, %1, %18
  %29 = tail call fastcc ptr @pool_new_chunk(ptr noundef nonnull %0) #66
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
  %39 = load i32, ptr %38, align 4, !tbaa !140
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
  %61 = load i64, ptr %0, align 8, !tbaa !150
  %62 = add nsw i64 %61, 1
  store i64 %62, ptr %0, align 8, !tbaa !150
  br i1 %49, label %63, label %82

63:                                               ; preds = %48
  %64 = getelementptr inbounds %struct.PoolChunk, ptr %31, i32 0, i32 3
  %65 = load i32, ptr %64, align 4, !tbaa !23
  %66 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 7
  %67 = load i32, ptr %66, align 8, !tbaa !143
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
  %77 = load ptr, ptr %76, align 4, !tbaa !147
  %78 = lshr i32 %71, 6
  %79 = getelementptr inbounds i64, ptr %77, i32 %78
  %80 = load i64, ptr %79, align 8, !tbaa !89
  %81 = and i64 %80, %75
  store i64 %81, ptr %79, align 8, !tbaa !89
  br label %82

82:                                               ; preds = %69, %63, %48
  %83 = load ptr, ptr %31, align 8, !tbaa !6
  %84 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 6
  %85 = load i32, ptr %84, align 4, !tbaa !140
  %86 = mul i32 %85, %50
  %87 = getelementptr inbounds i8, ptr %83, i32 %86
  ret ptr %87
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.cttz.i64(i64, i1 immarg) #32

; Function Attrs: cold noinline nounwind optsize
define internal fastcc noundef ptr @pool_new_chunk(ptr nocapture noundef %0) unnamed_addr #22 {
  %2 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 3
  %3 = load ptr, ptr %2, align 8, !tbaa !151
  store ptr null, ptr %2, align 8, !tbaa !151
  %4 = icmp eq ptr %3, null
  br i1 %4, label %5, label %40

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 8
  %7 = load i32, ptr %6, align 4, !tbaa !144
  %8 = icmp eq i32 %7, 1
  br i1 %8, label %9, label %11

9:                                                ; preds = %5
  %10 = tail call fastcc ptr @take_pages(i32 noundef 1) #61
  br label %34

11:                                               ; preds = %5
  %12 = shl i32 %7, 1
  %13 = add i32 %12, -1
  %14 = tail call fastcc ptr @take_pages(i32 noundef %13) #61
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
  tail call fastcc void @give_pages(ptr noundef nonnull %14, i32 noundef %25) #61
  br label %28

28:                                               ; preds = %27, %16
  %29 = xor i32 %25, -1
  %30 = add i32 %7, %29
  %31 = icmp eq i32 %30, 0
  br i1 %31, label %34, label %32

32:                                               ; preds = %28
  %33 = getelementptr inbounds i8, ptr %23, i32 %17
  tail call fastcc void @give_pages(ptr noundef %33, i32 noundef %30) #61
  br label %34

34:                                               ; preds = %9, %28, %32
  %35 = phi ptr [ %10, %9 ], [ %23, %32 ], [ %23, %28 ]
  %36 = icmp eq ptr %35, null
  br i1 %36, label %37, label %38

37:                                               ; preds = %11, %34
  tail call fastcc void @out_of_memory() #65
  unreachable

38:                                               ; preds = %34
  %39 = atomicrmw add ptr @fa_live_blocks, i32 1 monotonic, align 4
  br label %40

40:                                               ; preds = %38, %1
  %41 = phi ptr [ %35, %38 ], [ %3, %1 ]
  %42 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 7
  %43 = load i32, ptr %42, align 8, !tbaa !143
  %44 = add i32 %43, 63
  %45 = lshr i32 %44, 3
  %46 = and i32 %45, 536870904
  %47 = add nuw nsw i32 %46, 24
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(1) %41, i8 0, i32 %47, i1 false)
  %48 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 9
  %49 = load i32, ptr %48, align 8, !tbaa !145
  %50 = getelementptr inbounds i8, ptr %41, i32 %49
  store ptr %50, ptr %41, align 8, !tbaa !6
  %51 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 4
  %52 = load i32, ptr %51, align 4, !tbaa !146
  %53 = icmp eq i32 %52, 0
  br i1 %53, label %65, label %54

54:                                               ; preds = %40
  %55 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 1
  %56 = load ptr, ptr %55, align 8, !tbaa !149
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
  br i1 %64, label %68, label %57, !llvm.loop !152

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
  %77 = tail call ptr @fa_alloc(i32 noundef %76, i32 noundef 4) #62
  %78 = add i32 %75, 63
  %79 = lshr i32 %78, 3
  %80 = and i32 %79, 536870904
  %81 = tail call ptr @fa_alloc(i32 noundef %80, i32 noundef 8) #62
  tail call void @llvm.memset.p0.i32(ptr align 4 %77, i8 0, i32 %76, i1 false)
  tail call void @llvm.memset.p0.i32(ptr align 8 %81, i8 0, i32 %80, i1 false)
  %82 = load i32, ptr %70, align 8, !tbaa !153
  %83 = icmp eq i32 %82, 0
  br i1 %83, label %103, label %84

84:                                               ; preds = %73
  %85 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 1
  %86 = load ptr, ptr %85, align 8, !tbaa !149
  %87 = load i32, ptr %51, align 4, !tbaa !146
  %88 = shl i32 %87, 2
  tail call fastcc void @copy_bytes(ptr noundef %77, ptr noundef %86, i32 noundef %88) #61
  %89 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 2
  %90 = load ptr, ptr %89, align 4, !tbaa !147
  %91 = load i32, ptr %70, align 8, !tbaa !153
  %92 = add i32 %91, 63
  %93 = lshr i32 %92, 3
  %94 = and i32 %93, 536870904
  tail call fastcc void @copy_bytes(ptr noundef %81, ptr noundef %90, i32 noundef %94) #61
  %95 = load ptr, ptr %85, align 8, !tbaa !149
  %96 = load i32, ptr %70, align 8, !tbaa !153
  %97 = shl i32 %96, 2
  tail call void @fa_free(ptr noundef %95, i32 noundef %97, i32 noundef 4) #62
  %98 = load ptr, ptr %89, align 4, !tbaa !147
  %99 = load i32, ptr %70, align 8, !tbaa !153
  %100 = add i32 %99, 63
  %101 = lshr i32 %100, 3
  %102 = and i32 %101, 536870904
  tail call void @fa_free(ptr noundef %98, i32 noundef %102, i32 noundef 8) #62
  br label %103

103:                                              ; preds = %84, %73
  %104 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 1
  store ptr %77, ptr %104, align 8, !tbaa !149
  %105 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 2
  store ptr %81, ptr %105, align 4, !tbaa !147
  store i32 %75, ptr %70, align 8, !tbaa !153
  %106 = load i32, ptr %51, align 4, !tbaa !146
  br label %107

107:                                              ; preds = %103, %68
  %108 = phi i32 [ %106, %103 ], [ %52, %68 ]
  %109 = add i32 %108, 1
  store i32 %109, ptr %51, align 4, !tbaa !146
  br label %110

110:                                              ; preds = %107, %65
  %111 = phi i32 [ %69, %107 ], [ %66, %65 ]
  %112 = getelementptr inbounds %struct.PoolChunk, ptr %41, i32 0, i32 1
  store i32 %111, ptr %112, align 4, !tbaa !23
  %113 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 1
  %114 = load ptr, ptr %113, align 8, !tbaa !149
  %115 = getelementptr inbounds ptr, ptr %114, i32 %111
  store ptr %41, ptr %115, align 4, !tbaa !6
  %116 = and i32 %111, 63
  %117 = zext nneg i32 %116 to i64
  %118 = shl nuw i64 1, %117
  %119 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 2
  %120 = load ptr, ptr %119, align 4, !tbaa !147
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
  %5 = load i32, ptr %4, align 4, !tbaa !144
  %6 = mul i32 %5, -65536
  %7 = and i32 %6, %3
  %8 = inttoptr i32 %7 to ptr
  %9 = load ptr, ptr %8, align 65536, !tbaa !6
  %10 = ptrtoint ptr %9 to i32
  %11 = sub i32 %3, %10
  %12 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 6
  %13 = load i32, ptr %12, align 4, !tbaa !140
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
  %29 = load i64, ptr %0, align 8, !tbaa !150
  %30 = add nsw i64 %29, -1
  store i64 %30, ptr %0, align 8, !tbaa !150
  %31 = getelementptr inbounds %struct.PoolChunk, ptr %8, i32 0, i32 1
  %32 = load i32, ptr %31, align 4, !tbaa !23
  %33 = icmp eq i32 %28, 0
  br i1 %33, label %34, label %69

34:                                               ; preds = %2
  %35 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 1
  %36 = load ptr, ptr %35, align 8, !tbaa !149
  %37 = getelementptr inbounds ptr, ptr %36, i32 %32
  store ptr null, ptr %37, align 4, !tbaa !6
  %38 = and i32 %32, 63
  %39 = zext nneg i32 %38 to i64
  %40 = shl nuw i64 1, %39
  %41 = xor i64 %40, -1
  %42 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 2
  %43 = load ptr, ptr %42, align 4, !tbaa !147
  %44 = lshr i32 %32, 6
  %45 = getelementptr inbounds i64, ptr %43, i32 %44
  %46 = load i64, ptr %45, align 8, !tbaa !89
  %47 = and i64 %46, %41
  store i64 %47, ptr %45, align 8, !tbaa !89
  %48 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 4
  %49 = load i32, ptr %48, align 4, !tbaa !146
  %50 = icmp eq i32 %49, 0
  br i1 %50, label %61, label %51

51:                                               ; preds = %34
  %52 = load ptr, ptr %35, align 8, !tbaa !149
  br label %53

53:                                               ; preds = %51, %59
  %54 = phi i32 [ %49, %51 ], [ %55, %59 ]
  %55 = add i32 %54, -1
  %56 = getelementptr inbounds ptr, ptr %52, i32 %55
  %57 = load ptr, ptr %56, align 4, !tbaa !6
  %58 = icmp eq ptr %57, null
  br i1 %58, label %59, label %61

59:                                               ; preds = %53
  store i32 %55, ptr %48, align 4, !tbaa !146
  %60 = icmp eq i32 %55, 0
  br i1 %60, label %61, label %53, !llvm.loop !154

61:                                               ; preds = %53, %59, %34
  %62 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 3
  %63 = load ptr, ptr %62, align 8, !tbaa !151
  %64 = icmp eq ptr %63, null
  br i1 %64, label %65, label %66

65:                                               ; preds = %61
  store ptr %8, ptr %62, align 8, !tbaa !151
  br label %79

66:                                               ; preds = %61
  %67 = load i32, ptr %4, align 4, !tbaa !144
  tail call fastcc void @give_pages(ptr noundef nonnull %8, i32 noundef %67) #61
  %68 = atomicrmw sub ptr @fa_live_blocks, i32 1 monotonic, align 4
  br label %79

69:                                               ; preds = %2
  %70 = and i32 %32, 63
  %71 = zext nneg i32 %70 to i64
  %72 = shl nuw i64 1, %71
  %73 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 2
  %74 = load ptr, ptr %73, align 4, !tbaa !147
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
  %6 = load i32, ptr %5, align 4, !tbaa !146
  %7 = icmp ugt i32 %6, %4
  br i1 %7, label %8, label %48

8:                                                ; preds = %2
  %9 = trunc i64 %1 to i32
  %10 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 1
  %11 = load ptr, ptr %10, align 8, !tbaa !149
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
  %32 = tail call i64 @llvm.cttz.i64(i64 %29, i1 true), !range !112
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
  br i1 %47, label %48, label %12, !llvm.loop !155

48:                                               ; preds = %45, %2, %40
  %49 = phi i64 [ %44, %40 ], [ -1, %2 ], [ -1, %45 ]
  ret i64 %49
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(read, inaccessiblemem: none)
define hidden ptr @fa_pool_at(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #35 {
  %3 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 1
  %4 = load ptr, ptr %3, align 8, !tbaa !149
  %5 = lshr i64 %1, 32
  %6 = trunc i64 %5 to i32
  %7 = getelementptr inbounds ptr, ptr %4, i32 %6
  %8 = load ptr, ptr %7, align 4, !tbaa !6
  %9 = load ptr, ptr %8, align 8, !tbaa !6
  %10 = trunc i64 %1 to i32
  %11 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 6
  %12 = load i32, ptr %11, align 4, !tbaa !140
  %13 = mul i32 %12, %10
  %14 = getelementptr inbounds i8, ptr %9, i32 %13
  ret ptr %14
}

; Function Attrs: nofree norecurse nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden void @fa_pool_clear(ptr nocapture noundef %0) local_unnamed_addr #33 {
  tail call fastcc void @pool_release_chunks(ptr noundef %0) #61
  ret void
}

; Function Attrs: nofree norecurse nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc void @pool_release_chunks(ptr nocapture noundef %0) unnamed_addr #33 {
  %2 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 4
  %3 = load i32, ptr %2, align 4, !tbaa !146
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
  %15 = load ptr, ptr %6, align 8, !tbaa !149
  %16 = getelementptr inbounds ptr, ptr %15, i32 %14
  %17 = load ptr, ptr %16, align 4, !tbaa !6
  %18 = icmp eq ptr %17, null
  br i1 %18, label %23, label %19

19:                                               ; preds = %12
  %20 = load i32, ptr %7, align 4, !tbaa !144
  tail call fastcc void @give_pages(ptr noundef nonnull %17, i32 noundef %20) #61
  %21 = atomicrmw sub ptr @fa_live_blocks, i32 1 monotonic, align 4
  %22 = load i32, ptr %2, align 4, !tbaa !146
  br label %23

23:                                               ; preds = %12, %19
  %24 = phi i32 [ %13, %12 ], [ %22, %19 ]
  %25 = add nuw i32 %14, 1
  %26 = icmp ult i32 %25, %24
  br i1 %26, label %12, label %8, !llvm.loop !156

27:                                               ; preds = %8
  %28 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 1
  %29 = load ptr, ptr %28, align 8, !tbaa !149
  %30 = shl i32 %10, 2
  tail call void @llvm.memset.p0.i32(ptr align 4 %29, i8 0, i32 %30, i1 false)
  %31 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 2
  %32 = load ptr, ptr %31, align 4, !tbaa !147
  %33 = load i32, ptr %9, align 8, !tbaa !153
  %34 = add i32 %33, 63
  %35 = lshr i32 %34, 3
  %36 = and i32 %35, 536870904
  tail call void @llvm.memset.p0.i32(ptr align 8 %32, i8 0, i32 %36, i1 false)
  br label %37

37:                                               ; preds = %27, %8
  store i32 0, ptr %2, align 4, !tbaa !146
  store i64 0, ptr %0, align 8, !tbaa !150
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_pool_free(ptr noundef %0) local_unnamed_addr #3 {
  tail call fastcc void @pool_release_chunks(ptr noundef %0) #61
  %2 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 3
  %3 = load ptr, ptr %2, align 8, !tbaa !151
  %4 = icmp eq ptr %3, null
  br i1 %4, label %9, label %5

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 8
  %7 = load i32, ptr %6, align 4, !tbaa !144
  tail call fastcc void @give_pages(ptr noundef nonnull %3, i32 noundef %7) #61
  %8 = atomicrmw sub ptr @fa_live_blocks, i32 1 monotonic, align 4
  br label %9

9:                                                ; preds = %5, %1
  %10 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 5
  %11 = load i32, ptr %10, align 8, !tbaa !153
  %12 = icmp eq i32 %11, 0
  br i1 %12, label %23, label %13

13:                                               ; preds = %9
  %14 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 1
  %15 = load ptr, ptr %14, align 8, !tbaa !149
  %16 = shl i32 %11, 2
  tail call void @fa_free(ptr noundef %15, i32 noundef %16, i32 noundef 4) #62
  %17 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 2
  %18 = load ptr, ptr %17, align 4, !tbaa !147
  %19 = load i32, ptr %10, align 8, !tbaa !153
  %20 = add i32 %19, 63
  %21 = lshr i32 %20, 3
  %22 = and i32 %21, 536870904
  tail call void @fa_free(ptr noundef %18, i32 noundef %22, i32 noundef 8) #62
  br label %23

23:                                               ; preds = %13, %9
  tail call void @fa_free(ptr noundef nonnull %0, i32 noundef 48, i32 noundef 8) #62
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(read, inaccessiblemem: none)
define hidden i32 @fa_links_has(ptr noundef readonly %0, ptr noundef readonly %1) local_unnamed_addr #34 {
  %3 = tail call fastcc i64 @links_find(ptr noundef %0, ptr noundef %1) #61
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
  %6 = load ptr, ptr %5, align 8, !tbaa !157
  %7 = icmp eq ptr %6, null
  br i1 %7, label %8, label %24

8:                                                ; preds = %4
  %9 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 2
  %10 = load i64, ptr %9, align 8, !tbaa !159
  %11 = icmp sgt i64 %10, 0
  br i1 %11, label %12, label %57

12:                                               ; preds = %8
  %13 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 1
  %14 = load ptr, ptr %13, align 8, !tbaa !160
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
  br i1 %23, label %57, label %15, !llvm.loop !161

24:                                               ; preds = %4
  %25 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 5
  %26 = load i64, ptr %25, align 8, !tbaa !162
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
  %39 = load ptr, ptr %38, align 8, !tbaa !160
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
  %6 = tail call ptr @fa_alloc(i32 noundef 48, i32 noundef 8) #62
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(48) %6, i8 0, i32 48, i1 false)
  store ptr %6, ptr %0, align 4, !tbaa !6
  br label %10

7:                                                ; preds = %2
  %8 = tail call fastcc i64 @links_find(ptr noundef nonnull %3, ptr noundef %1) #61
  %9 = icmp sgt i64 %8, -1
  br i1 %9, label %157, label %10

10:                                               ; preds = %7, %5
  %11 = phi ptr [ %6, %5 ], [ %3, %7 ]
  %12 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 2
  %13 = load i64, ptr %12, align 8, !tbaa !159
  %14 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 3
  %15 = load i64, ptr %14, align 8, !tbaa !163
  %16 = icmp eq i64 %13, %15
  br i1 %16, label %17, label %124

17:                                               ; preds = %10
  %18 = icmp sgt i64 %13, 0
  br i1 %18, label %19, label %42

19:                                               ; preds = %17
  %20 = load i64, ptr %11, align 8, !tbaa !164
  %21 = shl nsw i64 %20, 1
  %22 = icmp sgt i64 %21, %13
  br i1 %22, label %42, label %23

23:                                               ; preds = %19
  %24 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 1
  br label %26

25:                                               ; preds = %38
  store i64 %39, ptr %12, align 8, !tbaa !159
  br label %63

26:                                               ; preds = %23, %38
  %27 = phi i64 [ 0, %23 ], [ %40, %38 ]
  %28 = phi i64 [ 0, %23 ], [ %39, %38 ]
  %29 = load ptr, ptr %24, align 8, !tbaa !160
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
  br i1 %41, label %25, label %26, !llvm.loop !165

42:                                               ; preds = %19, %17
  %43 = icmp eq i64 %13, 0
  %44 = shl nsw i64 %13, 1
  %45 = select i1 %43, i64 4, i64 %44
  %46 = trunc i64 %45 to i32
  %47 = shl i32 %46, 2
  %48 = tail call ptr @fa_alloc(i32 noundef %47, i32 noundef 4) #62
  %49 = load i64, ptr %14, align 8, !tbaa !163
  %50 = icmp sgt i64 %49, 0
  br i1 %50, label %51, label %61

51:                                               ; preds = %42
  %52 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 1
  %53 = load ptr, ptr %52, align 8, !tbaa !160
  %54 = load i64, ptr %12, align 8, !tbaa !159
  %55 = trunc i64 %54 to i32
  %56 = shl i32 %55, 2
  tail call fastcc void @copy_bytes(ptr noundef %48, ptr noundef %53, i32 noundef %56) #61
  %57 = load ptr, ptr %52, align 8, !tbaa !160
  %58 = load i64, ptr %14, align 8, !tbaa !163
  %59 = trunc i64 %58 to i32
  %60 = shl i32 %59, 2
  tail call void @fa_free(ptr noundef %57, i32 noundef %60, i32 noundef 4) #62
  br label %61

61:                                               ; preds = %51, %42
  %62 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 1
  store ptr %48, ptr %62, align 8, !tbaa !160
  store i64 %45, ptr %14, align 8, !tbaa !163
  br label %63

63:                                               ; preds = %61, %25
  %64 = phi i64 [ %45, %61 ], [ %13, %25 ]
  %65 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 4
  %66 = load ptr, ptr %65, align 8, !tbaa !157
  %67 = icmp eq ptr %66, null
  br i1 %67, label %75, label %68

68:                                               ; preds = %63
  %69 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 5
  %70 = load i64, ptr %69, align 8, !tbaa !162
  %71 = trunc i64 %70 to i32
  %72 = shl i32 %71, 2
  %73 = add i32 %72, 4
  tail call void @fa_free(ptr noundef nonnull %66, i32 noundef %73, i32 noundef 4) #62
  store ptr null, ptr %65, align 8, !tbaa !157
  %74 = load i64, ptr %14, align 8, !tbaa !163
  br label %75

75:                                               ; preds = %68, %63
  %76 = phi i64 [ %74, %68 ], [ %64, %63 ]
  %77 = icmp slt i64 %76, 9
  br i1 %77, label %78, label %80

78:                                               ; preds = %75
  %79 = load i64, ptr %12, align 8, !tbaa !159
  br label %124

80:                                               ; preds = %75
  %81 = shl nuw nsw i64 %76, 1
  br label %82

82:                                               ; preds = %82, %80
  %83 = phi i64 [ %85, %82 ], [ 16, %80 ]
  %84 = icmp slt i64 %83, %81
  %85 = shl nsw i64 %83, 1
  br i1 %84, label %82, label %86, !llvm.loop !166

86:                                               ; preds = %82
  %87 = add nsw i64 %83, -1
  %88 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 5
  store i64 %87, ptr %88, align 8, !tbaa !162
  %89 = trunc i64 %83 to i32
  %90 = shl i32 %89, 2
  %91 = tail call ptr @fa_alloc(i32 noundef %90, i32 noundef 4) #62
  store ptr %91, ptr %65, align 8, !tbaa !157
  tail call void @llvm.memset.p0.i32(ptr align 4 %91, i8 0, i32 %90, i1 false)
  %92 = load i64, ptr %12, align 8, !tbaa !159
  %93 = icmp sgt i64 %92, 0
  br i1 %93, label %94, label %124

94:                                               ; preds = %86
  %95 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 1
  %96 = load ptr, ptr %95, align 8, !tbaa !160
  br label %97

97:                                               ; preds = %121, %94
  %98 = phi i64 [ 0, %94 ], [ %122, %121 ]
  %99 = trunc i64 %98 to i32
  %100 = getelementptr inbounds ptr, ptr %96, i32 %99
  %101 = load ptr, ptr %100, align 4, !tbaa !6
  %102 = icmp eq ptr %101, null
  br i1 %102, label %121, label %103

103:                                              ; preds = %97
  %104 = load i64, ptr %88, align 8, !tbaa !162
  %105 = ptrtoint ptr %101 to i32
  %106 = zext i32 %105 to i64
  %107 = mul i64 %106, -49064778989728563
  %108 = lshr i64 %107, 33
  %109 = xor i64 %108, %107
  %110 = load ptr, ptr %65, align 8, !tbaa !157
  br label %111

111:                                              ; preds = %111, %103
  %112 = phi i64 [ %109, %103 ], [ %118, %111 ]
  %113 = and i64 %112, %104
  %114 = trunc i64 %113 to i32
  %115 = getelementptr inbounds i32, ptr %110, i32 %114
  %116 = load i32, ptr %115, align 4, !tbaa !23
  %117 = icmp eq i32 %116, 0
  %118 = add i64 %113, 1
  br i1 %117, label %119, label %111, !llvm.loop !167

119:                                              ; preds = %111
  %120 = add i32 %99, 1
  store i32 %120, ptr %115, align 4, !tbaa !23
  br label %121

121:                                              ; preds = %119, %97
  %122 = add nuw nsw i64 %98, 1
  %123 = icmp eq i64 %122, %92
  br i1 %123, label %124, label %97, !llvm.loop !168

124:                                              ; preds = %121, %78, %86, %10
  %125 = phi i64 [ %79, %78 ], [ %92, %86 ], [ %13, %10 ], [ %92, %121 ]
  %126 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 1
  %127 = load ptr, ptr %126, align 8, !tbaa !160
  %128 = trunc i64 %125 to i32
  %129 = getelementptr inbounds ptr, ptr %127, i32 %128
  store ptr %1, ptr %129, align 4, !tbaa !6
  %130 = add nsw i64 %125, 1
  store i64 %130, ptr %12, align 8, !tbaa !159
  %131 = load i64, ptr %11, align 8, !tbaa !164
  %132 = add nsw i64 %131, 1
  store i64 %132, ptr %11, align 8, !tbaa !164
  %133 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 4
  %134 = load ptr, ptr %133, align 8, !tbaa !157
  %135 = icmp eq ptr %134, null
  br i1 %135, label %157, label %136

136:                                              ; preds = %124
  %137 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 5
  %138 = load i64, ptr %137, align 8, !tbaa !162
  %139 = load ptr, ptr %126, align 8, !tbaa !160
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
  br i1 %153, label %155, label %147, !llvm.loop !167

155:                                              ; preds = %147
  %156 = add i32 %128, 1
  store i32 %156, ptr %151, align 4, !tbaa !23
  br label %157

157:                                              ; preds = %124, %155, %7
  %158 = phi i32 [ 0, %7 ], [ 1, %155 ], [ 1, %124 ]
  ret i32 %158
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden noundef i32 @fa_links_delete(ptr noundef %0, ptr noundef %1) local_unnamed_addr #20 {
  %3 = tail call fastcc i64 @links_find(ptr noundef %0, ptr noundef %1) #61
  %4 = icmp slt i64 %3, 0
  br i1 %4, label %78, label %5

5:                                                ; preds = %2
  %6 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 1
  %7 = load ptr, ptr %6, align 8, !tbaa !160
  %8 = trunc i64 %3 to i32
  %9 = getelementptr inbounds ptr, ptr %7, i32 %8
  store ptr null, ptr %9, align 4, !tbaa !6
  %10 = load i64, ptr %0, align 8, !tbaa !164
  %11 = add nsw i64 %10, -1
  store i64 %11, ptr %0, align 8, !tbaa !164
  %12 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 4
  %13 = load ptr, ptr %12, align 8, !tbaa !157
  %14 = icmp eq ptr %13, null
  br i1 %14, label %74, label %15

15:                                               ; preds = %5
  %16 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 5
  %17 = load i64, ptr %16, align 8, !tbaa !162
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
  br i1 %30, label %32, label %24, !llvm.loop !169

32:                                               ; preds = %24
  %33 = and i64 %31, %17
  %34 = trunc i64 %33 to i32
  %35 = getelementptr inbounds i32, ptr %13, i32 %34
  %36 = load i32, ptr %35, align 4, !tbaa !23
  %37 = icmp eq i32 %36, 0
  br i1 %37, label %71, label %38

38:                                               ; preds = %32
  %39 = load ptr, ptr %6, align 8, !tbaa !160
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
  store i64 0, ptr %77, align 8, !tbaa !159
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
  %5 = load ptr, ptr %4, align 8, !tbaa !157
  %6 = icmp eq ptr %5, null
  br i1 %6, label %13, label %7

7:                                                ; preds = %3
  %8 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 5
  %9 = load i64, ptr %8, align 8, !tbaa !162
  %10 = trunc i64 %9 to i32
  %11 = shl i32 %10, 2
  %12 = add i32 %11, 4
  tail call void @fa_free(ptr noundef nonnull %5, i32 noundef %12, i32 noundef 4) #62
  br label %13

13:                                               ; preds = %7, %3
  %14 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 3
  %15 = load i64, ptr %14, align 8, !tbaa !163
  %16 = icmp sgt i64 %15, 0
  br i1 %16, label %17, label %22

17:                                               ; preds = %13
  %18 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 1
  %19 = load ptr, ptr %18, align 8, !tbaa !160
  %20 = trunc i64 %15 to i32
  %21 = shl i32 %20, 2
  tail call void @fa_free(ptr noundef %19, i32 noundef %21, i32 noundef 4) #62
  br label %22

22:                                               ; preds = %17, %13
  tail call void @fa_free(ptr noundef nonnull %0, i32 noundef 48, i32 noundef 8) #62
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
  %6 = load i64, ptr %5, align 8, !tbaa !159
  %7 = icmp ugt i64 %6, %1
  br i1 %7, label %8, label %20

8:                                                ; preds = %4
  %9 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 1
  %10 = load ptr, ptr %9, align 8, !tbaa !160
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
  br i1 %19, label %20, label %11, !llvm.loop !170

20:                                               ; preds = %11, %17, %4, %2
  %21 = phi i64 [ -1, %2 ], [ -1, %4 ], [ %12, %11 ], [ -1, %17 ]
  ret i64 %21
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(read, inaccessiblemem: none)
define hidden ptr @fa_links_at(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #35 {
  %3 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 1
  %4 = load ptr, ptr %3, align 8, !tbaa !160
  %5 = trunc i64 %1 to i32
  %6 = getelementptr inbounds ptr, ptr %4, i32 %5
  %7 = load ptr, ptr %6, align 4, !tbaa !6
  ret ptr %7
}

; Function Attrs: nounwind optsize
define hidden void @fa_relate_one(ptr noundef %0, i64 noundef %1, i64 noundef %2, ptr noundef %3, i32 noundef %4, i32 noundef %5) local_unnamed_addr #3 {
  %7 = trunc i64 %1 to i32
  %8 = getelementptr inbounds i8, ptr %0, i32 %7
  %9 = load ptr, ptr %8, align 4, !tbaa !6
  %10 = icmp eq ptr %9, %3
  br i1 %10, label %41, label %11

11:                                               ; preds = %6
  %12 = icmp eq i32 %4, 0
  %13 = icmp eq ptr %9, null
  br i1 %12, label %26, label %14

14:                                               ; preds = %11
  br i1 %13, label %20, label %15

15:                                               ; preds = %14
  %16 = trunc i64 %2 to i32
  %17 = getelementptr inbounds i8, ptr %9, i32 %16
  %18 = load ptr, ptr %17, align 4, !tbaa !6
  %19 = tail call i32 @fa_links_delete(ptr noundef %18, ptr noundef nonnull %0) #61, !range !50
  tail call fastcc void @notify_side(i32 noundef %5, ptr noundef nonnull %9, i64 noundef %2) #61
  br label %20

20:                                               ; preds = %15, %14
  store ptr %3, ptr %8, align 4, !tbaa !6
  tail call fastcc void @notify_side(i32 noundef %5, ptr noundef nonnull %0, i64 noundef %1) #61
  %21 = icmp eq ptr %3, null
  br i1 %21, label %41, label %22

22:                                               ; preds = %20
  %23 = trunc i64 %2 to i32
  %24 = getelementptr inbounds i8, ptr %3, i32 %23
  %25 = tail call i32 @fa_links_add(ptr noundef nonnull %24, ptr noundef nonnull %0) #61, !range !50
  tail call fastcc void @notify_side(i32 noundef %5, ptr noundef nonnull %3, i64 noundef %2) #61
  br label %41

26:                                               ; preds = %11
  br i1 %13, label %30, label %27

27:                                               ; preds = %26
  %28 = trunc i64 %2 to i32
  %29 = getelementptr inbounds i8, ptr %9, i32 %28
  store ptr null, ptr %29, align 4, !tbaa !6
  tail call fastcc void @notify_side(i32 noundef %5, ptr noundef nonnull %9, i64 noundef %2) #61
  br label %30

30:                                               ; preds = %27, %26
  %31 = icmp eq ptr %3, null
  br i1 %31, label %40, label %32

32:                                               ; preds = %30
  %33 = trunc i64 %2 to i32
  %34 = getelementptr inbounds i8, ptr %3, i32 %33
  %35 = load ptr, ptr %34, align 4, !tbaa !6
  %36 = icmp eq ptr %35, null
  br i1 %36, label %39, label %37

37:                                               ; preds = %32
  %38 = getelementptr inbounds i8, ptr %35, i32 %7
  store ptr null, ptr %38, align 4, !tbaa !6
  tail call fastcc void @notify_side(i32 noundef %5, ptr noundef nonnull %35, i64 noundef %1) #61
  br label %39

39:                                               ; preds = %37, %32
  store ptr %0, ptr %34, align 4, !tbaa !6
  tail call fastcc void @notify_side(i32 noundef %5, ptr noundef nonnull %3, i64 noundef %2) #61
  br label %40

40:                                               ; preds = %39, %30
  store ptr %3, ptr %8, align 4, !tbaa !6
  tail call fastcc void @notify_side(i32 noundef %5, ptr noundef nonnull %0, i64 noundef %1) #61
  br label %41

41:                                               ; preds = %20, %22, %6, %40
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @notify_side(i32 noundef %0, ptr noundef %1, i64 noundef %2) unnamed_addr #3 {
  %4 = icmp eq i32 %0, 0
  %5 = icmp eq ptr %1, null
  %6 = or i1 %4, %5
  br i1 %6, label %31, label %7

7:                                                ; preds = %3
  %8 = ptrtoint ptr %1 to i32
  %9 = zext i32 %8 to i64
  %10 = load i64, ptr @field_source_count, align 8, !tbaa !89
  %11 = icmp sgt i64 %10, 0
  br i1 %11, label %12, label %31

12:                                               ; preds = %7
  %13 = load ptr, ptr @field_sources, align 4, !tbaa !6
  %14 = or i64 %2, 4611686018427387904
  br label %15

15:                                               ; preds = %12, %28
  %16 = phi i64 [ 0, %12 ], [ %29, %28 ]
  %17 = trunc i64 %16 to i32
  %18 = getelementptr inbounds %struct.FaChild, ptr %13, i32 %17
  %19 = load i64, ptr %18, align 8, !tbaa !171
  %20 = icmp eq i64 %19, %9
  br i1 %20, label %21, label %28

21:                                               ; preds = %15
  %22 = getelementptr inbounds %struct.FaChild, ptr %13, i32 %17, i32 1
  %23 = load i64, ptr %22, align 8, !tbaa !173
  %24 = icmp eq i64 %23, %14
  br i1 %24, label %25, label %28

25:                                               ; preds = %21
  %26 = getelementptr inbounds %struct.FaChild, ptr %13, i32 %17, i32 2
  %27 = load ptr, ptr %26, align 8, !tbaa !174
  tail call void @fa_notify(ptr noundef %27) #61
  br label %31

28:                                               ; preds = %21, %15
  %29 = add nuw nsw i64 %16, 1
  %30 = icmp eq i64 %29, %10
  br i1 %30, label %31, label %15, !llvm.loop !175

31:                                               ; preds = %28, %7, %25, %3
  ret void
}

; Function Attrs: nounwind optsize
define hidden noundef i32 @fa_relate_add(ptr noundef %0, i64 noundef %1, i64 noundef %2, ptr noundef %3, i32 noundef %4) local_unnamed_addr #3 {
  %6 = trunc i64 %1 to i32
  %7 = getelementptr inbounds i8, ptr %0, i32 %6
  %8 = tail call i32 @fa_links_add(ptr noundef %7, ptr noundef %3) #61, !range !50
  %9 = icmp eq i32 %8, 0
  br i1 %9, label %14, label %10

10:                                               ; preds = %5
  %11 = trunc i64 %2 to i32
  %12 = getelementptr inbounds i8, ptr %3, i32 %11
  %13 = tail call i32 @fa_links_add(ptr noundef %12, ptr noundef %0) #61, !range !50
  tail call fastcc void @notify_side(i32 noundef %4, ptr noundef %0, i64 noundef %1) #61
  tail call fastcc void @notify_side(i32 noundef %4, ptr noundef %3, i64 noundef %2) #61
  br label %14

14:                                               ; preds = %5, %10
  %15 = phi i32 [ 1, %10 ], [ 0, %5 ]
  ret i32 %15
}

; Function Attrs: nounwind optsize
define hidden noundef i32 @fa_relate_delete(ptr noundef %0, i64 noundef %1, i64 noundef %2, ptr noundef %3, i32 noundef %4) local_unnamed_addr #3 {
  %6 = trunc i64 %1 to i32
  %7 = getelementptr inbounds i8, ptr %0, i32 %6
  %8 = load ptr, ptr %7, align 4, !tbaa !6
  %9 = tail call i32 @fa_links_delete(ptr noundef %8, ptr noundef %3) #61, !range !50
  %10 = icmp eq i32 %9, 0
  br i1 %10, label %16, label %11

11:                                               ; preds = %5
  %12 = trunc i64 %2 to i32
  %13 = getelementptr inbounds i8, ptr %3, i32 %12
  %14 = load ptr, ptr %13, align 4, !tbaa !6
  %15 = tail call i32 @fa_links_delete(ptr noundef %14, ptr noundef nonnull %0) #61, !range !50
  tail call fastcc void @notify_side(i32 noundef %4, ptr noundef nonnull %0, i64 noundef %1) #61
  tail call fastcc void @notify_side(i32 noundef %4, ptr noundef %3, i64 noundef %2) #61
  br label %16

16:                                               ; preds = %5, %11
  %17 = phi i32 [ 1, %11 ], [ 0, %5 ]
  ret i32 %17
}

; Function Attrs: nounwind optsize
define hidden void @fa_relate_clear(ptr noundef %0, i64 noundef %1, i64 noundef %2, i32 noundef %3) local_unnamed_addr #3 {
  %5 = trunc i64 %1 to i32
  %6 = getelementptr inbounds i8, ptr %0, i32 %5
  %7 = load ptr, ptr %6, align 4, !tbaa !6
  %8 = icmp eq ptr %7, null
  br i1 %8, label %34, label %9

9:                                                ; preds = %4
  %10 = getelementptr inbounds %struct.FaLinks, ptr %7, i32 0, i32 2
  %11 = load i64, ptr %10, align 8, !tbaa !159
  %12 = icmp sgt i64 %11, 0
  br i1 %12, label %13, label %16

13:                                               ; preds = %9
  %14 = getelementptr inbounds %struct.FaLinks, ptr %7, i32 0, i32 1
  %15 = trunc i64 %2 to i32
  br label %17

16:                                               ; preds = %30, %9
  tail call void @fa_links_free(ptr noundef nonnull %7) #61
  store ptr null, ptr %6, align 4, !tbaa !6
  tail call fastcc void @notify_side(i32 noundef %3, ptr noundef %0, i64 noundef %1) #61
  br label %34

17:                                               ; preds = %13, %30
  %18 = phi i64 [ %11, %13 ], [ %31, %30 ]
  %19 = phi i64 [ 0, %13 ], [ %32, %30 ]
  %20 = load ptr, ptr %14, align 8, !tbaa !160
  %21 = trunc i64 %19 to i32
  %22 = getelementptr inbounds ptr, ptr %20, i32 %21
  %23 = load ptr, ptr %22, align 4, !tbaa !6
  %24 = icmp eq ptr %23, null
  br i1 %24, label %30, label %25

25:                                               ; preds = %17
  %26 = getelementptr inbounds i8, ptr %23, i32 %15
  %27 = load ptr, ptr %26, align 4, !tbaa !6
  %28 = tail call i32 @fa_links_delete(ptr noundef %27, ptr noundef %0) #61, !range !50
  tail call fastcc void @notify_side(i32 noundef %3, ptr noundef nonnull %23, i64 noundef %2) #61
  %29 = load i64, ptr %10, align 8, !tbaa !159
  br label %30

30:                                               ; preds = %25, %17
  %31 = phi i64 [ %29, %25 ], [ %18, %17 ]
  %32 = add nuw nsw i64 %19, 1
  %33 = icmp slt i64 %32, %31
  br i1 %33, label %17, label %16, !llvm.loop !176

34:                                               ; preds = %4, %16
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_unlink(ptr noundef %0, i32 noundef %1, i32 noundef %2, i64 noundef %3, i64 noundef %4, ptr noundef %5, i32 noundef %6) local_unnamed_addr #3 {
  %8 = icmp eq i32 %2, 0
  br i1 %8, label %48, label %9

9:                                                ; preds = %7
  %10 = trunc i64 %3 to i32
  %11 = getelementptr inbounds i8, ptr %0, i32 %10
  %12 = load ptr, ptr %11, align 4, !tbaa !6
  %13 = icmp eq i32 %1, 2
  br i1 %13, label %14, label %37

14:                                               ; preds = %9
  %15 = getelementptr inbounds %struct.FaLinks, ptr %12, i32 0, i32 2
  %16 = icmp eq ptr %12, null
  br i1 %16, label %102, label %17

17:                                               ; preds = %14
  %18 = getelementptr inbounds %struct.FaLinks, ptr %12, i32 0, i32 1
  %19 = trunc i64 %4 to i32
  %20 = load i64, ptr %15, align 8, !tbaa !159
  %21 = icmp sgt i64 %20, 0
  br i1 %21, label %22, label %102

22:                                               ; preds = %17, %33
  %23 = phi i64 [ %34, %33 ], [ 0, %17 ]
  %24 = load ptr, ptr %18, align 8, !tbaa !160
  %25 = trunc i64 %23 to i32
  %26 = getelementptr inbounds ptr, ptr %24, i32 %25
  %27 = load ptr, ptr %26, align 4, !tbaa !6
  %28 = icmp eq ptr %27, null
  br i1 %28, label %33, label %29

29:                                               ; preds = %22
  %30 = getelementptr inbounds i8, ptr %27, i32 %19
  %31 = load ptr, ptr %30, align 4, !tbaa !6
  %32 = tail call i32 @fa_links_delete(ptr noundef %31, ptr noundef %0) #61, !range !50
  tail call fastcc void @notify_side(i32 noundef %6, ptr noundef nonnull %27, i64 noundef %4) #61
  br label %33

33:                                               ; preds = %29, %22
  %34 = add nuw nsw i64 %23, 1
  %35 = load i64, ptr %15, align 8, !tbaa !159
  %36 = icmp slt i64 %34, %35
  br i1 %36, label %22, label %102

37:                                               ; preds = %9
  %38 = icmp eq ptr %12, null
  br i1 %38, label %102, label %39

39:                                               ; preds = %37
  %40 = icmp eq i32 %1, 0
  %41 = trunc i64 %4 to i32
  %42 = getelementptr inbounds i8, ptr %12, i32 %41
  br i1 %40, label %43, label %46

43:                                               ; preds = %39
  %44 = load ptr, ptr %42, align 4, !tbaa !6
  %45 = tail call i32 @fa_links_delete(ptr noundef %44, ptr noundef nonnull %0) #61, !range !50
  br label %47

46:                                               ; preds = %39
  store ptr null, ptr %42, align 4, !tbaa !6
  br label %47

47:                                               ; preds = %46, %43
  tail call fastcc void @notify_side(i32 noundef %6, ptr noundef nonnull %12, i64 noundef %4) #61
  br label %102

48:                                               ; preds = %7
  switch i32 %1, label %96 [
    i32 0, label %49
    i32 2, label %49
    i32 1, label %79
    i32 3, label %87
  ]

49:                                               ; preds = %48, %48
  %50 = trunc i64 %4 to i32
  %51 = getelementptr inbounds i8, ptr %0, i32 %50
  %52 = load ptr, ptr %51, align 4, !tbaa !6
  %53 = getelementptr inbounds %struct.FaLinks, ptr %52, i32 0, i32 2
  %54 = icmp eq ptr %52, null
  br i1 %54, label %102, label %55

55:                                               ; preds = %49
  %56 = getelementptr inbounds %struct.FaLinks, ptr %52, i32 0, i32 1
  %57 = icmp eq i32 %1, 0
  %58 = trunc i64 %3 to i32
  %59 = load i64, ptr %53, align 8, !tbaa !159
  %60 = icmp sgt i64 %59, 0
  br i1 %60, label %61, label %102

61:                                               ; preds = %55, %75
  %62 = phi i64 [ %76, %75 ], [ 0, %55 ]
  %63 = load ptr, ptr %56, align 8, !tbaa !160
  %64 = trunc i64 %62 to i32
  %65 = getelementptr inbounds ptr, ptr %63, i32 %64
  %66 = load ptr, ptr %65, align 4, !tbaa !6
  %67 = icmp eq ptr %66, null
  br i1 %67, label %75, label %68

68:                                               ; preds = %61
  %69 = getelementptr inbounds i8, ptr %66, i32 %58
  br i1 %57, label %70, label %71

70:                                               ; preds = %68
  store ptr null, ptr %69, align 4, !tbaa !6
  br label %74

71:                                               ; preds = %68
  %72 = load ptr, ptr %69, align 4, !tbaa !6
  %73 = tail call i32 @fa_links_delete(ptr noundef %72, ptr noundef %0) #61, !range !50
  br label %74

74:                                               ; preds = %71, %70
  tail call fastcc void @notify_side(i32 noundef %6, ptr noundef nonnull %66, i64 noundef %3) #61
  br label %75

75:                                               ; preds = %61, %74
  %76 = add nuw nsw i64 %62, 1
  %77 = load i64, ptr %53, align 8, !tbaa !159
  %78 = icmp slt i64 %76, %77
  br i1 %78, label %61, label %102

79:                                               ; preds = %48
  %80 = trunc i64 %4 to i32
  %81 = getelementptr inbounds i8, ptr %0, i32 %80
  %82 = load ptr, ptr %81, align 4, !tbaa !6
  %83 = icmp eq ptr %82, null
  br i1 %83, label %102, label %84

84:                                               ; preds = %79
  %85 = trunc i64 %3 to i32
  %86 = getelementptr inbounds i8, ptr %82, i32 %85
  store ptr null, ptr %86, align 4, !tbaa !6
  tail call fastcc void @notify_side(i32 noundef %6, ptr noundef nonnull %82, i64 noundef %3) #61
  br label %102

87:                                               ; preds = %48
  %88 = trunc i64 %3 to i32
  %89 = getelementptr inbounds i8, ptr %0, i32 %88
  %90 = load ptr, ptr %89, align 4, !tbaa !6
  %91 = icmp eq ptr %90, null
  %92 = icmp eq ptr %90, %0
  %93 = or i1 %91, %92
  br i1 %93, label %102, label %94

94:                                               ; preds = %87
  %95 = getelementptr inbounds i8, ptr %90, i32 %88
  store ptr null, ptr %95, align 4, !tbaa !6
  tail call fastcc void @notify_side(i32 noundef %6, ptr noundef nonnull %90, i64 noundef %3) #61
  br label %102

96:                                               ; preds = %48
  %97 = icmp eq ptr %5, null
  br i1 %97, label %102, label %98

98:                                               ; preds = %96
  %99 = load ptr, ptr %5, align 4, !tbaa !6
  %100 = icmp eq ptr %99, %0
  br i1 %100, label %101, label %102

101:                                              ; preds = %98
  store ptr null, ptr %5, align 4, !tbaa !6
  br label %102

102:                                              ; preds = %33, %75, %17, %55, %14, %49, %87, %94, %79, %84, %47, %37, %96, %98, %101
  ret void
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_address_map_new(i64 noundef %0) local_unnamed_addr #3 {
  %2 = tail call ptr @fa_alloc(i32 noundef 16, i32 noundef 8) #62
  %3 = shl nsw i64 %0, 1
  br label %4

4:                                                ; preds = %4, %1
  %5 = phi i64 [ 16, %1 ], [ %7, %4 ]
  %6 = icmp slt i64 %5, %3
  %7 = shl nsw i64 %5, 1
  br i1 %6, label %4, label %8, !llvm.loop !177

8:                                                ; preds = %4
  %9 = add nsw i64 %5, -1
  %10 = getelementptr inbounds %struct.FaAddressMap, ptr %2, i32 0, i32 2
  store i64 %9, ptr %10, align 8, !tbaa !178
  %11 = trunc i64 %5 to i32
  %12 = shl i32 %11, 2
  %13 = tail call ptr @fa_alloc(i32 noundef %12, i32 noundef 4) #62
  store ptr %13, ptr %2, align 8, !tbaa !180
  %14 = tail call ptr @fa_alloc(i32 noundef %12, i32 noundef 4) #62
  %15 = getelementptr inbounds %struct.FaAddressMap, ptr %2, i32 0, i32 1
  store ptr %14, ptr %15, align 4, !tbaa !181
  %16 = load ptr, ptr %2, align 8, !tbaa !180
  tail call void @llvm.memset.p0.i32(ptr align 4 %16, i8 0, i32 %12, i1 false)
  ret ptr %2
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden void @fa_address_map_put(ptr nocapture noundef readonly %0, ptr noundef %1, ptr noundef %2) local_unnamed_addr #20 {
  %4 = getelementptr inbounds %struct.FaAddressMap, ptr %0, i32 0, i32 2
  %5 = load i64, ptr %4, align 8, !tbaa !178
  %6 = ptrtoint ptr %1 to i32
  %7 = zext i32 %6 to i64
  %8 = mul i64 %7, -49064778989728563
  %9 = lshr i64 %8, 33
  %10 = xor i64 %9, %8
  %11 = load ptr, ptr %0, align 8, !tbaa !180
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
  br i1 %20, label %22, label %12, !llvm.loop !182

22:                                               ; preds = %12
  store ptr %1, ptr %16, align 4, !tbaa !6
  %23 = getelementptr inbounds %struct.FaAddressMap, ptr %0, i32 0, i32 1
  %24 = load ptr, ptr %23, align 4, !tbaa !181
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
  %6 = load i64, ptr %5, align 8, !tbaa !178
  %7 = ptrtoint ptr %1 to i32
  %8 = zext i32 %7 to i64
  %9 = mul i64 %8, -49064778989728563
  %10 = lshr i64 %9, 33
  %11 = xor i64 %10, %9
  %12 = load ptr, ptr %0, align 8, !tbaa !180
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
  br i1 %27, label %36, label %28, !llvm.loop !183

28:                                               ; preds = %20
  %29 = icmp eq ptr %26, %1
  br i1 %29, label %30, label %20, !llvm.loop !183

30:                                               ; preds = %28, %18
  %31 = phi i32 [ %14, %18 ], [ %24, %28 ]
  %32 = getelementptr inbounds %struct.FaAddressMap, ptr %0, i32 0, i32 1
  %33 = load ptr, ptr %32, align 4, !tbaa !181
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
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %3) #64
  store ptr null, ptr %3, align 4, !tbaa !6
  %4 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 2
  %5 = icmp eq ptr %0, null
  br i1 %5, label %12, label %6

6:                                                ; preds = %2
  %7 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 1
  %8 = load i64, ptr %4, align 8, !tbaa !159
  %9 = icmp sgt i64 %8, 0
  br i1 %9, label %14, label %10

10:                                               ; preds = %24, %6
  %11 = load ptr, ptr %3, align 4, !tbaa !6
  br label %12

12:                                               ; preds = %10, %2
  %13 = phi ptr [ %11, %10 ], [ null, %2 ]
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %3) #64
  ret ptr %13

14:                                               ; preds = %6, %24
  %15 = phi i64 [ %25, %24 ], [ 0, %6 ]
  %16 = load ptr, ptr %7, align 8, !tbaa !160
  %17 = trunc i64 %15 to i32
  %18 = getelementptr inbounds ptr, ptr %16, i32 %17
  %19 = load ptr, ptr %18, align 4, !tbaa !6
  %20 = tail call ptr @fa_address_map_get(ptr noundef %1, ptr noundef %19) #61
  %21 = icmp eq ptr %20, null
  br i1 %21, label %24, label %22

22:                                               ; preds = %14
  %23 = call i32 @fa_links_add(ptr noundef nonnull %3, ptr noundef nonnull %20) #61, !range !50
  br label %24

24:                                               ; preds = %22, %14
  %25 = add nuw nsw i64 %15, 1
  %26 = load i64, ptr %4, align 8, !tbaa !159
  %27 = icmp slt i64 %25, %26
  br i1 %27, label %14, label %10
}

; Function Attrs: nounwind optsize
define hidden void @fa_address_map_free(ptr noundef %0) local_unnamed_addr #3 {
  %2 = load ptr, ptr %0, align 8, !tbaa !180
  %3 = getelementptr inbounds %struct.FaAddressMap, ptr %0, i32 0, i32 2
  %4 = load i64, ptr %3, align 8, !tbaa !178
  %5 = trunc i64 %4 to i32
  %6 = shl i32 %5, 2
  %7 = add i32 %6, 4
  tail call void @fa_free(ptr noundef %2, i32 noundef %7, i32 noundef 4) #62
  %8 = getelementptr inbounds %struct.FaAddressMap, ptr %0, i32 0, i32 1
  %9 = load ptr, ptr %8, align 4, !tbaa !181
  %10 = load i64, ptr %3, align 8, !tbaa !178
  %11 = trunc i64 %10 to i32
  %12 = shl i32 %11, 2
  %13 = add i32 %12, 4
  tail call void @fa_free(ptr noundef %9, i32 noundef %13, i32 noundef 4) #62
  tail call void @fa_free(ptr noundef nonnull %0, i32 noundef 16, i32 noundef 8) #62
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
  %1 = tail call ptr @fa_alloc(i32 noundef 64, i32 noundef 8) #62
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(64) %1, i8 0, i64 64, i1 false)
  %2 = load ptr, ptr @reactive_made, align 4, !tbaa !6
  %3 = load i64, ptr @reactive_made_count, align 8, !tbaa !89
  %4 = tail call fastcc ptr @grow_items(ptr noundef %2, i64 noundef %3, ptr noundef nonnull @reactive_made_capacity, i32 noundef 4) #61
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
  %7 = load i64, ptr %6, align 8, !tbaa !184
  %8 = icmp sgt i64 %7, 0
  br i1 %8, label %9, label %28

9:                                                ; preds = %5
  %10 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 6
  %11 = load ptr, ptr %10, align 4, !tbaa !186
  br label %12

12:                                               ; preds = %9, %22
  %13 = phi i64 [ 0, %9 ], [ %23, %22 ]
  %14 = trunc i64 %13 to i32
  %15 = getelementptr inbounds %struct.FaChild, ptr %11, i32 %14
  %16 = load i64, ptr %15, align 8, !tbaa !171
  %17 = icmp eq i64 %16, %1
  br i1 %17, label %18, label %22

18:                                               ; preds = %12
  %19 = getelementptr inbounds %struct.FaChild, ptr %11, i32 %14, i32 1
  %20 = load i64, ptr %19, align 8, !tbaa !173
  %21 = icmp eq i64 %20, %2
  br i1 %21, label %25, label %22

22:                                               ; preds = %18, %12
  %23 = add nuw nsw i64 %13, 1
  %24 = icmp eq i64 %23, %7
  br i1 %24, label %28, label %12, !llvm.loop !187

25:                                               ; preds = %18
  %26 = getelementptr inbounds %struct.FaChild, ptr %11, i32 %14, i32 2
  %27 = load ptr, ptr %26, align 8, !tbaa !174
  br label %50

28:                                               ; preds = %22, %5
  %29 = tail call ptr @fa_alloc(i32 noundef 64, i32 noundef 8) #62
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(64) %29, i8 0, i32 64, i1 false)
  %30 = getelementptr inbounds %struct.FaSource, ptr %29, i32 0, i32 5
  store ptr %0, ptr %30, align 8, !tbaa !188
  %31 = load ptr, ptr @reactive_made, align 4, !tbaa !6
  %32 = load i64, ptr @reactive_made_count, align 8, !tbaa !89
  %33 = tail call fastcc ptr @grow_items(ptr noundef %31, i64 noundef %32, ptr noundef nonnull @reactive_made_capacity, i32 noundef 4) #61
  store ptr %33, ptr @reactive_made, align 4, !tbaa !6
  %34 = load i64, ptr @reactive_made_count, align 8, !tbaa !89
  %35 = trunc i64 %34 to i32
  %36 = getelementptr inbounds ptr, ptr %33, i32 %35
  store ptr %29, ptr %36, align 4, !tbaa !6
  %37 = add nsw i64 %34, 1
  store i64 %37, ptr @reactive_made_count, align 8, !tbaa !89
  %38 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 6
  %39 = load ptr, ptr %38, align 4, !tbaa !186
  %40 = load i64, ptr %6, align 8, !tbaa !184
  %41 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 8
  %42 = tail call fastcc ptr @grow_items(ptr noundef %39, i64 noundef %40, ptr noundef nonnull %41, i32 noundef 24) #61
  store ptr %42, ptr %38, align 4, !tbaa !186
  %43 = load i64, ptr %6, align 8, !tbaa !184
  %44 = trunc i64 %43 to i32
  %45 = getelementptr inbounds %struct.FaChild, ptr %42, i32 %44
  store i64 %1, ptr %45, align 8, !tbaa.struct !189
  %46 = getelementptr inbounds i8, ptr %45, i32 8
  store i64 %2, ptr %46, align 8, !tbaa.struct !190
  %47 = getelementptr inbounds i8, ptr %45, i32 16
  store ptr %29, ptr %47, align 8, !tbaa.struct !191
  %48 = load i64, ptr %6, align 8, !tbaa !184
  %49 = add nsw i64 %48, 1
  store i64 %49, ptr %6, align 8, !tbaa !184
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
  %13 = tail call ptr @fa_alloc(i32 noundef %12, i32 noundef 4) #62
  %14 = load i64, ptr %2, align 8, !tbaa !89
  %15 = icmp sgt i64 %14, 0
  br i1 %15, label %16, label %22

16:                                               ; preds = %7
  %17 = trunc i64 %1 to i32
  %18 = mul i32 %17, %3
  tail call fastcc void @copy_bytes(ptr noundef %13, ptr noundef %0, i32 noundef %18) #61
  %19 = load i64, ptr %2, align 8, !tbaa !89
  %20 = trunc i64 %19 to i32
  %21 = mul i32 %20, %3
  tail call void @fa_free(ptr noundef %0, i32 noundef %21, i32 noundef 4) #62
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
  %13 = load i64, ptr %12, align 8, !tbaa !171
  %14 = icmp eq i64 %13, %4
  br i1 %14, label %15, label %22

15:                                               ; preds = %9
  %16 = getelementptr inbounds %struct.FaChild, ptr %8, i32 %11, i32 1
  %17 = load i64, ptr %16, align 8, !tbaa !173
  %18 = icmp eq i64 %17, %1
  br i1 %18, label %19, label %22

19:                                               ; preds = %15
  %20 = getelementptr inbounds %struct.FaChild, ptr %8, i32 %11, i32 2
  %21 = load ptr, ptr %20, align 8, !tbaa !174
  br label %36

22:                                               ; preds = %15, %9
  %23 = add nuw nsw i64 %10, 1
  %24 = icmp eq i64 %23, %5
  br i1 %24, label %25, label %9, !llvm.loop !192

25:                                               ; preds = %22, %2
  %26 = tail call ptr @fa_source_new() #61
  %27 = load ptr, ptr @field_sources, align 4, !tbaa !6
  %28 = load i64, ptr @field_source_count, align 8, !tbaa !89
  %29 = tail call fastcc ptr @grow_items(ptr noundef %27, i64 noundef %28, ptr noundef nonnull @field_source_capacity, i32 noundef 24) #61
  store ptr %29, ptr @field_sources, align 4, !tbaa !6
  %30 = load i64, ptr @field_source_count, align 8, !tbaa !89
  %31 = trunc i64 %30 to i32
  %32 = getelementptr inbounds %struct.FaChild, ptr %29, i32 %31
  store i64 %4, ptr %32, align 8, !tbaa.struct !189
  %33 = getelementptr inbounds i8, ptr %32, i32 8
  store i64 %1, ptr %33, align 8, !tbaa.struct !190
  %34 = getelementptr inbounds i8, ptr %32, i32 16
  store ptr %26, ptr %34, align 8, !tbaa.struct !191
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
  %9 = load i64, ptr %8, align 8, !tbaa !193
  %10 = icmp sgt i64 %9, 0
  %11 = getelementptr inbounds %struct.FaObserver, ptr %3, i32 0, i32 6
  %12 = load ptr, ptr %11, align 4, !tbaa !195
  br i1 %10, label %16, label %22

13:                                               ; preds = %16
  %14 = add nuw nsw i64 %17, 1
  %15 = icmp eq i64 %14, %9
  br i1 %15, label %22, label %16, !llvm.loop !196

16:                                               ; preds = %7, %13
  %17 = phi i64 [ %14, %13 ], [ 0, %7 ]
  %18 = trunc i64 %17 to i32
  %19 = getelementptr inbounds ptr, ptr %12, i32 %18
  %20 = load ptr, ptr %19, align 4, !tbaa !6
  %21 = icmp eq ptr %20, %0
  br i1 %21, label %66, label %13

22:                                               ; preds = %13, %7
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %2) #64
  %23 = getelementptr inbounds %struct.FaObserver, ptr %3, i32 0, i32 9
  %24 = load i64, ptr %23, align 8, !tbaa !197
  store i64 %24, ptr %2, align 8, !tbaa !89
  %25 = getelementptr inbounds %struct.FaObserver, ptr %3, i32 0, i32 6
  %26 = call fastcc ptr @grow_items(ptr noundef %12, i64 noundef %9, ptr noundef nonnull %2, i32 noundef 4) #61
  store ptr %26, ptr %25, align 4, !tbaa !195
  %27 = load i64, ptr %23, align 8, !tbaa !197
  store i64 %27, ptr %2, align 8, !tbaa !89
  %28 = getelementptr inbounds %struct.FaObserver, ptr %3, i32 0, i32 7
  %29 = load ptr, ptr %28, align 8, !tbaa !198
  %30 = load i64, ptr %8, align 8, !tbaa !193
  %31 = call fastcc ptr @grow_items(ptr noundef %29, i64 noundef %30, ptr noundef nonnull %2, i32 noundef 8) #61
  store ptr %31, ptr %28, align 8, !tbaa !198
  %32 = load i64, ptr %2, align 8, !tbaa !89
  store i64 %32, ptr %23, align 8, !tbaa !197
  %33 = load ptr, ptr %25, align 4, !tbaa !195
  %34 = load i64, ptr %8, align 8, !tbaa !193
  %35 = trunc i64 %34 to i32
  %36 = getelementptr inbounds ptr, ptr %33, i32 %35
  store ptr %0, ptr %36, align 4, !tbaa !6
  %37 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 1
  %38 = load i64, ptr %37, align 8, !tbaa !199
  %39 = load ptr, ptr %28, align 8, !tbaa !198
  %40 = getelementptr inbounds i64, ptr %39, i32 %35
  store i64 %38, ptr %40, align 8, !tbaa !89
  %41 = load i64, ptr %8, align 8, !tbaa !193
  %42 = add nsw i64 %41, 1
  store i64 %42, ptr %8, align 8, !tbaa !193
  %43 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 3
  %44 = load i64, ptr %43, align 8, !tbaa !200
  %45 = icmp sgt i64 %44, 0
  %46 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 2
  %47 = load ptr, ptr %46, align 8, !tbaa !201
  br i1 %45, label %51, label %57

48:                                               ; preds = %51
  %49 = add nuw nsw i64 %52, 1
  %50 = icmp eq i64 %49, %44
  br i1 %50, label %57, label %51, !llvm.loop !202

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
  %60 = tail call fastcc ptr @grow_items(ptr noundef %47, i64 noundef %44, ptr noundef nonnull %59, i32 noundef 4) #61
  store ptr %60, ptr %58, align 8, !tbaa !201
  %61 = load i64, ptr %43, align 8, !tbaa !200
  %62 = trunc i64 %61 to i32
  %63 = getelementptr inbounds ptr, ptr %60, i32 %62
  store ptr %3, ptr %63, align 4, !tbaa !6
  %64 = add nsw i64 %61, 1
  store i64 %64, ptr %43, align 8, !tbaa !200
  br label %65

65:                                               ; preds = %51, %57
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %2) #64
  br label %66

66:                                               ; preds = %16, %1, %65
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_notify(ptr noundef %0) local_unnamed_addr #3 {
  %2 = icmp eq ptr %0, null
  br i1 %2, label %15, label %3

3:                                                ; preds = %1
  tail call fastcc void @bump(ptr noundef nonnull %0) #61
  %4 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 5
  %5 = load ptr, ptr %4, align 8, !tbaa !188
  %6 = icmp eq ptr %5, null
  br i1 %6, label %15, label %7

7:                                                ; preds = %3, %7
  %8 = phi ptr [ %13, %7 ], [ %5, %3 ]
  %9 = getelementptr inbounds %struct.FaSource, ptr %8, i32 0, i32 1
  %10 = load i64, ptr %9, align 8, !tbaa !199
  %11 = add nsw i64 %10, 1
  store i64 %11, ptr %9, align 8, !tbaa !199
  tail call fastcc void @mark_observers(ptr noundef nonnull %8) #61
  %12 = getelementptr inbounds %struct.FaSource, ptr %8, i32 0, i32 5
  %13 = load ptr, ptr %12, align 8, !tbaa !188
  %14 = icmp eq ptr %13, null
  br i1 %14, label %15, label %7, !llvm.loop !203

15:                                               ; preds = %7, %3, %1
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @bump(ptr nocapture noundef %0) unnamed_addr #3 {
  %2 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 1
  %3 = load i64, ptr %2, align 8, !tbaa !199
  %4 = add nsw i64 %3, 1
  store i64 %4, ptr %2, align 8, !tbaa !199
  tail call fastcc void @mark_observers(ptr noundef %0) #61
  %5 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 7
  %6 = load i64, ptr %5, align 8, !tbaa !184
  %7 = icmp sgt i64 %6, 0
  br i1 %7, label %8, label %10

8:                                                ; preds = %1
  %9 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 6
  br label %11

10:                                               ; preds = %11, %1
  ret void

11:                                               ; preds = %8, %11
  %12 = phi i64 [ 0, %8 ], [ %17, %11 ]
  %13 = load ptr, ptr %9, align 4, !tbaa !186
  %14 = trunc i64 %12 to i32
  %15 = getelementptr inbounds %struct.FaChild, ptr %13, i32 %14, i32 2
  %16 = load ptr, ptr %15, align 8, !tbaa !174
  tail call fastcc void @bump(ptr noundef %16) #61
  %17 = add nuw nsw i64 %12, 1
  %18 = load i64, ptr %5, align 8, !tbaa !184
  %19 = icmp slt i64 %17, %18
  br i1 %19, label %11, label %10, !llvm.loop !204
}

; Function Attrs: nounwind optsize
define internal fastcc void @mark_observers(ptr nocapture noundef readonly %0) unnamed_addr #3 {
  %2 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 3
  %3 = load i64, ptr %2, align 8, !tbaa !200
  %4 = icmp sgt i64 %3, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 2
  br label %8

7:                                                ; preds = %8, %1
  ret void

8:                                                ; preds = %5, %8
  %9 = phi i64 [ 0, %5 ], [ %14, %8 ]
  %10 = load ptr, ptr %6, align 8, !tbaa !201
  %11 = trunc i64 %9 to i32
  %12 = getelementptr inbounds ptr, ptr %10, i32 %11
  %13 = load ptr, ptr %12, align 4, !tbaa !6
  tail call fastcc void @mark(ptr noundef %13) #61
  %14 = add nuw nsw i64 %9, 1
  %15 = load i64, ptr %2, align 8, !tbaa !200
  %16 = icmp slt i64 %14, %15
  br i1 %16, label %8, label %7, !llvm.loop !205
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_derived_new(ptr noundef %0, ptr noundef %1, ptr noundef %2, i64 noundef %3) local_unnamed_addr #3 {
  %5 = tail call fastcc ptr @observer_new(ptr noundef %0, ptr noundef null, ptr noundef %1, ptr noundef %2, i64 noundef %3) #61
  store i64 1, ptr %5, align 8, !tbaa !206
  %6 = getelementptr inbounds %struct.FaObserver, ptr %5, i32 0, i32 11
  store i32 1, ptr %6, align 8, !tbaa !207
  ret ptr %5
}

; Function Attrs: nounwind optsize
define internal fastcc noundef ptr @observer_new(ptr noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef %3, i64 noundef %4) unnamed_addr #3 {
  %6 = tail call ptr @fa_alloc(i32 noundef 168, i32 noundef 8) #62
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(168) %6, i8 0, i32 168, i1 false)
  %7 = getelementptr inbounds %struct.FaObserver, ptr %6, i32 0, i32 3
  store ptr %0, ptr %7, align 8, !tbaa !208
  %8 = getelementptr inbounds %struct.FaObserver, ptr %6, i32 0, i32 4
  store ptr %1, ptr %8, align 4, !tbaa !209
  %9 = getelementptr inbounds %struct.FaObserver, ptr %6, i32 0, i32 5
  store ptr %2, ptr %9, align 8, !tbaa !210
  %10 = getelementptr inbounds %struct.FaObserver, ptr %6, i32 0, i32 2
  store ptr %3, ptr %10, align 4, !tbaa !211
  %11 = icmp sgt i64 %4, 0
  br i1 %11, label %12, label %17

12:                                               ; preds = %5
  %13 = trunc i64 %4 to i32
  %14 = tail call ptr @fa_alloc(i32 noundef %13, i32 noundef 16) #62
  %15 = getelementptr inbounds %struct.FaObserver, ptr %6, i32 0, i32 1
  store ptr %14, ptr %15, align 8, !tbaa !212
  tail call void @llvm.memset.p0.i32(ptr align 1 %14, i8 0, i32 %13, i1 false)
  %16 = getelementptr inbounds %struct.FaObserver, ptr %6, i32 0, i32 19
  store i64 %4, ptr %16, align 8, !tbaa !213
  br label %17

17:                                               ; preds = %12, %5
  %18 = load ptr, ptr @reactive_made, align 4, !tbaa !6
  %19 = load i64, ptr @reactive_made_count, align 8, !tbaa !89
  %20 = tail call fastcc ptr @grow_items(ptr noundef %18, i64 noundef %19, ptr noundef nonnull @reactive_made_capacity, i32 noundef 4) #61
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
  tail call fastcc void @refresh(ptr noundef %0) #61
  tail call void @fa_track(ptr noundef %0) #61
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 1
  %3 = load ptr, ptr %2, align 8, !tbaa !212
  ret ptr %3
}

; Function Attrs: nounwind optsize
define internal fastcc void @refresh(ptr noundef %0) unnamed_addr #3 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 11
  %3 = load i32, ptr %2, align 8, !tbaa !207
  %4 = icmp eq i32 %3, 0
  br i1 %4, label %29, label %5

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 12
  %7 = load i32, ptr %6, align 4, !tbaa !214
  %8 = icmp eq i32 %7, 0
  br i1 %8, label %13, label %9

9:                                                ; preds = %5
  %10 = tail call fastcc i32 @outdated(ptr noundef nonnull %0) #61
  %11 = icmp eq i32 %10, 0
  br i1 %11, label %12, label %13

12:                                               ; preds = %9
  store i32 0, ptr %2, align 8, !tbaa !207
  br label %29

13:                                               ; preds = %9, %5
  %14 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 13
  %15 = load i32, ptr %14, align 8, !tbaa !215
  %16 = icmp eq i32 %15, 0
  br i1 %16, label %18, label %17

17:                                               ; preds = %13
  tail call fastcc void @reactive_stop(ptr noundef nonnull @.str.42) #65
  unreachable

18:                                               ; preds = %13
  tail call fastcc void @unsubscribe(ptr noundef nonnull %0) #61
  %19 = load ptr, ptr @reactive_observer, align 4, !tbaa !6
  store ptr %0, ptr @reactive_observer, align 4, !tbaa !6
  store i32 1, ptr %14, align 8, !tbaa !215
  %20 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 16
  store i32 1, ptr %20, align 4, !tbaa !216
  %21 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 3
  %22 = load ptr, ptr %21, align 8, !tbaa !208
  tail call void %22(ptr noundef nonnull %0) #62
  store i32 0, ptr %14, align 8, !tbaa !215
  store ptr %19, ptr @reactive_observer, align 4, !tbaa !6
  store i32 0, ptr %2, align 8, !tbaa !207
  store i32 1, ptr %6, align 4, !tbaa !214
  %23 = load i32, ptr %20, align 4, !tbaa !216
  %24 = icmp eq i32 %23, 0
  br i1 %24, label %29, label %25

25:                                               ; preds = %18
  %26 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 1
  %27 = load i64, ptr %26, align 8, !tbaa !217
  %28 = add nsw i64 %27, 1
  store i64 %28, ptr %26, align 8, !tbaa !217
  br label %29

29:                                               ; preds = %18, %25, %1, %12
  ret void
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_watch_new(ptr noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef %3) local_unnamed_addr #3 {
  %5 = tail call fastcc ptr @observer_new(ptr noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef %3, i64 noundef 0) #61
  store i64 2, ptr %5, align 8, !tbaa !206
  %6 = getelementptr inbounds %struct.FaObserver, ptr %5, i32 0, i32 17
  store i32 1, ptr %6, align 8, !tbaa !218
  %7 = load i64, ptr @reactive_sequence, align 8, !tbaa !89
  %8 = add nsw i64 %7, 1
  store i64 %8, ptr @reactive_sequence, align 8, !tbaa !89
  %9 = getelementptr inbounds %struct.FaObserver, ptr %5, i32 0, i32 10
  store i64 %8, ptr %9, align 8, !tbaa !219
  tail call fastcc void @unsubscribe(ptr noundef nonnull %5) #61
  %10 = load ptr, ptr @reactive_observer, align 4, !tbaa !6
  store ptr null, ptr @reactive_observer, align 4, !tbaa !6
  %11 = getelementptr inbounds %struct.FaObserver, ptr %5, i32 0, i32 4
  %12 = load ptr, ptr %11, align 4, !tbaa !209
  tail call void %12(ptr noundef nonnull %5) #62
  store ptr %5, ptr @reactive_observer, align 4, !tbaa !6
  %13 = getelementptr inbounds %struct.FaObserver, ptr %5, i32 0, i32 3
  %14 = load ptr, ptr %13, align 8, !tbaa !208
  tail call void %14(ptr noundef nonnull %5) #62
  store ptr %10, ptr @reactive_observer, align 4, !tbaa !6
  ret ptr %5
}

; Function Attrs: nounwind optsize
define hidden void @fa_dispose(ptr noundef %0) local_unnamed_addr #3 {
  %2 = icmp eq ptr %0, null
  br i1 %2, label %11, label %3

3:                                                ; preds = %1
  %4 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 15
  %5 = load i32, ptr %4, align 8, !tbaa !220
  %6 = icmp eq i32 %5, 0
  br i1 %6, label %7, label %11

7:                                                ; preds = %3
  store i32 1, ptr %4, align 8, !tbaa !220
  tail call fastcc void @unsubscribe(ptr noundef nonnull %0) #61
  %8 = load ptr, ptr @reactive_observer, align 4, !tbaa !6
  store ptr null, ptr @reactive_observer, align 4, !tbaa !6
  %9 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 4
  %10 = load ptr, ptr %9, align 4, !tbaa !209
  tail call void %10(ptr noundef nonnull %0) #62
  store ptr %8, ptr @reactive_observer, align 4, !tbaa !6
  br label %11

11:                                               ; preds = %1, %3, %7
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc void @unsubscribe(ptr noundef %0) unnamed_addr #20 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 8
  %3 = load i64, ptr %2, align 8, !tbaa !193
  %4 = icmp sgt i64 %3, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 6
  br label %8

7:                                                ; preds = %35, %1
  store i64 0, ptr %2, align 8, !tbaa !193
  ret void

8:                                                ; preds = %5, %35
  %9 = phi i64 [ 0, %5 ], [ %36, %35 ]
  %10 = load ptr, ptr %6, align 4, !tbaa !195
  %11 = trunc i64 %9 to i32
  %12 = getelementptr inbounds ptr, ptr %10, i32 %11
  %13 = load ptr, ptr %12, align 4, !tbaa !6
  %14 = getelementptr inbounds %struct.FaSource, ptr %13, i32 0, i32 3
  %15 = load i64, ptr %14, align 8, !tbaa !200
  %16 = icmp sgt i64 %15, 0
  br i1 %16, label %17, label %35

17:                                               ; preds = %8
  %18 = getelementptr inbounds %struct.FaSource, ptr %13, i32 0, i32 2
  %19 = load ptr, ptr %18, align 8, !tbaa !201
  br label %23

20:                                               ; preds = %23
  %21 = add nuw nsw i64 %24, 1
  %22 = icmp eq i64 %21, %15
  br i1 %22, label %35, label %23, !llvm.loop !221

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
  store i64 %34, ptr %14, align 8, !tbaa !200
  br label %35

35:                                               ; preds = %20, %8, %29
  %36 = add nuw nsw i64 %9, 1
  %37 = icmp eq i64 %36, %3
  br i1 %37, label %7, label %8, !llvm.loop !222
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
  tail call fastcc void @reactive_stop(ptr noundef nonnull @.str.15) #65
  unreachable

13:                                               ; preds = %7
  %14 = trunc i64 %9 to i32
  %15 = shl i32 %14, 2
  %16 = tail call ptr @fa_alloc(i32 noundef %15, i32 noundef 4) #62
  %17 = load ptr, ptr @reactive_pending, align 4, !tbaa !6
  tail call fastcc void @copy_bytes(ptr noundef %16, ptr noundef %17, i32 noundef %15) #61
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
  %26 = load i64, ptr %25, align 8, !tbaa !219
  br label %27

27:                                               ; preds = %20, %36
  %28 = phi i64 [ %21, %20 ], [ %29, %36 ]
  %29 = add nsw i64 %28, -1
  %30 = trunc i64 %29 to i32
  %31 = getelementptr inbounds ptr, ptr %16, i32 %30
  %32 = load ptr, ptr %31, align 4, !tbaa !6
  %33 = getelementptr inbounds %struct.FaObserver, ptr %32, i32 0, i32 10
  %34 = load i64, ptr %33, align 8, !tbaa !219
  %35 = icmp sgt i64 %34, %26
  br i1 %35, label %36, label %40

36:                                               ; preds = %27
  %37 = trunc i64 %28 to i32
  %38 = getelementptr inbounds ptr, ptr %16, i32 %37
  store ptr %32, ptr %38, align 4, !tbaa !6
  %39 = icmp sgt i64 %28, 1
  br i1 %39, label %27, label %40, !llvm.loop !223

40:                                               ; preds = %36, %27
  %41 = phi i64 [ 0, %36 ], [ %28, %27 ]
  %42 = trunc i64 %41 to i32
  %43 = getelementptr inbounds ptr, ptr %16, i32 %42
  store ptr %24, ptr %43, align 4, !tbaa !6
  %44 = add nuw nsw i64 %21, 1
  %45 = icmp eq i64 %44, %9
  br i1 %45, label %19, label %20, !llvm.loop !224

46:                                               ; preds = %67
  tail call void @fa_free(ptr noundef nonnull %16, i32 noundef %15, i32 noundef 4) #62
  %47 = load i64, ptr @reactive_pending_count, align 8, !tbaa !89
  %48 = icmp sgt i64 %47, 0
  br i1 %48, label %7, label %70, !llvm.loop !225

49:                                               ; preds = %19, %67
  %50 = phi i64 [ %68, %67 ], [ 0, %19 ]
  %51 = trunc i64 %50 to i32
  %52 = getelementptr inbounds ptr, ptr %16, i32 %51
  %53 = load ptr, ptr %52, align 4, !tbaa !6
  %54 = getelementptr inbounds %struct.FaObserver, ptr %53, i32 0, i32 14
  store i32 0, ptr %54, align 4, !tbaa !226
  %55 = getelementptr inbounds %struct.FaObserver, ptr %53, i32 0, i32 15
  %56 = load i32, ptr %55, align 8, !tbaa !220
  %57 = icmp eq i32 %56, 0
  br i1 %57, label %58, label %67

58:                                               ; preds = %49
  %59 = tail call fastcc i32 @outdated(ptr noundef nonnull %53) #61
  %60 = icmp eq i32 %59, 0
  br i1 %60, label %67, label %61

61:                                               ; preds = %58
  tail call fastcc void @unsubscribe(ptr noundef nonnull %53) #61
  %62 = load ptr, ptr @reactive_observer, align 4, !tbaa !6
  store ptr null, ptr @reactive_observer, align 4, !tbaa !6
  %63 = getelementptr inbounds %struct.FaObserver, ptr %53, i32 0, i32 4
  %64 = load ptr, ptr %63, align 4, !tbaa !209
  tail call void %64(ptr noundef nonnull %53) #62
  store ptr %53, ptr @reactive_observer, align 4, !tbaa !6
  %65 = getelementptr inbounds %struct.FaObserver, ptr %53, i32 0, i32 3
  %66 = load ptr, ptr %65, align 8, !tbaa !208
  tail call void %66(ptr noundef nonnull %53) #62
  store ptr %62, ptr @reactive_observer, align 4, !tbaa !6
  br label %67

67:                                               ; preds = %61, %58, %49
  %68 = add nuw i64 %50, 1
  %69 = icmp eq i64 %68, %9
  br i1 %69, label %46, label %49, !llvm.loop !227

70:                                               ; preds = %46, %5
  store i1 false, ptr @reactive_flushing, align 4
  br label %71

71:                                               ; preds = %0, %70
  ret void
}

; Function Attrs: noreturn nounwind optsize
define internal fastcc void @reactive_stop(ptr noundef %0) unnamed_addr #5 {
  tail call void @fa_flush() #61
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.43, i32 noundef 6) #62
  br label %2

2:                                                ; preds = %2, %1
  %3 = phi i32 [ 0, %1 ], [ %7, %2 ]
  %4 = getelementptr inbounds i8, ptr %0, i32 %3
  %5 = load i8, ptr %4, align 1, !tbaa !1
  %6 = icmp eq i8 %5, 0
  %7 = add i32 %3, 1
  br i1 %6, label %8, label %2, !llvm.loop !28

8:                                                ; preds = %2
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull %0, i32 noundef %3) #62
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.22, i32 noundef 1) #62
  tail call void @fa_exit(i32 noundef 101) #63
  unreachable
}

; Function Attrs: nounwind optsize
define internal fastcc noundef i32 @outdated(ptr nocapture noundef readonly %0) unnamed_addr #3 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 8
  %3 = load i64, ptr %2, align 8, !tbaa !193
  %4 = icmp sgt i64 %3, 0
  br i1 %4, label %5, label %30

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 6
  %7 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 7
  br label %12

8:                                                ; preds = %21
  %9 = add nuw nsw i64 %13, 1
  %10 = load i64, ptr %2, align 8, !tbaa !193
  %11 = icmp slt i64 %9, %10
  br i1 %11, label %12, label %28, !llvm.loop !228

12:                                               ; preds = %5, %8
  %13 = phi i64 [ 0, %5 ], [ %9, %8 ]
  %14 = load ptr, ptr %6, align 4, !tbaa !195
  %15 = trunc i64 %13 to i32
  %16 = getelementptr inbounds ptr, ptr %14, i32 %15
  %17 = load ptr, ptr %16, align 4, !tbaa !6
  %18 = load i64, ptr %17, align 8, !tbaa !229
  %19 = icmp eq i64 %18, 1
  br i1 %19, label %20, label %21

20:                                               ; preds = %12
  tail call fastcc void @refresh(ptr noundef nonnull %17) #61
  br label %21

21:                                               ; preds = %20, %12
  %22 = getelementptr inbounds %struct.FaSource, ptr %17, i32 0, i32 1
  %23 = load i64, ptr %22, align 8, !tbaa !199
  %24 = load ptr, ptr %7, align 8, !tbaa !198
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
  %3 = load ptr, ptr %2, align 8, !tbaa !212
  ret ptr %3
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read)
define hidden ptr @fa_observer_env(ptr nocapture noundef readonly %0) local_unnamed_addr #37 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 2
  %3 = load ptr, ptr %2, align 4, !tbaa !211
  ret ptr %3
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(none)
define hidden nonnull ptr @fa_observer_cleanup(ptr noundef readnone %0) local_unnamed_addr #23 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 20
  ret ptr %2
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read)
define hidden i32 @fa_observer_computed(ptr nocapture noundef readonly %0) local_unnamed_addr #37 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 12
  %3 = load i32, ptr %2, align 4, !tbaa !214
  ret i32 %3
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: write)
define hidden void @fa_observer_changed(ptr nocapture noundef writeonly %0, i32 noundef %1) local_unnamed_addr #24 {
  %3 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 16
  store i32 %1, ptr %3, align 4, !tbaa !216
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
  tail call void @fa_free(ptr noundef %7, i32 noundef %9, i32 noundef 4) #62
  br label %10

10:                                               ; preds = %3, %6
  %11 = load i64, ptr @reactive_pending_capacity, align 8, !tbaa !89
  %12 = icmp sgt i64 %11, 0
  br i1 %12, label %13, label %17

13:                                               ; preds = %10
  %14 = load ptr, ptr @reactive_pending, align 4, !tbaa !6
  %15 = trunc i64 %11 to i32
  %16 = shl i32 %15, 2
  tail call void @fa_free(ptr noundef %14, i32 noundef %16, i32 noundef 4) #62
  br label %17

17:                                               ; preds = %10, %13
  %18 = load i64, ptr @field_source_capacity, align 8, !tbaa !89
  %19 = icmp sgt i64 %18, 0
  br i1 %19, label %20, label %24

20:                                               ; preds = %17
  %21 = load ptr, ptr @field_sources, align 4, !tbaa !6
  %22 = trunc i64 %18 to i32
  %23 = mul i32 %22, 24
  tail call void @fa_free(ptr noundef %21, i32 noundef %23, i32 noundef 4) #62
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
  %33 = load i64, ptr %32, align 8, !tbaa !230
  %34 = icmp sgt i64 %33, 0
  br i1 %34, label %35, label %40

35:                                               ; preds = %25
  %36 = getelementptr inbounds %struct.FaSource, ptr %31, i32 0, i32 2
  %37 = load ptr, ptr %36, align 8, !tbaa !201
  %38 = trunc i64 %33 to i32
  %39 = shl i32 %38, 2
  tail call void @fa_free(ptr noundef %37, i32 noundef %39, i32 noundef 4) #62
  br label %40

40:                                               ; preds = %25, %35
  %41 = getelementptr inbounds %struct.FaSource, ptr %31, i32 0, i32 8
  %42 = load i64, ptr %41, align 8, !tbaa !231
  %43 = icmp sgt i64 %42, 0
  br i1 %43, label %44, label %49

44:                                               ; preds = %40
  %45 = getelementptr inbounds %struct.FaSource, ptr %31, i32 0, i32 6
  %46 = load ptr, ptr %45, align 4, !tbaa !186
  %47 = trunc i64 %42 to i32
  %48 = mul i32 %47, 24
  tail call void @fa_free(ptr noundef %46, i32 noundef %48, i32 noundef 4) #62
  br label %49

49:                                               ; preds = %40, %44
  %50 = load i64, ptr %31, align 8, !tbaa !229
  %51 = icmp eq i64 %50, 0
  br i1 %51, label %78, label %52

52:                                               ; preds = %49
  %53 = getelementptr inbounds %struct.FaObserver, ptr %31, i32 0, i32 5
  %54 = load ptr, ptr %53, align 8, !tbaa !210
  tail call void %54(ptr noundef nonnull %31) #62
  %55 = getelementptr inbounds %struct.FaObserver, ptr %31, i32 0, i32 9
  %56 = load i64, ptr %55, align 8, !tbaa !197
  %57 = icmp sgt i64 %56, 0
  br i1 %57, label %58, label %70

58:                                               ; preds = %52
  %59 = getelementptr inbounds %struct.FaObserver, ptr %31, i32 0, i32 6
  %60 = load ptr, ptr %59, align 4, !tbaa !195
  %61 = trunc i64 %56 to i32
  %62 = shl i32 %61, 2
  tail call void @fa_free(ptr noundef %60, i32 noundef %62, i32 noundef 4) #62
  %63 = load i64, ptr %55, align 8, !tbaa !197
  %64 = icmp sgt i64 %63, 0
  br i1 %64, label %65, label %70

65:                                               ; preds = %58
  %66 = getelementptr inbounds %struct.FaObserver, ptr %31, i32 0, i32 7
  %67 = load ptr, ptr %66, align 8, !tbaa !198
  %68 = trunc i64 %63 to i32
  %69 = shl i32 %68, 3
  tail call void @fa_free(ptr noundef %67, i32 noundef %69, i32 noundef 4) #62
  br label %70

70:                                               ; preds = %52, %58, %65
  %71 = getelementptr inbounds %struct.FaObserver, ptr %31, i32 0, i32 19
  %72 = load i64, ptr %71, align 8, !tbaa !213
  %73 = icmp sgt i64 %72, 0
  br i1 %73, label %74, label %78

74:                                               ; preds = %70
  %75 = getelementptr inbounds %struct.FaObserver, ptr %31, i32 0, i32 1
  %76 = load ptr, ptr %75, align 8, !tbaa !212
  %77 = trunc i64 %72 to i32
  tail call void @fa_free(ptr noundef %76, i32 noundef %77, i32 noundef 16) #62
  br label %78

78:                                               ; preds = %70, %74, %49
  %79 = phi i32 [ 64, %49 ], [ 168, %74 ], [ 168, %70 ]
  tail call void @fa_free(ptr noundef nonnull %31, i32 noundef %79, i32 noundef 8) #62
  %80 = icmp ugt i64 %26, 1
  br i1 %80, label %25, label %3, !llvm.loop !232
}

; Function Attrs: cold noinline noreturn nounwind optsize
define hidden void @fa_overflow(i64 noundef %0, i64 noundef %1, i32 noundef %2, ptr noundef %3) local_unnamed_addr #38 {
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  store i32 0, ptr @builder_simple, align 4, !tbaa !4
  tail call fastcc void @append_c(ptr noundef nonnull @.str.16) #61
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
  tail call fastcc void @append_u128(i128 noundef %13, i32 noundef %15) #61
  br label %17

16:                                               ; preds = %4
  tail call fastcc void @append_u128(i128 noundef %9, i32 noundef 0) #61
  br label %17

17:                                               ; preds = %16, %10
  tail call fastcc void @append_c(ptr noundef nonnull @.str.17) #61
  tail call fastcc void @append_c(ptr noundef %3) #61
  br label %18

18:                                               ; preds = %18, %17
  %19 = phi i32 [ 0, %17 ], [ %23, %18 ]
  %20 = getelementptr inbounds i8, ptr %3, i32 %19
  %21 = load i8, ptr %20, align 1, !tbaa !1
  %22 = icmp eq i8 %21, 0
  %23 = add i32 %19, 1
  br i1 %22, label %24, label %18, !llvm.loop !28

24:                                               ; preds = %18
  %25 = getelementptr inbounds i8, ptr %20, i32 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %25) #65
  unreachable
}

; Function Attrs: noreturn nounwind optsize
define internal fastcc void @stop_with_builder(ptr noundef %0) unnamed_addr #5 {
  tail call void @fa_flush() #61
  %2 = load ptr, ptr @builder, align 4, !tbaa !6
  %3 = load i32, ptr @builder_length, align 4, !tbaa !4
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.43, i32 noundef 6) #62
  tail call void @fa_write(i32 noundef 2, ptr noundef %2, i32 noundef %3) #62
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.44, i32 noundef 6) #62
  br label %4

4:                                                ; preds = %4, %1
  %5 = phi i32 [ 0, %1 ], [ %9, %4 ]
  %6 = getelementptr inbounds i8, ptr %0, i32 %5
  %7 = load i8, ptr %6, align 1, !tbaa !1
  %8 = icmp eq i8 %7, 0
  %9 = add i32 %5, 1
  br i1 %8, label %10, label %4, !llvm.loop !28

10:                                               ; preds = %4
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull %0, i32 noundef %5) #62
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.22, i32 noundef 1) #62
  tail call void @fa_exit(i32 noundef 101) #63
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define hidden void @fa_overflow_unknown(ptr noundef %0) local_unnamed_addr #38 {
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  store i32 0, ptr @builder_simple, align 4, !tbaa !4
  tail call fastcc void @append_c(ptr noundef nonnull @.str.18) #61
  tail call fastcc void @append_c(ptr noundef %0) #61
  br label %2

2:                                                ; preds = %2, %1
  %3 = phi i32 [ 0, %1 ], [ %7, %2 ]
  %4 = getelementptr inbounds i8, ptr %0, i32 %3
  %5 = load i8, ptr %4, align 1, !tbaa !1
  %6 = icmp eq i8 %5, 0
  %7 = add i32 %3, 1
  br i1 %6, label %8, label %2, !llvm.loop !28

8:                                                ; preds = %2
  %9 = getelementptr inbounds i8, ptr %4, i32 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %9) #65
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define hidden void @fa_float_overflow(double noundef %0, ptr noundef %1) local_unnamed_addr #38 {
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  store i32 0, ptr @builder_simple, align 4, !tbaa !4
  tail call fastcc void @append_c(ptr noundef nonnull @.str.16) #61
  tail call void @fa_b_f64(double noundef %0) #61
  tail call fastcc void @append_c(ptr noundef nonnull @.str.17) #61
  tail call fastcc void @append_c(ptr noundef %1) #61
  br label %3

3:                                                ; preds = %3, %2
  %4 = phi i32 [ 0, %2 ], [ %8, %3 ]
  %5 = getelementptr inbounds i8, ptr %1, i32 %4
  %6 = load i8, ptr %5, align 1, !tbaa !1
  %7 = icmp eq i8 %6, 0
  %8 = add i32 %4, 1
  br i1 %7, label %9, label %3, !llvm.loop !28

9:                                                ; preds = %3
  %10 = getelementptr inbounds i8, ptr %5, i32 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %10) #65
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define hidden void @fa_assert_failed(ptr noundef %0) local_unnamed_addr #38 {
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  store i32 0, ptr @builder_simple, align 4, !tbaa !4
  tail call fastcc void @append_c(ptr noundef nonnull @.str.19) #61
  tail call fastcc void @append_c(ptr noundef %0) #61
  br label %2

2:                                                ; preds = %2, %1
  %3 = phi i32 [ 0, %1 ], [ %7, %2 ]
  %4 = getelementptr inbounds i8, ptr %0, i32 %3
  %5 = load i8, ptr %4, align 1, !tbaa !1
  %6 = icmp eq i8 %5, 0
  %7 = add i32 %3, 1
  br i1 %6, label %8, label %2, !llvm.loop !28

8:                                                ; preds = %2
  %9 = getelementptr inbounds i8, ptr %4, i32 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %9) #65
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define hidden void @fa_assert_failed_operands(ptr noundef %0) local_unnamed_addr #38 {
  %2 = alloca %struct.FaString, align 4
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %2) #64
  call void @fa_b_take(ptr noundef nonnull %2) #61
  tail call fastcc void @append_c(ptr noundef nonnull @.str.19) #61
  tail call fastcc void @append_c(ptr noundef %0) #61
  tail call fastcc void @append_c(ptr noundef nonnull @.str.20) #61
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
  call void @fa_b_bytes(ptr noundef %12, i32 noundef %13) #61
  br label %14

14:                                               ; preds = %14, %1
  %15 = phi i32 [ 0, %1 ], [ %19, %14 ]
  %16 = getelementptr inbounds i8, ptr %0, i32 %15
  %17 = load i8, ptr %16, align 1, !tbaa !1
  %18 = icmp eq i8 %17, 0
  %19 = add i32 %15, 1
  br i1 %18, label %20, label %14, !llvm.loop !28

20:                                               ; preds = %14
  %21 = getelementptr inbounds i8, ptr %16, i32 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %21) #65
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define hidden void @fa_uncaught(ptr noundef %0) local_unnamed_addr #38 {
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  store i32 0, ptr @builder_simple, align 4, !tbaa !4
  tail call void @fa_format_union(ptr noundef %0, i32 noundef 1) #62
  tail call void @fa_flush() #61
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.21, i32 noundef 7) #62
  %2 = load ptr, ptr @builder, align 4, !tbaa !6
  %3 = load i32, ptr @builder_length, align 4, !tbaa !4
  tail call void @fa_write(i32 noundef 2, ptr noundef %2, i32 noundef %3) #62
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.22, i32 noundef 1) #62
  tail call void @fa_exit(i32 noundef 1) #63
  unreachable
}

; Function Attrs: optsize
declare void @fa_format_union(ptr noundef, i32 noundef) local_unnamed_addr #4

; Function Attrs: cold mustprogress nofree noinline norecurse nosync nounwind optsize willreturn memory(none)
define hidden noundef nonnull ptr @fa_error_division_by_zero() local_unnamed_addr #39 {
  ret ptr @division_by_zero
}

; Function Attrs: cold noinline nounwind optsize
define hidden noundef ptr @fa_error_conversion(i64 noundef %0, i64 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3) local_unnamed_addr #22 {
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  store i32 0, ptr @builder_simple, align 4, !tbaa !4
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
  tail call fastcc void @append_u128(i128 noundef %13, i32 noundef %15) #61
  br label %17

16:                                               ; preds = %4
  tail call fastcc void @append_u128(i128 noundef %9, i32 noundef 0) #61
  br label %17

17:                                               ; preds = %16, %10
  tail call fastcc void @append_c(ptr noundef nonnull @.str.17) #61
  tail call fastcc void @append_c(ptr noundef %3) #61
  %18 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #62
  store i64 4, ptr %18, align 8, !tbaa !233
  %19 = getelementptr inbounds %struct.MessageError, ptr %18, i32 0, i32 1
  tail call void @fa_b_take(ptr noundef nonnull %19) #61
  ret ptr %18
}

; Function Attrs: cold noinline nounwind optsize
define hidden noundef ptr @fa_error_negative_exponent(i64 noundef %0) local_unnamed_addr #22 {
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  store i32 0, ptr @builder_simple, align 4, !tbaa !4
  tail call fastcc void @append_c(ptr noundef nonnull @.str.23) #61
  %2 = tail call i64 @llvm.abs.i64(i64 %0, i1 false)
  %3 = lshr i64 %0, 63
  %4 = trunc i64 %3 to i32
  tail call fastcc void @append_u64(i64 noundef %2, i32 noundef %4) #61
  %5 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #62
  store i64 4, ptr %5, align 8, !tbaa !233
  %6 = getelementptr inbounds %struct.MessageError, ptr %5, i32 0, i32 1
  tail call void @fa_b_take(ptr noundef nonnull %6) #61
  ret ptr %5
}

; Function Attrs: cold noinline nounwind optsize
define hidden noundef ptr @fa_error_float(double noundef %0, i32 noundef %1) local_unnamed_addr #22 {
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  store i32 0, ptr @builder_simple, align 4, !tbaa !4
  tail call void @fa_b_f64(double noundef %0) #61
  %3 = icmp eq i32 %1, 0
  %4 = select i1 %3, ptr @.str.25, ptr @.str.24
  tail call fastcc void @append_c(ptr noundef nonnull %4) #61
  %5 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #62
  store i64 4, ptr %5, align 8, !tbaa !233
  %6 = getelementptr inbounds %struct.MessageError, ptr %5, i32 0, i32 1
  tail call void @fa_b_take(ptr noundef nonnull %6) #61
  ret ptr %5
}

; Function Attrs: cold noinline nounwind optsize
define hidden noundef ptr @fa_error_shift(i64 noundef %0, i64 noundef %1) local_unnamed_addr #22 {
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  store i32 0, ptr @builder_simple, align 4, !tbaa !4
  tail call fastcc void @append_c(ptr noundef nonnull @.str.26) #61
  %3 = tail call i64 @llvm.abs.i64(i64 %0, i1 false)
  %4 = lshr i64 %0, 63
  %5 = trunc i64 %4 to i32
  tail call fastcc void @append_u64(i64 noundef %3, i32 noundef %5) #61
  tail call fastcc void @append_c(ptr noundef nonnull @.str.27) #61
  %6 = tail call i64 @llvm.abs.i64(i64 %1, i1 false)
  %7 = lshr i64 %1, 63
  %8 = trunc i64 %7 to i32
  tail call fastcc void @append_u64(i64 noundef %6, i32 noundef %8) #61
  %9 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #62
  store i64 4, ptr %9, align 8, !tbaa !233
  %10 = getelementptr inbounds %struct.MessageError, ptr %9, i32 0, i32 1
  tail call void @fa_b_take(ptr noundef nonnull %10) #61
  ret ptr %9
}

; Function Attrs: cold noinline nounwind optsize
define hidden noundef ptr @fa_error_checked(i64 noundef %0, i64 noundef %1, ptr nocapture noundef readonly %2) local_unnamed_addr #22 {
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  store i32 0, ptr @builder_simple, align 4, !tbaa !4
  tail call fastcc void @append_c(ptr noundef nonnull @.str.16) #61
  %4 = zext i64 %1 to i128
  %5 = shl nuw i128 %4, 64
  %6 = zext i64 %0 to i128
  %7 = or disjoint i128 %5, %6
  %8 = icmp slt i64 %1, 0
  %9 = sub i128 0, %7
  %10 = select i1 %8, i128 %9, i128 %7
  %11 = lshr i64 %1, 63
  %12 = trunc i64 %11 to i32
  tail call fastcc void @append_u128(i128 noundef %10, i32 noundef %12) #61
  tail call fastcc void @append_c(ptr noundef nonnull @.str.17) #61
  tail call fastcc void @append_c(ptr noundef %2) #61
  %13 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #62
  store i64 4, ptr %13, align 8, !tbaa !233
  %14 = getelementptr inbounds %struct.MessageError, ptr %13, i32 0, i32 1
  tail call void @fa_b_take(ptr noundef nonnull %14) #61
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
  store i32 0, ptr @builder_simple, align 4, !tbaa !4
  br label %54

14:                                               ; preds = %22
  %15 = icmp eq i32 %29, 0
  br i1 %15, label %16, label %17

16:                                               ; preds = %14
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  store i32 0, ptr @builder_simple, align 4, !tbaa !4
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
  br i1 %31, label %14, label %22, !llvm.loop !235

32:                                               ; preds = %9
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  store i32 0, ptr @builder_simple, align 4, !tbaa !4
  br i1 %4, label %54, label %33

33:                                               ; preds = %16, %32
  tail call fastcc void @reserve(i32 noundef 1) #61
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
  tail call fastcc void @reserve(i32 noundef 1) #61
  %43 = load ptr, ptr @builder, align 4, !tbaa !6
  %44 = load i32, ptr @builder_length, align 4, !tbaa !4
  %45 = getelementptr inbounds i8, ptr %43, i32 %44
  store i8 92, ptr %45, align 1, !tbaa !1
  %46 = add i32 %44, 1
  store i32 %46, ptr @builder_length, align 4, !tbaa !4
  br label %47

47:                                               ; preds = %38, %42
  tail call fastcc void @reserve(i32 noundef 1) #61
  %48 = load ptr, ptr @builder, align 4, !tbaa !6
  %49 = load i32, ptr @builder_length, align 4, !tbaa !4
  %50 = getelementptr inbounds i8, ptr %48, i32 %49
  store i8 %41, ptr %50, align 1, !tbaa !1
  %51 = add i32 %49, 1
  store i32 %51, ptr @builder_length, align 4, !tbaa !4
  %52 = add nuw i32 %39, 1
  %53 = icmp eq i32 %52, %1
  br i1 %53, label %54, label %38, !llvm.loop !236

54:                                               ; preds = %47, %32, %13
  %55 = phi ptr [ @.str.28, %13 ], [ @.str.28, %32 ], [ @.str.29, %47 ]
  tail call fastcc void @append_c(ptr noundef nonnull %55) #61
  %56 = tail call fastcc ptr @parse_error(ptr noundef %0, i32 noundef %1) #61
  br label %90

57:                                               ; preds = %64
  %58 = getelementptr inbounds i8, ptr %0, i32 %71
  %59 = load i8, ptr %58, align 1, !tbaa !1
  %60 = zext i8 %59 to i64
  %61 = sub nsw i64 47, %60
  %62 = udiv i64 %61, 10
  %63 = icmp ugt i64 %70, %62
  br i1 %63, label %73, label %64, !llvm.loop !237

64:                                               ; preds = %18, %57
  %65 = phi i64 [ %21, %18 ], [ %60, %57 ]
  %66 = phi i64 [ 0, %18 ], [ %70, %57 ]
  %67 = phi i32 [ %11, %18 ], [ %71, %57 ]
  %68 = mul nuw i64 %66, 10
  %69 = add i64 %68, -48
  %70 = add i64 %69, %65
  %71 = add nuw i32 %67, 1
  %72 = icmp eq i32 %71, %1
  br i1 %72, label %73, label %57, !llvm.loop !237

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
  store i32 0, ptr @builder_simple, align 4, !tbaa !4
  tail call fastcc void @reserve(i32 noundef %1) #61
  %82 = load ptr, ptr @builder, align 4, !tbaa !6
  %83 = load i32, ptr @builder_length, align 4, !tbaa !4
  %84 = getelementptr inbounds i8, ptr %82, i32 %83
  tail call fastcc void @copy_bytes(ptr noundef %84, ptr noundef nonnull %0, i32 noundef %1) #61
  %85 = add i32 %83, %1
  store i32 %85, ptr @builder_length, align 4, !tbaa !4
  tail call fastcc void @append_c(ptr noundef nonnull @.str.24) #61
  %86 = tail call fastcc ptr @parse_error(ptr noundef nonnull %0, i32 noundef %1) #61
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
  %4 = tail call ptr @fa_alloc(i32 noundef 32, i32 noundef 8) #62
  store i64 5, ptr %4, align 8, !tbaa !238
  %5 = getelementptr inbounds %struct.ParseError, ptr %4, i32 0, i32 1
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %3) #64
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %3, ptr noundef %0, i32 noundef %1, i32 noundef 0) #61
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 8 dereferenceable(12) %5, ptr noundef nonnull align 4 dereferenceable(12) %3, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %3) #64
  %6 = getelementptr inbounds %struct.ParseError, ptr %4, i32 0, i32 2
  tail call void @fa_b_take(ptr noundef nonnull %6) #61
  ret ptr %4
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
  tail call void @fa_overflow_unknown(ptr noundef %3) #67
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
  tail call void @fa_overflow_unknown(ptr noundef %3) #67
  unreachable
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare { i64, i1 } @llvm.smul.with.overflow.i64(i64, i64) #32

; Function Attrs: nounwind optsize
define weak hidden double @pow(double noundef %0, double noundef %1) local_unnamed_addr #3 {
  %3 = tail call double @fa_host_pow(double noundef %0, double noundef %1) #62
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
  %4 = and i64 %2, 2199023255552
  %5 = icmp eq i64 %4, 0
  br i1 %5, label %14, label %6

6:                                                ; preds = %3
  %7 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %8 = load ptr, ptr %7, align 8, !tbaa !240
  %9 = add i64 %2, 4
  %10 = mul i64 %9, %1
  %11 = trunc i64 %10 to i32
  %12 = getelementptr inbounds i8, ptr %8, i32 %11
  %13 = getelementptr inbounds i8, ptr %12, i32 4
  br label %33

14:                                               ; preds = %3
  %15 = and i64 %2, 1099511627776
  %16 = icmp eq i64 %15, 0
  br i1 %16, label %23, label %17

17:                                               ; preds = %14
  %18 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 8
  %19 = load ptr, ptr %18, align 8, !tbaa !242
  %20 = mul i64 %2, %1
  %21 = trunc i64 %20 to i32
  %22 = getelementptr inbounds i8, ptr %19, i32 %21
  br label %33

23:                                               ; preds = %14
  %24 = load ptr, ptr %0, align 8, !tbaa !243
  %25 = lshr i64 %1, 5
  %26 = trunc i64 %25 to i32
  %27 = getelementptr inbounds %struct.TableChunk, ptr %24, i32 %26
  %28 = load ptr, ptr %27, align 4, !tbaa !244
  %29 = and i64 %1, 31
  %30 = mul nsw i64 %29, %2
  %31 = trunc i64 %30 to i32
  %32 = getelementptr inbounds i8, ptr %28, i32 %31
  br label %33

33:                                               ; preds = %23, %17, %6
  %34 = phi ptr [ %13, %6 ], [ %22, %17 ], [ %32, %23 ]
  ret ptr %34
}

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(read, inaccessiblemem: none)
define hidden i32 @fa_table_live(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #41 {
  %4 = and i64 %2, 2199023255552
  %5 = icmp eq i64 %4, 0
  br i1 %5, label %16, label %6

6:                                                ; preds = %3
  %7 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %8 = load ptr, ptr %7, align 8, !tbaa !240
  %9 = add i64 %2, 4
  %10 = mul i64 %9, %1
  %11 = trunc i64 %10 to i32
  %12 = getelementptr inbounds i8, ptr %8, i32 %11
  %13 = load i32, ptr %12, align 1
  %14 = icmp ne i32 %13, 0
  %15 = zext i1 %14 to i32
  br label %26

16:                                               ; preds = %3
  %17 = load ptr, ptr %0, align 8, !tbaa !243
  %18 = lshr i64 %1, 5
  %19 = trunc i64 %18 to i32
  %20 = getelementptr inbounds %struct.TableChunk, ptr %17, i32 %19, i32 1
  %21 = load i32, ptr %20, align 4, !tbaa !246
  %22 = trunc i64 %1 to i32
  %23 = and i32 %22, 31
  %24 = lshr i32 %21, %23
  %25 = and i32 %24, 1
  br label %26

26:                                               ; preds = %16, %6
  %27 = phi i32 [ %15, %6 ], [ %25, %16 ]
  ret i32 %27
}

; Function Attrs: alwaysinline nounwind optsize
define hidden i64 @fa_table_find(ptr nocapture noundef readonly %0, i64 noundef %1, ptr noundef %2, ptr noundef readonly %3, i64 noundef %4, i64 noundef %5) local_unnamed_addr #13 {
  %7 = and i64 %4, 2199023255552
  %8 = icmp eq i64 %7, 0
  %9 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %10 = load i64, ptr %9, align 8, !tbaa !247
  %11 = and i64 %10, %1
  br i1 %8, label %87, label %12

12:                                               ; preds = %6
  %13 = trunc i64 %1 to i32
  %14 = icmp eq i32 %13, 0
  %15 = select i1 %14, i32 -2147483648, i32 %13
  %16 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %17 = add i64 %4, 4
  %18 = load ptr, ptr %16, align 8, !tbaa !240
  %19 = mul i64 %11, %17
  %20 = trunc i64 %19 to i32
  %21 = getelementptr inbounds i8, ptr %18, i32 %20
  %22 = load i32, ptr %21, align 1
  %23 = icmp eq i32 %22, 0
  br i1 %23, label %27, label %24

24:                                               ; preds = %12
  %25 = icmp eq ptr %3, null
  %26 = icmp slt i64 %5, 1
  br label %30

27:                                               ; preds = %78, %12
  %28 = phi i64 [ %11, %12 ], [ %81, %78 ]
  %29 = xor i64 %28, -1
  br label %254

30:                                               ; preds = %24, %78
  %31 = phi ptr [ %18, %24 ], [ %79, %78 ]
  %32 = phi i32 [ %22, %24 ], [ %85, %78 ]
  %33 = phi ptr [ %21, %24 ], [ %84, %78 ]
  %34 = phi i64 [ %11, %24 ], [ %81, %78 ]
  %35 = icmp eq i32 %32, %15
  br i1 %35, label %36, label %78

36:                                               ; preds = %30
  %37 = getelementptr inbounds i8, ptr %33, i32 4
  br i1 %25, label %40, label %38

38:                                               ; preds = %36
  %39 = tail call i32 %3(ptr noundef nonnull %37, ptr noundef %2) #62
  br label %73

40:                                               ; preds = %36
  switch i64 %5, label %41 [
    i64 1, label %42
    i64 2, label %46
    i64 4, label %50
    i64 8, label %54
  ]

41:                                               ; preds = %40
  br i1 %26, label %70, label %58

42:                                               ; preds = %40
  %43 = load i8, ptr %37, align 1, !tbaa !1
  %44 = load i8, ptr %2, align 1, !tbaa !1
  %45 = icmp eq i8 %43, %44
  br label %70

46:                                               ; preds = %40
  %47 = load i16, ptr %37, align 1
  %48 = load i16, ptr %2, align 1
  %49 = icmp eq i16 %47, %48
  br label %70

50:                                               ; preds = %40
  %51 = load i32, ptr %37, align 1
  %52 = load i32, ptr %2, align 1
  %53 = icmp eq i32 %51, %52
  br label %70

54:                                               ; preds = %40
  %55 = load i64, ptr %37, align 1
  %56 = load i64, ptr %2, align 1
  %57 = icmp eq i64 %55, %56
  br label %70

58:                                               ; preds = %58, %41
  %59 = phi i64 [ 0, %41 ], [ %67, %58 ]
  %60 = trunc i64 %59 to i32
  %61 = getelementptr inbounds i8, ptr %37, i32 %60
  %62 = load i8, ptr %61, align 1, !tbaa !1
  %63 = getelementptr inbounds i8, ptr %2, i32 %60
  %64 = load i8, ptr %63, align 1, !tbaa !1
  %65 = icmp eq i8 %62, %64
  %66 = xor i1 %65, true
  %67 = add nuw nsw i64 %59, 1
  %68 = icmp eq i64 %67, %5
  %69 = select i1 %66, i1 true, i1 %68
  br i1 %69, label %70, label %58, !llvm.loop !248

70:                                               ; preds = %58, %41, %54, %50, %46, %42
  %71 = phi i1 [ %57, %54 ], [ %53, %50 ], [ %49, %46 ], [ %45, %42 ], [ true, %41 ], [ %65, %58 ]
  %72 = zext i1 %71 to i32
  br label %73

73:                                               ; preds = %38, %70
  %74 = phi i32 [ %39, %38 ], [ %72, %70 ]
  %75 = icmp eq i32 %74, 0
  br i1 %75, label %76, label %254

76:                                               ; preds = %73
  %77 = load ptr, ptr %16, align 8, !tbaa !240
  br label %78

78:                                               ; preds = %76, %30
  %79 = phi ptr [ %77, %76 ], [ %31, %30 ]
  %80 = add i64 %34, 1
  %81 = and i64 %80, %10
  %82 = mul i64 %81, %17
  %83 = trunc i64 %82 to i32
  %84 = getelementptr inbounds i8, ptr %79, i32 %83
  %85 = load i32, ptr %84, align 1
  %86 = icmp eq i32 %85, 0
  br i1 %86, label %27, label %30

87:                                               ; preds = %6
  %88 = icmp ugt i64 %10, 2147483647
  %89 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %90 = load ptr, ptr %89, align 8, !tbaa !240
  br i1 %88, label %172, label %91, !prof !35

91:                                               ; preds = %87
  %92 = trunc i64 %1 to i32
  %93 = trunc i64 %11 to i32
  %94 = getelementptr inbounds i64, ptr %90, i32 %93
  %95 = load i64, ptr %94, align 8, !tbaa !89
  %96 = icmp eq i64 %95, 0
  br i1 %96, label %103, label %97

97:                                               ; preds = %91
  %98 = and i64 %4, 1099511627776
  %99 = icmp eq i64 %98, 0
  %100 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 8
  %101 = icmp eq ptr %3, null
  %102 = icmp slt i64 %5, 1
  br label %106

103:                                              ; preds = %165, %91
  %104 = phi i64 [ %11, %91 ], [ %167, %165 ]
  %105 = xor i64 %104, -1
  br label %254

106:                                              ; preds = %97, %165
  %107 = phi i64 [ %95, %97 ], [ %170, %165 ]
  %108 = phi i64 [ %11, %97 ], [ %167, %165 ]
  %109 = trunc i64 %107 to i32
  %110 = icmp eq i32 %109, %92
  br i1 %110, label %111, label %165

111:                                              ; preds = %106
  %112 = lshr i64 %107, 32
  %113 = add nsw i64 %112, -1
  br i1 %99, label %114, label %120

114:                                              ; preds = %111
  %115 = load ptr, ptr %0, align 8, !tbaa !243
  %116 = lshr i64 %113, 5
  %117 = trunc i64 %116 to i32
  %118 = getelementptr inbounds %struct.TableChunk, ptr %115, i32 %117
  %119 = and i64 %113, 31
  br label %120

120:                                              ; preds = %111, %114
  %121 = phi i64 [ %119, %114 ], [ %113, %111 ]
  %122 = phi ptr [ %118, %114 ], [ %100, %111 ]
  %123 = load ptr, ptr %122, align 4, !tbaa !6
  %124 = mul i64 %121, %4
  %125 = trunc i64 %124 to i32
  %126 = getelementptr inbounds i8, ptr %123, i32 %125
  br i1 %101, label %129, label %127

127:                                              ; preds = %120
  %128 = tail call i32 %3(ptr noundef %126, ptr noundef %2) #62
  br label %162

129:                                              ; preds = %120
  switch i64 %5, label %130 [
    i64 1, label %131
    i64 2, label %135
    i64 4, label %139
    i64 8, label %143
  ]

130:                                              ; preds = %129
  br i1 %102, label %159, label %147

131:                                              ; preds = %129
  %132 = load i8, ptr %126, align 1, !tbaa !1
  %133 = load i8, ptr %2, align 1, !tbaa !1
  %134 = icmp eq i8 %132, %133
  br label %159

135:                                              ; preds = %129
  %136 = load i16, ptr %126, align 1
  %137 = load i16, ptr %2, align 1
  %138 = icmp eq i16 %136, %137
  br label %159

139:                                              ; preds = %129
  %140 = load i32, ptr %126, align 1
  %141 = load i32, ptr %2, align 1
  %142 = icmp eq i32 %140, %141
  br label %159

143:                                              ; preds = %129
  %144 = load i64, ptr %126, align 1
  %145 = load i64, ptr %2, align 1
  %146 = icmp eq i64 %144, %145
  br label %159

147:                                              ; preds = %147, %130
  %148 = phi i64 [ 0, %130 ], [ %156, %147 ]
  %149 = trunc i64 %148 to i32
  %150 = getelementptr inbounds i8, ptr %126, i32 %149
  %151 = load i8, ptr %150, align 1, !tbaa !1
  %152 = getelementptr inbounds i8, ptr %2, i32 %149
  %153 = load i8, ptr %152, align 1, !tbaa !1
  %154 = icmp eq i8 %151, %153
  %155 = xor i1 %154, true
  %156 = add nuw nsw i64 %148, 1
  %157 = icmp eq i64 %156, %5
  %158 = select i1 %155, i1 true, i1 %157
  br i1 %158, label %159, label %147, !llvm.loop !248

159:                                              ; preds = %147, %130, %143, %139, %135, %131
  %160 = phi i1 [ %146, %143 ], [ %142, %139 ], [ %138, %135 ], [ %134, %131 ], [ true, %130 ], [ %154, %147 ]
  %161 = zext i1 %160 to i32
  br label %162

162:                                              ; preds = %127, %159
  %163 = phi i32 [ %128, %127 ], [ %161, %159 ]
  %164 = icmp eq i32 %163, 0
  br i1 %164, label %165, label %254

165:                                              ; preds = %162, %106
  %166 = add nsw i64 %108, 1
  %167 = and i64 %166, %10
  %168 = trunc i64 %167 to i32
  %169 = getelementptr inbounds i64, ptr %90, i32 %168
  %170 = load i64, ptr %169, align 8, !tbaa !89
  %171 = icmp eq i64 %170, 0
  br i1 %171, label %103, label %106

172:                                              ; preds = %87
  %173 = trunc i64 %11 to i32
  %174 = getelementptr inbounds %struct.WideSlot, ptr %90, i32 %173
  %175 = getelementptr inbounds i8, ptr %174, i32 8
  %176 = load i64, ptr %175, align 8, !tbaa.struct !249
  %177 = icmp eq i64 %176, 0
  br i1 %177, label %184, label %178

178:                                              ; preds = %172
  %179 = and i64 %4, 1099511627776
  %180 = icmp eq i64 %179, 0
  %181 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 8
  %182 = icmp eq ptr %3, null
  %183 = icmp slt i64 %5, 1
  br label %187

184:                                              ; preds = %246, %172
  %185 = phi i64 [ %11, %172 ], [ %248, %246 ]
  %186 = xor i64 %185, -1
  br label %254

187:                                              ; preds = %178, %246
  %188 = phi i64 [ %176, %178 ], [ %252, %246 ]
  %189 = phi ptr [ %174, %178 ], [ %250, %246 ]
  %190 = phi i64 [ %11, %178 ], [ %248, %246 ]
  %191 = load i64, ptr %189, align 8, !tbaa.struct !250
  %192 = icmp eq i64 %191, %1
  br i1 %192, label %193, label %246

193:                                              ; preds = %187
  %194 = add nsw i64 %188, -1
  br i1 %180, label %195, label %201

195:                                              ; preds = %193
  %196 = load ptr, ptr %0, align 8, !tbaa !243
  %197 = lshr i64 %194, 5
  %198 = trunc i64 %197 to i32
  %199 = getelementptr inbounds %struct.TableChunk, ptr %196, i32 %198
  %200 = and i64 %194, 31
  br label %201

201:                                              ; preds = %193, %195
  %202 = phi i64 [ %200, %195 ], [ %194, %193 ]
  %203 = phi ptr [ %199, %195 ], [ %181, %193 ]
  %204 = load ptr, ptr %203, align 4, !tbaa !6
  %205 = mul i64 %202, %4
  %206 = trunc i64 %205 to i32
  %207 = getelementptr inbounds i8, ptr %204, i32 %206
  br i1 %182, label %210, label %208

208:                                              ; preds = %201
  %209 = tail call i32 %3(ptr noundef %207, ptr noundef %2) #62
  br label %243

210:                                              ; preds = %201
  switch i64 %5, label %211 [
    i64 1, label %212
    i64 2, label %216
    i64 4, label %220
    i64 8, label %224
  ]

211:                                              ; preds = %210
  br i1 %183, label %240, label %228

212:                                              ; preds = %210
  %213 = load i8, ptr %207, align 1, !tbaa !1
  %214 = load i8, ptr %2, align 1, !tbaa !1
  %215 = icmp eq i8 %213, %214
  br label %240

216:                                              ; preds = %210
  %217 = load i16, ptr %207, align 1
  %218 = load i16, ptr %2, align 1
  %219 = icmp eq i16 %217, %218
  br label %240

220:                                              ; preds = %210
  %221 = load i32, ptr %207, align 1
  %222 = load i32, ptr %2, align 1
  %223 = icmp eq i32 %221, %222
  br label %240

224:                                              ; preds = %210
  %225 = load i64, ptr %207, align 1
  %226 = load i64, ptr %2, align 1
  %227 = icmp eq i64 %225, %226
  br label %240

228:                                              ; preds = %228, %211
  %229 = phi i64 [ 0, %211 ], [ %237, %228 ]
  %230 = trunc i64 %229 to i32
  %231 = getelementptr inbounds i8, ptr %207, i32 %230
  %232 = load i8, ptr %231, align 1, !tbaa !1
  %233 = getelementptr inbounds i8, ptr %2, i32 %230
  %234 = load i8, ptr %233, align 1, !tbaa !1
  %235 = icmp eq i8 %232, %234
  %236 = xor i1 %235, true
  %237 = add nuw nsw i64 %229, 1
  %238 = icmp eq i64 %237, %5
  %239 = select i1 %236, i1 true, i1 %238
  br i1 %239, label %240, label %228, !llvm.loop !248

240:                                              ; preds = %228, %211, %224, %220, %216, %212
  %241 = phi i1 [ %227, %224 ], [ %223, %220 ], [ %219, %216 ], [ %215, %212 ], [ true, %211 ], [ %235, %228 ]
  %242 = zext i1 %241 to i32
  br label %243

243:                                              ; preds = %208, %240
  %244 = phi i32 [ %209, %208 ], [ %242, %240 ]
  %245 = icmp eq i32 %244, 0
  br i1 %245, label %246, label %254

246:                                              ; preds = %243, %187
  %247 = add i64 %190, 1
  %248 = and i64 %247, %10
  %249 = trunc i64 %248 to i32
  %250 = getelementptr inbounds %struct.WideSlot, ptr %90, i32 %249
  %251 = getelementptr inbounds i8, ptr %250, i32 8
  %252 = load i64, ptr %251, align 8, !tbaa.struct !249
  %253 = icmp eq i64 %252, 0
  br i1 %253, label %184, label %187

254:                                              ; preds = %73, %162, %243, %184, %103, %27
  %255 = phi i64 [ %29, %27 ], [ %105, %103 ], [ %186, %184 ], [ %194, %243 ], [ %113, %162 ], [ %34, %73 ]
  ret i64 %255
}

; Function Attrs: alwaysinline nounwind optsize
define hidden i64 @fa_table_remove(ptr nocapture noundef readonly %0, i64 noundef %1, ptr noundef %2, ptr noundef readonly %3, i64 noundef %4, i64 noundef %5) local_unnamed_addr #13 {
  %7 = and i64 %4, 2199023255552
  %8 = icmp eq i64 %7, 0
  %9 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %10 = load i64, ptr %9, align 8, !tbaa !247
  %11 = and i64 %10, %1
  br i1 %8, label %90, label %12

12:                                               ; preds = %6
  %13 = trunc i64 %1 to i32
  %14 = icmp eq i32 %13, 0
  %15 = select i1 %14, i32 -2147483648, i32 %13
  %16 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %17 = add i64 %4, 4
  %18 = load ptr, ptr %16, align 8, !tbaa !240
  %19 = mul i64 %11, %17
  %20 = trunc i64 %19 to i32
  %21 = getelementptr inbounds i8, ptr %18, i32 %20
  %22 = load i32, ptr %21, align 1
  %23 = icmp eq i32 %22, 0
  br i1 %23, label %27, label %24

24:                                               ; preds = %12
  %25 = icmp eq ptr %3, null
  %26 = icmp slt i64 %5, 1
  br label %30

27:                                               ; preds = %78, %12
  %28 = phi i64 [ %11, %12 ], [ %81, %78 ]
  %29 = xor i64 %28, -1
  br label %87

30:                                               ; preds = %24, %78
  %31 = phi ptr [ %18, %24 ], [ %79, %78 ]
  %32 = phi i32 [ %22, %24 ], [ %85, %78 ]
  %33 = phi ptr [ %21, %24 ], [ %84, %78 ]
  %34 = phi i64 [ %11, %24 ], [ %81, %78 ]
  %35 = icmp eq i32 %32, %15
  br i1 %35, label %36, label %78

36:                                               ; preds = %30
  %37 = getelementptr inbounds i8, ptr %33, i32 4
  br i1 %25, label %40, label %38

38:                                               ; preds = %36
  %39 = tail call i32 %3(ptr noundef nonnull %37, ptr noundef %2) #62
  br label %73

40:                                               ; preds = %36
  switch i64 %5, label %41 [
    i64 1, label %42
    i64 2, label %46
    i64 4, label %50
    i64 8, label %54
  ]

41:                                               ; preds = %40
  br i1 %26, label %70, label %58

42:                                               ; preds = %40
  %43 = load i8, ptr %37, align 1, !tbaa !1
  %44 = load i8, ptr %2, align 1, !tbaa !1
  %45 = icmp eq i8 %43, %44
  br label %70

46:                                               ; preds = %40
  %47 = load i16, ptr %37, align 1
  %48 = load i16, ptr %2, align 1
  %49 = icmp eq i16 %47, %48
  br label %70

50:                                               ; preds = %40
  %51 = load i32, ptr %37, align 1
  %52 = load i32, ptr %2, align 1
  %53 = icmp eq i32 %51, %52
  br label %70

54:                                               ; preds = %40
  %55 = load i64, ptr %37, align 1
  %56 = load i64, ptr %2, align 1
  %57 = icmp eq i64 %55, %56
  br label %70

58:                                               ; preds = %58, %41
  %59 = phi i64 [ 0, %41 ], [ %67, %58 ]
  %60 = trunc i64 %59 to i32
  %61 = getelementptr inbounds i8, ptr %37, i32 %60
  %62 = load i8, ptr %61, align 1, !tbaa !1
  %63 = getelementptr inbounds i8, ptr %2, i32 %60
  %64 = load i8, ptr %63, align 1, !tbaa !1
  %65 = icmp eq i8 %62, %64
  %66 = xor i1 %65, true
  %67 = add nuw nsw i64 %59, 1
  %68 = icmp eq i64 %67, %5
  %69 = select i1 %66, i1 true, i1 %68
  br i1 %69, label %70, label %58, !llvm.loop !248

70:                                               ; preds = %58, %41, %54, %50, %46, %42
  %71 = phi i1 [ %57, %54 ], [ %53, %50 ], [ %49, %46 ], [ %45, %42 ], [ true, %41 ], [ %65, %58 ]
  %72 = zext i1 %71 to i32
  br label %73

73:                                               ; preds = %38, %70
  %74 = phi i32 [ %39, %38 ], [ %72, %70 ]
  %75 = icmp eq i32 %74, 0
  br i1 %75, label %76, label %87

76:                                               ; preds = %73
  %77 = load ptr, ptr %16, align 8, !tbaa !240
  br label %78

78:                                               ; preds = %76, %30
  %79 = phi ptr [ %77, %76 ], [ %31, %30 ]
  %80 = add i64 %34, 1
  %81 = and i64 %80, %10
  %82 = mul i64 %81, %17
  %83 = trunc i64 %82 to i32
  %84 = getelementptr inbounds i8, ptr %79, i32 %83
  %85 = load i32, ptr %84, align 1
  %86 = icmp eq i32 %85, 0
  br i1 %86, label %27, label %30

87:                                               ; preds = %73, %27
  %88 = phi i64 [ %29, %27 ], [ %34, %73 ]
  %89 = tail call i64 @llvm.smax.i64(i64 %88, i64 -1)
  br label %333

90:                                               ; preds = %6
  %91 = icmp ugt i64 %10, 2147483647
  %92 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %93 = load ptr, ptr %92, align 8, !tbaa !240
  br i1 %91, label %175, label %94, !prof !35

94:                                               ; preds = %90
  %95 = trunc i64 %1 to i32
  %96 = trunc i64 %11 to i32
  %97 = getelementptr inbounds i64, ptr %93, i32 %96
  %98 = load i64, ptr %97, align 8, !tbaa !89
  %99 = icmp eq i64 %98, 0
  br i1 %99, label %106, label %100

100:                                              ; preds = %94
  %101 = and i64 %4, 1099511627776
  %102 = icmp eq i64 %101, 0
  %103 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 8
  %104 = icmp eq ptr %3, null
  %105 = icmp slt i64 %5, 1
  br label %109

106:                                              ; preds = %168, %94
  %107 = phi i64 [ %11, %94 ], [ %170, %168 ]
  %108 = xor i64 %107, -1
  br label %257

109:                                              ; preds = %100, %168
  %110 = phi i64 [ %98, %100 ], [ %173, %168 ]
  %111 = phi i64 [ %11, %100 ], [ %170, %168 ]
  %112 = trunc i64 %110 to i32
  %113 = icmp eq i32 %112, %95
  br i1 %113, label %114, label %168

114:                                              ; preds = %109
  %115 = lshr i64 %110, 32
  %116 = add nsw i64 %115, -1
  br i1 %102, label %117, label %123

117:                                              ; preds = %114
  %118 = load ptr, ptr %0, align 8, !tbaa !243
  %119 = lshr i64 %116, 5
  %120 = trunc i64 %119 to i32
  %121 = getelementptr inbounds %struct.TableChunk, ptr %118, i32 %120
  %122 = and i64 %116, 31
  br label %123

123:                                              ; preds = %114, %117
  %124 = phi i64 [ %122, %117 ], [ %116, %114 ]
  %125 = phi ptr [ %121, %117 ], [ %103, %114 ]
  %126 = load ptr, ptr %125, align 4, !tbaa !6
  %127 = mul i64 %124, %4
  %128 = trunc i64 %127 to i32
  %129 = getelementptr inbounds i8, ptr %126, i32 %128
  br i1 %104, label %132, label %130

130:                                              ; preds = %123
  %131 = tail call i32 %3(ptr noundef %129, ptr noundef %2) #62
  br label %165

132:                                              ; preds = %123
  switch i64 %5, label %133 [
    i64 1, label %134
    i64 2, label %138
    i64 4, label %142
    i64 8, label %146
  ]

133:                                              ; preds = %132
  br i1 %105, label %162, label %150

134:                                              ; preds = %132
  %135 = load i8, ptr %129, align 1, !tbaa !1
  %136 = load i8, ptr %2, align 1, !tbaa !1
  %137 = icmp eq i8 %135, %136
  br label %162

138:                                              ; preds = %132
  %139 = load i16, ptr %129, align 1
  %140 = load i16, ptr %2, align 1
  %141 = icmp eq i16 %139, %140
  br label %162

142:                                              ; preds = %132
  %143 = load i32, ptr %129, align 1
  %144 = load i32, ptr %2, align 1
  %145 = icmp eq i32 %143, %144
  br label %162

146:                                              ; preds = %132
  %147 = load i64, ptr %129, align 1
  %148 = load i64, ptr %2, align 1
  %149 = icmp eq i64 %147, %148
  br label %162

150:                                              ; preds = %150, %133
  %151 = phi i64 [ 0, %133 ], [ %159, %150 ]
  %152 = trunc i64 %151 to i32
  %153 = getelementptr inbounds i8, ptr %129, i32 %152
  %154 = load i8, ptr %153, align 1, !tbaa !1
  %155 = getelementptr inbounds i8, ptr %2, i32 %152
  %156 = load i8, ptr %155, align 1, !tbaa !1
  %157 = icmp eq i8 %154, %156
  %158 = xor i1 %157, true
  %159 = add nuw nsw i64 %151, 1
  %160 = icmp eq i64 %159, %5
  %161 = select i1 %158, i1 true, i1 %160
  br i1 %161, label %162, label %150, !llvm.loop !248

162:                                              ; preds = %150, %133, %146, %142, %138, %134
  %163 = phi i1 [ %149, %146 ], [ %145, %142 ], [ %141, %138 ], [ %137, %134 ], [ true, %133 ], [ %157, %150 ]
  %164 = zext i1 %163 to i32
  br label %165

165:                                              ; preds = %130, %162
  %166 = phi i32 [ %131, %130 ], [ %164, %162 ]
  %167 = icmp eq i32 %166, 0
  br i1 %167, label %168, label %257

168:                                              ; preds = %165, %109
  %169 = add nsw i64 %111, 1
  %170 = and i64 %169, %10
  %171 = trunc i64 %170 to i32
  %172 = getelementptr inbounds i64, ptr %93, i32 %171
  %173 = load i64, ptr %172, align 8, !tbaa !89
  %174 = icmp eq i64 %173, 0
  br i1 %174, label %106, label %109

175:                                              ; preds = %90
  %176 = trunc i64 %11 to i32
  %177 = getelementptr inbounds %struct.WideSlot, ptr %93, i32 %176
  %178 = getelementptr inbounds i8, ptr %177, i32 8
  %179 = load i64, ptr %178, align 8, !tbaa.struct !249
  %180 = icmp eq i64 %179, 0
  br i1 %180, label %187, label %181

181:                                              ; preds = %175
  %182 = and i64 %4, 1099511627776
  %183 = icmp eq i64 %182, 0
  %184 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 8
  %185 = icmp eq ptr %3, null
  %186 = icmp slt i64 %5, 1
  br label %190

187:                                              ; preds = %249, %175
  %188 = phi i64 [ %11, %175 ], [ %251, %249 ]
  %189 = xor i64 %188, -1
  br label %257

190:                                              ; preds = %181, %249
  %191 = phi i64 [ %179, %181 ], [ %255, %249 ]
  %192 = phi ptr [ %177, %181 ], [ %253, %249 ]
  %193 = phi i64 [ %11, %181 ], [ %251, %249 ]
  %194 = load i64, ptr %192, align 8, !tbaa.struct !250
  %195 = icmp eq i64 %194, %1
  br i1 %195, label %196, label %249

196:                                              ; preds = %190
  %197 = add nsw i64 %191, -1
  br i1 %183, label %198, label %204

198:                                              ; preds = %196
  %199 = load ptr, ptr %0, align 8, !tbaa !243
  %200 = lshr i64 %197, 5
  %201 = trunc i64 %200 to i32
  %202 = getelementptr inbounds %struct.TableChunk, ptr %199, i32 %201
  %203 = and i64 %197, 31
  br label %204

204:                                              ; preds = %196, %198
  %205 = phi i64 [ %203, %198 ], [ %197, %196 ]
  %206 = phi ptr [ %202, %198 ], [ %184, %196 ]
  %207 = load ptr, ptr %206, align 4, !tbaa !6
  %208 = mul i64 %205, %4
  %209 = trunc i64 %208 to i32
  %210 = getelementptr inbounds i8, ptr %207, i32 %209
  br i1 %185, label %213, label %211

211:                                              ; preds = %204
  %212 = tail call i32 %3(ptr noundef %210, ptr noundef %2) #62
  br label %246

213:                                              ; preds = %204
  switch i64 %5, label %214 [
    i64 1, label %215
    i64 2, label %219
    i64 4, label %223
    i64 8, label %227
  ]

214:                                              ; preds = %213
  br i1 %186, label %243, label %231

215:                                              ; preds = %213
  %216 = load i8, ptr %210, align 1, !tbaa !1
  %217 = load i8, ptr %2, align 1, !tbaa !1
  %218 = icmp eq i8 %216, %217
  br label %243

219:                                              ; preds = %213
  %220 = load i16, ptr %210, align 1
  %221 = load i16, ptr %2, align 1
  %222 = icmp eq i16 %220, %221
  br label %243

223:                                              ; preds = %213
  %224 = load i32, ptr %210, align 1
  %225 = load i32, ptr %2, align 1
  %226 = icmp eq i32 %224, %225
  br label %243

227:                                              ; preds = %213
  %228 = load i64, ptr %210, align 1
  %229 = load i64, ptr %2, align 1
  %230 = icmp eq i64 %228, %229
  br label %243

231:                                              ; preds = %231, %214
  %232 = phi i64 [ 0, %214 ], [ %240, %231 ]
  %233 = trunc i64 %232 to i32
  %234 = getelementptr inbounds i8, ptr %210, i32 %233
  %235 = load i8, ptr %234, align 1, !tbaa !1
  %236 = getelementptr inbounds i8, ptr %2, i32 %233
  %237 = load i8, ptr %236, align 1, !tbaa !1
  %238 = icmp eq i8 %235, %237
  %239 = xor i1 %238, true
  %240 = add nuw nsw i64 %232, 1
  %241 = icmp eq i64 %240, %5
  %242 = select i1 %239, i1 true, i1 %241
  br i1 %242, label %243, label %231, !llvm.loop !248

243:                                              ; preds = %231, %214, %227, %223, %219, %215
  %244 = phi i1 [ %230, %227 ], [ %226, %223 ], [ %222, %219 ], [ %218, %215 ], [ true, %214 ], [ %238, %231 ]
  %245 = zext i1 %244 to i32
  br label %246

246:                                              ; preds = %211, %243
  %247 = phi i32 [ %212, %211 ], [ %245, %243 ]
  %248 = icmp eq i32 %247, 0
  br i1 %248, label %249, label %257

249:                                              ; preds = %246, %190
  %250 = add i64 %193, 1
  %251 = and i64 %250, %10
  %252 = trunc i64 %251 to i32
  %253 = getelementptr inbounds %struct.WideSlot, ptr %93, i32 %252
  %254 = getelementptr inbounds i8, ptr %253, i32 8
  %255 = load i64, ptr %254, align 8, !tbaa.struct !249
  %256 = icmp eq i64 %255, 0
  br i1 %256, label %187, label %190

257:                                              ; preds = %165, %246, %187, %106
  %258 = phi i64 [ %107, %106 ], [ %188, %187 ], [ %193, %246 ], [ %111, %165 ]
  %259 = phi i64 [ %108, %106 ], [ %189, %187 ], [ %197, %246 ], [ %116, %165 ]
  %260 = icmp sgt i64 %259, -1
  br i1 %260, label %261, label %333

261:                                              ; preds = %257
  %262 = load i64, ptr %9, align 8, !tbaa !247
  %263 = icmp ugt i64 %262, 2147483647
  %264 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %265 = load ptr, ptr %264, align 8, !tbaa !240
  %266 = add i64 %258, 1
  %267 = and i64 %266, %262
  %268 = trunc i64 %267 to i32
  br i1 %263, label %298, label %269, !prof !35

269:                                              ; preds = %261
  %270 = getelementptr inbounds i64, ptr %265, i32 %268
  %271 = load i64, ptr %270, align 8, !tbaa !89
  %272 = icmp eq i64 %271, 0
  br i1 %272, label %294, label %273

273:                                              ; preds = %269, %286
  %274 = phi i64 [ %292, %286 ], [ %271, %269 ]
  %275 = phi i64 [ %289, %286 ], [ %267, %269 ]
  %276 = phi i64 [ %287, %286 ], [ %258, %269 ]
  %277 = and i64 %274, %262
  %278 = sub nsw i64 %275, %277
  %279 = and i64 %278, %262
  %280 = sub i64 %275, %276
  %281 = and i64 %280, %262
  %282 = icmp ult i64 %279, %281
  br i1 %282, label %286, label %283

283:                                              ; preds = %273
  %284 = trunc i64 %276 to i32
  %285 = getelementptr inbounds i64, ptr %265, i32 %284
  store i64 %274, ptr %285, align 8, !tbaa !89
  br label %286

286:                                              ; preds = %283, %273
  %287 = phi i64 [ %275, %283 ], [ %276, %273 ]
  %288 = add nsw i64 %275, 1
  %289 = and i64 %288, %262
  %290 = trunc i64 %289 to i32
  %291 = getelementptr inbounds i64, ptr %265, i32 %290
  %292 = load i64, ptr %291, align 8, !tbaa !89
  %293 = icmp eq i64 %292, 0
  br i1 %293, label %294, label %273

294:                                              ; preds = %286, %269
  %295 = phi i64 [ %258, %269 ], [ %287, %286 ]
  %296 = trunc i64 %295 to i32
  %297 = getelementptr inbounds i64, ptr %265, i32 %296
  store i64 0, ptr %297, align 8, !tbaa !89
  br label %333

298:                                              ; preds = %261
  %299 = getelementptr inbounds %struct.WideSlot, ptr %265, i32 %268
  %300 = getelementptr inbounds i8, ptr %299, i32 8
  %301 = load i64, ptr %300, align 8, !tbaa.struct !249
  %302 = icmp eq i64 %301, 0
  br i1 %302, label %328, label %303

303:                                              ; preds = %298, %319
  %304 = phi i64 [ %326, %319 ], [ %301, %298 ]
  %305 = phi ptr [ %324, %319 ], [ %299, %298 ]
  %306 = phi i64 [ %322, %319 ], [ %267, %298 ]
  %307 = phi i64 [ %320, %319 ], [ %258, %298 ]
  %308 = load i64, ptr %305, align 8
  %309 = and i64 %308, %262
  %310 = sub i64 %306, %309
  %311 = and i64 %310, %262
  %312 = sub i64 %306, %307
  %313 = and i64 %312, %262
  %314 = icmp ult i64 %311, %313
  br i1 %314, label %319, label %315

315:                                              ; preds = %303
  %316 = trunc i64 %307 to i32
  %317 = getelementptr inbounds %struct.WideSlot, ptr %265, i32 %316
  store i64 %308, ptr %317, align 8, !tbaa.struct !250
  %318 = getelementptr inbounds i8, ptr %317, i32 8
  store i64 %304, ptr %318, align 8, !tbaa.struct !249
  br label %319

319:                                              ; preds = %315, %303
  %320 = phi i64 [ %306, %315 ], [ %307, %303 ]
  %321 = add i64 %306, 1
  %322 = and i64 %321, %262
  %323 = trunc i64 %322 to i32
  %324 = getelementptr inbounds %struct.WideSlot, ptr %265, i32 %323
  %325 = getelementptr inbounds i8, ptr %324, i32 8
  %326 = load i64, ptr %325, align 8, !tbaa.struct !249
  %327 = icmp eq i64 %326, 0
  br i1 %327, label %328, label %303

328:                                              ; preds = %319, %298
  %329 = phi i64 [ %258, %298 ], [ %320, %319 ]
  %330 = trunc i64 %329 to i32
  %331 = getelementptr inbounds %struct.WideSlot, ptr %265, i32 %330
  store i64 0, ptr %331, align 8, !tbaa !251
  %332 = getelementptr inbounds %struct.WideSlot, ptr %265, i32 %330, i32 1
  store i64 0, ptr %332, align 8, !tbaa !253
  br label %333

333:                                              ; preds = %257, %294, %328, %87
  %334 = phi i64 [ %89, %87 ], [ %259, %328 ], [ %259, %294 ], [ %259, %257 ]
  ret i64 %334
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_table_new(i64 noundef %0, i64 noundef %1) local_unnamed_addr #3 {
  %3 = tail call ptr @fa_alloc(i32 noundef 80, i32 noundef 8) #62
  store ptr null, ptr %3, align 8, !tbaa !243
  %4 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 1
  store i64 0, ptr %4, align 8, !tbaa !254
  %5 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 2
  store i64 0, ptr %5, align 8, !tbaa !255
  %6 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 3
  store i64 0, ptr %6, align 8, !tbaa !256
  %7 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 6
  store i64 0, ptr %7, align 8, !tbaa !257
  %8 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 7
  store i64 0, ptr %8, align 8, !tbaa !258
  %9 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 8
  store ptr null, ptr %9, align 8, !tbaa !242
  %10 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 9
  store i64 0, ptr %10, align 8, !tbaa !259
  %11 = and i64 %1, 2199023255552
  %12 = icmp eq i64 %11, 0
  %13 = shl nsw i64 %0, 1
  br i1 %12, label %19, label %14

14:                                               ; preds = %2, %14
  %15 = phi i64 [ %17, %14 ], [ 8, %2 ]
  %16 = icmp slt i64 %15, %13
  %17 = shl nsw i64 %15, 1
  br i1 %16, label %14, label %18, !llvm.loop !260

18:                                               ; preds = %14
  tail call fastcc void @inline_new_slots(ptr noundef nonnull %3, i64 noundef %15, i64 noundef %1) #61
  br label %32

19:                                               ; preds = %2, %19
  %20 = phi i64 [ %22, %19 ], [ 8, %2 ]
  %21 = icmp slt i64 %20, %13
  %22 = shl nsw i64 %20, 1
  br i1 %21, label %19, label %23, !llvm.loop !260

23:                                               ; preds = %19
  %24 = trunc i64 %20 to i32
  %25 = add nsw i64 %20, -1
  %26 = icmp ugt i64 %25, 2147483647
  %27 = select i1 %26, i32 4, i32 3
  %28 = shl i32 %24, %27
  %29 = tail call ptr @fa_alloc(i32 noundef %28, i32 noundef 8) #62
  %30 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 4
  store ptr %29, ptr %30, align 8, !tbaa !240
  tail call void @llvm.memset.p0.i32(ptr align 1 %29, i8 0, i32 %28, i1 false)
  %31 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 5
  store i64 %25, ptr %31, align 8, !tbaa !247
  br label %32

32:                                               ; preds = %23, %18
  ret ptr %3
}

; Function Attrs: nounwind optsize
define internal fastcc void @inline_new_slots(ptr nocapture noundef writeonly %0, i64 noundef %1, i64 noundef %2) unnamed_addr #3 {
  %4 = icmp sgt i64 %1, 2147483648
  br i1 %4, label %5, label %6

5:                                                ; preds = %3
  tail call fastcc void @out_of_memory() #65
  unreachable

6:                                                ; preds = %3
  %7 = add nsw i64 %1, -1
  %8 = add i64 %2, 4
  %9 = mul i64 %8, %1
  %10 = trunc i64 %9 to i32
  %11 = tail call ptr @fa_alloc(i32 noundef %10, i32 noundef 8) #62
  %12 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  store ptr %11, ptr %12, align 8, !tbaa !240
  tail call void @llvm.memset.p0.i32(ptr align 1 %11, i8 0, i32 %10, i1 false)
  %13 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  store i64 %7, ptr %13, align 8, !tbaa !247
  %14 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 2
  store i64 %1, ptr %14, align 8, !tbaa !255
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read)
define hidden i64 @fa_table_capacity(ptr nocapture noundef readonly %0) local_unnamed_addr #37 {
  %2 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %3 = load i64, ptr %2, align 8, !tbaa !247
  %4 = add nsw i64 %3, 1
  %5 = sdiv i64 %4, 2
  ret i64 %5
}

; Function Attrs: nounwind optsize
define hidden void @fa_table_reserve(ptr nocapture noundef %0, i64 noundef %1, i64 noundef %2, ptr nocapture noundef readonly %3) local_unnamed_addr #3 {
  %5 = icmp slt i64 %1, 1
  br i1 %5, label %38, label %6

6:                                                ; preds = %4
  %7 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 1
  %8 = load i64, ptr %7, align 8, !tbaa !254
  %9 = add nsw i64 %8, %1
  %10 = and i64 %2, 1099511627775
  %11 = add nuw nsw i64 %10, 16
  %12 = udiv i64 1073741823, %11
  %13 = icmp sgt i64 %9, %12
  br i1 %13, label %14, label %15

14:                                               ; preds = %6
  tail call fastcc void @out_of_memory() #65
  unreachable

15:                                               ; preds = %6
  %16 = shl nsw i64 %9, 1
  br label %17

17:                                               ; preds = %17, %15
  %18 = phi i64 [ 8, %15 ], [ %20, %17 ]
  %19 = icmp slt i64 %18, %16
  %20 = shl nsw i64 %18, 1
  br i1 %19, label %17, label %21, !llvm.loop !260

21:                                               ; preds = %17
  %22 = and i64 %2, 2199023255552
  %23 = icmp eq i64 %22, 0
  %24 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %25 = load i64, ptr %24, align 8, !tbaa !247
  %26 = add nsw i64 %25, 1
  %27 = icmp sgt i64 %18, %26
  br i1 %23, label %30, label %28

28:                                               ; preds = %21
  br i1 %27, label %29, label %38

29:                                               ; preds = %28
  tail call fastcc void @inline_grow(ptr noundef nonnull %0, i64 noundef %18, i64 noundef %2) #66
  br label %38

30:                                               ; preds = %21
  br i1 %27, label %31, label %32

31:                                               ; preds = %30
  tail call fastcc void @table_grow_index(ptr noundef nonnull %0, i64 noundef %18, ptr noundef %3, i64 noundef %2) #66
  br label %32

32:                                               ; preds = %31, %30
  %33 = and i64 %2, 1099511627776
  %34 = icmp eq i64 %33, 0
  br i1 %34, label %38, label %35

35:                                               ; preds = %32
  %36 = add nsw i64 %9, 31
  %37 = and i64 %36, -32
  tail call fastcc void @table_dense_room(ptr noundef nonnull %0, i64 noundef %37, i64 noundef %10) #66
  br label %38

38:                                               ; preds = %29, %28, %35, %32, %4
  ret void
}

; Function Attrs: cold noinline nounwind optsize
define internal fastcc void @inline_grow(ptr nocapture noundef %0, i64 noundef %1, i64 noundef %2) unnamed_addr #22 {
  %4 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %5 = load ptr, ptr %4, align 8, !tbaa !240
  %6 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %7 = load i64, ptr %6, align 8, !tbaa !247
  %8 = and i64 %2, 1099511627775
  %9 = add nuw nsw i64 %8, 4
  tail call fastcc void @inline_new_slots(ptr noundef %0, i64 noundef %1, i64 noundef %2) #61
  %10 = load i64, ptr %6, align 8, !tbaa !247
  %11 = icmp slt i64 %7, 0
  br i1 %11, label %14, label %12

12:                                               ; preds = %3
  %13 = trunc i64 %9 to i32
  br label %19

14:                                               ; preds = %46, %3
  %15 = add i64 %7, 1
  %16 = add i64 %2, 4
  %17 = mul i64 %15, %16
  %18 = trunc i64 %17 to i32
  tail call void @fa_free(ptr noundef %5, i32 noundef %18, i32 noundef 8) #62
  ret void

19:                                               ; preds = %12, %46
  %20 = phi i64 [ 0, %12 ], [ %47, %46 ]
  %21 = mul nuw nsw i64 %20, %9
  %22 = trunc i64 %21 to i32
  %23 = getelementptr inbounds i8, ptr %5, i32 %22
  %24 = load i32, ptr %23, align 1
  %25 = icmp eq i32 %24, 0
  br i1 %25, label %46, label %26

26:                                               ; preds = %19
  %27 = zext i32 %24 to i64
  %28 = and i64 %10, %27
  %29 = load ptr, ptr %4, align 8, !tbaa !240
  %30 = mul nsw i64 %28, %9
  %31 = trunc i64 %30 to i32
  %32 = getelementptr inbounds i8, ptr %29, i32 %31
  %33 = load i32, ptr %32, align 1
  %34 = icmp eq i32 %33, 0
  br i1 %34, label %44, label %35

35:                                               ; preds = %26, %35
  %36 = phi i64 [ %38, %35 ], [ %28, %26 ]
  %37 = add i64 %36, 1
  %38 = and i64 %37, %10
  %39 = mul nsw i64 %38, %9
  %40 = trunc i64 %39 to i32
  %41 = getelementptr inbounds i8, ptr %29, i32 %40
  %42 = load i32, ptr %41, align 1
  %43 = icmp eq i32 %42, 0
  br i1 %43, label %44, label %35, !llvm.loop !261

44:                                               ; preds = %35, %26
  %45 = phi ptr [ %32, %26 ], [ %41, %35 ]
  tail call fastcc void @copy_bytes(ptr noundef nonnull %45, ptr noundef nonnull %23, i32 noundef %13) #61
  br label %46

46:                                               ; preds = %44, %19
  %47 = add nuw i64 %20, 1
  %48 = icmp eq i64 %20, %7
  br i1 %48, label %14, label %19, !llvm.loop !262
}

; Function Attrs: cold noinline nounwind optsize
define internal fastcc void @table_grow_index(ptr nocapture noundef %0, i64 noundef %1, ptr nocapture noundef readonly %2, i64 noundef %3) unnamed_addr #22 {
  %5 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %6 = load ptr, ptr %5, align 8, !tbaa !240
  %7 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %8 = load i64, ptr %7, align 8, !tbaa !247
  %9 = icmp ugt i64 %8, 2147483647
  %10 = trunc i64 %1 to i32
  %11 = add nsw i64 %1, -1
  %12 = icmp ugt i64 %11, 2147483647
  %13 = select i1 %12, i32 4, i32 3
  %14 = shl i32 %10, %13
  %15 = tail call ptr @fa_alloc(i32 noundef %14, i32 noundef 8) #62
  store ptr %15, ptr %5, align 8, !tbaa !240
  tail call void @llvm.memset.p0.i32(ptr align 1 %15, i8 0, i32 %14, i1 false)
  store i64 %11, ptr %7, align 8, !tbaa !247
  br i1 %9, label %16, label %18

16:                                               ; preds = %4
  %17 = icmp slt i64 %8, 0
  br i1 %17, label %209, label %159

18:                                               ; preds = %4
  br i1 %12, label %19, label %112

19:                                               ; preds = %18
  %20 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 2
  %21 = load i64, ptr %20, align 8, !tbaa !255
  %22 = icmp sgt i64 %21, 0
  br i1 %22, label %23, label %209

23:                                               ; preds = %19
  %24 = and i64 %3, 2199023255552
  %25 = icmp eq i64 %24, 0
  %26 = add i64 %3, 4
  %27 = and i64 %3, 1099511627776
  %28 = icmp eq i64 %27, 0
  %29 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 8
  br label %30

30:                                               ; preds = %23, %108
  %31 = phi i64 [ 0, %23 ], [ %109, %108 ]
  br i1 %25, label %40, label %32

32:                                               ; preds = %30
  %33 = load ptr, ptr %5, align 8, !tbaa !240
  %34 = mul i64 %31, %26
  %35 = trunc i64 %34 to i32
  %36 = getelementptr inbounds i8, ptr %33, i32 %35
  %37 = load i32, ptr %36, align 1
  %38 = icmp ne i32 %37, 0
  %39 = zext i1 %38 to i32
  br label %50

40:                                               ; preds = %30
  %41 = load ptr, ptr %0, align 8, !tbaa !243
  %42 = lshr i64 %31, 5
  %43 = trunc i64 %42 to i32
  %44 = getelementptr inbounds %struct.TableChunk, ptr %41, i32 %43, i32 1
  %45 = load i32, ptr %44, align 4, !tbaa !246
  %46 = trunc i64 %31 to i32
  %47 = and i32 %46, 31
  %48 = lshr i32 %45, %47
  %49 = and i32 %48, 1
  br label %50

50:                                               ; preds = %32, %40
  %51 = phi i32 [ %39, %32 ], [ %49, %40 ]
  %52 = icmp eq i32 %51, 0
  br i1 %52, label %108, label %53

53:                                               ; preds = %50
  br i1 %28, label %54, label %60

54:                                               ; preds = %53
  %55 = load ptr, ptr %0, align 8, !tbaa !243
  %56 = lshr i64 %31, 5
  %57 = trunc i64 %56 to i32
  %58 = getelementptr inbounds %struct.TableChunk, ptr %55, i32 %57
  %59 = and i64 %31, 31
  br label %60

60:                                               ; preds = %53, %54
  %61 = phi i64 [ %59, %54 ], [ %31, %53 ]
  %62 = phi ptr [ %58, %54 ], [ %29, %53 ]
  %63 = load ptr, ptr %62, align 4, !tbaa !6
  %64 = mul i64 %61, %3
  %65 = trunc i64 %64 to i32
  %66 = getelementptr inbounds i8, ptr %63, i32 %65
  %67 = tail call i64 %2(ptr noundef %66) #62
  %68 = load i64, ptr %7, align 8, !tbaa !247
  %69 = icmp ugt i64 %68, 2147483647
  %70 = load ptr, ptr %5, align 8, !tbaa !240
  %71 = and i64 %68, %67
  %72 = trunc i64 %71 to i32
  br i1 %69, label %91, label %73

73:                                               ; preds = %60
  %74 = getelementptr inbounds i64, ptr %70, i32 %72
  %75 = load i64, ptr %74, align 8, !tbaa !89
  %76 = icmp eq i64 %75, 0
  br i1 %76, label %85, label %77

77:                                               ; preds = %73, %77
  %78 = phi i64 [ %80, %77 ], [ %71, %73 ]
  %79 = add nsw i64 %78, 1
  %80 = and i64 %79, %68
  %81 = trunc i64 %80 to i32
  %82 = getelementptr inbounds i64, ptr %70, i32 %81
  %83 = load i64, ptr %82, align 8, !tbaa !89
  %84 = icmp eq i64 %83, 0
  br i1 %84, label %85, label %77, !llvm.loop !263

85:                                               ; preds = %77, %73
  %86 = phi ptr [ %74, %73 ], [ %82, %77 ]
  %87 = and i64 %67, 4294967295
  %88 = shl i64 %31, 32
  %89 = add i64 %88, 4294967296
  %90 = or disjoint i64 %87, %89
  store i64 %90, ptr %86, align 8, !tbaa !89
  br label %108

91:                                               ; preds = %60
  %92 = getelementptr inbounds %struct.WideSlot, ptr %70, i32 %72, i32 1
  %93 = load i64, ptr %92, align 8, !tbaa !253
  %94 = icmp eq i64 %93, 0
  br i1 %94, label %103, label %95

95:                                               ; preds = %91, %95
  %96 = phi i64 [ %98, %95 ], [ %71, %91 ]
  %97 = add i64 %96, 1
  %98 = and i64 %97, %68
  %99 = trunc i64 %98 to i32
  %100 = getelementptr inbounds %struct.WideSlot, ptr %70, i32 %99, i32 1
  %101 = load i64, ptr %100, align 8, !tbaa !253
  %102 = icmp eq i64 %101, 0
  br i1 %102, label %103, label %95, !llvm.loop !264

103:                                              ; preds = %95, %91
  %104 = phi i32 [ %72, %91 ], [ %99, %95 ]
  %105 = phi ptr [ %92, %91 ], [ %100, %95 ]
  %106 = getelementptr inbounds %struct.WideSlot, ptr %70, i32 %104
  store i64 %67, ptr %106, align 8, !tbaa !251
  %107 = add nuw nsw i64 %31, 1
  store i64 %107, ptr %105, align 8, !tbaa !253
  br label %108

108:                                              ; preds = %103, %85, %50
  %109 = add nuw nsw i64 %31, 1
  %110 = load i64, ptr %20, align 8, !tbaa !255
  %111 = icmp slt i64 %109, %110
  br i1 %111, label %30, label %209, !llvm.loop !265

112:                                              ; preds = %18, %156
  %113 = phi i64 [ %157, %156 ], [ 0, %18 ]
  %114 = trunc i64 %113 to i32
  %115 = getelementptr inbounds i64, ptr %6, i32 %114
  %116 = load i64, ptr %115, align 8, !tbaa !89
  %117 = icmp eq i64 %116, 0
  br i1 %117, label %156, label %118

118:                                              ; preds = %112
  %119 = and i64 %116, 4294967295
  %120 = lshr i64 %116, 32
  %121 = load i64, ptr %7, align 8, !tbaa !247
  %122 = icmp ugt i64 %121, 2147483647
  %123 = load ptr, ptr %5, align 8, !tbaa !240
  %124 = and i64 %121, %119
  %125 = trunc i64 %124 to i32
  br i1 %122, label %140, label %126

126:                                              ; preds = %118
  %127 = getelementptr inbounds i64, ptr %123, i32 %125
  %128 = load i64, ptr %127, align 8, !tbaa !89
  %129 = icmp eq i64 %128, 0
  br i1 %129, label %138, label %130

130:                                              ; preds = %126, %130
  %131 = phi i64 [ %133, %130 ], [ %124, %126 ]
  %132 = add nsw i64 %131, 1
  %133 = and i64 %132, %121
  %134 = trunc i64 %133 to i32
  %135 = getelementptr inbounds i64, ptr %123, i32 %134
  %136 = load i64, ptr %135, align 8, !tbaa !89
  %137 = icmp eq i64 %136, 0
  br i1 %137, label %138, label %130, !llvm.loop !263

138:                                              ; preds = %130, %126
  %139 = phi ptr [ %127, %126 ], [ %135, %130 ]
  store i64 %116, ptr %139, align 8, !tbaa !89
  br label %156

140:                                              ; preds = %118
  %141 = getelementptr inbounds %struct.WideSlot, ptr %123, i32 %125, i32 1
  %142 = load i64, ptr %141, align 8, !tbaa !253
  %143 = icmp eq i64 %142, 0
  br i1 %143, label %152, label %144

144:                                              ; preds = %140, %144
  %145 = phi i64 [ %147, %144 ], [ %124, %140 ]
  %146 = add i64 %145, 1
  %147 = and i64 %146, %121
  %148 = trunc i64 %147 to i32
  %149 = getelementptr inbounds %struct.WideSlot, ptr %123, i32 %148, i32 1
  %150 = load i64, ptr %149, align 8, !tbaa !253
  %151 = icmp eq i64 %150, 0
  br i1 %151, label %152, label %144, !llvm.loop !264

152:                                              ; preds = %144, %140
  %153 = phi i32 [ %125, %140 ], [ %148, %144 ]
  %154 = phi ptr [ %141, %140 ], [ %149, %144 ]
  %155 = getelementptr inbounds %struct.WideSlot, ptr %123, i32 %153
  store i64 %119, ptr %155, align 8, !tbaa !251
  store i64 %120, ptr %154, align 8, !tbaa !253
  br label %156

156:                                              ; preds = %152, %138, %112
  %157 = add nuw i64 %113, 1
  %158 = icmp eq i64 %113, %8
  br i1 %158, label %209, label %112, !llvm.loop !266

159:                                              ; preds = %16, %206
  %160 = phi i64 [ %207, %206 ], [ 0, %16 ]
  %161 = trunc i64 %160 to i32
  %162 = getelementptr inbounds %struct.WideSlot, ptr %6, i32 %161, i32 1
  %163 = load i64, ptr %162, align 8, !tbaa !253
  %164 = icmp eq i64 %163, 0
  br i1 %164, label %206, label %165

165:                                              ; preds = %159
  %166 = getelementptr inbounds %struct.WideSlot, ptr %6, i32 %161
  %167 = load i64, ptr %166, align 8, !tbaa !251
  %168 = load i64, ptr %7, align 8, !tbaa !247
  %169 = icmp ugt i64 %168, 2147483647
  %170 = load ptr, ptr %5, align 8, !tbaa !240
  %171 = and i64 %168, %167
  %172 = trunc i64 %171 to i32
  br i1 %169, label %190, label %173

173:                                              ; preds = %165
  %174 = getelementptr inbounds i64, ptr %170, i32 %172
  %175 = load i64, ptr %174, align 8, !tbaa !89
  %176 = icmp eq i64 %175, 0
  br i1 %176, label %185, label %177

177:                                              ; preds = %173, %177
  %178 = phi i64 [ %180, %177 ], [ %171, %173 ]
  %179 = add nsw i64 %178, 1
  %180 = and i64 %179, %168
  %181 = trunc i64 %180 to i32
  %182 = getelementptr inbounds i64, ptr %170, i32 %181
  %183 = load i64, ptr %182, align 8, !tbaa !89
  %184 = icmp eq i64 %183, 0
  br i1 %184, label %185, label %177, !llvm.loop !263

185:                                              ; preds = %177, %173
  %186 = phi ptr [ %174, %173 ], [ %182, %177 ]
  %187 = and i64 %167, 4294967295
  %188 = shl i64 %163, 32
  %189 = or disjoint i64 %187, %188
  store i64 %189, ptr %186, align 8, !tbaa !89
  br label %206

190:                                              ; preds = %165
  %191 = getelementptr inbounds %struct.WideSlot, ptr %170, i32 %172, i32 1
  %192 = load i64, ptr %191, align 8, !tbaa !253
  %193 = icmp eq i64 %192, 0
  br i1 %193, label %202, label %194

194:                                              ; preds = %190, %194
  %195 = phi i64 [ %197, %194 ], [ %171, %190 ]
  %196 = add i64 %195, 1
  %197 = and i64 %196, %168
  %198 = trunc i64 %197 to i32
  %199 = getelementptr inbounds %struct.WideSlot, ptr %170, i32 %198, i32 1
  %200 = load i64, ptr %199, align 8, !tbaa !253
  %201 = icmp eq i64 %200, 0
  br i1 %201, label %202, label %194, !llvm.loop !264

202:                                              ; preds = %194, %190
  %203 = phi i32 [ %172, %190 ], [ %198, %194 ]
  %204 = phi ptr [ %191, %190 ], [ %199, %194 ]
  %205 = getelementptr inbounds %struct.WideSlot, ptr %170, i32 %203
  store i64 %167, ptr %205, align 8, !tbaa !251
  store i64 %163, ptr %204, align 8, !tbaa !253
  br label %206

206:                                              ; preds = %202, %185, %159
  %207 = add nuw i64 %160, 1
  %208 = icmp eq i64 %160, %8
  br i1 %208, label %209, label %159, !llvm.loop !267

209:                                              ; preds = %156, %108, %206, %19, %16
  %210 = phi i32 [ 3, %19 ], [ 4, %16 ], [ 4, %206 ], [ 3, %108 ], [ 3, %156 ]
  %211 = trunc i64 %8 to i32
  %212 = add i32 %211, 1
  %213 = shl i32 %212, %210
  tail call void @fa_free(ptr noundef %6, i32 noundef %213, i32 noundef 8) #62
  ret void
}

; Function Attrs: cold noinline nounwind optsize
define internal fastcc void @table_dense_room(ptr nocapture noundef %0, i64 noundef %1, i64 noundef %2) unnamed_addr #22 {
  %4 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 9
  %5 = load i64, ptr %4, align 8, !tbaa !259
  %6 = icmp slt i64 %5, %1
  br i1 %6, label %7, label %29

7:                                                ; preds = %3
  %8 = icmp eq i64 %5, 0
  %9 = shl nsw i64 %5, 1
  %10 = select i1 %8, i64 32, i64 %9
  %11 = tail call i64 @llvm.smax.i64(i64 %10, i64 %1)
  %12 = tail call i64 @llvm.smax.i64(i64 %2, i64 1)
  %13 = mul nsw i64 %11, %12
  %14 = trunc i64 %13 to i32
  %15 = tail call ptr @fa_alloc(i32 noundef %14, i32 noundef 16) #62
  %16 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 8
  %17 = load ptr, ptr %16, align 8, !tbaa !242
  %18 = icmp eq ptr %17, null
  br i1 %18, label %28, label %19

19:                                               ; preds = %7
  %20 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 3
  %21 = load i64, ptr %20, align 8, !tbaa !256
  %22 = mul nsw i64 %21, %2
  %23 = trunc i64 %22 to i32
  tail call fastcc void @copy_bytes(ptr noundef %15, ptr noundef nonnull %17, i32 noundef %23) #61
  %24 = load ptr, ptr %16, align 8, !tbaa !242
  %25 = load i64, ptr %4, align 8, !tbaa !259
  %26 = mul nsw i64 %25, %12
  %27 = trunc i64 %26 to i32
  tail call void @fa_free(ptr noundef %24, i32 noundef %27, i32 noundef 16) #62
  br label %28

28:                                               ; preds = %19, %7
  store ptr %15, ptr %16, align 8, !tbaa !242
  store i64 %11, ptr %4, align 8, !tbaa !259
  br label %29

29:                                               ; preds = %3, %28
  ret void
}

; Function Attrs: alwaysinline nounwind optsize
define hidden ptr @fa_table_append(ptr nocapture noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3, ptr nocapture noundef readonly %4) local_unnamed_addr #13 {
  %6 = and i64 %2, 2199023255552
  %7 = icmp eq i64 %6, 0
  br i1 %7, label %59, label %8

8:                                                ; preds = %5
  %9 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 1
  %10 = load i64, ptr %9, align 8, !tbaa !254
  %11 = shl i64 %10, 1
  %12 = add i64 %11, 2
  %13 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %14 = load i64, ptr %13, align 8, !tbaa !247
  %15 = add nsw i64 %14, 1
  %16 = icmp sgt i64 %12, %15
  br i1 %16, label %17, label %20, !prof !35

17:                                               ; preds = %8
  %18 = shl nsw i64 %15, 1
  tail call fastcc void @inline_grow(ptr noundef nonnull %0, i64 noundef %18, i64 noundef %2) #66
  %19 = load i64, ptr %13, align 8, !tbaa !247
  br label %29

20:                                               ; preds = %8
  %21 = icmp sgt i64 %3, -1
  br i1 %21, label %29, label %22

22:                                               ; preds = %20
  %23 = xor i64 %3, -1
  %24 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %25 = load ptr, ptr %24, align 8, !tbaa !240
  %26 = add i64 %2, 4
  %27 = mul i64 %26, %23
  %28 = trunc i64 %27 to i32
  br label %49

29:                                               ; preds = %17, %20
  %30 = phi i64 [ %19, %17 ], [ %14, %20 ]
  %31 = and i64 %30, %1
  %32 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %33 = load ptr, ptr %32, align 8, !tbaa !240
  %34 = add i64 %2, 4
  %35 = mul i64 %31, %34
  %36 = trunc i64 %35 to i32
  %37 = getelementptr inbounds i8, ptr %33, i32 %36
  %38 = load i32, ptr %37, align 1
  %39 = icmp eq i32 %38, 0
  br i1 %39, label %49, label %40

40:                                               ; preds = %29, %40
  %41 = phi i64 [ %43, %40 ], [ %31, %29 ]
  %42 = add i64 %41, 1
  %43 = and i64 %42, %30
  %44 = mul i64 %43, %34
  %45 = trunc i64 %44 to i32
  %46 = getelementptr inbounds i8, ptr %33, i32 %45
  %47 = load i32, ptr %46, align 1
  %48 = icmp eq i32 %47, 0
  br i1 %48, label %49, label %40, !llvm.loop !268

49:                                               ; preds = %40, %22, %29
  %50 = phi i32 [ %28, %22 ], [ %36, %29 ], [ %45, %40 ]
  %51 = phi ptr [ %25, %22 ], [ %33, %29 ], [ %33, %40 ]
  %52 = getelementptr inbounds i8, ptr %51, i32 %50
  %53 = trunc i64 %1 to i32
  %54 = icmp eq i32 %53, 0
  %55 = select i1 %54, i32 -2147483648, i32 %53
  store i32 %55, ptr %52, align 1
  %56 = load i64, ptr %9, align 8, !tbaa !254
  %57 = add nsw i64 %56, 1
  store i64 %57, ptr %9, align 8, !tbaa !254
  %58 = getelementptr inbounds i8, ptr %52, i32 4
  br label %168

59:                                               ; preds = %5
  %60 = tail call i64 @llvm.smin.i64(i64 %3, i64 0)
  %61 = xor i64 %60, -1
  %62 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 1
  %63 = load i64, ptr %62, align 8, !tbaa !254
  %64 = shl i64 %63, 1
  %65 = add i64 %64, 2
  %66 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %67 = load i64, ptr %66, align 8, !tbaa !247
  %68 = add nsw i64 %67, 1
  %69 = icmp sgt i64 %65, %68
  br i1 %69, label %70, label %72, !prof !35

70:                                               ; preds = %59
  %71 = shl nsw i64 %68, 1
  tail call fastcc void @table_grow_index(ptr noundef nonnull %0, i64 noundef %71, ptr noundef %4, i64 noundef %2) #66
  br label %72

72:                                               ; preds = %70, %59
  %73 = phi i64 [ -1, %70 ], [ %61, %59 ]
  %74 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 7
  %75 = load i64, ptr %74, align 8, !tbaa !258
  %76 = icmp eq i64 %75, 0
  br i1 %76, label %77, label %79, !prof !35

77:                                               ; preds = %72
  tail call fastcc void @table_new_chunk(ptr noundef nonnull %0, i64 noundef %2) #66
  %78 = load i64, ptr %74, align 8, !tbaa !258
  br label %79

79:                                               ; preds = %77, %72
  %80 = phi i64 [ %78, %77 ], [ %75, %72 ]
  %81 = add nsw i64 %80, -1
  %82 = load ptr, ptr %0, align 8, !tbaa !243
  %83 = trunc i64 %81 to i32
  %84 = getelementptr inbounds %struct.TableChunk, ptr %82, i32 %83, i32 1
  %85 = load i32, ptr %84, align 4, !tbaa !246
  %86 = xor i32 %85, -1
  %87 = tail call i32 @llvm.cttz.i32(i32 %86, i1 false), !range !269
  %88 = shl nuw i32 1, %87
  %89 = or i32 %88, %85
  store i32 %89, ptr %84, align 4, !tbaa !246
  %90 = icmp eq i32 %89, -1
  br i1 %90, label %91, label %95

91:                                               ; preds = %79
  %92 = getelementptr inbounds %struct.TableChunk, ptr %82, i32 %83, i32 2
  %93 = load i32, ptr %92, align 4, !tbaa !270
  %94 = zext i32 %93 to i64
  store i64 %94, ptr %74, align 8, !tbaa !258
  br label %95

95:                                               ; preds = %91, %79
  %96 = shl i64 %81, 5
  %97 = zext nneg i32 %87 to i64
  %98 = or i64 %96, %97
  %99 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 2
  %100 = load i64, ptr %99, align 8, !tbaa !255
  %101 = icmp slt i64 %98, %100
  br i1 %101, label %104, label %102

102:                                              ; preds = %95
  %103 = add nsw i64 %98, 1
  store i64 %103, ptr %99, align 8, !tbaa !255
  br label %104

104:                                              ; preds = %102, %95
  %105 = load i64, ptr %66, align 8, !tbaa !247
  %106 = icmp ugt i64 %105, 2147483647
  %107 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %108 = load ptr, ptr %107, align 8, !tbaa !240
  %109 = and i64 %105, %1
  %110 = icmp slt i64 %73, 0
  %111 = select i1 %110, i64 %109, i64 %73
  %112 = trunc i64 %111 to i32
  br i1 %106, label %131, label %113

113:                                              ; preds = %104
  %114 = getelementptr inbounds i64, ptr %108, i32 %112
  %115 = load i64, ptr %114, align 8, !tbaa !89
  %116 = icmp eq i64 %115, 0
  br i1 %116, label %125, label %117

117:                                              ; preds = %113, %117
  %118 = phi i64 [ %120, %117 ], [ %111, %113 ]
  %119 = add i64 %118, 1
  %120 = and i64 %119, %105
  %121 = trunc i64 %120 to i32
  %122 = getelementptr inbounds i64, ptr %108, i32 %121
  %123 = load i64, ptr %122, align 8, !tbaa !89
  %124 = icmp eq i64 %123, 0
  br i1 %124, label %125, label %117, !llvm.loop !263

125:                                              ; preds = %117, %113
  %126 = phi ptr [ %114, %113 ], [ %122, %117 ]
  %127 = and i64 %1, 4294967295
  %128 = shl i64 %98, 32
  %129 = add i64 %128, 4294967296
  %130 = or disjoint i64 %129, %127
  store i64 %130, ptr %126, align 8, !tbaa !89
  br label %148

131:                                              ; preds = %104
  %132 = getelementptr inbounds %struct.WideSlot, ptr %108, i32 %112, i32 1
  %133 = load i64, ptr %132, align 8, !tbaa !253
  %134 = icmp eq i64 %133, 0
  br i1 %134, label %143, label %135

135:                                              ; preds = %131, %135
  %136 = phi i64 [ %138, %135 ], [ %111, %131 ]
  %137 = add i64 %136, 1
  %138 = and i64 %137, %105
  %139 = trunc i64 %138 to i32
  %140 = getelementptr inbounds %struct.WideSlot, ptr %108, i32 %139, i32 1
  %141 = load i64, ptr %140, align 8, !tbaa !253
  %142 = icmp eq i64 %141, 0
  br i1 %142, label %143, label %135, !llvm.loop !264

143:                                              ; preds = %135, %131
  %144 = phi i32 [ %112, %131 ], [ %139, %135 ]
  %145 = phi ptr [ %132, %131 ], [ %140, %135 ]
  %146 = getelementptr inbounds %struct.WideSlot, ptr %108, i32 %144
  store i64 %1, ptr %146, align 8, !tbaa !251
  %147 = add nsw i64 %98, 1
  store i64 %147, ptr %145, align 8, !tbaa !253
  br label %148

148:                                              ; preds = %125, %143
  %149 = load i64, ptr %62, align 8, !tbaa !254
  %150 = add nsw i64 %149, 1
  store i64 %150, ptr %62, align 8, !tbaa !254
  %151 = and i64 %2, 1099511627776
  %152 = icmp eq i64 %151, 0
  br i1 %152, label %159, label %153

153:                                              ; preds = %148
  %154 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 8
  %155 = load ptr, ptr %154, align 8, !tbaa !242
  %156 = mul i64 %98, %2
  %157 = trunc i64 %156 to i32
  %158 = getelementptr inbounds i8, ptr %155, i32 %157
  br label %168

159:                                              ; preds = %148
  %160 = lshr i64 %98, 5
  %161 = trunc i64 %160 to i32
  %162 = getelementptr inbounds %struct.TableChunk, ptr %82, i32 %161
  %163 = load ptr, ptr %162, align 4, !tbaa !244
  %164 = and i64 %97, 31
  %165 = mul nsw i64 %164, %2
  %166 = trunc i64 %165 to i32
  %167 = getelementptr inbounds i8, ptr %163, i32 %166
  br label %168

168:                                              ; preds = %159, %153, %49
  %169 = phi ptr [ %58, %49 ], [ %158, %153 ], [ %167, %159 ]
  ret ptr %169
}

; Function Attrs: cold noinline nounwind optsize
define internal fastcc void @table_new_chunk(ptr nocapture noundef %0, i64 noundef %1) unnamed_addr #22 {
  %3 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 3
  %4 = load i64, ptr %3, align 8, !tbaa !256
  %5 = ashr i64 %4, 5
  %6 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 6
  %7 = load i64, ptr %6, align 8, !tbaa !257
  %8 = icmp eq i64 %5, %7
  br i1 %8, label %9, label %26

9:                                                ; preds = %2
  %10 = icmp ult i64 %4, 32
  %11 = shl nsw i64 %5, 1
  %12 = select i1 %10, i64 4, i64 %11
  %13 = trunc i64 %12 to i32
  %14 = mul i32 %13, 12
  %15 = tail call ptr @fa_alloc(i32 noundef %14, i32 noundef 4) #62
  %16 = load ptr, ptr %0, align 8, !tbaa !243
  %17 = icmp eq ptr %16, null
  br i1 %17, label %25, label %18

18:                                               ; preds = %9
  %19 = trunc i64 %5 to i32
  %20 = mul i32 %19, 12
  tail call fastcc void @copy_bytes(ptr noundef %15, ptr noundef nonnull %16, i32 noundef %20) #61
  %21 = load ptr, ptr %0, align 8, !tbaa !243
  %22 = load i64, ptr %6, align 8, !tbaa !257
  %23 = trunc i64 %22 to i32
  %24 = mul i32 %23, 12
  tail call void @fa_free(ptr noundef %21, i32 noundef %24, i32 noundef 4) #62
  br label %25

25:                                               ; preds = %18, %9
  store ptr %15, ptr %0, align 8, !tbaa !243
  store i64 %12, ptr %6, align 8, !tbaa !257
  br label %26

26:                                               ; preds = %25, %2
  %27 = and i64 %1, 1099511627776
  %28 = icmp eq i64 %27, 0
  br i1 %28, label %36, label %29

29:                                               ; preds = %26
  %30 = load i64, ptr %3, align 8, !tbaa !256
  %31 = add nsw i64 %30, 32
  %32 = and i64 %1, 1099511627775
  tail call fastcc void @table_dense_room(ptr noundef nonnull %0, i64 noundef %31, i64 noundef %32) #66
  %33 = load ptr, ptr %0, align 8, !tbaa !243
  %34 = trunc i64 %5 to i32
  %35 = getelementptr inbounds %struct.TableChunk, ptr %33, i32 %34
  store ptr null, ptr %35, align 4, !tbaa !244
  br label %46

36:                                               ; preds = %26
  %37 = and i64 %1, 1099511627775
  %38 = icmp eq i64 %37, 0
  %39 = trunc i64 %1 to i32
  %40 = shl i32 %39, 5
  %41 = select i1 %38, i32 16, i32 %40
  %42 = tail call ptr @fa_alloc(i32 noundef %41, i32 noundef 16) #62
  %43 = load ptr, ptr %0, align 8, !tbaa !243
  %44 = trunc i64 %5 to i32
  %45 = getelementptr inbounds %struct.TableChunk, ptr %43, i32 %44
  store ptr %42, ptr %45, align 4, !tbaa !244
  br label %46

46:                                               ; preds = %36, %29
  %47 = phi i32 [ %44, %36 ], [ %34, %29 ]
  %48 = phi ptr [ %43, %36 ], [ %33, %29 ]
  %49 = getelementptr inbounds %struct.TableChunk, ptr %48, i32 %47, i32 1
  store i32 0, ptr %49, align 4, !tbaa !246
  %50 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 7
  %51 = load i64, ptr %50, align 8, !tbaa !258
  %52 = trunc i64 %51 to i32
  %53 = getelementptr inbounds %struct.TableChunk, ptr %48, i32 %47, i32 2
  store i32 %52, ptr %53, align 4, !tbaa !270
  %54 = add nsw i64 %5, 1
  store i64 %54, ptr %50, align 8, !tbaa !258
  %55 = load i64, ptr %3, align 8, !tbaa !256
  %56 = add nsw i64 %55, 32
  store i64 %56, ptr %3, align 8, !tbaa !256
  ret void
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.cttz.i32(i32, i1 immarg) #32

; Function Attrs: alwaysinline nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden void @fa_table_release(ptr nocapture noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #42 {
  %4 = and i64 %2, 2199023255552
  %5 = icmp eq i64 %4, 0
  br i1 %5, label %56, label %6

6:                                                ; preds = %3
  %7 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %8 = load i64, ptr %7, align 8, !tbaa !247
  %9 = and i64 %2, 1099511627775
  %10 = add nuw nsw i64 %9, 4
  %11 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %12 = add i64 %1, 1
  %13 = and i64 %12, %8
  %14 = load ptr, ptr %11, align 8, !tbaa !240
  %15 = mul nsw i64 %13, %10
  %16 = trunc i64 %15 to i32
  %17 = getelementptr inbounds i8, ptr %14, i32 %16
  %18 = load i32, ptr %17, align 1
  %19 = icmp eq i32 %18, 0
  br i1 %19, label %50, label %20

20:                                               ; preds = %6
  %21 = trunc i64 %10 to i32
  br label %22

22:                                               ; preds = %20, %40
  %23 = phi ptr [ %14, %20 ], [ %41, %40 ]
  %24 = phi i32 [ %18, %20 ], [ %48, %40 ]
  %25 = phi ptr [ %17, %20 ], [ %47, %40 ]
  %26 = phi i64 [ %13, %20 ], [ %44, %40 ]
  %27 = phi i64 [ %1, %20 ], [ %42, %40 ]
  %28 = zext i32 %24 to i64
  %29 = and i64 %8, %28
  %30 = sub i64 %26, %29
  %31 = and i64 %30, %8
  %32 = sub i64 %26, %27
  %33 = and i64 %32, %8
  %34 = icmp ult i64 %31, %33
  br i1 %34, label %40, label %35

35:                                               ; preds = %22
  %36 = mul nsw i64 %27, %10
  %37 = trunc i64 %36 to i32
  %38 = getelementptr inbounds i8, ptr %23, i32 %37
  tail call fastcc void @copy_bytes(ptr noundef %38, ptr noundef nonnull %25, i32 noundef %21) #61
  %39 = load ptr, ptr %11, align 8, !tbaa !240
  br label %40

40:                                               ; preds = %35, %22
  %41 = phi ptr [ %39, %35 ], [ %23, %22 ]
  %42 = phi i64 [ %26, %35 ], [ %27, %22 ]
  %43 = add i64 %26, 1
  %44 = and i64 %43, %8
  %45 = mul nsw i64 %44, %10
  %46 = trunc i64 %45 to i32
  %47 = getelementptr inbounds i8, ptr %41, i32 %46
  %48 = load i32, ptr %47, align 1
  %49 = icmp eq i32 %48, 0
  br i1 %49, label %50, label %22

50:                                               ; preds = %40, %6
  %51 = phi ptr [ %14, %6 ], [ %41, %40 ]
  %52 = phi i64 [ %1, %6 ], [ %42, %40 ]
  %53 = mul nsw i64 %52, %10
  %54 = trunc i64 %53 to i32
  %55 = getelementptr inbounds i8, ptr %51, i32 %54
  store i32 0, ptr %55, align 1
  br label %75

56:                                               ; preds = %3
  %57 = load ptr, ptr %0, align 8, !tbaa !243
  %58 = ashr i64 %1, 5
  %59 = trunc i64 %58 to i32
  %60 = getelementptr inbounds %struct.TableChunk, ptr %57, i32 %59, i32 1
  %61 = load i32, ptr %60, align 4, !tbaa !246
  %62 = icmp eq i32 %61, -1
  br i1 %62, label %63, label %69

63:                                               ; preds = %56
  %64 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 7
  %65 = load i64, ptr %64, align 8, !tbaa !258
  %66 = trunc i64 %65 to i32
  %67 = getelementptr inbounds %struct.TableChunk, ptr %57, i32 %59, i32 2
  store i32 %66, ptr %67, align 4, !tbaa !270
  %68 = add nsw i64 %58, 1
  store i64 %68, ptr %64, align 8, !tbaa !258
  br label %69

69:                                               ; preds = %63, %56
  %70 = trunc i64 %1 to i32
  %71 = and i32 %70, 31
  %72 = shl nuw i32 1, %71
  %73 = xor i32 %72, -1
  %74 = and i32 %61, %73
  store i32 %74, ptr %60, align 4, !tbaa !246
  br label %75

75:                                               ; preds = %69, %50
  %76 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 1
  %77 = load i64, ptr %76, align 8, !tbaa !254
  %78 = add nsw i64 %77, -1
  store i64 %78, ptr %76, align 8, !tbaa !254
  ret void
}

; Function Attrs: alwaysinline nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden void @fa_table_kill(ptr nocapture noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3) local_unnamed_addr #42 {
  %5 = and i64 %3, 2199023255552
  %6 = icmp eq i64 %5, 0
  %7 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %8 = load i64, ptr %7, align 8, !tbaa !247
  %9 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  br i1 %6, label %57, label %10

10:                                               ; preds = %4
  %11 = and i64 %3, 1099511627775
  %12 = add nuw nsw i64 %11, 4
  %13 = add i64 %2, 1
  %14 = and i64 %13, %8
  %15 = load ptr, ptr %9, align 8, !tbaa !240
  %16 = mul nsw i64 %14, %12
  %17 = trunc i64 %16 to i32
  %18 = getelementptr inbounds i8, ptr %15, i32 %17
  %19 = load i32, ptr %18, align 1
  %20 = icmp eq i32 %19, 0
  br i1 %20, label %51, label %21

21:                                               ; preds = %10
  %22 = trunc i64 %12 to i32
  br label %23

23:                                               ; preds = %21, %41
  %24 = phi ptr [ %15, %21 ], [ %42, %41 ]
  %25 = phi i32 [ %19, %21 ], [ %49, %41 ]
  %26 = phi ptr [ %18, %21 ], [ %48, %41 ]
  %27 = phi i64 [ %14, %21 ], [ %45, %41 ]
  %28 = phi i64 [ %2, %21 ], [ %43, %41 ]
  %29 = zext i32 %25 to i64
  %30 = and i64 %8, %29
  %31 = sub i64 %27, %30
  %32 = and i64 %31, %8
  %33 = sub i64 %27, %28
  %34 = and i64 %33, %8
  %35 = icmp ult i64 %32, %34
  br i1 %35, label %41, label %36

36:                                               ; preds = %23
  %37 = mul nsw i64 %28, %12
  %38 = trunc i64 %37 to i32
  %39 = getelementptr inbounds i8, ptr %24, i32 %38
  tail call fastcc void @copy_bytes(ptr noundef %39, ptr noundef nonnull %26, i32 noundef %22) #61
  %40 = load ptr, ptr %9, align 8, !tbaa !240
  br label %41

41:                                               ; preds = %36, %23
  %42 = phi ptr [ %40, %36 ], [ %24, %23 ]
  %43 = phi i64 [ %27, %36 ], [ %28, %23 ]
  %44 = add i64 %27, 1
  %45 = and i64 %44, %8
  %46 = mul nsw i64 %45, %12
  %47 = trunc i64 %46 to i32
  %48 = getelementptr inbounds i8, ptr %42, i32 %47
  %49 = load i32, ptr %48, align 1
  %50 = icmp eq i32 %49, 0
  br i1 %50, label %51, label %23

51:                                               ; preds = %41, %10
  %52 = phi ptr [ %15, %10 ], [ %42, %41 ]
  %53 = phi i64 [ %2, %10 ], [ %43, %41 ]
  %54 = mul nsw i64 %53, %12
  %55 = trunc i64 %54 to i32
  %56 = getelementptr inbounds i8, ptr %52, i32 %55
  store i32 0, ptr %56, align 1
  br label %186

57:                                               ; preds = %4
  %58 = and i64 %8, %1
  %59 = icmp ugt i64 %8, 2147483647
  %60 = load ptr, ptr %9, align 8, !tbaa !240
  br i1 %59, label %78, label %61, !prof !35

61:                                               ; preds = %57
  %62 = shl i64 %2, 32
  %63 = add i64 %62, 4294967296
  %64 = trunc i64 %58 to i32
  %65 = getelementptr inbounds i64, ptr %60, i32 %64
  %66 = load i64, ptr %65, align 8, !tbaa !89
  %67 = and i64 %66, -4294967296
  %68 = icmp eq i64 %67, %63
  br i1 %68, label %94, label %69

69:                                               ; preds = %61, %69
  %70 = phi i64 [ %72, %69 ], [ %58, %61 ]
  %71 = add nsw i64 %70, 1
  %72 = and i64 %71, %8
  %73 = trunc i64 %72 to i32
  %74 = getelementptr inbounds i64, ptr %60, i32 %73
  %75 = load i64, ptr %74, align 8, !tbaa !89
  %76 = and i64 %75, -4294967296
  %77 = icmp eq i64 %76, %63
  br i1 %77, label %92, label %69, !llvm.loop !271

78:                                               ; preds = %57
  %79 = add nsw i64 %2, 1
  %80 = trunc i64 %58 to i32
  %81 = getelementptr inbounds %struct.WideSlot, ptr %60, i32 %80, i32 1
  %82 = load i64, ptr %81, align 8, !tbaa !253
  %83 = icmp eq i64 %82, %79
  br i1 %83, label %127, label %84

84:                                               ; preds = %78, %84
  %85 = phi i64 [ %87, %84 ], [ %58, %78 ]
  %86 = add i64 %85, 1
  %87 = and i64 %86, %8
  %88 = trunc i64 %87 to i32
  %89 = getelementptr inbounds %struct.WideSlot, ptr %60, i32 %88, i32 1
  %90 = load i64, ptr %89, align 8, !tbaa !253
  %91 = icmp eq i64 %90, %79
  br i1 %91, label %92, label %84, !llvm.loop !272

92:                                               ; preds = %69, %84
  %93 = phi i64 [ %87, %84 ], [ %72, %69 ]
  br i1 %59, label %127, label %94, !prof !273

94:                                               ; preds = %61, %92
  %95 = phi i64 [ %93, %92 ], [ %58, %61 ]
  %96 = add i64 %95, 1
  %97 = and i64 %96, %8
  %98 = trunc i64 %97 to i32
  %99 = getelementptr inbounds i64, ptr %60, i32 %98
  %100 = load i64, ptr %99, align 8, !tbaa !89
  %101 = icmp eq i64 %100, 0
  br i1 %101, label %123, label %102

102:                                              ; preds = %94, %115
  %103 = phi i64 [ %121, %115 ], [ %100, %94 ]
  %104 = phi i64 [ %118, %115 ], [ %97, %94 ]
  %105 = phi i64 [ %116, %115 ], [ %95, %94 ]
  %106 = and i64 %103, %8
  %107 = sub nsw i64 %104, %106
  %108 = and i64 %107, %8
  %109 = sub i64 %104, %105
  %110 = and i64 %109, %8
  %111 = icmp ult i64 %108, %110
  br i1 %111, label %115, label %112

112:                                              ; preds = %102
  %113 = trunc i64 %105 to i32
  %114 = getelementptr inbounds i64, ptr %60, i32 %113
  store i64 %103, ptr %114, align 8, !tbaa !89
  br label %115

115:                                              ; preds = %112, %102
  %116 = phi i64 [ %104, %112 ], [ %105, %102 ]
  %117 = add nsw i64 %104, 1
  %118 = and i64 %117, %8
  %119 = trunc i64 %118 to i32
  %120 = getelementptr inbounds i64, ptr %60, i32 %119
  %121 = load i64, ptr %120, align 8, !tbaa !89
  %122 = icmp eq i64 %121, 0
  br i1 %122, label %123, label %102

123:                                              ; preds = %115, %94
  %124 = phi i64 [ %95, %94 ], [ %116, %115 ]
  %125 = trunc i64 %124 to i32
  %126 = getelementptr inbounds i64, ptr %60, i32 %125
  br label %166

127:                                              ; preds = %78, %92
  %128 = phi i64 [ %93, %92 ], [ %58, %78 ]
  %129 = add i64 %128, 1
  %130 = and i64 %129, %8
  %131 = trunc i64 %130 to i32
  %132 = getelementptr inbounds %struct.WideSlot, ptr %60, i32 %131
  %133 = getelementptr inbounds i8, ptr %132, i32 8
  %134 = load i64, ptr %133, align 8, !tbaa.struct !249
  %135 = icmp eq i64 %134, 0
  br i1 %135, label %161, label %136

136:                                              ; preds = %127, %152
  %137 = phi i64 [ %159, %152 ], [ %134, %127 ]
  %138 = phi ptr [ %157, %152 ], [ %132, %127 ]
  %139 = phi i64 [ %155, %152 ], [ %130, %127 ]
  %140 = phi i64 [ %153, %152 ], [ %128, %127 ]
  %141 = load i64, ptr %138, align 8
  %142 = and i64 %141, %8
  %143 = sub i64 %139, %142
  %144 = and i64 %143, %8
  %145 = sub i64 %139, %140
  %146 = and i64 %145, %8
  %147 = icmp ult i64 %144, %146
  br i1 %147, label %152, label %148

148:                                              ; preds = %136
  %149 = trunc i64 %140 to i32
  %150 = getelementptr inbounds %struct.WideSlot, ptr %60, i32 %149
  store i64 %141, ptr %150, align 8, !tbaa.struct !250
  %151 = getelementptr inbounds i8, ptr %150, i32 8
  store i64 %137, ptr %151, align 8, !tbaa.struct !249
  br label %152

152:                                              ; preds = %148, %136
  %153 = phi i64 [ %139, %148 ], [ %140, %136 ]
  %154 = add i64 %139, 1
  %155 = and i64 %154, %8
  %156 = trunc i64 %155 to i32
  %157 = getelementptr inbounds %struct.WideSlot, ptr %60, i32 %156
  %158 = getelementptr inbounds i8, ptr %157, i32 8
  %159 = load i64, ptr %158, align 8, !tbaa.struct !249
  %160 = icmp eq i64 %159, 0
  br i1 %160, label %161, label %136

161:                                              ; preds = %152, %127
  %162 = phi i64 [ %128, %127 ], [ %153, %152 ]
  %163 = trunc i64 %162 to i32
  %164 = getelementptr inbounds %struct.WideSlot, ptr %60, i32 %163
  store i64 0, ptr %164, align 8, !tbaa !251
  %165 = getelementptr inbounds %struct.WideSlot, ptr %60, i32 %163, i32 1
  br label %166

166:                                              ; preds = %161, %123
  %167 = phi ptr [ %165, %161 ], [ %126, %123 ]
  store i64 0, ptr %167, align 8, !tbaa !89
  %168 = load ptr, ptr %0, align 8, !tbaa !243
  %169 = ashr i64 %2, 5
  %170 = trunc i64 %169 to i32
  %171 = getelementptr inbounds %struct.TableChunk, ptr %168, i32 %170, i32 1
  %172 = load i32, ptr %171, align 4, !tbaa !246
  %173 = icmp eq i32 %172, -1
  br i1 %173, label %174, label %180

174:                                              ; preds = %166
  %175 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 7
  %176 = load i64, ptr %175, align 8, !tbaa !258
  %177 = trunc i64 %176 to i32
  %178 = getelementptr inbounds %struct.TableChunk, ptr %168, i32 %170, i32 2
  store i32 %177, ptr %178, align 4, !tbaa !270
  %179 = add nsw i64 %169, 1
  store i64 %179, ptr %175, align 8, !tbaa !258
  br label %180

180:                                              ; preds = %174, %166
  %181 = trunc i64 %2 to i32
  %182 = and i32 %181, 31
  %183 = shl nuw i32 1, %182
  %184 = xor i32 %183, -1
  %185 = and i32 %172, %184
  store i32 %185, ptr %171, align 4, !tbaa !246
  br label %186

186:                                              ; preds = %180, %51
  %187 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 1
  %188 = load i64, ptr %187, align 8, !tbaa !254
  %189 = add nsw i64 %188, -1
  store i64 %189, ptr %187, align 8, !tbaa !254
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(write, argmem: readwrite, inaccessiblemem: none)
define hidden void @fa_table_clear(ptr nocapture noundef %0, i64 noundef %1) local_unnamed_addr #43 {
  %3 = and i64 %1, 2199023255552
  %4 = icmp eq i64 %3, 0
  br i1 %4, label %15, label %5

5:                                                ; preds = %2
  %6 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %7 = load ptr, ptr %6, align 8, !tbaa !240
  %8 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %9 = load i64, ptr %8, align 8, !tbaa !247
  %10 = add nsw i64 %9, 1
  %11 = add i64 %1, 4
  %12 = mul i64 %10, %11
  %13 = trunc i64 %12 to i32
  tail call void @llvm.memset.p0.i32(ptr align 1 %7, i8 0, i32 %13, i1 false)
  %14 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 1
  store i64 0, ptr %14, align 8, !tbaa !254
  br label %46

15:                                               ; preds = %2
  %16 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 3
  %17 = load i64, ptr %16, align 8, !tbaa !256
  %18 = ashr i64 %17, 5
  %19 = icmp sgt i64 %18, 0
  br i1 %19, label %20, label %22

20:                                               ; preds = %15
  %21 = load ptr, ptr %0, align 8, !tbaa !243
  br label %36

22:                                               ; preds = %36, %15
  %23 = zext i1 %19 to i64
  %24 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 7
  store i64 %23, ptr %24, align 8, !tbaa !258
  %25 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 1
  store i64 0, ptr %25, align 8, !tbaa !254
  %26 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 2
  store i64 0, ptr %26, align 8, !tbaa !255
  %27 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %28 = load ptr, ptr %27, align 8, !tbaa !240
  %29 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %30 = load i64, ptr %29, align 8, !tbaa !247
  %31 = trunc i64 %30 to i32
  %32 = add i32 %31, 1
  %33 = icmp ugt i64 %30, 2147483647
  %34 = select i1 %33, i32 4, i32 3
  %35 = shl i32 %32, %34
  tail call void @llvm.memset.p0.i32(ptr align 1 %28, i8 0, i32 %35, i1 false)
  br label %46

36:                                               ; preds = %20, %36
  %37 = phi i64 [ 0, %20 ], [ %40, %36 ]
  %38 = trunc i64 %37 to i32
  %39 = getelementptr inbounds %struct.TableChunk, ptr %21, i32 %38, i32 1
  store i32 0, ptr %39, align 4, !tbaa !246
  %40 = add nuw nsw i64 %37, 1
  %41 = icmp slt i64 %40, %18
  %42 = add i32 %38, 2
  %43 = select i1 %41, i32 %42, i32 0
  %44 = getelementptr inbounds %struct.TableChunk, ptr %21, i32 %38, i32 2
  store i32 %43, ptr %44, align 4, !tbaa !270
  %45 = icmp eq i64 %40, %18
  br i1 %45, label %22, label %36, !llvm.loop !274

46:                                               ; preds = %22, %5
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_table_free(ptr noundef %0, i64 noundef %1) local_unnamed_addr #3 {
  %3 = and i64 %1, 2199023255552
  %4 = icmp eq i64 %3, 0
  br i1 %4, label %14, label %5

5:                                                ; preds = %2
  %6 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %7 = load ptr, ptr %6, align 8, !tbaa !240
  %8 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %9 = load i64, ptr %8, align 8, !tbaa !247
  %10 = add nsw i64 %9, 1
  %11 = add i64 %1, 4
  %12 = mul i64 %10, %11
  %13 = trunc i64 %12 to i32
  tail call void @fa_free(ptr noundef %7, i32 noundef %13, i32 noundef 8) #62
  tail call void @fa_free(ptr noundef %0, i32 noundef 80, i32 noundef 8) #62
  br label %65

14:                                               ; preds = %2
  %15 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 3
  %16 = load i64, ptr %15, align 8, !tbaa !256
  %17 = ashr i64 %16, 5
  %18 = and i64 %1, 1099511627776
  %19 = icmp eq i64 %18, 0
  br i1 %19, label %31, label %20

20:                                               ; preds = %14
  %21 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 8
  %22 = load ptr, ptr %21, align 8, !tbaa !242
  %23 = icmp eq ptr %22, null
  br i1 %23, label %39, label %24

24:                                               ; preds = %20
  %25 = and i64 %1, 1099511627775
  %26 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 9
  %27 = load i64, ptr %26, align 8, !tbaa !259
  %28 = tail call i64 @llvm.umax.i64(i64 %25, i64 1)
  %29 = mul nsw i64 %27, %28
  %30 = trunc i64 %29 to i32
  tail call void @fa_free(ptr noundef nonnull %22, i32 noundef %30, i32 noundef 16) #62
  br label %39

31:                                               ; preds = %14
  %32 = icmp sgt i64 %17, 0
  br i1 %32, label %33, label %39

33:                                               ; preds = %31
  %34 = and i64 %1, 1099511627775
  %35 = icmp eq i64 %34, 0
  %36 = trunc i64 %1 to i32
  %37 = shl i32 %36, 5
  %38 = select i1 %35, i32 16, i32 %37
  br label %42

39:                                               ; preds = %42, %20, %24, %31
  %40 = load ptr, ptr %0, align 8, !tbaa !243
  %41 = icmp eq ptr %40, null
  br i1 %41, label %55, label %50

42:                                               ; preds = %33, %42
  %43 = phi i64 [ 0, %33 ], [ %48, %42 ]
  %44 = load ptr, ptr %0, align 8, !tbaa !243
  %45 = trunc i64 %43 to i32
  %46 = getelementptr inbounds %struct.TableChunk, ptr %44, i32 %45
  %47 = load ptr, ptr %46, align 4, !tbaa !244
  tail call void @fa_free(ptr noundef %47, i32 noundef %38, i32 noundef 16) #62
  %48 = add nuw nsw i64 %43, 1
  %49 = icmp eq i64 %48, %17
  br i1 %49, label %39, label %42, !llvm.loop !275

50:                                               ; preds = %39
  %51 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 6
  %52 = load i64, ptr %51, align 8, !tbaa !257
  %53 = trunc i64 %52 to i32
  %54 = mul i32 %53, 12
  tail call void @fa_free(ptr noundef nonnull %40, i32 noundef %54, i32 noundef 4) #62
  br label %55

55:                                               ; preds = %50, %39
  %56 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %57 = load ptr, ptr %56, align 8, !tbaa !240
  %58 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %59 = load i64, ptr %58, align 8, !tbaa !247
  %60 = trunc i64 %59 to i32
  %61 = add i32 %60, 1
  %62 = icmp ugt i64 %59, 2147483647
  %63 = select i1 %62, i32 4, i32 3
  %64 = shl i32 %61, %63
  tail call void @fa_free(ptr noundef %57, i32 noundef %64, i32 noundef 8) #62
  tail call void @fa_free(ptr noundef nonnull %0, i32 noundef 80, i32 noundef 8) #62
  br label %65

65:                                               ; preds = %55, %5
  ret void
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_table_copy(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #3 {
  %3 = tail call ptr @fa_alloc(i32 noundef 80, i32 noundef 8) #62
  tail call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 8 dereferenceable(80) %3, ptr noundef nonnull align 8 dereferenceable(80) %0, i32 80, i1 false), !tbaa.struct !276
  %4 = and i64 %1, 2199023255552
  %5 = icmp eq i64 %4, 0
  br i1 %5, label %17, label %6

6:                                                ; preds = %2
  %7 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %8 = load i64, ptr %7, align 8, !tbaa !247
  %9 = add nsw i64 %8, 1
  %10 = add i64 %1, 4
  %11 = mul i64 %9, %10
  %12 = trunc i64 %11 to i32
  %13 = tail call ptr @fa_alloc(i32 noundef %12, i32 noundef 8) #62
  %14 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 4
  store ptr %13, ptr %14, align 8, !tbaa !240
  %15 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %16 = load ptr, ptr %15, align 8, !tbaa !240
  tail call fastcc void @copy_bytes(ptr noundef %13, ptr noundef %16, i32 noundef %12) #61
  br label %82

17:                                               ; preds = %2
  %18 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 3
  %19 = load i64, ptr %18, align 8, !tbaa !256
  %20 = ashr i64 %19, 5
  %21 = and i64 %1, 1099511627776
  %22 = icmp eq i64 %21, 0
  %23 = icmp sgt i64 %20, 0
  br i1 %23, label %24, label %52

24:                                               ; preds = %17
  %25 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 6
  %26 = load i64, ptr %25, align 8, !tbaa !257
  %27 = trunc i64 %26 to i32
  %28 = mul i32 %27, 12
  %29 = tail call ptr @fa_alloc(i32 noundef %28, i32 noundef 4) #62
  store ptr %29, ptr %3, align 8, !tbaa !243
  %30 = and i64 %1, 1099511627775
  %31 = icmp eq i64 %30, 0
  %32 = trunc i64 %1 to i32
  %33 = shl i32 %32, 5
  %34 = select i1 %31, i32 16, i32 %33
  br label %35

35:                                               ; preds = %24, %49
  %36 = phi i64 [ 0, %24 ], [ %50, %49 ]
  %37 = load ptr, ptr %3, align 8, !tbaa !243
  %38 = trunc i64 %36 to i32
  %39 = getelementptr inbounds %struct.TableChunk, ptr %37, i32 %38
  %40 = load ptr, ptr %0, align 8, !tbaa !243
  %41 = getelementptr inbounds %struct.TableChunk, ptr %40, i32 %38
  tail call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %39, ptr noundef nonnull align 4 dereferenceable(12) %41, i32 12, i1 false), !tbaa.struct !277
  br i1 %22, label %42, label %49

42:                                               ; preds = %35
  %43 = tail call ptr @fa_alloc(i32 noundef %34, i32 noundef 16) #62
  %44 = load ptr, ptr %3, align 8, !tbaa !243
  %45 = getelementptr inbounds %struct.TableChunk, ptr %44, i32 %38
  store ptr %43, ptr %45, align 4, !tbaa !244
  %46 = load ptr, ptr %0, align 8, !tbaa !243
  %47 = getelementptr inbounds %struct.TableChunk, ptr %46, i32 %38
  %48 = load ptr, ptr %47, align 4, !tbaa !244
  tail call fastcc void @copy_bytes(ptr noundef %43, ptr noundef %48, i32 noundef %34) #61
  br label %49

49:                                               ; preds = %35, %42
  %50 = add nuw nsw i64 %36, 1
  %51 = icmp eq i64 %50, %20
  br i1 %51, label %52, label %35, !llvm.loop !278

52:                                               ; preds = %49, %17
  br i1 %22, label %70, label %53

53:                                               ; preds = %52
  %54 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 8
  %55 = load ptr, ptr %54, align 8, !tbaa !242
  %56 = icmp eq ptr %55, null
  br i1 %56, label %70, label %57

57:                                               ; preds = %53
  %58 = and i64 %1, 1099511627775
  %59 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 9
  %60 = load i64, ptr %59, align 8, !tbaa !259
  %61 = tail call i64 @llvm.umax.i64(i64 %58, i64 1)
  %62 = mul nsw i64 %60, %61
  %63 = trunc i64 %62 to i32
  %64 = tail call ptr @fa_alloc(i32 noundef %63, i32 noundef 16) #62
  %65 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 8
  store ptr %64, ptr %65, align 8, !tbaa !242
  %66 = load ptr, ptr %54, align 8, !tbaa !242
  %67 = load i64, ptr %18, align 8, !tbaa !256
  %68 = mul i64 %67, %1
  %69 = trunc i64 %68 to i32
  tail call fastcc void @copy_bytes(ptr noundef %64, ptr noundef %66, i32 noundef %69) #61
  br label %70

70:                                               ; preds = %57, %53, %52
  %71 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %72 = load i64, ptr %71, align 8, !tbaa !247
  %73 = trunc i64 %72 to i32
  %74 = add i32 %73, 1
  %75 = icmp ugt i64 %72, 2147483647
  %76 = select i1 %75, i32 4, i32 3
  %77 = shl i32 %74, %76
  %78 = tail call ptr @fa_alloc(i32 noundef %77, i32 noundef 8) #62
  %79 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 4
  store ptr %78, ptr %79, align 8, !tbaa !240
  %80 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %81 = load ptr, ptr %80, align 8, !tbaa !240
  tail call fastcc void @copy_bytes(ptr noundef %78, ptr noundef %81, i32 noundef %77) #61
  br label %82

82:                                               ; preds = %70, %6
  ret ptr %3
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
  br i1 %18, label %19, label %7, !llvm.loop !279

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
define hidden i64 @fa_hash_string_bits(i32 noundef %0, i32 noundef %1, i32 noundef %2) local_unnamed_addr #44 {
  %4 = and i32 %2, 1073741824
  %5 = icmp eq i32 %4, 0
  br i1 %5, label %6, label %9

6:                                                ; preds = %3
  %7 = inttoptr i32 %0 to ptr
  %8 = tail call i64 @fa_hash_bytes(ptr noundef %7, i32 noundef %1) #61
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
define hidden noundef ptr @fa_error_key_not_found() local_unnamed_addr #22 {
  %1 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #62
  store i64 2, ptr %1, align 8, !tbaa !233
  %2 = getelementptr inbounds %struct.MessageError, ptr %1, i32 0, i32 1
  tail call void @fa_b_take(ptr noundef nonnull %2) #61
  ret ptr %1
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(argmem: readwrite)
define hidden noundef ptr @fa_shared_retain(ptr noundef returned %0) local_unnamed_addr #45 {
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
  %8 = load ptr, ptr %7, align 8, !tbaa !280
  %9 = icmp eq ptr %8, null
  br i1 %9, label %11, label %10

10:                                               ; preds = %6
  tail call void %8(ptr noundef nonnull %0) #62
  br label %11

11:                                               ; preds = %10, %6
  %12 = getelementptr inbounds %struct.FaShared, ptr %0, i32 0, i32 1
  %13 = load i64, ptr %12, align 8, !tbaa !282
  %14 = trunc i64 %13 to i32
  tail call void @fa_free(ptr noundef nonnull %0, i32 noundef %14, i32 noundef 16) #62
  br label %15

15:                                               ; preds = %11, %3, %1
  ret void
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_atomic_new(i64 noundef %0) local_unnamed_addr #3 {
  %2 = tail call fastcc ptr @shared_new(i32 noundef 40, ptr noundef null) #61
  %3 = getelementptr inbounds %struct.FaAtomicBlock, ptr %2, i32 0, i32 1
  store i64 %0, ptr %3, align 8, !tbaa !283
  ret ptr %2
}

; Function Attrs: nounwind optsize
define internal fastcc ptr @shared_new(i32 noundef %0, ptr noundef %1) unnamed_addr #3 {
  %3 = tail call ptr @fa_alloc(i32 noundef %0, i32 noundef 16) #62
  %4 = icmp eq ptr %3, null
  br i1 %4, label %5, label %6

5:                                                ; preds = %2
  tail call fastcc void @out_of_memory() #65
  unreachable

6:                                                ; preds = %2
  tail call void @llvm.memset.p0.i32(ptr nonnull align 8 %3, i8 0, i32 %0, i1 false)
  store i64 1, ptr %3, align 8, !tbaa !285
  %7 = zext i32 %0 to i64
  %8 = getelementptr inbounds %struct.FaShared, ptr %3, i32 0, i32 1
  store i64 %7, ptr %8, align 8, !tbaa !282
  %9 = getelementptr inbounds %struct.FaShared, ptr %3, i32 0, i32 2
  store ptr %1, ptr %9, align 8, !tbaa !280
  ret ptr %3
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(argmem: readwrite)
define hidden void @fa_remote_source_start(ptr nocapture noundef %0) local_unnamed_addr #45 {
  %2 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 12
  %3 = atomicrmw add ptr %2, i64 1 acq_rel, align 8
  ret void
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(argmem: readwrite)
define hidden void @fa_remote_source_end(ptr nocapture noundef %0) local_unnamed_addr #45 {
  %2 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 14
  %3 = atomicrmw add ptr %2, i64 1 seq_cst, align 8
  %4 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 12
  %5 = atomicrmw sub ptr %4, i64 1 seq_cst, align 8
  %6 = atomicrmw sub ptr %2, i64 1 seq_cst, align 8
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_task_spawn(ptr noundef %0, i32 noundef %1) local_unnamed_addr #3 {
  %3 = tail call fastcc ptr @executor() #61
  %4 = icmp eq i32 %1, 0
  %5 = getelementptr inbounds i8, ptr %0, i32 32
  %6 = load i32, ptr %5, align 4, !tbaa !286
  br i1 %4, label %12, label %7

7:                                                ; preds = %2
  %8 = or i32 %6, 1
  %9 = getelementptr inbounds %struct.FaExecutor, ptr %3, i32 0, i32 11
  %10 = load i64, ptr %9, align 8, !tbaa !288
  %11 = add nsw i64 %10, 1
  store i64 %11, ptr %9, align 8, !tbaa !288
  br label %12

12:                                               ; preds = %2, %7
  %13 = phi i32 [ %8, %7 ], [ %6, %2 ]
  %14 = getelementptr inbounds i8, ptr %0, i32 16
  %15 = getelementptr inbounds i8, ptr %0, i32 32
  %16 = and i32 %13, -5
  store i32 %16, ptr %15, align 4, !tbaa !286
  %17 = getelementptr inbounds %struct.FaExecutor, ptr %3, i32 0, i32 1
  %18 = getelementptr inbounds i8, ptr %0, i32 20
  store ptr null, ptr %18, align 4, !tbaa !290
  %19 = load ptr, ptr %17, align 4, !tbaa !6
  %20 = getelementptr inbounds i8, ptr %0, i32 24
  store ptr %19, ptr %20, align 4, !tbaa !291
  %21 = icmp eq ptr %19, null
  %22 = getelementptr inbounds %struct.FaTask, ptr %19, i32 0, i32 1
  %23 = select i1 %21, ptr %3, ptr %22
  store ptr %14, ptr %23, align 4, !tbaa !6
  store ptr %14, ptr %17, align 4, !tbaa !6
  %24 = getelementptr inbounds %struct.FaExecutor, ptr %3, i32 0, i32 4
  %25 = load i64, ptr %24, align 8, !tbaa !292
  %26 = add nsw i64 %25, 1
  store i64 %26, ptr %24, align 8, !tbaa !292
  %27 = getelementptr inbounds %struct.FaExecutor, ptr %3, i32 0, i32 5
  %28 = load i64, ptr %27, align 8, !tbaa !293
  %29 = add nsw i64 %28, 1
  store i64 %29, ptr %27, align 8, !tbaa !293
  %30 = getelementptr inbounds i8, ptr %0, i32 28
  store atomic i32 1, ptr %30 release, align 4
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc ptr @executor() unnamed_addr #3 {
  %1 = load ptr, ptr @current_executor, align 4, !tbaa !6
  %2 = icmp eq ptr %1, null
  br i1 %2, label %3, label %10

3:                                                ; preds = %0
  %4 = tail call ptr @fa_alloc(i32 noundef 320, i32 noundef 16) #62
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(320) %4, i8 0, i32 320, i1 false)
  %5 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 17
  %6 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 15
  store ptr %5, ptr %6, align 8, !tbaa !294
  %7 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 16
  store ptr %5, ptr %7, align 4, !tbaa !295
  %8 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 19
  store i32 -1, ptr %8, align 8, !tbaa !296
  %9 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 20
  store i32 -1, ptr %9, align 4, !tbaa !297
  store i32 128, ptr @fa_coop_budget, align 4, !tbaa !23
  store ptr %4, ptr @current_executor, align 4, !tbaa !6
  br label %10

10:                                               ; preds = %0, %3
  %11 = phi ptr [ %4, %3 ], [ %1, %0 ]
  ret ptr %11
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define hidden void @fa_task_await(ptr nocapture noundef writeonly %0, ptr noundef %1) local_unnamed_addr #46 {
  %3 = getelementptr inbounds i8, ptr %0, i32 16
  store ptr %1, ptr %3, align 4, !tbaa !298
  %4 = getelementptr inbounds i8, ptr %1, i32 28
  store atomic i32 2, ptr %4 release, align 4
  %5 = load ptr, ptr @current_executor, align 4, !tbaa !6
  %6 = icmp eq ptr %5, null
  br i1 %6, label %15, label %7

7:                                                ; preds = %2
  %8 = getelementptr inbounds %struct.FaExecutor, ptr %5, i32 0, i32 10
  %9 = load ptr, ptr %8, align 8, !tbaa !299
  %10 = icmp eq ptr %9, %1
  %11 = icmp eq ptr %9, null
  %12 = or i1 %10, %11
  br i1 %12, label %13, label %15

13:                                               ; preds = %7
  %14 = getelementptr inbounds %struct.FaExecutor, ptr %5, i32 0, i32 25
  store ptr null, ptr %14, align 8, !tbaa !300
  br label %15

15:                                               ; preds = %7, %13, %2
  ret void
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define hidden void @fa_task_finish(ptr noundef %0) local_unnamed_addr #46 {
  %2 = getelementptr inbounds i8, ptr %0, i32 16
  %3 = getelementptr inbounds i8, ptr %0, i32 32
  %4 = load i32, ptr %3, align 4, !tbaa !286
  %5 = or i32 %4, 2
  store i32 %5, ptr %3, align 4, !tbaa !286
  %6 = getelementptr inbounds i8, ptr %0, i32 28
  store atomic i32 3, ptr %6 release, align 4
  %7 = and i32 %4, 1
  %8 = icmp eq i32 %7, 0
  br i1 %8, label %14, label %9

9:                                                ; preds = %1
  %10 = load ptr, ptr @current_executor, align 4, !tbaa !6
  %11 = getelementptr inbounds %struct.FaExecutor, ptr %10, i32 0, i32 11
  %12 = load i64, ptr %11, align 8, !tbaa !288
  %13 = add nsw i64 %12, -1
  store i64 %13, ptr %11, align 8, !tbaa !288
  store ptr %0, ptr @frame_to_free, align 4, !tbaa !6
  br label %40

14:                                               ; preds = %1
  %15 = load ptr, ptr %2, align 4, !tbaa !298
  %16 = icmp eq ptr %15, null
  br i1 %16, label %40, label %17

17:                                               ; preds = %14
  store ptr null, ptr %2, align 4, !tbaa !298
  %18 = ptrtoint ptr %15 to i32
  %19 = and i32 %18, 1
  %20 = icmp eq i32 %19, 0
  br i1 %20, label %39, label %21

21:                                               ; preds = %17
  %22 = and i32 %18, -2
  %23 = inttoptr i32 %22 to ptr
  %24 = getelementptr inbounds i8, ptr %0, i32 20
  store ptr null, ptr %24, align 4, !tbaa !290
  %25 = getelementptr inbounds %struct.FaJoin, ptr %23, i32 0, i32 3
  %26 = load ptr, ptr %25, align 4, !tbaa !301
  %27 = icmp eq ptr %26, null
  br i1 %27, label %30, label %28

28:                                               ; preds = %21
  %29 = getelementptr inbounds %struct.FaTask, ptr %26, i32 0, i32 1
  store ptr %2, ptr %29, align 4, !tbaa !290
  br label %32

30:                                               ; preds = %21
  %31 = getelementptr inbounds %struct.FaJoin, ptr %23, i32 0, i32 2
  store ptr %2, ptr %31, align 8, !tbaa !303
  br label %32

32:                                               ; preds = %30, %28
  store ptr %2, ptr %25, align 4, !tbaa !301
  %33 = getelementptr inbounds %struct.FaJoin, ptr %23, i32 0, i32 1
  %34 = load i64, ptr %33, align 8, !tbaa !304
  %35 = add nsw i64 %34, -1
  store i64 %35, ptr %33, align 8, !tbaa !304
  %36 = load ptr, ptr %23, align 8, !tbaa !305
  %37 = icmp eq ptr %36, null
  br i1 %37, label %40, label %38

38:                                               ; preds = %32
  store ptr %36, ptr @next_frame, align 4, !tbaa !6
  store ptr null, ptr %23, align 8, !tbaa !305
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
  br i1 %2, label %17, label %3

3:                                                ; preds = %1
  %4 = getelementptr inbounds i8, ptr %0, i32 16
  %5 = load ptr, ptr @current_executor, align 4, !tbaa !6
  %6 = icmp eq ptr %5, null
  br i1 %6, label %16, label %7

7:                                                ; preds = %3
  %8 = getelementptr inbounds i8, ptr %0, i32 28
  %9 = load atomic i32, ptr %8 acquire, align 4
  %10 = icmp eq i32 %9, 5
  br i1 %10, label %11, label %12

11:                                               ; preds = %7
  tail call fastcc void @drain_remote(ptr noundef nonnull %5) #61
  br label %12

12:                                               ; preds = %11, %7
  %13 = load atomic i32, ptr %8 acquire, align 4
  %14 = icmp eq i32 %13, 1
  br i1 %14, label %15, label %16

15:                                               ; preds = %12
  tail call fastcc void @ready_unlink(ptr noundef nonnull %5, ptr noundef nonnull %4) #61
  br label %16

16:                                               ; preds = %12, %15, %3
  tail call void @fa_destroy(ptr noundef %0) #62
  br label %17

17:                                               ; preds = %1, %16
  ret void
}

; Function Attrs: nofree norecurse nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc void @drain_remote(ptr noundef %0) unnamed_addr #33 {
  %2 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 16
  %3 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 17, i32 1
  %4 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 15
  %5 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 17
  %6 = ptrtoint ptr %5 to i32
  %7 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 1
  %8 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 4
  %9 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 5
  br label %10

10:                                               ; preds = %39, %1
  %11 = load ptr, ptr %4, align 8, !tbaa !294
  %12 = getelementptr inbounds %struct.FaTask, ptr %11, i32 0, i32 1
  %13 = load atomic i32, ptr %12 acquire, align 4
  %14 = inttoptr i32 %13 to ptr
  %15 = icmp eq ptr %11, %5
  br i1 %15, label %16, label %22

16:                                               ; preds = %10
  %17 = icmp eq i32 %13, 0
  br i1 %17, label %55, label %18

18:                                               ; preds = %16
  store ptr %14, ptr %4, align 8, !tbaa !294
  %19 = getelementptr inbounds %struct.FaTask, ptr %14, i32 0, i32 1
  %20 = load atomic i32, ptr %19 acquire, align 4
  %21 = inttoptr i32 %20 to ptr
  br label %22

22:                                               ; preds = %18, %10
  %23 = phi ptr [ %21, %18 ], [ %14, %10 ]
  %24 = phi ptr [ %14, %18 ], [ %11, %10 ]
  %25 = icmp eq ptr %23, null
  br i1 %25, label %26, label %39

26:                                               ; preds = %22
  %27 = load atomic i32, ptr %2 acquire, align 4
  %28 = inttoptr i32 %27 to ptr
  %29 = icmp eq ptr %24, %28
  br i1 %29, label %30, label %55

30:                                               ; preds = %26
  store atomic i32 0, ptr %3 monotonic, align 4
  %31 = atomicrmw xchg ptr %2, i32 %6 acq_rel, align 4
  %32 = inttoptr i32 %31 to ptr
  %33 = getelementptr inbounds %struct.FaTask, ptr %32, i32 0, i32 1
  store atomic i32 %6, ptr %33 release, align 4
  %34 = getelementptr inbounds %struct.FaTask, ptr %24, i32 0, i32 1
  %35 = load atomic i32, ptr %34 acquire, align 4
  %36 = icmp eq i32 %35, 0
  br i1 %36, label %55, label %37

37:                                               ; preds = %30
  %38 = inttoptr i32 %35 to ptr
  br label %39

39:                                               ; preds = %37, %22
  %40 = phi ptr [ %38, %37 ], [ %23, %22 ]
  store ptr %40, ptr %4, align 8, !tbaa !294
  %41 = getelementptr inbounds %struct.FaTask, ptr %24, i32 0, i32 4
  %42 = load i32, ptr %41, align 4, !tbaa !286
  %43 = and i32 %42, -5
  store i32 %43, ptr %41, align 4, !tbaa !286
  %44 = getelementptr inbounds %struct.FaTask, ptr %24, i32 0, i32 1
  store ptr null, ptr %44, align 4, !tbaa !290
  %45 = load ptr, ptr %7, align 4, !tbaa !6
  %46 = getelementptr inbounds %struct.FaTask, ptr %24, i32 0, i32 2
  store ptr %45, ptr %46, align 4, !tbaa !291
  %47 = icmp eq ptr %45, null
  %48 = getelementptr inbounds %struct.FaTask, ptr %45, i32 0, i32 1
  %49 = select i1 %47, ptr %0, ptr %48
  store ptr %24, ptr %49, align 4, !tbaa !6
  store ptr %24, ptr %7, align 4, !tbaa !6
  %50 = load i64, ptr %8, align 8, !tbaa !292
  %51 = add nsw i64 %50, 1
  store i64 %51, ptr %8, align 8, !tbaa !292
  %52 = load i64, ptr %9, align 8, !tbaa !293
  %53 = add nsw i64 %52, 1
  store i64 %53, ptr %9, align 8, !tbaa !293
  %54 = getelementptr inbounds %struct.FaTask, ptr %24, i32 0, i32 3
  store atomic i32 1, ptr %54 release, align 4
  br label %10, !llvm.loop !306

55:                                               ; preds = %16, %26, %30
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, argmem: readwrite, inaccessiblemem: none)
define internal fastcc void @ready_unlink(ptr nocapture noundef %0, ptr nocapture noundef %1) unnamed_addr #47 {
  %3 = getelementptr inbounds %struct.FaTask, ptr %1, i32 0, i32 4
  %4 = load i32, ptr %3, align 4, !tbaa !286
  %5 = and i32 %4, 4
  %6 = icmp eq i32 %5, 0
  br i1 %6, label %22, label %7

7:                                                ; preds = %2
  %8 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 2
  %9 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 3
  %10 = getelementptr inbounds %struct.FaTask, ptr %1, i32 0, i32 2
  %11 = load ptr, ptr %10, align 4, !tbaa !291
  %12 = icmp eq ptr %11, null
  %13 = getelementptr inbounds %struct.FaTask, ptr %1, i32 0, i32 1
  %14 = load ptr, ptr %13, align 4, !tbaa !290
  %15 = getelementptr inbounds %struct.FaTask, ptr %11, i32 0, i32 1
  %16 = select i1 %12, ptr %8, ptr %15
  store ptr %14, ptr %16, align 4, !tbaa !6
  %17 = icmp eq ptr %14, null
  %18 = load ptr, ptr %10, align 4, !tbaa !291
  %19 = getelementptr inbounds %struct.FaTask, ptr %14, i32 0, i32 2
  %20 = select i1 %17, ptr %9, ptr %19
  store ptr %18, ptr %20, align 4, !tbaa !6
  store ptr null, ptr %13, align 4, !tbaa !290
  store ptr null, ptr %10, align 4, !tbaa !291
  %21 = and i32 %4, -5
  store i32 %21, ptr %3, align 4, !tbaa !286
  br label %38

22:                                               ; preds = %2
  %23 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 1
  %24 = getelementptr inbounds %struct.FaTask, ptr %1, i32 0, i32 2
  %25 = load ptr, ptr %24, align 4, !tbaa !291
  %26 = icmp eq ptr %25, null
  %27 = getelementptr inbounds %struct.FaTask, ptr %1, i32 0, i32 1
  %28 = load ptr, ptr %27, align 4, !tbaa !290
  %29 = getelementptr inbounds %struct.FaTask, ptr %25, i32 0, i32 1
  %30 = select i1 %26, ptr %0, ptr %29
  store ptr %28, ptr %30, align 4, !tbaa !6
  %31 = icmp eq ptr %28, null
  %32 = load ptr, ptr %24, align 4, !tbaa !291
  %33 = getelementptr inbounds %struct.FaTask, ptr %28, i32 0, i32 2
  %34 = select i1 %31, ptr %23, ptr %33
  store ptr %32, ptr %34, align 4, !tbaa !6
  store ptr null, ptr %27, align 4, !tbaa !290
  store ptr null, ptr %24, align 4, !tbaa !291
  %35 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 5
  %36 = load i64, ptr %35, align 8, !tbaa !293
  %37 = add nsw i64 %36, -1
  store i64 %37, ptr %35, align 8, !tbaa !293
  br label %38

38:                                               ; preds = %22, %7
  %39 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 4
  %40 = load i64, ptr %39, align 8, !tbaa !292
  %41 = add nsw i64 %40, -1
  store i64 %41, ptr %39, align 8, !tbaa !292
  ret void
}

; Function Attrs: optsize
declare void @fa_destroy(ptr noundef) local_unnamed_addr #4

; Function Attrs: nounwind optsize
define hidden void @fa_task_yield(ptr noundef %0) local_unnamed_addr #3 {
  %2 = tail call fastcc ptr @executor() #61
  %3 = getelementptr inbounds %struct.FaExecutor, ptr %2, i32 0, i32 23
  store i32 1, ptr %3, align 8, !tbaa !307
  %4 = getelementptr inbounds i8, ptr %0, i32 16
  %5 = getelementptr inbounds i8, ptr %0, i32 32
  %6 = load i32, ptr %5, align 4, !tbaa !286
  %7 = or i32 %6, 4
  store i32 %7, ptr %5, align 4, !tbaa !286
  %8 = getelementptr inbounds %struct.FaExecutor, ptr %2, i32 0, i32 2
  %9 = getelementptr inbounds %struct.FaExecutor, ptr %2, i32 0, i32 3
  %10 = getelementptr inbounds i8, ptr %0, i32 20
  store ptr null, ptr %10, align 4, !tbaa !290
  %11 = load ptr, ptr %9, align 4, !tbaa !6
  %12 = getelementptr inbounds i8, ptr %0, i32 24
  store ptr %11, ptr %12, align 4, !tbaa !291
  %13 = icmp eq ptr %11, null
  %14 = getelementptr inbounds %struct.FaTask, ptr %11, i32 0, i32 1
  %15 = select i1 %13, ptr %8, ptr %14
  store ptr %4, ptr %15, align 4, !tbaa !6
  store ptr %4, ptr %9, align 4, !tbaa !6
  %16 = getelementptr inbounds %struct.FaExecutor, ptr %2, i32 0, i32 4
  %17 = load i64, ptr %16, align 8, !tbaa !292
  %18 = add nsw i64 %17, 1
  store i64 %18, ptr %16, align 8, !tbaa !292
  %19 = getelementptr inbounds i8, ptr %0, i32 28
  store atomic i32 1, ptr %19 release, align 4
  ret void
}

; Function Attrs: nounwind optsize
define hidden i32 @fa_coop_back_edge() local_unnamed_addr #3 {
  %1 = load ptr, ptr @current_executor, align 4, !tbaa !6
  store i32 128, ptr @fa_coop_budget, align 4, !tbaa !23
  %2 = icmp eq ptr %1, null
  br i1 %2, label %13, label %3

3:                                                ; preds = %0
  %4 = tail call i64 @fa_clock() #62
  %5 = getelementptr inbounds %struct.FaExecutor, ptr %1, i32 0, i32 24
  %6 = load i64, ptr %5, align 8, !tbaa !308
  %7 = icmp eq i64 %6, 0
  br i1 %7, label %8, label %9

8:                                                ; preds = %3
  store i64 %4, ptr %5, align 8, !tbaa !308
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
  %4 = tail call fastcc ptr @executor() #61
  store ptr %1, ptr %0, align 8, !tbaa !309
  %5 = getelementptr inbounds %struct.FaWait, ptr %0, i32 0, i32 1
  store ptr %4, ptr %5, align 4, !tbaa !311
  %6 = getelementptr inbounds %struct.FaWait, ptr %0, i32 0, i32 8
  store i64 -1, ptr %6, align 8, !tbaa !312
  %7 = getelementptr inbounds %struct.FaWait, ptr %0, i32 0, i32 11
  store ptr null, ptr %7, align 8, !tbaa !313
  %8 = icmp slt i64 %2, 1
  br i1 %8, label %9, label %26

9:                                                ; preds = %3
  %10 = getelementptr inbounds i8, ptr %1, i32 16
  %11 = getelementptr inbounds i8, ptr %1, i32 32
  %12 = load i32, ptr %11, align 4, !tbaa !286
  %13 = or i32 %12, 4
  store i32 %13, ptr %11, align 4, !tbaa !286
  %14 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 2
  %15 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 3
  %16 = getelementptr inbounds i8, ptr %1, i32 20
  store ptr null, ptr %16, align 4, !tbaa !290
  %17 = load ptr, ptr %15, align 4, !tbaa !6
  %18 = getelementptr inbounds i8, ptr %1, i32 24
  store ptr %17, ptr %18, align 4, !tbaa !291
  %19 = icmp eq ptr %17, null
  %20 = getelementptr inbounds %struct.FaTask, ptr %17, i32 0, i32 1
  %21 = select i1 %19, ptr %14, ptr %20
  store ptr %10, ptr %21, align 4, !tbaa !6
  store ptr %10, ptr %15, align 4, !tbaa !6
  %22 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 4
  %23 = load i64, ptr %22, align 8, !tbaa !292
  %24 = add nsw i64 %23, 1
  store i64 %24, ptr %22, align 8, !tbaa !292
  %25 = getelementptr inbounds i8, ptr %1, i32 28
  store atomic i32 1, ptr %25 release, align 4
  br label %38

26:                                               ; preds = %3
  %27 = tail call i64 @fa_clock() #62
  %28 = mul nsw i64 %2, 1000000
  %29 = add nsw i64 %27, %28
  tail call fastcc void @timer_insert(ptr noundef %4, ptr noundef nonnull %0, i64 noundef %29) #61
  %30 = getelementptr inbounds i8, ptr %1, i32 28
  store atomic i32 2, ptr %30 release, align 4
  %31 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 10
  %32 = load ptr, ptr %31, align 8, !tbaa !299
  %33 = icmp eq ptr %32, %1
  %34 = icmp eq ptr %32, null
  %35 = or i1 %33, %34
  br i1 %35, label %36, label %38

36:                                               ; preds = %26
  %37 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 25
  store ptr null, ptr %37, align 8, !tbaa !300
  br label %38

38:                                               ; preds = %36, %26, %9
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @timer_insert(ptr noundef %0, ptr noundef %1, i64 noundef %2) unnamed_addr #3 {
  %4 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 1
  store ptr %0, ptr %4, align 4, !tbaa !311
  %5 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 7
  %6 = load i64, ptr %5, align 8, !tbaa !314
  %7 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 8
  %8 = load i64, ptr %7, align 8, !tbaa !315
  %9 = icmp eq i64 %6, %8
  br i1 %9, label %13, label %10

10:                                               ; preds = %3
  %11 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 6
  %12 = load ptr, ptr %11, align 8, !tbaa !316
  br label %46

13:                                               ; preds = %3
  %14 = icmp slt i64 %6, 16
  %15 = shl nuw nsw i64 %6, 1
  %16 = select i1 %14, i64 16, i64 %15
  %17 = trunc i64 %16 to i32
  %18 = shl i32 %17, 4
  %19 = tail call ptr @fa_alloc(i32 noundef %18, i32 noundef 64) #62
  %20 = load i64, ptr %5, align 8, !tbaa !314
  %21 = icmp sgt i64 %20, 0
  br i1 %21, label %22, label %24

22:                                               ; preds = %13
  %23 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 6
  br label %28

24:                                               ; preds = %28, %13
  %25 = phi i64 [ %20, %13 ], [ %35, %28 ]
  %26 = load i64, ptr %7, align 8, !tbaa !315
  %27 = icmp sgt i64 %26, 0
  br i1 %27, label %37, label %43

28:                                               ; preds = %22, %28
  %29 = phi i64 [ 0, %22 ], [ %34, %28 ]
  %30 = trunc i64 %29 to i32
  %31 = getelementptr inbounds %struct.FaTimer, ptr %19, i32 %30
  %32 = load ptr, ptr %23, align 8, !tbaa !316
  %33 = getelementptr inbounds %struct.FaTimer, ptr %32, i32 %30
  tail call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 8 dereferenceable(16) %31, ptr noundef nonnull align 8 dereferenceable(16) %33, i32 16, i1 false), !tbaa.struct !190
  %34 = add nuw nsw i64 %29, 1
  %35 = load i64, ptr %5, align 8, !tbaa !314
  %36 = icmp slt i64 %34, %35
  br i1 %36, label %28, label %24, !llvm.loop !317

37:                                               ; preds = %24
  %38 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 6
  %39 = load ptr, ptr %38, align 8, !tbaa !316
  %40 = trunc i64 %26 to i32
  %41 = shl i32 %40, 4
  tail call void @fa_free(ptr noundef %39, i32 noundef %41, i32 noundef 64) #62
  %42 = load i64, ptr %5, align 8, !tbaa !314
  br label %43

43:                                               ; preds = %37, %24
  %44 = phi i64 [ %42, %37 ], [ %25, %24 ]
  %45 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 6
  store ptr %19, ptr %45, align 8, !tbaa !316
  store i64 %16, ptr %7, align 8, !tbaa !315
  br label %46

46:                                               ; preds = %10, %43
  %47 = phi ptr [ %19, %43 ], [ %12, %10 ]
  %48 = phi i64 [ %44, %43 ], [ %6, %10 ]
  %49 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 6
  store i64 %2, ptr %49, align 8, !tbaa !318
  %50 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 9
  %51 = load i64, ptr %50, align 8, !tbaa !319
  %52 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 7
  store i64 %51, ptr %52, align 8, !tbaa !320
  %53 = add nsw i64 %51, 1
  store i64 %53, ptr %50, align 8, !tbaa !319
  %54 = add nsw i64 %48, 1
  store i64 %54, ptr %5, align 8, !tbaa !314
  %55 = trunc i64 %48 to i32
  %56 = getelementptr inbounds %struct.FaTimer, ptr %47, i32 %55
  store i64 %2, ptr %56, align 8, !tbaa.struct !190
  %57 = getelementptr inbounds i8, ptr %56, i32 8
  store ptr %1, ptr %57, align 8, !tbaa.struct !191
  %58 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 8
  store i64 %48, ptr %58, align 8, !tbaa !312
  %59 = load i64, ptr %5, align 8, !tbaa !314
  %60 = add nsw i64 %59, -1
  tail call fastcc void @timer_up(ptr noundef nonnull %0, i64 noundef %60) #61
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden void @fa_timer_cancel(ptr nocapture noundef readonly %0) local_unnamed_addr #20 {
  %2 = getelementptr inbounds %struct.FaWait, ptr %0, i32 0, i32 8
  %3 = load i64, ptr %2, align 8, !tbaa !312
  %4 = icmp sgt i64 %3, -1
  br i1 %4, label %5, label %8

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaWait, ptr %0, i32 0, i32 1
  %7 = load ptr, ptr %6, align 4, !tbaa !311
  tail call fastcc void @timer_remove(ptr noundef %7, i64 noundef %3) #61
  br label %8

8:                                                ; preds = %5, %1
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc void @timer_remove(ptr nocapture noundef %0, i64 noundef %1) unnamed_addr #20 {
  %3 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 6
  %4 = load ptr, ptr %3, align 8, !tbaa !316
  %5 = trunc i64 %1 to i32
  %6 = getelementptr inbounds %struct.FaTimer, ptr %4, i32 %5, i32 1
  %7 = load ptr, ptr %6, align 8, !tbaa !321
  %8 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 7
  %9 = load i64, ptr %8, align 8, !tbaa !314
  %10 = add nsw i64 %9, -1
  store i64 %10, ptr %8, align 8, !tbaa !314
  %11 = icmp sgt i64 %10, %1
  br i1 %11, label %12, label %134

12:                                               ; preds = %2
  %13 = trunc i64 %10 to i32
  %14 = getelementptr inbounds %struct.FaTimer, ptr %4, i32 %13
  %15 = load i64, ptr %14, align 8, !tbaa.struct !190
  %16 = getelementptr inbounds i8, ptr %14, i32 8
  %17 = load ptr, ptr %16, align 8, !tbaa.struct !191
  %18 = getelementptr inbounds i8, ptr %14, i32 12
  %19 = load i32, ptr %18, align 4, !tbaa.struct !323
  %20 = getelementptr inbounds %struct.FaTimer, ptr %4, i32 %5
  store i64 %15, ptr %20, align 8, !tbaa.struct !190
  %21 = getelementptr inbounds i8, ptr %20, i32 8
  store ptr %17, ptr %21, align 8, !tbaa.struct !191
  %22 = getelementptr inbounds i8, ptr %20, i32 12
  store i32 %19, ptr %22, align 4, !tbaa.struct !323
  %23 = getelementptr inbounds %struct.FaWait, ptr %17, i32 0, i32 8
  store i64 %1, ptr %23, align 8, !tbaa !312
  %24 = icmp sgt i64 %1, 0
  %25 = load ptr, ptr %3, align 8, !tbaa !316
  br i1 %24, label %26, label %44

26:                                               ; preds = %12
  %27 = add nuw i64 %1, 17179869183
  %28 = lshr i64 %27, 2
  %29 = trunc i64 %28 to i32
  %30 = getelementptr inbounds %struct.FaTimer, ptr %25, i32 %29
  %31 = load i64, ptr %30, align 1
  %32 = getelementptr inbounds i8, ptr %30, i32 8
  %33 = load ptr, ptr %32, align 1
  %34 = icmp slt i64 %15, %31
  br i1 %34, label %43, label %35

35:                                               ; preds = %26
  %36 = icmp eq i64 %15, %31
  br i1 %36, label %37, label %44

37:                                               ; preds = %35
  %38 = getelementptr inbounds %struct.FaWait, ptr %17, i32 0, i32 7
  %39 = load i64, ptr %38, align 8, !tbaa !320
  %40 = getelementptr inbounds %struct.FaWait, ptr %33, i32 0, i32 7
  %41 = load i64, ptr %40, align 8, !tbaa !320
  %42 = icmp slt i64 %39, %41
  br i1 %42, label %43, label %44

43:                                               ; preds = %26, %37
  tail call fastcc void @timer_up(ptr noundef nonnull %0, i64 noundef %1) #61
  br label %134

44:                                               ; preds = %35, %37, %12
  %45 = getelementptr inbounds %struct.FaTimer, ptr %25, i32 %5
  %46 = load i64, ptr %45, align 8, !tbaa.struct !190
  %47 = getelementptr inbounds i8, ptr %45, i32 8
  %48 = load ptr, ptr %47, align 8, !tbaa.struct !191
  %49 = getelementptr inbounds i8, ptr %45, i32 12
  %50 = load i32, ptr %49, align 4, !tbaa.struct !323
  %51 = shl nsw i64 %1, 2
  %52 = or disjoint i64 %51, 1
  %53 = load i64, ptr %8, align 8, !tbaa !314
  %54 = icmp slt i64 %52, %53
  br i1 %54, label %55, label %126

55:                                               ; preds = %44
  %56 = getelementptr inbounds %struct.FaWait, ptr %48, i32 0, i32 7
  br label %57

57:                                               ; preds = %111, %55
  %58 = phi i64 [ %53, %55 ], [ %120, %111 ]
  %59 = phi i64 [ %52, %55 ], [ %119, %111 ]
  %60 = phi i64 [ %51, %55 ], [ %118, %111 ]
  %61 = phi i64 [ %1, %55 ], [ %68, %111 ]
  %62 = add i64 %60, 5
  %63 = tail call i64 @llvm.smin.i64(i64 %62, i64 %58)
  %64 = or disjoint i64 %60, 2
  %65 = icmp slt i64 %64, %63
  %66 = load ptr, ptr %3, align 8, !tbaa !316
  br i1 %65, label %83, label %67

67:                                               ; preds = %107, %57
  %68 = phi i64 [ %59, %57 ], [ %108, %107 ]
  %69 = trunc i64 %68 to i32
  %70 = getelementptr inbounds %struct.FaTimer, ptr %66, i32 %69
  %71 = load i64, ptr %70, align 1
  %72 = getelementptr inbounds i8, ptr %70, i32 8
  %73 = load ptr, ptr %72, align 1
  %74 = getelementptr inbounds i8, ptr %70, i32 12
  %75 = icmp slt i64 %71, %46
  br i1 %75, label %111, label %76

76:                                               ; preds = %67
  %77 = icmp eq i64 %71, %46
  br i1 %77, label %78, label %124

78:                                               ; preds = %76
  %79 = getelementptr inbounds %struct.FaWait, ptr %73, i32 0, i32 7
  %80 = load i64, ptr %79, align 8, !tbaa !320
  %81 = load i64, ptr %56, align 8, !tbaa !320
  %82 = icmp slt i64 %80, %81
  br i1 %82, label %111, label %124

83:                                               ; preds = %57, %107
  %84 = phi i64 [ %109, %107 ], [ %64, %57 ]
  %85 = phi i64 [ %108, %107 ], [ %59, %57 ]
  %86 = trunc i64 %84 to i32
  %87 = getelementptr inbounds %struct.FaTimer, ptr %66, i32 %86
  %88 = trunc i64 %85 to i32
  %89 = getelementptr inbounds %struct.FaTimer, ptr %66, i32 %88
  %90 = load i64, ptr %89, align 1
  %91 = getelementptr inbounds i8, ptr %89, i32 8
  %92 = load ptr, ptr %91, align 1
  %93 = load i64, ptr %87, align 1
  %94 = getelementptr inbounds i8, ptr %87, i32 8
  %95 = load ptr, ptr %94, align 1
  %96 = icmp slt i64 %93, %90
  br i1 %96, label %107, label %97

97:                                               ; preds = %83
  %98 = icmp eq i64 %93, %90
  br i1 %98, label %99, label %106

99:                                               ; preds = %97
  %100 = getelementptr inbounds %struct.FaWait, ptr %95, i32 0, i32 7
  %101 = load i64, ptr %100, align 8, !tbaa !320
  %102 = getelementptr inbounds %struct.FaWait, ptr %92, i32 0, i32 7
  %103 = load i64, ptr %102, align 8, !tbaa !320
  %104 = icmp sge i64 %101, %103
  %105 = freeze i1 %104
  br i1 %105, label %106, label %107

106:                                              ; preds = %99, %97
  br label %107

107:                                              ; preds = %106, %99, %83
  %108 = phi i64 [ %85, %106 ], [ %84, %99 ], [ %84, %83 ]
  %109 = add nsw i64 %84, 1
  %110 = icmp eq i64 %109, %63
  br i1 %110, label %67, label %83, !llvm.loop !324

111:                                              ; preds = %78, %67
  %112 = load i32, ptr %74, align 1
  %113 = trunc i64 %61 to i32
  %114 = getelementptr inbounds %struct.FaTimer, ptr %66, i32 %113
  store i64 %71, ptr %114, align 8, !tbaa.struct !190
  %115 = getelementptr inbounds i8, ptr %114, i32 8
  store ptr %73, ptr %115, align 8, !tbaa.struct !191
  %116 = getelementptr inbounds i8, ptr %114, i32 12
  store i32 %112, ptr %116, align 4, !tbaa.struct !323
  %117 = getelementptr inbounds %struct.FaWait, ptr %73, i32 0, i32 8
  store i64 %61, ptr %117, align 8, !tbaa !312
  %118 = shl nsw i64 %68, 2
  %119 = or disjoint i64 %118, 1
  %120 = load i64, ptr %8, align 8, !tbaa !314
  %121 = icmp slt i64 %119, %120
  br i1 %121, label %57, label %122

122:                                              ; preds = %111
  %123 = load ptr, ptr %3, align 8, !tbaa !316
  br label %126

124:                                              ; preds = %78, %76
  %125 = trunc i64 %61 to i32
  br label %126

126:                                              ; preds = %44, %122, %124
  %127 = phi i32 [ %5, %44 ], [ %125, %124 ], [ %69, %122 ]
  %128 = phi ptr [ %25, %44 ], [ %66, %124 ], [ %123, %122 ]
  %129 = phi i64 [ %1, %44 ], [ %61, %124 ], [ %68, %122 ]
  %130 = getelementptr inbounds %struct.FaTimer, ptr %128, i32 %127
  store i64 %46, ptr %130, align 8, !tbaa.struct !190
  %131 = getelementptr inbounds i8, ptr %130, i32 8
  store ptr %48, ptr %131, align 8, !tbaa.struct !191
  %132 = getelementptr inbounds i8, ptr %130, i32 12
  store i32 %50, ptr %132, align 4, !tbaa.struct !323
  %133 = getelementptr inbounds %struct.FaWait, ptr %48, i32 0, i32 8
  store i64 %129, ptr %133, align 8, !tbaa !312
  br label %134

134:                                              ; preds = %43, %126, %2
  %135 = getelementptr inbounds %struct.FaWait, ptr %7, i32 0, i32 8
  store i64 -1, ptr %135, align 8, !tbaa !312
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_sleep(i64 noundef %0) local_unnamed_addr #3 {
  %2 = icmp sgt i64 %0, 0
  br i1 %2, label %3, label %5

3:                                                ; preds = %1
  %4 = mul nsw i64 %0, 1000000
  tail call void @fa_nanosleep(i64 noundef %4) #62
  br label %5

5:                                                ; preds = %3, %1
  ret void
}

; Function Attrs: optsize
declare void @fa_nanosleep(i64 noundef) local_unnamed_addr #4

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: write)
define hidden void @fa_join_init(ptr nocapture noundef writeonly %0) local_unnamed_addr #24 {
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(24) %0, i8 0, i32 24, i1 false)
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, argmem: readwrite, inaccessiblemem: none)
define hidden void @fa_join_add(ptr noundef %0, ptr noundef %1) local_unnamed_addr #47 {
  %3 = icmp ult ptr %1, inttoptr (i32 2 to ptr)
  br i1 %3, label %25, label %4

4:                                                ; preds = %2
  %5 = getelementptr inbounds i8, ptr %1, i32 16
  %6 = load ptr, ptr %1, align 4, !tbaa !6
  %7 = icmp eq ptr %6, null
  br i1 %7, label %8, label %18

8:                                                ; preds = %4
  %9 = getelementptr inbounds i8, ptr %1, i32 20
  store ptr null, ptr %9, align 4, !tbaa !290
  %10 = getelementptr inbounds %struct.FaJoin, ptr %0, i32 0, i32 3
  %11 = load ptr, ptr %10, align 4, !tbaa !301
  %12 = icmp eq ptr %11, null
  br i1 %12, label %15, label %13

13:                                               ; preds = %8
  %14 = getelementptr inbounds %struct.FaTask, ptr %11, i32 0, i32 1
  store ptr %5, ptr %14, align 4, !tbaa !290
  br label %17

15:                                               ; preds = %8
  %16 = getelementptr inbounds %struct.FaJoin, ptr %0, i32 0, i32 2
  store ptr %5, ptr %16, align 8, !tbaa !303
  br label %17

17:                                               ; preds = %13, %15
  store ptr %5, ptr %10, align 4, !tbaa !301
  br label %25

18:                                               ; preds = %4
  %19 = ptrtoint ptr %0 to i32
  %20 = or i32 %19, 1
  %21 = inttoptr i32 %20 to ptr
  store ptr %21, ptr %5, align 4, !tbaa !298
  %22 = getelementptr inbounds %struct.FaJoin, ptr %0, i32 0, i32 1
  %23 = load i64, ptr %22, align 8, !tbaa !304
  %24 = add nsw i64 %23, 1
  store i64 %24, ptr %22, align 8, !tbaa !304
  br label %25

25:                                               ; preds = %17, %18, %2
  ret void
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define hidden ptr @fa_join_next(ptr nocapture noundef %0, ptr noundef %1) local_unnamed_addr #46 {
  %3 = getelementptr inbounds %struct.FaJoin, ptr %0, i32 0, i32 2
  %4 = load ptr, ptr %3, align 8, !tbaa !303
  %5 = icmp eq ptr %4, null
  br i1 %5, label %14, label %6

6:                                                ; preds = %2
  %7 = getelementptr inbounds %struct.FaTask, ptr %4, i32 0, i32 1
  %8 = load ptr, ptr %7, align 4, !tbaa !290
  store ptr %8, ptr %3, align 8, !tbaa !303
  %9 = icmp eq ptr %8, null
  br i1 %9, label %10, label %12

10:                                               ; preds = %6
  %11 = getelementptr inbounds %struct.FaJoin, ptr %0, i32 0, i32 3
  store ptr null, ptr %11, align 4, !tbaa !301
  br label %12

12:                                               ; preds = %10, %6
  store ptr null, ptr %7, align 4, !tbaa !290
  %13 = getelementptr inbounds i8, ptr %4, i32 -16
  br label %22

14:                                               ; preds = %2
  %15 = getelementptr inbounds %struct.FaJoin, ptr %0, i32 0, i32 1
  %16 = load i64, ptr %15, align 8, !tbaa !304
  %17 = icmp eq i64 %16, 0
  br i1 %17, label %22, label %18

18:                                               ; preds = %14
  %19 = icmp eq ptr %1, null
  br i1 %19, label %22, label %20

20:                                               ; preds = %18
  store ptr %1, ptr %0, align 8, !tbaa !305
  %21 = getelementptr inbounds i8, ptr %1, i32 28
  store atomic i32 2, ptr %21 release, align 4
  br label %22

22:                                               ; preds = %18, %14, %20, %12
  %23 = phi ptr [ %13, %12 ], [ null, %20 ], [ inttoptr (i32 1 to ptr), %14 ], [ null, %18 ]
  ret ptr %23
}

; Function Attrs: cold noinline noreturn nounwind optsize
define hidden void @fa_task_stuck() local_unnamed_addr #38 {
  tail call fastcc void @deadlock(ptr noundef nonnull @.str.30) #67
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define internal fastcc void @deadlock(ptr noundef %0) unnamed_addr #38 {
  tail call void @fa_flush() #61
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.45, i32 noundef 16) #62
  br label %2

2:                                                ; preds = %2, %1
  %3 = phi i32 [ 0, %1 ], [ %7, %2 ]
  %4 = getelementptr inbounds i8, ptr %0, i32 %3
  %5 = load i8, ptr %4, align 1, !tbaa !1
  %6 = icmp eq i8 %5, 0
  %7 = add i32 %3, 1
  br i1 %6, label %8, label %2, !llvm.loop !28

8:                                                ; preds = %2
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull %0, i32 noundef %3) #62
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.22, i32 noundef 1) #62
  tail call void @fa_exit(i32 noundef 101) #63
  unreachable
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(none)
define hidden noalias noundef ptr @fa_io_register(i32 %0) local_unnamed_addr #23 {
  ret ptr null
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define hidden noundef i32 @fa_io_wait(ptr nocapture noundef %0, i32 noundef %1, ptr noundef %2, ptr noundef %3) local_unnamed_addr #46 {
  %5 = icmp eq i32 %1, 0
  %6 = select i1 %5, i32 -2, i32 -5
  %7 = getelementptr inbounds %struct.FaIo, ptr %0, i32 0, i32 1
  %8 = load i32, ptr %7, align 4, !tbaa !325
  %9 = and i32 %8, %6
  store i32 %9, ptr %7, align 4, !tbaa !325
  store ptr %3, ptr %2, align 8, !tbaa !309
  %10 = getelementptr inbounds %struct.FaIo, ptr %0, i32 0, i32 4
  %11 = load ptr, ptr %10, align 4, !tbaa !327
  %12 = getelementptr inbounds %struct.FaWait, ptr %2, i32 0, i32 1
  store ptr %11, ptr %12, align 4, !tbaa !311
  %13 = getelementptr inbounds %struct.FaWait, ptr %2, i32 0, i32 9
  store i32 0, ptr %13, align 8, !tbaa !328
  %14 = getelementptr inbounds %struct.FaWait, ptr %2, i32 0, i32 10
  store i32 0, ptr %14, align 4, !tbaa !329
  %15 = getelementptr inbounds %struct.FaWait, ptr %2, i32 0, i32 8
  store i64 -1, ptr %15, align 8, !tbaa !312
  %16 = getelementptr inbounds %struct.FaIo, ptr %0, i32 0, i32 2
  %17 = getelementptr inbounds %struct.FaIo, ptr %0, i32 0, i32 3
  %18 = select i1 %5, ptr %16, ptr %17
  store ptr %2, ptr %18, align 4, !tbaa !6
  %19 = getelementptr inbounds %struct.FaExecutor, ptr %11, i32 0, i32 13
  %20 = load i64, ptr %19, align 8, !tbaa !330
  %21 = add nsw i64 %20, 1
  store i64 %21, ptr %19, align 8, !tbaa !330
  %22 = getelementptr inbounds i8, ptr %3, i32 28
  store atomic i32 2, ptr %22 release, align 4
  ret i32 1
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define hidden void @fa_io_cancel(ptr nocapture noundef %0, ptr noundef readnone %1) local_unnamed_addr #48 {
  %3 = getelementptr inbounds %struct.FaIo, ptr %0, i32 0, i32 2
  %4 = load ptr, ptr %3, align 4, !tbaa !331
  %5 = icmp eq ptr %4, %1
  br i1 %5, label %6, label %12

6:                                                ; preds = %2
  store ptr null, ptr %3, align 4, !tbaa !331
  %7 = getelementptr inbounds %struct.FaIo, ptr %0, i32 0, i32 4
  %8 = load ptr, ptr %7, align 4, !tbaa !327
  %9 = getelementptr inbounds %struct.FaExecutor, ptr %8, i32 0, i32 13
  %10 = load i64, ptr %9, align 8, !tbaa !330
  %11 = add nsw i64 %10, -1
  store i64 %11, ptr %9, align 8, !tbaa !330
  br label %12

12:                                               ; preds = %6, %2
  %13 = getelementptr inbounds %struct.FaIo, ptr %0, i32 0, i32 3
  %14 = load ptr, ptr %13, align 4, !tbaa !332
  %15 = icmp eq ptr %14, %1
  br i1 %15, label %16, label %22

16:                                               ; preds = %12
  store ptr null, ptr %13, align 4, !tbaa !332
  %17 = getelementptr inbounds %struct.FaIo, ptr %0, i32 0, i32 4
  %18 = load ptr, ptr %17, align 4, !tbaa !327
  %19 = getelementptr inbounds %struct.FaExecutor, ptr %18, i32 0, i32 13
  %20 = load i64, ptr %19, align 8, !tbaa !330
  %21 = add nsw i64 %20, -1
  store i64 %21, ptr %19, align 8, !tbaa !330
  br label %22

22:                                               ; preds = %16, %12
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_io_close(ptr noundef %0) local_unnamed_addr #3 {
  %2 = icmp eq ptr %0, null
  br i1 %2, label %4, label %3

3:                                                ; preds = %1
  tail call void @fa_free(ptr noundef nonnull %0, i32 noundef 20, i32 noundef 16) #62
  br label %4

4:                                                ; preds = %1, %3
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_run_main(ptr noundef %0, i32 noundef %1) local_unnamed_addr #3 {
  %3 = alloca i64, align 8
  %4 = tail call fastcc ptr @executor() #61
  %5 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 10
  store ptr %0, ptr %5, align 8, !tbaa !299
  %6 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 21
  store i32 %1, ptr %6, align 8, !tbaa !333
  %7 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 11
  %8 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 12
  %9 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 4
  %10 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 5
  %11 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 2
  %12 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 7
  %13 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 22
  %14 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 13
  %15 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 6
  %16 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 19
  %17 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 23
  br label %18

18:                                               ; preds = %105, %2
  %19 = load ptr, ptr %0, align 4, !tbaa !6
  %20 = icmp eq ptr %19, null
  br i1 %20, label %21, label %27

21:                                               ; preds = %18
  %22 = load i64, ptr %7, align 8, !tbaa !288
  %23 = icmp sgt i64 %22, 0
  br i1 %23, label %27, label %24

24:                                               ; preds = %21
  %25 = load atomic i64, ptr %8 acquire, align 8
  %26 = icmp sgt i64 %25, 0
  br i1 %26, label %27, label %106

27:                                               ; preds = %21, %18, %24
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %3) #64
  store i64 0, ptr %3, align 8, !tbaa !89
  %28 = load i64, ptr %9, align 8, !tbaa !292
  %29 = tail call i64 @llvm.smin.i64(i64 %28, i64 61)
  %30 = load i64, ptr %10, align 8, !tbaa !293
  %31 = icmp sgt i64 %30, 0
  br i1 %31, label %32, label %35

32:                                               ; preds = %27
  %33 = load ptr, ptr %11, align 8, !tbaa !334
  %34 = icmp eq ptr %33, null
  br i1 %34, label %35, label %36

35:                                               ; preds = %32, %27
  br label %36

36:                                               ; preds = %35, %32
  %37 = phi i64 [ %29, %35 ], [ 30, %32 ]
  %38 = icmp sgt i64 %28, 0
  br i1 %38, label %39, label %57

39:                                               ; preds = %36, %51
  %40 = phi i64 [ %55, %51 ], [ 0, %36 ]
  %41 = and i64 %40, 7
  %42 = icmp eq i64 %41, 7
  br i1 %42, label %43, label %48

43:                                               ; preds = %39
  %44 = load i64, ptr %12, align 8, !tbaa !314
  %45 = icmp sgt i64 %44, 0
  br i1 %45, label %46, label %48

46:                                               ; preds = %43
  %47 = tail call i64 @fa_clock() #62
  tail call fastcc void @fire_timers(ptr noundef nonnull %4, i64 noundef %47) #61
  br label %48

48:                                               ; preds = %46, %43, %39
  %49 = call fastcc ptr @ready_pop(ptr noundef nonnull %4, ptr noundef nonnull %3, i64 noundef %37) #61
  %50 = icmp eq ptr %49, null
  br i1 %50, label %57, label %51

51:                                               ; preds = %48
  %52 = getelementptr inbounds i8, ptr %49, i32 -16
  tail call fastcc void @turn(ptr noundef nonnull %4, ptr noundef nonnull %52) #61
  %53 = load i64, ptr %13, align 8, !tbaa !335
  %54 = add nsw i64 %53, 1
  store i64 %54, ptr %13, align 8, !tbaa !335
  %55 = add nuw nsw i64 %40, 1
  %56 = icmp eq i64 %55, %29
  br i1 %56, label %57, label %39, !llvm.loop !336

57:                                               ; preds = %51, %48, %36
  %58 = load i64, ptr %12, align 8, !tbaa !314
  %59 = icmp sgt i64 %58, 0
  br i1 %59, label %60, label %65

60:                                               ; preds = %57
  %61 = tail call i64 @fa_clock() #62
  %62 = load i64, ptr %12, align 8, !tbaa !314
  %63 = icmp sgt i64 %62, 0
  br i1 %63, label %64, label %65

64:                                               ; preds = %60
  tail call fastcc void @fire_timers(ptr noundef nonnull %4, i64 noundef %61) #61
  br label %65

65:                                               ; preds = %64, %60, %57
  %66 = phi i64 [ %61, %64 ], [ %61, %60 ], [ 0, %57 ]
  tail call fastcc void @drain_remote(ptr noundef nonnull %4) #61
  %67 = load i64, ptr %9, align 8, !tbaa !292
  %68 = icmp sgt i64 %67, 0
  br i1 %68, label %69, label %79

69:                                               ; preds = %65
  %70 = load i32, ptr %16, align 8, !tbaa !296
  %71 = icmp sgt i32 %70, -1
  br i1 %71, label %72, label %105

72:                                               ; preds = %69
  %73 = load i64, ptr %13, align 8, !tbaa !335
  %74 = icmp sgt i64 %73, 60
  br i1 %74, label %78, label %75

75:                                               ; preds = %72
  %76 = load i32, ptr %17, align 8, !tbaa !307
  %77 = icmp eq i32 %76, 0
  br i1 %77, label %105, label %78

78:                                               ; preds = %75, %72
  store i64 0, ptr %13, align 8, !tbaa !335
  store i32 0, ptr %17, align 8, !tbaa !307
  br label %105

79:                                               ; preds = %65
  store i64 0, ptr %13, align 8, !tbaa !335
  %80 = load ptr, ptr %5, align 8, !tbaa !299
  %81 = load ptr, ptr %80, align 4, !tbaa !6
  %82 = icmp eq ptr %81, null
  br i1 %82, label %83, label %89

83:                                               ; preds = %79
  %84 = load i64, ptr %7, align 8, !tbaa !288
  %85 = icmp eq i64 %84, 0
  br i1 %85, label %86, label %89

86:                                               ; preds = %83
  %87 = load atomic i64, ptr %8 acquire, align 8
  %88 = icmp eq i64 %87, 0
  br i1 %88, label %105, label %89

89:                                               ; preds = %86, %83, %79
  %90 = load i64, ptr %12, align 8, !tbaa !314
  %91 = icmp sgt i64 %90, 0
  br i1 %91, label %92, label %98

92:                                               ; preds = %89
  %93 = load ptr, ptr %15, align 8, !tbaa !316
  %94 = load i64, ptr %93, align 8, !tbaa !337
  %95 = sub nsw i64 %94, %66
  %96 = icmp slt i64 %95, 1
  br i1 %96, label %105, label %97

97:                                               ; preds = %92
  tail call void @fa_nanosleep(i64 noundef %95) #62
  br label %105

98:                                               ; preds = %89
  %99 = load atomic i64, ptr %8 acquire, align 8
  %100 = icmp sgt i64 %99, 0
  br i1 %100, label %105, label %101

101:                                              ; preds = %98
  %102 = load i64, ptr %14, align 8, !tbaa !330
  %103 = icmp sgt i64 %102, 0
  br i1 %103, label %105, label %104

104:                                              ; preds = %101
  tail call fastcc void @deadlock_stop(ptr noundef nonnull %4) #67
  unreachable

105:                                              ; preds = %69, %75, %78, %86, %92, %97, %98, %101
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %3) #64
  br label %18, !llvm.loop !338

106:                                              ; preds = %24
  tail call void @fa_destroy(ptr noundef nonnull %0) #62
  %107 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 8
  %108 = load i64, ptr %107, align 8, !tbaa !315
  %109 = icmp sgt i64 %108, 0
  br i1 %109, label %110, label %114

110:                                              ; preds = %106
  %111 = load ptr, ptr %15, align 8, !tbaa !316
  %112 = trunc i64 %108 to i32
  %113 = shl i32 %112, 4
  tail call void @fa_free(ptr noundef %111, i32 noundef %113, i32 noundef 64) #62
  br label %114

114:                                              ; preds = %110, %106
  %115 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 14
  br label %116

116:                                              ; preds = %116, %114
  %117 = load atomic i64, ptr %115 seq_cst, align 8
  %118 = icmp eq i64 %117, 0
  br i1 %118, label %119, label %116, !llvm.loop !339

119:                                              ; preds = %116
  store ptr null, ptr @current_executor, align 4, !tbaa !6
  tail call void @fa_free(ptr noundef nonnull %4, i32 noundef 320, i32 noundef 16) #62
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_wasm_start(ptr noundef %0, i32 noundef %1, ptr noundef %2) local_unnamed_addr #3 {
  %4 = tail call fastcc ptr @executor() #61
  %5 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 10
  store ptr %0, ptr %5, align 8, !tbaa !299
  %6 = getelementptr inbounds %struct.FaExecutor, ptr %4, i32 0, i32 21
  store i32 %1, ptr %6, align 8, !tbaa !333
  store ptr %2, ptr @wasm_finish, align 4, !tbaa !6
  ret void
}

; Function Attrs: nounwind optsize
define hidden noundef i32 @fa_wasm_run() local_unnamed_addr #3 {
  %1 = alloca i64, align 8
  %2 = load ptr, ptr @current_executor, align 4, !tbaa !6
  %3 = icmp eq ptr %2, null
  br i1 %3, label %92, label %4

4:                                                ; preds = %0
  %5 = getelementptr inbounds %struct.FaExecutor, ptr %2, i32 0, i32 7
  %6 = load i64, ptr %5, align 8, !tbaa !314
  %7 = icmp sgt i64 %6, 0
  br i1 %7, label %8, label %10

8:                                                ; preds = %4
  %9 = tail call i64 @fa_clock() #62
  tail call fastcc void @fire_timers(ptr noundef nonnull %2, i64 noundef %9) #61
  br label %10

10:                                               ; preds = %8, %4
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %1) #64
  store i64 0, ptr %1, align 8, !tbaa !89
  %11 = getelementptr inbounds %struct.FaExecutor, ptr %2, i32 0, i32 4
  %12 = load i64, ptr %11, align 8, !tbaa !292
  %13 = tail call i64 @llvm.smin.i64(i64 %12, i64 61)
  %14 = getelementptr inbounds %struct.FaExecutor, ptr %2, i32 0, i32 5
  %15 = load i64, ptr %14, align 8, !tbaa !293
  %16 = icmp sgt i64 %15, 0
  br i1 %16, label %17, label %21

17:                                               ; preds = %10
  %18 = getelementptr inbounds %struct.FaExecutor, ptr %2, i32 0, i32 2
  %19 = load ptr, ptr %18, align 8, !tbaa !334
  %20 = icmp eq ptr %19, null
  br i1 %20, label %21, label %22

21:                                               ; preds = %17, %10
  br label %22

22:                                               ; preds = %17, %21
  %23 = phi i64 [ %13, %21 ], [ 30, %17 ]
  %24 = icmp sgt i64 %12, 0
  br i1 %24, label %25, label %41

25:                                               ; preds = %22, %37
  %26 = phi i64 [ %39, %37 ], [ 0, %22 ]
  %27 = and i64 %26, 7
  %28 = icmp eq i64 %27, 7
  br i1 %28, label %29, label %34

29:                                               ; preds = %25
  %30 = load i64, ptr %5, align 8, !tbaa !314
  %31 = icmp sgt i64 %30, 0
  br i1 %31, label %32, label %34

32:                                               ; preds = %29
  %33 = tail call i64 @fa_clock() #62
  tail call fastcc void @fire_timers(ptr noundef nonnull %2, i64 noundef %33) #61
  br label %34

34:                                               ; preds = %32, %29, %25
  %35 = call fastcc ptr @ready_pop(ptr noundef nonnull %2, ptr noundef nonnull %1, i64 noundef %23) #61
  %36 = icmp eq ptr %35, null
  br i1 %36, label %41, label %37

37:                                               ; preds = %34
  %38 = getelementptr inbounds i8, ptr %35, i32 -16
  tail call fastcc void @turn(ptr noundef nonnull %2, ptr noundef nonnull %38) #61
  %39 = add nuw nsw i64 %26, 1
  %40 = icmp eq i64 %39, %13
  br i1 %40, label %41, label %25, !llvm.loop !340

41:                                               ; preds = %37, %34, %22
  %42 = load i64, ptr %5, align 8, !tbaa !314
  %43 = icmp sgt i64 %42, 0
  br i1 %43, label %44, label %49

44:                                               ; preds = %41
  %45 = tail call i64 @fa_clock() #62
  %46 = load i64, ptr %5, align 8, !tbaa !314
  %47 = icmp sgt i64 %46, 0
  br i1 %47, label %48, label %49

48:                                               ; preds = %44
  tail call fastcc void @fire_timers(ptr noundef nonnull %2, i64 noundef %45) #61
  br label %49

49:                                               ; preds = %41, %48, %44
  %50 = phi i64 [ %45, %48 ], [ %45, %44 ], [ 0, %41 ]
  %51 = load i64, ptr %11, align 8, !tbaa !292
  %52 = icmp sgt i64 %51, 0
  br i1 %52, label %90, label %53

53:                                               ; preds = %49
  %54 = getelementptr inbounds %struct.FaExecutor, ptr %2, i32 0, i32 10
  %55 = load ptr, ptr %54, align 8, !tbaa !299
  %56 = load ptr, ptr %55, align 4, !tbaa !6
  %57 = icmp eq ptr %56, null
  br i1 %57, label %58, label %73

58:                                               ; preds = %53
  %59 = getelementptr inbounds %struct.FaExecutor, ptr %2, i32 0, i32 11
  %60 = load i64, ptr %59, align 8, !tbaa !288
  %61 = icmp eq i64 %60, 0
  br i1 %61, label %62, label %73

62:                                               ; preds = %58
  tail call void @fa_destroy(ptr noundef nonnull %55) #62
  %63 = getelementptr inbounds %struct.FaExecutor, ptr %2, i32 0, i32 8
  %64 = load i64, ptr %63, align 8, !tbaa !315
  %65 = icmp sgt i64 %64, 0
  br i1 %65, label %66, label %71

66:                                               ; preds = %62
  %67 = getelementptr inbounds %struct.FaExecutor, ptr %2, i32 0, i32 6
  %68 = load ptr, ptr %67, align 8, !tbaa !316
  %69 = trunc i64 %64 to i32
  %70 = shl i32 %69, 2
  tail call void @fa_free(ptr noundef %68, i32 noundef %70, i32 noundef 4) #62
  br label %71

71:                                               ; preds = %66, %62
  store ptr null, ptr @current_executor, align 4, !tbaa !6
  tail call void @fa_free(ptr noundef nonnull %2, i32 noundef 320, i32 noundef 16) #62
  %72 = load ptr, ptr @wasm_finish, align 4, !tbaa !6
  tail call void %72() #62
  br label %90

73:                                               ; preds = %58, %53
  %74 = load i64, ptr %5, align 8, !tbaa !314
  %75 = icmp sgt i64 %74, 0
  br i1 %75, label %76, label %89

76:                                               ; preds = %73
  %77 = getelementptr inbounds %struct.FaExecutor, ptr %2, i32 0, i32 6
  %78 = load ptr, ptr %77, align 8, !tbaa !316
  %79 = load i64, ptr %78, align 8, !tbaa !337
  %80 = sub nsw i64 %79, %50
  %81 = load i32, ptr @wasm_timer_token, align 4, !tbaa !23
  %82 = add nsw i32 %81, 1
  store i32 %82, ptr @wasm_timer_token, align 4, !tbaa !23
  %83 = icmp sgt i64 %80, 0
  br i1 %83, label %84, label %87

84:                                               ; preds = %76
  %85 = sitofp i64 %80 to double
  %86 = fdiv double %85, 1.000000e+06
  br label %87

87:                                               ; preds = %76, %84
  %88 = phi double [ %86, %84 ], [ 0.000000e+00, %76 ]
  tail call void @fa_host_timer(i32 noundef %82, double noundef %88) #62
  br label %90

89:                                               ; preds = %73
  tail call fastcc void @deadlock_stop(ptr noundef nonnull %2) #67
  unreachable

90:                                               ; preds = %49, %87, %71
  %91 = phi i32 [ 2, %71 ], [ 0, %87 ], [ 1, %49 ]
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %1) #64
  br label %92

92:                                               ; preds = %0, %90
  %93 = phi i32 [ %91, %90 ], [ 2, %0 ]
  ret i32 %93
}

; Function Attrs: nounwind optsize
define internal fastcc void @fire_timers(ptr nocapture noundef %0, i64 noundef %1) unnamed_addr #3 {
  %3 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 6
  %4 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 7
  %5 = load i64, ptr %4, align 8, !tbaa !314
  %6 = icmp sgt i64 %5, 0
  br i1 %6, label %7, label %42

7:                                                ; preds = %2
  %8 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 1
  %9 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 4
  %10 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 5
  br label %11

11:                                               ; preds = %7, %39
  %12 = load ptr, ptr %3, align 8, !tbaa !316
  %13 = load i64, ptr %12, align 8, !tbaa !337
  %14 = icmp sgt i64 %13, %1
  br i1 %14, label %42, label %15

15:                                               ; preds = %11
  %16 = getelementptr inbounds %struct.FaTimer, ptr %12, i32 0, i32 1
  %17 = load ptr, ptr %16, align 8, !tbaa !321
  tail call fastcc void @timer_remove(ptr noundef nonnull %0, i64 noundef 0) #61
  %18 = getelementptr inbounds %struct.FaWait, ptr %17, i32 0, i32 11
  %19 = load ptr, ptr %18, align 8, !tbaa !313
  %20 = icmp eq ptr %19, null
  br i1 %20, label %22, label %21

21:                                               ; preds = %15
  tail call void %19(ptr noundef nonnull %17) #62
  br label %39

22:                                               ; preds = %15
  %23 = load ptr, ptr %17, align 8, !tbaa !309
  %24 = getelementptr inbounds i8, ptr %23, i32 16
  %25 = getelementptr inbounds i8, ptr %23, i32 32
  %26 = load i32, ptr %25, align 4, !tbaa !286
  %27 = and i32 %26, -5
  store i32 %27, ptr %25, align 4, !tbaa !286
  %28 = getelementptr inbounds i8, ptr %23, i32 20
  store ptr null, ptr %28, align 4, !tbaa !290
  %29 = load ptr, ptr %8, align 4, !tbaa !6
  %30 = getelementptr inbounds i8, ptr %23, i32 24
  store ptr %29, ptr %30, align 4, !tbaa !291
  %31 = icmp eq ptr %29, null
  %32 = getelementptr inbounds %struct.FaTask, ptr %29, i32 0, i32 1
  %33 = select i1 %31, ptr %0, ptr %32
  store ptr %24, ptr %33, align 4, !tbaa !6
  store ptr %24, ptr %8, align 4, !tbaa !6
  %34 = load i64, ptr %9, align 8, !tbaa !292
  %35 = add nsw i64 %34, 1
  store i64 %35, ptr %9, align 8, !tbaa !292
  %36 = load i64, ptr %10, align 8, !tbaa !293
  %37 = add nsw i64 %36, 1
  store i64 %37, ptr %10, align 8, !tbaa !293
  %38 = getelementptr inbounds i8, ptr %23, i32 28
  store atomic i32 1, ptr %38 release, align 4
  br label %39

39:                                               ; preds = %22, %21
  %40 = load i64, ptr %4, align 8, !tbaa !314
  %41 = icmp sgt i64 %40, 0
  br i1 %41, label %11, label %42, !llvm.loop !341

42:                                               ; preds = %11, %39, %2
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define internal fastcc noundef ptr @ready_pop(ptr nocapture noundef %0, ptr nocapture noundef %1, i64 noundef %2) unnamed_addr #48 {
  %4 = load ptr, ptr %0, align 8, !tbaa !342
  %5 = icmp eq ptr %4, null
  br i1 %5, label %15, label %6

6:                                                ; preds = %3
  %7 = load i64, ptr %1, align 8, !tbaa !89
  %8 = icmp slt i64 %7, %2
  br i1 %8, label %13, label %9

9:                                                ; preds = %6
  %10 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 2
  %11 = load ptr, ptr %10, align 8, !tbaa !334
  %12 = icmp eq ptr %11, null
  br i1 %12, label %13, label %19

13:                                               ; preds = %6, %9
  %14 = add nsw i64 %7, 1
  store i64 %14, ptr %1, align 8, !tbaa !89
  br label %19

15:                                               ; preds = %3
  %16 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 2
  %17 = load ptr, ptr %16, align 8, !tbaa !334
  %18 = icmp eq ptr %17, null
  br i1 %18, label %21, label %19

19:                                               ; preds = %9, %13, %15
  %20 = phi ptr [ %4, %13 ], [ %17, %15 ], [ %11, %9 ]
  tail call fastcc void @ready_unlink(ptr noundef nonnull %0, ptr noundef nonnull %20) #61
  br label %21

21:                                               ; preds = %19, %15
  %22 = phi ptr [ %20, %19 ], [ null, %15 ]
  ret ptr %22
}

; Function Attrs: nounwind optsize
define internal fastcc void @turn(ptr nocapture noundef %0, ptr noundef %1) unnamed_addr #3 {
  store i32 128, ptr @fa_coop_budget, align 4, !tbaa !23
  %3 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 24
  store i64 0, ptr %3, align 8, !tbaa !308
  %4 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 21
  %5 = load i32, ptr %4, align 8, !tbaa !333
  %6 = icmp eq i32 %5, 0
  br i1 %6, label %9, label %7

7:                                                ; preds = %2
  %8 = tail call i64 @fa_clock() #62
  br label %9

9:                                                ; preds = %2, %7
  %10 = phi i64 [ %8, %7 ], [ 0, %2 ]
  %11 = icmp eq ptr %1, null
  br i1 %11, label %21, label %12

12:                                               ; preds = %9, %18
  %13 = phi ptr [ %19, %18 ], [ %1, %9 ]
  %14 = getelementptr inbounds i8, ptr %13, i32 28
  store atomic i32 4, ptr %14 release, align 4
  tail call void @fa_resume(ptr noundef nonnull %13) #62
  %15 = load ptr, ptr @frame_to_free, align 4, !tbaa !6
  %16 = icmp eq ptr %15, null
  br i1 %16, label %18, label %17

17:                                               ; preds = %12
  store ptr null, ptr @frame_to_free, align 4, !tbaa !6
  tail call void @fa_destroy(ptr noundef nonnull %15) #62
  br label %18

18:                                               ; preds = %17, %12
  %19 = load ptr, ptr @next_frame, align 4, !tbaa !6
  store ptr null, ptr @next_frame, align 4, !tbaa !6
  %20 = icmp eq ptr %19, null
  br i1 %20, label %21, label %12, !llvm.loop !343

21:                                               ; preds = %18, %9
  %22 = load i32, ptr %4, align 8, !tbaa !333
  %23 = icmp eq i32 %22, 0
  br i1 %23, label %54, label %24

24:                                               ; preds = %21
  %25 = tail call i64 @fa_clock() #62
  %26 = sub nsw i64 %25, %10
  %27 = icmp sgt i64 %26, 50000000
  br i1 %27, label %28, label %54

28:                                               ; preds = %24
  %29 = load ptr, ptr @fa_turn_site, align 4, !tbaa !6
  %30 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 28
  %31 = load i32, ptr %30, align 8, !tbaa !344
  %32 = icmp sgt i32 %31, 0
  br i1 %32, label %36, label %43

33:                                               ; preds = %36
  %34 = add nuw nsw i32 %37, 1
  %35 = icmp eq i32 %34, %31
  br i1 %35, label %41, label %36, !llvm.loop !345

36:                                               ; preds = %28, %33
  %37 = phi i32 [ %34, %33 ], [ 0, %28 ]
  %38 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 27, i32 %37
  %39 = load ptr, ptr %38, align 4, !tbaa !6
  %40 = icmp eq ptr %39, %29
  br i1 %40, label %54, label %33

41:                                               ; preds = %33
  %42 = icmp slt i32 %31, 32
  br i1 %42, label %43, label %46

43:                                               ; preds = %41, %28
  %44 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 27, i32 %31
  store ptr %29, ptr %44, align 4, !tbaa !6
  %45 = add nsw i32 %31, 1
  store i32 %45, ptr %30, align 8, !tbaa !344
  br label %46

46:                                               ; preds = %43, %41
  %47 = load i32, ptr @output_length, align 4, !tbaa !4
  %48 = icmp eq i32 %47, 0
  br i1 %48, label %50, label %49

49:                                               ; preds = %46
  tail call void @fa_write(i32 noundef 1, ptr noundef nonnull @output, i32 noundef %47) #62
  store i32 0, ptr @output_length, align 4, !tbaa !4
  br label %50

50:                                               ; preds = %49, %46
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  store i32 0, ptr @builder_simple, align 4, !tbaa !4
  tail call fastcc void @append_c(ptr noundef nonnull @.str.46) #61
  %51 = udiv i64 %26, 1000000
  tail call fastcc void @append_u64(i64 noundef %51, i32 noundef 0) #61
  tail call fastcc void @append_c(ptr noundef nonnull @.str.47) #61
  %52 = icmp eq ptr %29, null
  %53 = select i1 %52, ptr @.str.48, ptr %29
  tail call fastcc void @append_c(ptr noundef nonnull %53) #61
  tail call fastcc void @append_c(ptr noundef nonnull @.str.49) #61
  tail call void @fa_b_line(i32 noundef 2) #61
  br label %54

54:                                               ; preds = %36, %50, %24, %21
  ret void
}

; Function Attrs: optsize
declare void @fa_host_timer(i32 noundef, double noundef) local_unnamed_addr #49

; Function Attrs: cold noinline noreturn nounwind optsize
define internal fastcc void @deadlock_stop(ptr nocapture noundef readonly %0) unnamed_addr #38 {
  %2 = load i32, ptr @output_length, align 4, !tbaa !4
  %3 = icmp eq i32 %2, 0
  br i1 %3, label %5, label %4

4:                                                ; preds = %1
  tail call void @fa_write(i32 noundef 1, ptr noundef nonnull @output, i32 noundef %2) #62
  store i32 0, ptr @output_length, align 4, !tbaa !4
  br label %5

5:                                                ; preds = %1, %4
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  store i32 0, ptr @builder_simple, align 4, !tbaa !4
  tail call fastcc void @append_c(ptr noundef nonnull @.str.50) #61
  %6 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 25
  %7 = load ptr, ptr %6, align 8, !tbaa !300
  %8 = icmp eq ptr %7, null
  br i1 %8, label %9, label %13

9:                                                ; preds = %5
  %10 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 26
  %11 = load ptr, ptr %10, align 4, !tbaa !346
  %12 = icmp eq ptr %11, null
  br i1 %12, label %17, label %13

13:                                               ; preds = %9, %5
  %14 = phi ptr [ @.str.51, %5 ], [ @.str.53, %9 ]
  %15 = phi ptr [ %6, %5 ], [ %10, %9 ]
  tail call fastcc void @append_c(ptr noundef nonnull %14) #61
  %16 = load ptr, ptr %15, align 4, !tbaa !6
  tail call fastcc void @append_c(ptr noundef %16) #61
  tail call fastcc void @append_c(ptr noundef nonnull @.str.52) #61
  br label %17

17:                                               ; preds = %13, %9
  tail call void @fa_b_line(i32 noundef 2) #61
  tail call void @fa_exit(i32 noundef 101) #63
  unreachable
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define hidden void @fa_wasm_wake(ptr nocapture noundef %0) local_unnamed_addr #46 {
  %2 = getelementptr inbounds %struct.FaWait, ptr %0, i32 0, i32 9
  store i32 1, ptr %2, align 8, !tbaa !328
  %3 = load ptr, ptr @current_executor, align 4, !tbaa !6
  %4 = load ptr, ptr %0, align 8, !tbaa !309
  %5 = getelementptr inbounds i8, ptr %4, i32 16
  %6 = getelementptr inbounds i8, ptr %4, i32 32
  %7 = load i32, ptr %6, align 4, !tbaa !286
  %8 = and i32 %7, -5
  store i32 %8, ptr %6, align 4, !tbaa !286
  %9 = getelementptr inbounds %struct.FaExecutor, ptr %3, i32 0, i32 1
  %10 = getelementptr inbounds i8, ptr %4, i32 20
  store ptr null, ptr %10, align 4, !tbaa !290
  %11 = load ptr, ptr %9, align 4, !tbaa !6
  %12 = getelementptr inbounds i8, ptr %4, i32 24
  store ptr %11, ptr %12, align 4, !tbaa !291
  %13 = icmp eq ptr %11, null
  %14 = getelementptr inbounds %struct.FaTask, ptr %11, i32 0, i32 1
  %15 = select i1 %13, ptr %3, ptr %14
  store ptr %5, ptr %15, align 4, !tbaa !6
  store ptr %5, ptr %9, align 4, !tbaa !6
  %16 = getelementptr inbounds %struct.FaExecutor, ptr %3, i32 0, i32 4
  %17 = load i64, ptr %16, align 8, !tbaa !292
  %18 = add nsw i64 %17, 1
  store i64 %18, ptr %16, align 8, !tbaa !292
  %19 = getelementptr inbounds %struct.FaExecutor, ptr %3, i32 0, i32 5
  %20 = load i64, ptr %19, align 8, !tbaa !293
  %21 = add nsw i64 %20, 1
  store i64 %21, ptr %19, align 8, !tbaa !293
  %22 = getelementptr inbounds i8, ptr %4, i32 28
  store atomic i32 1, ptr %22 release, align 4
  ret void
}

; Function Attrs: nounwind optsize
define hidden noundef i32 @fa_blocking_submit(ptr noundef %0, ptr nocapture noundef writeonly %1, ptr nocapture noundef readnone %2) local_unnamed_addr #3 {
  %4 = load ptr, ptr %0, align 4, !tbaa !347
  tail call void %4(ptr noundef nonnull %0) #62
  %5 = getelementptr inbounds %struct.FaJob, ptr %0, i32 0, i32 5
  store i32 2, ptr %5, align 4, !tbaa !349
  %6 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 9
  store i32 1, ptr %6, align 8, !tbaa !328
  ret i32 0
}

; Function Attrs: nounwind optsize
define hidden void @fa_blocking_cancel(ptr noundef %0) local_unnamed_addr #3 {
  %2 = getelementptr inbounds %struct.FaJob, ptr %0, i32 0, i32 1
  %3 = load ptr, ptr %2, align 4, !tbaa !350
  tail call void %3(ptr noundef %0) #62
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_tcp_close(ptr nocapture noundef %0) local_unnamed_addr #3 {
  %2 = getelementptr inbounds %struct.FaSocket, ptr %0, i32 0, i32 4
  %3 = load ptr, ptr %2, align 8, !tbaa !351
  %4 = icmp eq ptr %3, null
  br i1 %4, label %6, label %5

5:                                                ; preds = %1
  store ptr null, ptr %2, align 8, !tbaa !351
  tail call void @fa_free(ptr noundef nonnull %3, i32 noundef 20, i32 noundef 16) #62
  br label %6

6:                                                ; preds = %1, %5
  %7 = getelementptr inbounds %struct.FaSocket, ptr %0, i32 0, i32 1
  store i32 -1, ptr %7, align 8, !tbaa !353
  ret void
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_tcp_listen(ptr nocapture noundef readnone %0, i32 noundef %1, i64 noundef %2) local_unnamed_addr #3 {
  %4 = tail call fastcc ptr @shared_new(i32 noundef 56, ptr noundef nonnull @socket_drop) #61
  %5 = getelementptr inbounds %struct.FaSocket, ptr %4, i32 0, i32 1
  store i32 -1, ptr %5, align 8, !tbaa !353
  %6 = getelementptr inbounds %struct.FaSocket, ptr %4, i32 0, i32 2
  store i32 38, ptr %6, align 4, !tbaa !354
  ret ptr %4
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read)
define hidden i64 @fa_tcp_port(ptr nocapture noundef readonly %0) local_unnamed_addr #37 {
  %2 = getelementptr inbounds %struct.FaSocket, ptr %0, i32 0, i32 3
  %3 = load i64, ptr %2, align 8, !tbaa !355
  ret i64 %3
}

; Function Attrs: nounwind optsize
define hidden void @fa_tcp_cancel(ptr noundef readonly %0, ptr noundef readnone %1, ptr noundef %2) local_unnamed_addr #3 {
  %4 = icmp eq ptr %0, null
  br i1 %4, label %29, label %5

5:                                                ; preds = %3
  %6 = getelementptr inbounds %struct.FaSocket, ptr %0, i32 0, i32 4
  %7 = load ptr, ptr %6, align 8, !tbaa !351
  %8 = icmp eq ptr %7, null
  br i1 %8, label %29, label %9

9:                                                ; preds = %5
  %10 = getelementptr inbounds %struct.FaIo, ptr %7, i32 0, i32 2
  %11 = load ptr, ptr %10, align 4, !tbaa !331
  %12 = icmp eq ptr %11, %1
  br i1 %12, label %13, label %19

13:                                               ; preds = %9
  store ptr null, ptr %10, align 4, !tbaa !331
  %14 = getelementptr inbounds %struct.FaIo, ptr %7, i32 0, i32 4
  %15 = load ptr, ptr %14, align 4, !tbaa !327
  %16 = getelementptr inbounds %struct.FaExecutor, ptr %15, i32 0, i32 13
  %17 = load i64, ptr %16, align 8, !tbaa !330
  %18 = add nsw i64 %17, -1
  store i64 %18, ptr %16, align 8, !tbaa !330
  br label %19

19:                                               ; preds = %13, %9
  %20 = getelementptr inbounds %struct.FaIo, ptr %7, i32 0, i32 3
  %21 = load ptr, ptr %20, align 4, !tbaa !332
  %22 = icmp eq ptr %21, %1
  br i1 %22, label %23, label %29

23:                                               ; preds = %19
  store ptr null, ptr %20, align 4, !tbaa !332
  %24 = getelementptr inbounds %struct.FaIo, ptr %7, i32 0, i32 4
  %25 = load ptr, ptr %24, align 4, !tbaa !327
  %26 = getelementptr inbounds %struct.FaExecutor, ptr %25, i32 0, i32 13
  %27 = load i64, ptr %26, align 8, !tbaa !330
  %28 = add nsw i64 %27, -1
  store i64 %28, ptr %26, align 8, !tbaa !330
  br label %29

29:                                               ; preds = %23, %19, %5, %3
  %30 = icmp eq ptr %2, null
  br i1 %30, label %60, label %31

31:                                               ; preds = %29
  %32 = load ptr, ptr %2, align 4, !tbaa !6
  %33 = icmp eq ptr %32, null
  br i1 %33, label %60, label %34

34:                                               ; preds = %31
  %35 = getelementptr inbounds %struct.FaSocket, ptr %32, i32 0, i32 4
  %36 = load ptr, ptr %35, align 8, !tbaa !351
  %37 = icmp eq ptr %36, null
  br i1 %37, label %58, label %38

38:                                               ; preds = %34
  %39 = getelementptr inbounds %struct.FaIo, ptr %36, i32 0, i32 2
  %40 = load ptr, ptr %39, align 4, !tbaa !331
  %41 = icmp eq ptr %40, %1
  br i1 %41, label %42, label %48

42:                                               ; preds = %38
  store ptr null, ptr %39, align 4, !tbaa !331
  %43 = getelementptr inbounds %struct.FaIo, ptr %36, i32 0, i32 4
  %44 = load ptr, ptr %43, align 4, !tbaa !327
  %45 = getelementptr inbounds %struct.FaExecutor, ptr %44, i32 0, i32 13
  %46 = load i64, ptr %45, align 8, !tbaa !330
  %47 = add nsw i64 %46, -1
  store i64 %47, ptr %45, align 8, !tbaa !330
  br label %48

48:                                               ; preds = %42, %38
  %49 = getelementptr inbounds %struct.FaIo, ptr %36, i32 0, i32 3
  %50 = load ptr, ptr %49, align 4, !tbaa !332
  %51 = icmp eq ptr %50, %1
  br i1 %51, label %52, label %58

52:                                               ; preds = %48
  store ptr null, ptr %49, align 4, !tbaa !332
  %53 = getelementptr inbounds %struct.FaIo, ptr %36, i32 0, i32 4
  %54 = load ptr, ptr %53, align 4, !tbaa !327
  %55 = getelementptr inbounds %struct.FaExecutor, ptr %54, i32 0, i32 13
  %56 = load i64, ptr %55, align 8, !tbaa !330
  %57 = add nsw i64 %56, -1
  store i64 %57, ptr %55, align 8, !tbaa !330
  br label %58

58:                                               ; preds = %52, %48, %34
  %59 = load ptr, ptr %2, align 4, !tbaa !6
  tail call void @fa_shared_release(ptr noundef %59) #61
  store ptr null, ptr %2, align 4, !tbaa !6
  br label %60

60:                                               ; preds = %58, %31, %29
  ret void
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_tcp_accept(ptr nocapture noundef readonly %0, ptr nocapture noundef readnone %1) local_unnamed_addr #3 {
  %3 = alloca ptr, align 4
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %3) #64
  store ptr null, ptr %3, align 4, !tbaa !6
  call fastcc void @try_accept(ptr noundef %0, ptr noundef nonnull %3) #61
  %4 = load ptr, ptr %3, align 4, !tbaa !6
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %3) #64
  ret ptr %4
}

; Function Attrs: nounwind optsize
define internal fastcc void @try_accept(ptr nocapture noundef readonly %0, ptr nocapture noundef writeonly %1) unnamed_addr #3 {
  %3 = getelementptr inbounds %struct.FaSocket, ptr %0, i32 0, i32 1
  %4 = load i32, ptr %3, align 8, !tbaa !353
  %5 = icmp slt i32 %4, 0
  br i1 %5, label %6, label %12

6:                                                ; preds = %2
  %7 = getelementptr inbounds %struct.FaSocket, ptr %0, i32 0, i32 2
  %8 = load i32, ptr %7, align 4, !tbaa !354
  %9 = icmp eq i32 %8, 0
  %10 = select i1 %9, i32 9, i32 %8
  %11 = tail call fastcc ptr @net_error(i32 noundef %10, ptr noundef nonnull @.str.54) #61
  br label %14

12:                                               ; preds = %2
  %13 = tail call fastcc ptr @net_error(i32 noundef 38, ptr noundef nonnull @.str.55) #61
  br label %14

14:                                               ; preds = %12, %6
  %15 = phi ptr [ %13, %12 ], [ %11, %6 ]
  store ptr %15, ptr %1, align 4, !tbaa !6
  ret void
}

; Function Attrs: nounwind optsize
define hidden noundef i32 @fa_tcp_accept_async(ptr nocapture noundef readonly %0, ptr nocapture noundef readnone %1, ptr nocapture noundef readnone %2, ptr nocapture noundef readnone %3, ptr nocapture noundef writeonly %4) local_unnamed_addr #3 {
  tail call fastcc void @try_accept(ptr noundef %0, ptr noundef %4) #61
  ret i32 0
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_tcp_connect(ptr nocapture noundef readnone %0, i32 noundef %1, i64 noundef %2, ptr nocapture noundef writeonly %3) local_unnamed_addr #3 {
  store ptr null, ptr %3, align 4, !tbaa !6
  %5 = tail call fastcc ptr @net_error(i32 noundef 38, ptr noundef nonnull @.str.58) #61
  ret ptr %5
}

; Function Attrs: nounwind optsize
define hidden noundef i32 @fa_tcp_connect_async(ptr nocapture noundef readnone %0, i32 noundef %1, i64 noundef %2, ptr nocapture noundef readnone %3, ptr nocapture noundef readnone %4, ptr nocapture noundef readonly %5, ptr nocapture noundef writeonly %6) local_unnamed_addr #3 {
  %8 = tail call fastcc ptr @net_error(i32 noundef 38, ptr noundef nonnull @.str.58) #61
  store ptr %8, ptr %6, align 4, !tbaa !6
  ret i32 0
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_tcp_read(ptr nocapture noundef readonly %0, ptr nocapture noundef readnone %1, ptr nocapture noundef readnone %2) local_unnamed_addr #3 {
  %4 = getelementptr inbounds %struct.FaSocket, ptr %0, i32 0, i32 1
  %5 = load i32, ptr %4, align 8, !tbaa !353
  %6 = icmp slt i32 %5, 0
  %7 = select i1 %6, i32 9, i32 38
  %8 = tail call fastcc ptr @net_error(i32 noundef %7, ptr noundef nonnull @.str.59) #61
  ret ptr %8
}

; Function Attrs: nounwind optsize
define hidden noundef i32 @fa_tcp_read_async(ptr nocapture noundef readonly %0, ptr nocapture noundef readnone %1, ptr nocapture noundef readnone %2, ptr nocapture noundef readnone %3, ptr nocapture noundef readnone %4, ptr nocapture noundef writeonly %5) local_unnamed_addr #3 {
  %7 = getelementptr inbounds %struct.FaSocket, ptr %0, i32 0, i32 1
  %8 = load i32, ptr %7, align 8, !tbaa !353
  %9 = icmp slt i32 %8, 0
  %10 = select i1 %9, i32 9, i32 38
  %11 = tail call fastcc ptr @net_error(i32 noundef %10, ptr noundef nonnull @.str.59) #61
  store ptr %11, ptr %5, align 4, !tbaa !6
  ret i32 0
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_tcp_write(ptr nocapture noundef readonly %0, ptr nocapture noundef readnone %1, i64 noundef %2) local_unnamed_addr #3 {
  %4 = getelementptr inbounds %struct.FaSocket, ptr %0, i32 0, i32 1
  %5 = load i32, ptr %4, align 8, !tbaa !353
  %6 = icmp slt i32 %5, 0
  %7 = select i1 %6, i32 9, i32 38
  %8 = tail call fastcc ptr @net_error(i32 noundef %7, ptr noundef nonnull @.str.60) #61
  ret ptr %8
}

; Function Attrs: nounwind optsize
define hidden noundef i32 @fa_tcp_write_async(ptr nocapture noundef readonly %0, ptr nocapture noundef readnone %1, i64 noundef %2, ptr nocapture noundef readnone %3, ptr nocapture noundef readnone %4, ptr nocapture noundef readnone %5, ptr nocapture noundef writeonly %6) local_unnamed_addr #3 {
  %8 = getelementptr inbounds %struct.FaSocket, ptr %0, i32 0, i32 1
  %9 = load i32, ptr %8, align 8, !tbaa !353
  %10 = icmp slt i32 %9, 0
  %11 = select i1 %10, i32 9, i32 38
  %12 = tail call fastcc ptr @net_error(i32 noundef %11, ptr noundef nonnull @.str.60) #61
  store ptr %12, ptr %6, align 4, !tbaa !6
  ret i32 0
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_file_job(i32 noundef %0, ptr nocapture noundef readonly %1, i32 noundef %2, ptr nocapture noundef readonly %3, i32 noundef %4) local_unnamed_addr #3 {
  %6 = tail call ptr @fa_alloc(i32 noundef 52, i32 noundef 16) #62
  %7 = getelementptr inbounds i8, ptr %6, i32 8
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 4 dereferenceable(52) %7, i8 0, i32 44, i1 false)
  store ptr @file_job_run, ptr %6, align 4, !tbaa !356
  %8 = getelementptr inbounds %struct.FaJob, ptr %6, i32 0, i32 1
  store ptr @file_job_release, ptr %8, align 4, !tbaa !358
  %9 = getelementptr inbounds %struct.FaFileJob, ptr %6, i32 0, i32 1
  store i32 %0, ptr %9, align 4, !tbaa !359
  %10 = add i32 %2, 1
  %11 = tail call ptr @fa_alloc(i32 noundef %10, i32 noundef 1) #62
  %12 = getelementptr inbounds %struct.FaFileJob, ptr %6, i32 0, i32 3
  store ptr %11, ptr %12, align 4, !tbaa !360
  tail call fastcc void @copy_bytes(ptr noundef %11, ptr noundef %1, i32 noundef %2) #61
  %13 = load ptr, ptr %12, align 4, !tbaa !360
  %14 = getelementptr inbounds i8, ptr %13, i32 %2
  store i8 0, ptr %14, align 1, !tbaa !1
  %15 = getelementptr inbounds %struct.FaFileJob, ptr %6, i32 0, i32 4
  store i32 %2, ptr %15, align 4, !tbaa !361
  %16 = icmp eq i32 %0, 2
  %17 = icmp ne i32 %4, 0
  %18 = and i1 %16, %17
  br i1 %18, label %19, label %24

19:                                               ; preds = %5
  %20 = tail call ptr @fa_alloc(i32 noundef %4, i32 noundef 16) #62
  %21 = getelementptr inbounds %struct.FaFileJob, ptr %6, i32 0, i32 5
  store ptr %20, ptr %21, align 4, !tbaa !362
  tail call fastcc void @copy_bytes(ptr noundef %20, ptr noundef %3, i32 noundef %4) #61
  %22 = getelementptr inbounds %struct.FaFileJob, ptr %6, i32 0, i32 6
  store i32 %4, ptr %22, align 4, !tbaa !363
  %23 = getelementptr inbounds %struct.FaFileJob, ptr %6, i32 0, i32 7
  store i32 %4, ptr %23, align 4, !tbaa !364
  br label %24

24:                                               ; preds = %19, %5
  ret ptr %6
}

; Function Attrs: nounwind optsize
define internal void @file_job_run(ptr nocapture noundef %0) #3 {
  %2 = alloca i32, align 4
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %2) #64
  store i32 0, ptr %2, align 4, !tbaa !23
  %3 = getelementptr inbounds %struct.FaFileJob, ptr %0, i32 0, i32 1
  %4 = load i32, ptr %3, align 4, !tbaa !359
  %5 = getelementptr inbounds %struct.FaFileJob, ptr %0, i32 0, i32 3
  %6 = load ptr, ptr %5, align 4, !tbaa !360
  %7 = getelementptr inbounds %struct.FaFileJob, ptr %0, i32 0, i32 4
  %8 = load i32, ptr %7, align 4, !tbaa !361
  %9 = getelementptr inbounds %struct.FaFileJob, ptr %0, i32 0, i32 5
  %10 = load ptr, ptr %9, align 4, !tbaa !362
  %11 = getelementptr inbounds %struct.FaFileJob, ptr %0, i32 0, i32 6
  %12 = load i32, ptr %11, align 4, !tbaa !363
  %13 = call i32 @fa_host_file_run(i32 noundef %4, ptr noundef %6, i32 noundef %8, ptr noundef %10, i32 noundef %12, ptr noundef nonnull %2) #62
  %14 = getelementptr inbounds %struct.FaFileJob, ptr %0, i32 0, i32 7
  %15 = load i32, ptr %14, align 4, !tbaa !364
  %16 = icmp eq i32 %15, 0
  br i1 %16, label %19, label %17

17:                                               ; preds = %1
  %18 = load ptr, ptr %9, align 4, !tbaa !362
  call void @fa_free(ptr noundef %18, i32 noundef %15, i32 noundef 16) #62
  br label %19

19:                                               ; preds = %17, %1
  store ptr null, ptr %9, align 4, !tbaa !362
  store i32 0, ptr %11, align 4, !tbaa !363
  store i32 0, ptr %14, align 4, !tbaa !364
  %20 = load i32, ptr %2, align 4, !tbaa !23
  %21 = icmp sgt i32 %20, 0
  br i1 %21, label %22, label %25

22:                                               ; preds = %19
  %23 = call ptr @fa_alloc(i32 noundef %20, i32 noundef 16) #62
  store ptr %23, ptr %9, align 4, !tbaa !362
  call void @fa_host_file_take(ptr noundef %23) #62
  %24 = load i32, ptr %2, align 4, !tbaa !23
  store i32 %24, ptr %11, align 4, !tbaa !363
  store i32 %24, ptr %14, align 4, !tbaa !364
  br label %25

25:                                               ; preds = %22, %19
  %26 = icmp eq i32 %13, 0
  br i1 %26, label %29, label %27

27:                                               ; preds = %25
  %28 = getelementptr inbounds %struct.FaFileJob, ptr %0, i32 0, i32 2
  store i32 -2, ptr %28, align 4, !tbaa !365
  br label %29

29:                                               ; preds = %27, %25
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %2) #64
  ret void
}

; Function Attrs: nounwind optsize
define internal void @file_job_release(ptr noundef %0) #3 {
  %2 = getelementptr inbounds %struct.FaFileJob, ptr %0, i32 0, i32 3
  %3 = load ptr, ptr %2, align 4, !tbaa !360
  %4 = getelementptr inbounds %struct.FaFileJob, ptr %0, i32 0, i32 4
  %5 = load i32, ptr %4, align 4, !tbaa !361
  %6 = add i32 %5, 1
  tail call void @fa_free(ptr noundef %3, i32 noundef %6, i32 noundef 1) #62
  %7 = getelementptr inbounds %struct.FaFileJob, ptr %0, i32 0, i32 7
  %8 = load i32, ptr %7, align 4, !tbaa !364
  %9 = icmp eq i32 %8, 0
  br i1 %9, label %13, label %10

10:                                               ; preds = %1
  %11 = getelementptr inbounds %struct.FaFileJob, ptr %0, i32 0, i32 5
  %12 = load ptr, ptr %11, align 4, !tbaa !362
  tail call void @fa_free(ptr noundef %12, i32 noundef %8, i32 noundef 16) #62
  br label %13

13:                                               ; preds = %10, %1
  tail call void @fa_free(ptr noundef nonnull %0, i32 noundef 52, i32 noundef 16) #62
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_file_run(ptr nocapture noundef %0) local_unnamed_addr #3 {
  tail call void @file_job_run(ptr noundef %0) #61
  ret void
}

; Function Attrs: nounwind optsize
define hidden noundef i32 @fa_file_submit(ptr noundef %0, ptr nocapture noundef writeonly %1, ptr nocapture noundef readnone %2) local_unnamed_addr #3 {
  %4 = load ptr, ptr %0, align 4, !tbaa !347
  tail call void %4(ptr noundef nonnull %0) #62
  %5 = getelementptr inbounds %struct.FaJob, ptr %0, i32 0, i32 5
  store i32 2, ptr %5, align 4, !tbaa !349
  %6 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 9
  store i32 1, ptr %6, align 8, !tbaa !328
  ret i32 0
}

; Function Attrs: nounwind optsize
define hidden void @fa_file_cancel(ptr noundef %0) local_unnamed_addr #3 {
  %2 = getelementptr inbounds %struct.FaJob, ptr %0, i32 0, i32 1
  %3 = load ptr, ptr %2, align 4, !tbaa !350
  tail call void %3(ptr noundef %0) #62
  ret void
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_file_finish(ptr noundef %0, ptr nocapture noundef writeonly %1) local_unnamed_addr #3 {
  %3 = alloca %struct.FaString, align 4
  %4 = alloca %struct.FaString, align 4
  %5 = getelementptr inbounds %struct.FaFileJob, ptr %0, i32 0, i32 2
  %6 = load i32, ptr %5, align 4, !tbaa !365
  %7 = icmp eq i32 %6, 0
  br i1 %7, label %32, label %8

8:                                                ; preds = %2
  %9 = getelementptr inbounds %struct.FaFileJob, ptr %0, i32 0, i32 5
  %10 = load ptr, ptr %9, align 4, !tbaa !362
  %11 = getelementptr inbounds %struct.FaFileJob, ptr %0, i32 0, i32 6
  %12 = load i32, ptr %11, align 4, !tbaa !363
  %13 = getelementptr inbounds %struct.FaFileJob, ptr %0, i32 0, i32 3
  %14 = load ptr, ptr %13, align 4, !tbaa !360
  %15 = getelementptr inbounds %struct.FaFileJob, ptr %0, i32 0, i32 4
  %16 = load i32, ptr %15, align 4, !tbaa !361
  %17 = tail call ptr @fa_alloc(i32 noundef 32, i32 noundef 8) #62
  store i64 9, ptr %17, align 8, !tbaa !238
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  store i32 0, ptr @builder_simple, align 4, !tbaa !4
  %18 = icmp eq i32 %6, -2
  br i1 %18, label %19, label %24

19:                                               ; preds = %8
  tail call fastcc void @reserve(i32 noundef %12) #61
  %20 = load ptr, ptr @builder, align 4, !tbaa !6
  %21 = load i32, ptr @builder_length, align 4, !tbaa !4
  %22 = getelementptr inbounds i8, ptr %20, i32 %21
  tail call fastcc void @copy_bytes(ptr noundef %22, ptr noundef %10, i32 noundef %12) #61
  %23 = add i32 %21, %12
  store i32 %23, ptr @builder_length, align 4, !tbaa !4
  br label %29

24:                                               ; preds = %8
  tail call fastcc void @append_c(ptr noundef nonnull @.str.57) #61
  %25 = sext i32 %6 to i64
  %26 = tail call i64 @llvm.abs.i64(i64 %25, i1 true)
  %27 = lshr i64 %25, 63
  %28 = trunc i64 %27 to i32
  tail call fastcc void @append_u64(i64 noundef %26, i32 noundef %28) #61
  br label %29

29:                                               ; preds = %19, %24
  %30 = getelementptr inbounds %struct.FileErrorBox, ptr %17, i32 0, i32 2
  tail call void @fa_b_take(ptr noundef nonnull %30) #61
  %31 = getelementptr inbounds %struct.FileErrorBox, ptr %17, i32 0, i32 1
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %3) #64
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %3, ptr noundef %14, i32 noundef %16, i32 noundef 0) #61
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 8 dereferenceable(12) %31, ptr noundef nonnull align 4 dereferenceable(12) %3, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %3) #64
  br label %52

32:                                               ; preds = %2
  %33 = getelementptr inbounds %struct.FaFileJob, ptr %0, i32 0, i32 1
  %34 = load i32, ptr %33, align 4, !tbaa !359
  switch i32 %34, label %52 [
    i32 0, label %35
    i32 1, label %40
  ]

35:                                               ; preds = %32
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %4) #64
  %36 = getelementptr inbounds %struct.FaFileJob, ptr %0, i32 0, i32 5
  %37 = load ptr, ptr %36, align 4, !tbaa !362
  %38 = getelementptr inbounds %struct.FaFileJob, ptr %0, i32 0, i32 6
  %39 = load i32, ptr %38, align 4, !tbaa !363
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %4, ptr noundef %37, i32 noundef %39, i32 noundef 0) #61
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %1, ptr noundef nonnull align 4 dereferenceable(12) %4, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %4) #64
  br label %52

40:                                               ; preds = %32
  %41 = getelementptr inbounds %struct.FaFileJob, ptr %0, i32 0, i32 6
  %42 = load i32, ptr %41, align 4, !tbaa !363
  %43 = zext i32 %42 to i64
  %44 = tail call ptr @fa_array_new(i64 noundef %43, i64 noundef %43, i64 noundef 1) #61
  %45 = load i32, ptr %41, align 4, !tbaa !363
  %46 = icmp eq i32 %45, 0
  br i1 %46, label %51, label %47

47:                                               ; preds = %40
  %48 = load ptr, ptr %44, align 8, !tbaa !66
  %49 = getelementptr inbounds %struct.FaFileJob, ptr %0, i32 0, i32 5
  %50 = load ptr, ptr %49, align 4, !tbaa !362
  tail call fastcc void @copy_bytes(ptr noundef %48, ptr noundef %50, i32 noundef %45) #61
  br label %51

51:                                               ; preds = %47, %40
  store ptr %44, ptr %1, align 4, !tbaa !6
  br label %52

52:                                               ; preds = %32, %35, %51, %29
  %53 = phi ptr [ %17, %29 ], [ null, %35 ], [ null, %51 ], [ null, %32 ]
  tail call void @file_job_release(ptr noundef nonnull %0) #61
  ret ptr %53
}

; Function Attrs: nounwind optsize
define hidden void @fa_json_free(ptr noundef %0) local_unnamed_addr #3 {
  %2 = getelementptr inbounds %struct.FaJsonDocument, ptr %0, i32 0, i32 1
  %3 = load ptr, ptr %2, align 8, !tbaa !366
  %4 = icmp eq ptr %3, null
  br i1 %4, label %11, label %5

5:                                                ; preds = %1, %5
  %6 = phi ptr [ %7, %5 ], [ %3, %1 ]
  %7 = load ptr, ptr %6, align 4, !tbaa !370
  %8 = getelementptr inbounds %struct.JsonChunk, ptr %6, i32 0, i32 1
  %9 = load i32, ptr %8, align 4, !tbaa !372
  tail call void @fa_free(ptr noundef nonnull %6, i32 noundef %9, i32 noundef 16) #62
  %10 = icmp eq ptr %7, null
  br i1 %10, label %11, label %5, !llvm.loop !373

11:                                               ; preds = %5, %1
  tail call void @fa_free(ptr noundef %0, i32 noundef 64, i32 noundef 16) #62
  ret void
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_json_parse(ptr noundef %0, i32 noundef %1, ptr noundef %2, ptr nocapture noundef writeonly %3) local_unnamed_addr #3 {
  %5 = alloca %struct.JsonReader, align 4
  %6 = alloca %struct.JsonCheck, align 4
  %7 = tail call ptr @fa_alloc(i32 noundef 64, i32 noundef 16) #62
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(64) %7, i8 0, i32 64, i1 false)
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %5) #64
  %8 = getelementptr inbounds i8, ptr %5, i32 8
  call void @llvm.memset.p0.i32(ptr noundef nonnull align 4 dereferenceable(48) %8, i8 0, i32 40, i1 false)
  store ptr %0, ptr %5, align 4, !tbaa !374
  %9 = getelementptr inbounds %struct.JsonReader, ptr %5, i32 0, i32 1
  store i32 %1, ptr %9, align 4, !tbaa !376
  %10 = getelementptr inbounds %struct.JsonReader, ptr %5, i32 0, i32 3
  store ptr %7, ptr %10, align 4, !tbaa !377
  %11 = call fastcc i32 @json_value(ptr noundef nonnull %5, ptr noundef %7) #61
  %12 = icmp eq i32 %11, 0
  br i1 %12, label %36, label %13

13:                                               ; preds = %4
  %14 = getelementptr inbounds %struct.JsonReader, ptr %5, i32 0, i32 2
  %15 = load i32, ptr %9, align 4, !tbaa !376
  %16 = load i32, ptr %14, align 4, !tbaa !378
  %17 = icmp ult i32 %16, %15
  br i1 %17, label %18, label %28

18:                                               ; preds = %13
  %19 = load ptr, ptr %5, align 4, !tbaa !374
  br label %20

20:                                               ; preds = %26, %18
  %21 = phi i32 [ %16, %18 ], [ %25, %26 ]
  %22 = getelementptr inbounds i8, ptr %19, i32 %21
  %23 = load i8, ptr %22, align 1, !tbaa !1
  switch i8 %23, label %28 [
    i8 32, label %24
    i8 13, label %24
    i8 10, label %24
    i8 9, label %24
  ]

24:                                               ; preds = %20, %20, %20, %20
  %25 = add i32 %21, 1
  store i32 %25, ptr %14, align 4, !tbaa !378
  switch i8 %23, label %28 [
    i8 32, label %26
    i8 13, label %26
    i8 10, label %26
    i8 9, label %26
  ]

26:                                               ; preds = %24, %24, %24, %24
  %27 = icmp eq i32 %25, %15
  br i1 %27, label %36, label %20

28:                                               ; preds = %20, %24, %13
  %29 = phi i32 [ %16, %13 ], [ %21, %20 ], [ %25, %24 ]
  %30 = icmp ult i32 %29, %15
  br i1 %30, label %31, label %36

31:                                               ; preds = %28
  %32 = getelementptr inbounds %struct.JsonReader, ptr %5, i32 0, i32 8
  store ptr @.str.31, ptr %32, align 4, !tbaa !379
  %33 = getelementptr inbounds %struct.JsonReader, ptr %5, i32 0, i32 9
  store ptr null, ptr %33, align 4, !tbaa !380
  %34 = getelementptr inbounds %struct.JsonReader, ptr %5, i32 0, i32 10
  store i32 0, ptr %34, align 4, !tbaa !381
  %35 = getelementptr inbounds %struct.JsonReader, ptr %5, i32 0, i32 11
  store i8 0, ptr %35, align 4, !tbaa !382
  br label %36

36:                                               ; preds = %26, %28, %31, %4
  %37 = phi i1 [ true, %31 ], [ false, %28 ], [ true, %4 ], [ false, %26 ]
  %38 = getelementptr inbounds %struct.JsonReader, ptr %5, i32 0, i32 6
  %39 = load i32, ptr %38, align 4, !tbaa !383
  %40 = icmp eq i32 %39, 0
  br i1 %40, label %45, label %41

41:                                               ; preds = %36
  %42 = getelementptr inbounds %struct.JsonReader, ptr %5, i32 0, i32 4
  %43 = load ptr, ptr %42, align 4, !tbaa !384
  %44 = mul i32 %39, 48
  call void @fa_free(ptr noundef %43, i32 noundef %44, i32 noundef 16) #62
  br label %45

45:                                               ; preds = %41, %36
  br i1 %37, label %46, label %80

46:                                               ; preds = %45
  call void @fa_json_free(ptr noundef %7) #61
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  store i32 0, ptr @builder_simple, align 4, !tbaa !4
  call fastcc void @append_c(ptr noundef nonnull @.str.76) #61
  %47 = getelementptr inbounds %struct.JsonReader, ptr %5, i32 0, i32 8
  %48 = load ptr, ptr %47, align 4, !tbaa !379
  call fastcc void @append_c(ptr noundef %48) #61
  %49 = getelementptr inbounds %struct.JsonReader, ptr %5, i32 0, i32 9
  %50 = load ptr, ptr %49, align 4, !tbaa !380
  %51 = icmp eq ptr %50, null
  br i1 %51, label %77, label %52

52:                                               ; preds = %46
  %53 = getelementptr inbounds %struct.JsonReader, ptr %5, i32 0, i32 11
  %54 = load i8, ptr %53, align 4, !tbaa !382
  %55 = icmp eq i8 %54, 0
  br i1 %55, label %62, label %56

56:                                               ; preds = %52
  call fastcc void @reserve(i32 noundef 1) #61
  %57 = load ptr, ptr @builder, align 4, !tbaa !6
  %58 = load i32, ptr @builder_length, align 4, !tbaa !4
  %59 = getelementptr inbounds i8, ptr %57, i32 %58
  store i8 %54, ptr %59, align 1, !tbaa !1
  %60 = add i32 %58, 1
  store i32 %60, ptr @builder_length, align 4, !tbaa !4
  %61 = load ptr, ptr %49, align 4, !tbaa !380
  br label %62

62:                                               ; preds = %56, %52
  %63 = phi ptr [ %61, %56 ], [ %50, %52 ]
  %64 = getelementptr inbounds %struct.JsonReader, ptr %5, i32 0, i32 10
  %65 = load i32, ptr %64, align 4, !tbaa !381
  call fastcc void @reserve(i32 noundef %65) #61
  %66 = load ptr, ptr @builder, align 4, !tbaa !6
  %67 = load i32, ptr @builder_length, align 4, !tbaa !4
  %68 = getelementptr inbounds i8, ptr %66, i32 %67
  call fastcc void @copy_bytes(ptr noundef %68, ptr noundef %63, i32 noundef %65) #61
  %69 = add i32 %67, %65
  store i32 %69, ptr @builder_length, align 4, !tbaa !4
  %70 = load i8, ptr %53, align 4, !tbaa !382
  %71 = icmp eq i8 %70, 0
  br i1 %71, label %77, label %72

72:                                               ; preds = %62
  call fastcc void @reserve(i32 noundef 1) #61
  %73 = load ptr, ptr @builder, align 4, !tbaa !6
  %74 = load i32, ptr @builder_length, align 4, !tbaa !4
  %75 = getelementptr inbounds i8, ptr %73, i32 %74
  store i8 %70, ptr %75, align 1, !tbaa !1
  %76 = add i32 %74, 1
  store i32 %76, ptr @builder_length, align 4, !tbaa !4
  br label %77

77:                                               ; preds = %46, %62, %72
  %78 = call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #62
  store i64 10, ptr %78, align 8, !tbaa !233
  %79 = getelementptr inbounds %struct.MessageError, ptr %78, i32 0, i32 1
  call void @fa_b_take(ptr noundef nonnull %79) #61
  br label %95

80:                                               ; preds = %45
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %6) #64
  %81 = getelementptr inbounds i8, ptr %6, i32 4
  call void @llvm.memset.p0.i32(ptr noundef nonnull align 4 dereferenceable(16) %81, i8 0, i32 12, i1 false)
  %82 = getelementptr inbounds %struct.JsonCheck, ptr %6, i32 0, i32 2
  store i32 64, ptr %82, align 4, !tbaa !385
  %83 = call ptr @fa_alloc(i32 noundef 64, i32 noundef 1) #62
  store ptr %83, ptr %6, align 4, !tbaa !387
  call fastcc void @json_path(ptr noundef nonnull %6, ptr noundef nonnull @.str.32, i32 noundef 9) #61
  %84 = call fastcc i32 @json_check(ptr noundef nonnull %6, ptr noundef %7, ptr noundef %2, i32 noundef 1) #61
  %85 = load ptr, ptr %6, align 4, !tbaa !387
  %86 = load i32, ptr %82, align 4, !tbaa !385
  call void @fa_free(ptr noundef %85, i32 noundef %86, i32 noundef 1) #62
  %87 = getelementptr inbounds %struct.JsonCheck, ptr %6, i32 0, i32 3
  %88 = load ptr, ptr %87, align 4, !tbaa !388
  %89 = icmp eq ptr %88, null
  br i1 %89, label %92, label %90

90:                                               ; preds = %80
  call void @fa_json_free(ptr noundef %7) #61
  %91 = load ptr, ptr %87, align 4, !tbaa !388
  br label %93

92:                                               ; preds = %80
  store ptr %7, ptr %3, align 4, !tbaa !6
  br label %93

93:                                               ; preds = %92, %90
  %94 = phi ptr [ %91, %90 ], [ null, %92 ]
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %6) #64
  br label %95

95:                                               ; preds = %93, %77
  %96 = phi ptr [ %94, %93 ], [ %78, %77 ]
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %5) #64
  ret ptr %96
}

; Function Attrs: nounwind optsize
define internal fastcc noundef i32 @json_value(ptr noundef %0, ptr nocapture noundef %1) unnamed_addr #3 {
  %3 = alloca %struct.FaJson, align 8
  %4 = alloca %struct.FaJson, align 8
  tail call fastcc void @json_space(ptr noundef %0) #61
  %5 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 2
  %6 = load i32, ptr %5, align 4, !tbaa !378
  %7 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 1
  %8 = load i32, ptr %7, align 4, !tbaa !376
  %9 = icmp ult i32 %6, %8
  br i1 %9, label %15, label %10

10:                                               ; preds = %2
  %11 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 8
  store ptr @.str.61, ptr %11, align 4, !tbaa !379
  %12 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 9
  store ptr null, ptr %12, align 4, !tbaa !380
  %13 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 10
  store i32 0, ptr %13, align 4, !tbaa !381
  %14 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 11
  store i8 0, ptr %14, align 4, !tbaa !382
  br label %383

15:                                               ; preds = %2
  %16 = load ptr, ptr %0, align 4, !tbaa !374
  %17 = getelementptr inbounds i8, ptr %16, i32 %6
  %18 = load i8, ptr %17, align 1, !tbaa !1
  %19 = icmp eq i8 %18, 123
  switch i8 %18, label %167 [
    i8 123, label %20
    i8 91, label %20
    i8 34, label %163
    i8 45, label %170
  ]

20:                                               ; preds = %15, %15
  %21 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 7
  %22 = load i32, ptr %21, align 4, !tbaa !389
  %23 = icmp sgt i32 %22, 999
  br i1 %23, label %24, label %29

24:                                               ; preds = %20
  %25 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 8
  store ptr @.str.62, ptr %25, align 4, !tbaa !379
  %26 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 9
  store ptr null, ptr %26, align 4, !tbaa !380
  %27 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 10
  store i32 0, ptr %27, align 4, !tbaa !381
  %28 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 11
  store i8 0, ptr %28, align 4, !tbaa !382
  br label %383

29:                                               ; preds = %20
  %30 = add nsw i32 %22, 1
  store i32 %30, ptr %21, align 4, !tbaa !389
  %31 = add nuw i32 %6, 1
  store i32 %31, ptr %5, align 4, !tbaa !378
  %32 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 5
  br i1 %19, label %33, label %109

33:                                               ; preds = %29
  store i32 5, ptr %1, align 8, !tbaa !390
  %34 = load i32, ptr %32, align 4, !tbaa !391
  tail call fastcc void @json_space(ptr noundef nonnull %0) #61
  %35 = load i32, ptr %5, align 4, !tbaa !378
  %36 = load i32, ptr %7, align 4, !tbaa !376
  %37 = icmp ult i32 %35, %36
  br i1 %37, label %38, label %49

38:                                               ; preds = %33
  %39 = load ptr, ptr %0, align 4, !tbaa !374
  %40 = getelementptr inbounds i8, ptr %39, i32 %35
  %41 = load i8, ptr %40, align 1, !tbaa !1
  switch i8 %41, label %49 [
    i8 125, label %42
    i8 34, label %44
  ]

42:                                               ; preds = %38
  %43 = add nuw i32 %35, 1
  store i32 %43, ptr %5, align 4, !tbaa !378
  tail call fastcc void @json_collect(ptr noundef nonnull %0, i32 noundef %34, ptr noundef nonnull %1) #61
  br label %159

44:                                               ; preds = %38
  %45 = getelementptr inbounds %struct.FaJson, ptr %4, i32 0, i32 6
  %46 = getelementptr inbounds %struct.FaJson, ptr %4, i32 0, i32 7
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %4) #64
  call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(48) %4, i8 0, i32 48, i1 false)
  %47 = call fastcc i32 @json_string(ptr noundef nonnull %0, ptr noundef nonnull %45, ptr noundef nonnull %46) #61
  %48 = icmp eq i32 %47, 0
  br i1 %48, label %104, label %54

49:                                               ; preds = %38, %33
  %50 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 8
  store ptr @.str.66, ptr %50, align 4, !tbaa !379
  %51 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 9
  store ptr null, ptr %51, align 4, !tbaa !380
  %52 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 10
  store i32 0, ptr %52, align 4, !tbaa !381
  %53 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 11
  store i8 0, ptr %53, align 4, !tbaa !382
  br label %159

54:                                               ; preds = %44, %106
  tail call fastcc void @json_space(ptr noundef nonnull %0) #61
  %55 = load i32, ptr %5, align 4, !tbaa !378
  %56 = load i32, ptr %7, align 4, !tbaa !376
  %57 = icmp ult i32 %55, %56
  br i1 %57, label %58, label %63

58:                                               ; preds = %54
  %59 = load ptr, ptr %0, align 4, !tbaa !374
  %60 = getelementptr inbounds i8, ptr %59, i32 %55
  %61 = load i8, ptr %60, align 1, !tbaa !1
  %62 = icmp eq i8 %61, 58
  br i1 %62, label %68, label %63

63:                                               ; preds = %58, %54
  %64 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 8
  store ptr @.str.67, ptr %64, align 4, !tbaa !379
  %65 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 9
  store ptr null, ptr %65, align 4, !tbaa !380
  %66 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 10
  store i32 0, ptr %66, align 4, !tbaa !381
  %67 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 11
  store i8 0, ptr %67, align 4, !tbaa !382
  br label %104

68:                                               ; preds = %58
  %69 = add nuw i32 %55, 1
  store i32 %69, ptr %5, align 4, !tbaa !378
  %70 = load ptr, ptr %45, align 8, !tbaa !392
  %71 = load i32, ptr %46, align 4, !tbaa !393
  %72 = call fastcc i32 @json_value(ptr noundef nonnull %0, ptr noundef nonnull %4) #61
  %73 = icmp eq i32 %72, 0
  br i1 %73, label %104, label %74

74:                                               ; preds = %68
  store ptr %70, ptr %45, align 8, !tbaa !392
  store i32 %71, ptr %46, align 4, !tbaa !393
  call fastcc void @json_push(ptr noundef nonnull %0, ptr noundef nonnull %4) #61
  tail call fastcc void @json_space(ptr noundef nonnull %0) #61
  %75 = load i32, ptr %5, align 4, !tbaa !378
  %76 = load i32, ptr %7, align 4, !tbaa !376
  %77 = icmp ult i32 %75, %76
  br i1 %77, label %78, label %99

78:                                               ; preds = %74
  %79 = load ptr, ptr %0, align 4, !tbaa !374
  %80 = getelementptr inbounds i8, ptr %79, i32 %75
  %81 = load i8, ptr %80, align 1, !tbaa !1
  switch i8 %81, label %99 [
    i8 44, label %82
    i8 125, label %97
  ]

82:                                               ; preds = %78
  %83 = add nuw i32 %75, 1
  store i32 %83, ptr %5, align 4, !tbaa !378
  tail call fastcc void @json_space(ptr noundef nonnull %0) #61
  %84 = load i32, ptr %5, align 4, !tbaa !378
  %85 = load i32, ptr %7, align 4, !tbaa !376
  %86 = icmp ult i32 %84, %85
  br i1 %86, label %87, label %92

87:                                               ; preds = %82
  %88 = load ptr, ptr %0, align 4, !tbaa !374
  %89 = getelementptr inbounds i8, ptr %88, i32 %84
  %90 = load i8, ptr %89, align 1, !tbaa !1
  %91 = icmp eq i8 %90, 34
  br i1 %91, label %106, label %92

92:                                               ; preds = %87, %82
  %93 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 8
  store ptr @.str.68, ptr %93, align 4, !tbaa !379
  %94 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 9
  store ptr null, ptr %94, align 4, !tbaa !380
  %95 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 10
  store i32 0, ptr %95, align 4, !tbaa !381
  %96 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 11
  store i8 0, ptr %96, align 4, !tbaa !382
  br label %104

97:                                               ; preds = %78
  %98 = add nuw i32 %75, 1
  store i32 %98, ptr %5, align 4, !tbaa !378
  tail call fastcc void @json_collect(ptr noundef nonnull %0, i32 noundef %34, ptr noundef nonnull %1) #61
  tail call fastcc void @json_order(ptr noundef nonnull %0, ptr noundef nonnull %1) #61
  br label %104

99:                                               ; preds = %74, %78
  %100 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 8
  store ptr @.str.66, ptr %100, align 4, !tbaa !379
  %101 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 9
  store ptr null, ptr %101, align 4, !tbaa !380
  %102 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 10
  store i32 0, ptr %102, align 4, !tbaa !381
  %103 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 11
  store i8 0, ptr %103, align 4, !tbaa !382
  br label %104

104:                                              ; preds = %68, %106, %44, %63, %92, %97, %99
  %105 = phi i32 [ 0, %99 ], [ 1, %97 ], [ 0, %92 ], [ 0, %63 ], [ 0, %44 ], [ 0, %106 ], [ 0, %68 ]
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %4) #64
  br label %159

106:                                              ; preds = %87
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %4) #64
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %4) #64
  call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(48) %4, i8 0, i32 48, i1 false)
  %107 = call fastcc i32 @json_string(ptr noundef nonnull %0, ptr noundef nonnull %45, ptr noundef nonnull %46) #61
  %108 = icmp eq i32 %107, 0
  br i1 %108, label %104, label %54

109:                                              ; preds = %29
  store i32 4, ptr %1, align 8, !tbaa !390
  %110 = load i32, ptr %32, align 4, !tbaa !391
  tail call fastcc void @json_space(ptr noundef nonnull %0) #61
  %111 = load i32, ptr %5, align 4, !tbaa !378
  %112 = load i32, ptr %7, align 4, !tbaa !376
  %113 = icmp ult i32 %111, %112
  br i1 %113, label %114, label %121

114:                                              ; preds = %109
  %115 = load ptr, ptr %0, align 4, !tbaa !374
  %116 = getelementptr inbounds i8, ptr %115, i32 %111
  %117 = load i8, ptr %116, align 1, !tbaa !1
  %118 = icmp eq i8 %117, 93
  br i1 %118, label %119, label %121

119:                                              ; preds = %114
  %120 = add nuw i32 %111, 1
  store i32 %120, ptr %5, align 4, !tbaa !378
  tail call fastcc void @json_collect(ptr noundef nonnull %0, i32 noundef %110, ptr noundef nonnull %1) #61
  br label %159

121:                                              ; preds = %114, %109
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %3) #64
  call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(48) %3, i8 0, i32 48, i1 false)
  %122 = call fastcc i32 @json_value(ptr noundef nonnull %0, ptr noundef nonnull %3) #61
  %123 = icmp eq i32 %122, 0
  br i1 %123, label %154, label %124

124:                                              ; preds = %121, %156
  call fastcc void @json_push(ptr noundef nonnull %0, ptr noundef nonnull %3) #61
  tail call fastcc void @json_space(ptr noundef nonnull %0) #61
  %125 = load i32, ptr %5, align 4, !tbaa !378
  %126 = load i32, ptr %7, align 4, !tbaa !376
  %127 = icmp ult i32 %125, %126
  br i1 %127, label %128, label %149

128:                                              ; preds = %124
  %129 = load ptr, ptr %0, align 4, !tbaa !374
  %130 = getelementptr inbounds i8, ptr %129, i32 %125
  %131 = load i8, ptr %130, align 1, !tbaa !1
  switch i8 %131, label %149 [
    i8 44, label %132
    i8 93, label %147
  ]

132:                                              ; preds = %128
  %133 = add nuw i32 %125, 1
  store i32 %133, ptr %5, align 4, !tbaa !378
  tail call fastcc void @json_space(ptr noundef nonnull %0) #61
  %134 = load i32, ptr %5, align 4, !tbaa !378
  %135 = load i32, ptr %7, align 4, !tbaa !376
  %136 = icmp ult i32 %134, %135
  br i1 %136, label %137, label %156

137:                                              ; preds = %132
  %138 = load ptr, ptr %0, align 4, !tbaa !374
  %139 = getelementptr inbounds i8, ptr %138, i32 %134
  %140 = load i8, ptr %139, align 1, !tbaa !1
  %141 = icmp eq i8 %140, 93
  br i1 %141, label %142, label %156

142:                                              ; preds = %137
  %143 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 8
  store ptr @.str.69, ptr %143, align 4, !tbaa !379
  %144 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 9
  store ptr null, ptr %144, align 4, !tbaa !380
  %145 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 10
  store i32 0, ptr %145, align 4, !tbaa !381
  %146 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 11
  store i8 0, ptr %146, align 4, !tbaa !382
  br label %154

147:                                              ; preds = %128
  %148 = add nuw i32 %125, 1
  store i32 %148, ptr %5, align 4, !tbaa !378
  tail call fastcc void @json_collect(ptr noundef nonnull %0, i32 noundef %110, ptr noundef nonnull %1) #61
  br label %154

149:                                              ; preds = %124, %128
  %150 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 8
  store ptr @.str.70, ptr %150, align 4, !tbaa !379
  %151 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 9
  store ptr null, ptr %151, align 4, !tbaa !380
  %152 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 10
  store i32 0, ptr %152, align 4, !tbaa !381
  %153 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 11
  store i8 0, ptr %153, align 4, !tbaa !382
  br label %154

154:                                              ; preds = %156, %121, %142, %147, %149
  %155 = phi i32 [ 0, %149 ], [ 1, %147 ], [ 0, %142 ], [ 0, %121 ], [ 0, %156 ]
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %3) #64
  br label %159

156:                                              ; preds = %137, %132
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %3) #64
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %3) #64
  call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(48) %3, i8 0, i32 48, i1 false)
  %157 = call fastcc i32 @json_value(ptr noundef nonnull %0, ptr noundef nonnull %3) #61
  %158 = icmp eq i32 %157, 0
  br i1 %158, label %154, label %124

159:                                              ; preds = %119, %154, %49, %42, %104
  %160 = phi i32 [ 1, %42 ], [ 0, %49 ], [ %105, %104 ], [ 1, %119 ], [ %155, %154 ]
  %161 = load i32, ptr %21, align 4, !tbaa !389
  %162 = add nsw i32 %161, -1
  store i32 %162, ptr %21, align 4, !tbaa !389
  br label %383

163:                                              ; preds = %15
  store i32 3, ptr %1, align 8, !tbaa !390
  %164 = getelementptr inbounds %struct.FaJson, ptr %1, i32 0, i32 4
  %165 = getelementptr inbounds %struct.FaJson, ptr %1, i32 0, i32 5
  %166 = tail call fastcc i32 @json_string(ptr noundef nonnull %0, ptr noundef nonnull %164, ptr noundef nonnull %165) #61
  br label %383

167:                                              ; preds = %15
  %168 = add i8 %18, -48
  %169 = icmp ult i8 %168, 10
  br i1 %169, label %170, label %317

170:                                              ; preds = %15, %167
  %171 = icmp eq i8 %18, 45
  %172 = zext i1 %171 to i32
  %173 = add nuw i32 %6, %172
  %174 = icmp ult i32 %173, %8
  br i1 %174, label %175, label %180

175:                                              ; preds = %170
  %176 = getelementptr inbounds i8, ptr %16, i32 %173
  %177 = load i8, ptr %176, align 1, !tbaa !1
  %178 = add i8 %177, -58
  %179 = icmp ult i8 %178, -10
  br i1 %179, label %180, label %185

180:                                              ; preds = %175, %170
  %181 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 8
  store ptr @.str.74, ptr %181, align 4, !tbaa !379
  %182 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 9
  store ptr null, ptr %182, align 4, !tbaa !380
  %183 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 10
  store i32 0, ptr %183, align 4, !tbaa !381
  %184 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 11
  store i8 0, ptr %184, align 4, !tbaa !382
  br label %383

185:                                              ; preds = %175
  %186 = icmp eq i8 %177, 48
  br i1 %186, label %187, label %189

187:                                              ; preds = %185
  %188 = add nuw i32 %173, 1
  br label %214

189:                                              ; preds = %185, %197
  %190 = phi i32 [ %207, %197 ], [ %173, %185 ]
  %191 = phi i32 [ %206, %197 ], [ 1, %185 ]
  %192 = phi i64 [ %205, %197 ], [ 0, %185 ]
  %193 = getelementptr inbounds i8, ptr %16, i32 %190
  %194 = load i8, ptr %193, align 1, !tbaa !1
  %195 = add i8 %194, -58
  %196 = icmp ult i8 %195, -10
  br i1 %196, label %209, label %197

197:                                              ; preds = %189
  %198 = zext nneg i8 %194 to i64
  %199 = sub nsw i64 47, %198
  %200 = udiv i64 %199, 10
  %201 = icmp ugt i64 %192, %200
  %202 = mul nuw i64 %192, 10
  %203 = add i64 %202, -48
  %204 = add i64 %203, %198
  %205 = select i1 %201, i64 %192, i64 %204
  %206 = select i1 %201, i32 0, i32 %191
  %207 = add nuw i32 %190, 1
  %208 = icmp ult i32 %207, %8
  br i1 %208, label %189, label %209, !llvm.loop !394

209:                                              ; preds = %197, %189
  %210 = phi i64 [ %192, %189 ], [ %205, %197 ]
  %211 = phi i32 [ %191, %189 ], [ %206, %197 ]
  %212 = phi i32 [ %190, %189 ], [ %8, %197 ]
  %213 = icmp eq i32 %211, 0
  br label %214

214:                                              ; preds = %209, %187
  %215 = phi i64 [ 0, %187 ], [ %210, %209 ]
  %216 = phi i1 [ false, %187 ], [ %213, %209 ]
  %217 = phi i32 [ %188, %187 ], [ %212, %209 ]
  %218 = icmp ult i32 %217, %8
  br i1 %218, label %219, label %245

219:                                              ; preds = %214
  %220 = getelementptr inbounds i8, ptr %16, i32 %217
  %221 = load i8, ptr %220, align 1, !tbaa !1
  %222 = icmp eq i8 %221, 46
  br i1 %222, label %223, label %245

223:                                              ; preds = %219
  %224 = add nuw i32 %217, 1
  %225 = icmp ult i32 %224, %8
  br i1 %225, label %226, label %231

226:                                              ; preds = %223
  %227 = getelementptr inbounds i8, ptr %16, i32 %224
  %228 = load i8, ptr %227, align 1, !tbaa !1
  %229 = add i8 %228, -58
  %230 = icmp ult i8 %229, -10
  br i1 %230, label %231, label %236

231:                                              ; preds = %226, %223
  %232 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 8
  store ptr @.str.75, ptr %232, align 4, !tbaa !379
  %233 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 9
  store ptr null, ptr %233, align 4, !tbaa !380
  %234 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 10
  store i32 0, ptr %234, align 4, !tbaa !381
  %235 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 11
  store i8 0, ptr %235, align 4, !tbaa !382
  br label %383

236:                                              ; preds = %226, %242
  %237 = phi i32 [ %243, %242 ], [ %224, %226 ]
  %238 = getelementptr inbounds i8, ptr %16, i32 %237
  %239 = load i8, ptr %238, align 1, !tbaa !1
  %240 = add i8 %239, -58
  %241 = icmp ult i8 %240, -10
  br i1 %241, label %245, label %242

242:                                              ; preds = %236
  %243 = add i32 %237, 1
  %244 = icmp eq i32 %243, %8
  br i1 %244, label %282, label %236, !llvm.loop !395

245:                                              ; preds = %236, %219, %214
  %246 = phi i32 [ 1, %219 ], [ 1, %214 ], [ 0, %236 ]
  %247 = phi i32 [ %217, %219 ], [ %217, %214 ], [ %237, %236 ]
  %248 = icmp ult i32 %247, %8
  br i1 %248, label %249, label %282

249:                                              ; preds = %245
  %250 = getelementptr inbounds i8, ptr %16, i32 %247
  %251 = load i8, ptr %250, align 1, !tbaa !1
  switch i8 %251, label %282 [
    i8 101, label %252
    i8 69, label %252
  ]

252:                                              ; preds = %249, %249
  %253 = add nuw i32 %247, 1
  %254 = icmp ult i32 %253, %8
  br i1 %254, label %255, label %260

255:                                              ; preds = %252
  %256 = getelementptr inbounds i8, ptr %16, i32 %253
  %257 = load i8, ptr %256, align 1, !tbaa !1
  switch i8 %257, label %260 [
    i8 43, label %258
    i8 45, label %258
  ]

258:                                              ; preds = %255, %255
  %259 = add nuw i32 %247, 2
  br label %260

260:                                              ; preds = %258, %255, %252
  %261 = phi i32 [ %259, %258 ], [ %253, %252 ], [ %253, %255 ]
  %262 = icmp ult i32 %261, %8
  br i1 %262, label %263, label %268

263:                                              ; preds = %260
  %264 = getelementptr inbounds i8, ptr %16, i32 %261
  %265 = load i8, ptr %264, align 1, !tbaa !1
  %266 = add i8 %265, -58
  %267 = icmp ult i8 %266, -10
  br i1 %267, label %268, label %273

268:                                              ; preds = %263, %260
  %269 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 8
  store ptr @.str.31, ptr %269, align 4, !tbaa !379
  %270 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 9
  store ptr null, ptr %270, align 4, !tbaa !380
  %271 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 10
  store i32 0, ptr %271, align 4, !tbaa !381
  %272 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 11
  store i8 0, ptr %272, align 4, !tbaa !382
  br label %383

273:                                              ; preds = %263, %279
  %274 = phi i32 [ %280, %279 ], [ %261, %263 ]
  %275 = getelementptr inbounds i8, ptr %16, i32 %274
  %276 = load i8, ptr %275, align 1, !tbaa !1
  %277 = add i8 %276, -58
  %278 = icmp ult i8 %277, -10
  br i1 %278, label %282, label %279

279:                                              ; preds = %273
  %280 = add i32 %274, 1
  %281 = icmp eq i32 %280, %8
  br i1 %281, label %282, label %273, !llvm.loop !396

282:                                              ; preds = %242, %279, %273, %249, %245
  %283 = phi i32 [ %246, %245 ], [ %246, %249 ], [ 0, %273 ], [ 0, %279 ], [ 0, %242 ]
  %284 = phi i32 [ %247, %245 ], [ %247, %249 ], [ %8, %279 ], [ %274, %273 ], [ %8, %242 ]
  store i32 %284, ptr %5, align 4, !tbaa !378
  store i32 2, ptr %1, align 8, !tbaa !390
  %285 = icmp eq i32 %283, 0
  %286 = select i1 %285, i1 true, i1 %216
  %287 = select i1 %171, i64 -9223372036854775808, i64 9223372036854775807
  %288 = icmp ugt i64 %215, %287
  %289 = select i1 %286, i1 true, i1 %288
  %290 = getelementptr inbounds %struct.FaJson, ptr %1, i32 0, i32 1
  br i1 %289, label %300, label %291

291:                                              ; preds = %282
  %292 = sub i64 0, %215
  %293 = select i1 %171, i64 %292, i64 %215
  %294 = getelementptr inbounds %struct.FaJson, ptr %1, i32 0, i32 2
  store i64 %293, ptr %294, align 8, !tbaa !397
  store i32 3, ptr %290, align 4, !tbaa !398
  %295 = icmp eq i64 %215, 0
  %296 = select i1 %171, i1 %295, i1 false
  %297 = sitofp i64 %293 to double
  %298 = select i1 %296, double -0.000000e+00, double %297
  %299 = getelementptr inbounds %struct.FaJson, ptr %1, i32 0, i32 3
  store double %298, ptr %299, align 8, !tbaa !399
  br label %383

300:                                              ; preds = %282
  %301 = sub i32 %284, %6
  %302 = tail call double @fa_host_parse_number(ptr noundef nonnull %17, i32 noundef %301) #62
  %303 = getelementptr inbounds %struct.FaJson, ptr %1, i32 0, i32 3
  store double %302, ptr %303, align 8, !tbaa !399
  store i32 0, ptr %290, align 4, !tbaa !398
  %304 = fsub double %302, %302
  %305 = fcmp oeq double %304, 0.000000e+00
  br i1 %305, label %306, label %383

306:                                              ; preds = %300
  %307 = fcmp oge double %302, 0xC3E0000000000000
  %308 = fcmp olt double %302, 0x43E0000000000000
  %309 = and i1 %307, %308
  br i1 %309, label %310, label %316

310:                                              ; preds = %306
  %311 = fptosi double %302 to i64
  %312 = sitofp i64 %311 to double
  %313 = fcmp oeq double %302, %312
  br i1 %313, label %314, label %383

314:                                              ; preds = %310
  %315 = getelementptr inbounds %struct.FaJson, ptr %1, i32 0, i32 2
  store i64 %311, ptr %315, align 8, !tbaa !397
  store i32 3, ptr %290, align 4, !tbaa !398
  br label %383

316:                                              ; preds = %306
  store i32 1, ptr %290, align 4, !tbaa !398
  br label %383

317:                                              ; preds = %167
  switch i8 %18, label %323 [
    i8 125, label %318
    i8 93, label %318
    i8 58, label %318
    i8 46, label %318
    i8 44, label %318
  ]

318:                                              ; preds = %317, %317, %317, %317, %317
  %319 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 8
  store ptr @.str.63, ptr %319, align 4, !tbaa !379
  %320 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 9
  store ptr %17, ptr %320, align 4, !tbaa !380
  %321 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 10
  store i32 1, ptr %321, align 4, !tbaa !381
  %322 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 11
  store i8 39, ptr %322, align 4, !tbaa !382
  br label %383

323:                                              ; preds = %317
  %324 = and i8 %18, -33
  %325 = add i8 %324, -65
  %326 = icmp ult i8 %325, 26
  br i1 %326, label %328, label %327

327:                                              ; preds = %323
  switch i8 %18, label %366 [
    i8 95, label %328
    i8 36, label %328
  ]

328:                                              ; preds = %327, %327, %323
  br label %329

329:                                              ; preds = %328, %341
  %330 = phi i32 [ %342, %341 ], [ %6, %328 ]
  %331 = getelementptr inbounds i8, ptr %16, i32 %330
  %332 = load i8, ptr %331, align 1, !tbaa !1
  %333 = freeze i8 %332
  %334 = and i8 %333, -33
  %335 = add i8 %334, -65
  %336 = icmp ult i8 %335, 26
  %337 = add i8 %333, -48
  %338 = icmp ult i8 %337, 10
  %339 = or i1 %336, %338
  br i1 %339, label %341, label %340

340:                                              ; preds = %329
  switch i8 %333, label %344 [
    i8 95, label %341
    i8 36, label %341
  ]

341:                                              ; preds = %340, %340, %329
  %342 = add i32 %330, 1
  %343 = icmp eq i32 %342, %8
  br i1 %343, label %344, label %329, !llvm.loop !400

344:                                              ; preds = %341, %340
  %345 = phi i32 [ %8, %341 ], [ %330, %340 ]
  %346 = sub i32 %345, %6
  %347 = tail call fastcc i32 @json_same(ptr noundef nonnull %17, i32 noundef %346, ptr noundef nonnull @.str) #61, !range !50
  %348 = icmp eq i32 %347, 0
  br i1 %348, label %349, label %352

349:                                              ; preds = %344
  %350 = tail call fastcc i32 @json_same(ptr noundef nonnull %17, i32 noundef %346, ptr noundef nonnull @.str.1) #61, !range !50
  %351 = icmp eq i32 %350, 0
  br i1 %351, label %356, label %352

352:                                              ; preds = %349, %344
  store i32 1, ptr %1, align 8, !tbaa !390
  %353 = icmp eq i32 %346, 4
  %354 = zext i1 %353 to i32
  %355 = getelementptr inbounds %struct.FaJson, ptr %1, i32 0, i32 1
  store i32 %354, ptr %355, align 4, !tbaa !398
  br label %365

356:                                              ; preds = %349
  %357 = tail call fastcc i32 @json_same(ptr noundef nonnull %17, i32 noundef %346, ptr noundef nonnull @.str.36) #61, !range !50
  %358 = icmp eq i32 %357, 0
  br i1 %358, label %360, label %359

359:                                              ; preds = %356
  store i32 0, ptr %1, align 8, !tbaa !390
  br label %365

360:                                              ; preds = %356
  %361 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 8
  store ptr @.str.64, ptr %361, align 4, !tbaa !379
  %362 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 9
  store ptr %17, ptr %362, align 4, !tbaa !380
  %363 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 10
  store i32 %346, ptr %363, align 4, !tbaa !381
  %364 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 11
  store i8 34, ptr %364, align 4, !tbaa !382
  br label %383

365:                                              ; preds = %359, %352
  store i32 %345, ptr %5, align 4, !tbaa !378
  br label %383

366:                                              ; preds = %327
  %367 = icmp ugt i8 %18, -17
  br i1 %367, label %373, label %368

368:                                              ; preds = %366
  %369 = icmp ugt i8 %18, -33
  br i1 %369, label %373, label %370

370:                                              ; preds = %368
  %371 = icmp ugt i8 %18, -65
  %372 = select i1 %371, i32 2, i32 1
  br label %373

373:                                              ; preds = %370, %368, %366
  %374 = phi i32 [ 4, %366 ], [ 3, %368 ], [ %372, %370 ]
  %375 = add i32 %374, %6
  %376 = icmp ugt i32 %375, %8
  %377 = sub i32 %8, %6
  %378 = select i1 %376, i32 %377, i32 %374
  %379 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 8
  store ptr @.str.65, ptr %379, align 4, !tbaa !379
  %380 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 9
  store ptr %17, ptr %380, align 4, !tbaa !380
  %381 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 10
  store i32 %378, ptr %381, align 4, !tbaa !381
  %382 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 11
  store i8 39, ptr %382, align 4, !tbaa !382
  br label %383

383:                                              ; preds = %316, %314, %310, %300, %291, %268, %231, %180, %24, %159, %163, %318, %373, %365, %360, %10
  %384 = phi i32 [ 0, %10 ], [ 0, %24 ], [ %160, %159 ], [ %166, %163 ], [ 0, %318 ], [ 0, %373 ], [ 1, %365 ], [ 0, %360 ], [ 0, %180 ], [ 0, %231 ], [ 0, %268 ], [ 1, %300 ], [ 1, %316 ], [ 1, %314 ], [ 1, %310 ], [ 1, %291 ]
  ret i32 %384
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(read, argmem: readwrite, inaccessiblemem: none)
define internal fastcc void @json_space(ptr nocapture noundef %0) unnamed_addr #14 {
  %2 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 2
  %3 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 1
  %4 = load i32, ptr %3, align 4, !tbaa !376
  %5 = load i32, ptr %2, align 4, !tbaa !378
  %6 = icmp ult i32 %5, %4
  br i1 %6, label %7, label %17

7:                                                ; preds = %1
  %8 = load ptr, ptr %0, align 4, !tbaa !374
  br label %9

9:                                                ; preds = %7, %15
  %10 = phi i32 [ %5, %7 ], [ %14, %15 ]
  %11 = getelementptr inbounds i8, ptr %8, i32 %10
  %12 = load i8, ptr %11, align 1, !tbaa !1
  switch i8 %12, label %17 [
    i8 32, label %13
    i8 13, label %13
    i8 10, label %13
    i8 9, label %13
  ]

13:                                               ; preds = %9, %9, %9, %9
  %14 = add i32 %10, 1
  store i32 %14, ptr %2, align 4, !tbaa !378
  switch i8 %12, label %17 [
    i8 32, label %15
    i8 13, label %15
    i8 10, label %15
    i8 9, label %15
  ]

15:                                               ; preds = %13, %13, %13, %13
  %16 = icmp eq i32 %14, %4
  br i1 %16, label %17, label %9

17:                                               ; preds = %15, %13, %9, %1
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @json_path(ptr nocapture noundef %0, ptr nocapture noundef readonly %1, i32 noundef %2) unnamed_addr #3 {
  %4 = getelementptr inbounds %struct.JsonCheck, ptr %0, i32 0, i32 1
  %5 = load i32, ptr %4, align 4, !tbaa !401
  %6 = add i32 %5, %2
  %7 = getelementptr inbounds %struct.JsonCheck, ptr %0, i32 0, i32 2
  %8 = load i32, ptr %7, align 4, !tbaa !385
  %9 = icmp ugt i32 %6, %8
  br i1 %9, label %12, label %10

10:                                               ; preds = %3
  %11 = load ptr, ptr %0, align 4, !tbaa !387
  br label %23

12:                                               ; preds = %3, %12
  %13 = phi i32 [ %14, %12 ], [ %8, %3 ]
  %14 = shl i32 %13, 1
  %15 = icmp ult i32 %14, %6
  br i1 %15, label %12, label %16, !llvm.loop !402

16:                                               ; preds = %12
  %17 = tail call ptr @fa_alloc(i32 noundef %14, i32 noundef 1) #62
  %18 = load ptr, ptr %0, align 4, !tbaa !387
  %19 = load i32, ptr %4, align 4, !tbaa !401
  tail call fastcc void @copy_bytes(ptr noundef %17, ptr noundef %18, i32 noundef %19) #61
  %20 = load ptr, ptr %0, align 4, !tbaa !387
  %21 = load i32, ptr %7, align 4, !tbaa !385
  tail call void @fa_free(ptr noundef %20, i32 noundef %21, i32 noundef 1) #62
  store ptr %17, ptr %0, align 4, !tbaa !387
  store i32 %14, ptr %7, align 4, !tbaa !385
  %22 = load i32, ptr %4, align 4, !tbaa !401
  br label %23

23:                                               ; preds = %10, %16
  %24 = phi i32 [ %5, %10 ], [ %22, %16 ]
  %25 = phi ptr [ %11, %10 ], [ %17, %16 ]
  %26 = getelementptr inbounds i8, ptr %25, i32 %24
  tail call fastcc void @copy_bytes(ptr noundef %26, ptr noundef %1, i32 noundef %2) #61
  %27 = load i32, ptr %4, align 4, !tbaa !401
  %28 = add i32 %27, %2
  store i32 %28, ptr %4, align 4, !tbaa !401
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc i32 @json_check(ptr noundef %0, ptr nocapture noundef readonly %1, ptr noundef %2, i32 noundef %3) unnamed_addr #3 {
  %5 = alloca i32, align 4
  %6 = alloca ptr, align 4
  %7 = alloca i32, align 4
  %8 = alloca i32, align 4
  %9 = alloca ptr, align 4
  %10 = alloca ptr, align 4
  %11 = alloca [24 x i8], align 16
  %12 = alloca ptr, align 4
  %13 = alloca ptr, align 4
  %14 = alloca ptr, align 4
  %15 = alloca ptr, align 4
  %16 = alloca ptr, align 4
  %17 = alloca %struct.JsonMember, align 4
  %18 = alloca ptr, align 4
  %19 = alloca %struct.JsonMember, align 4
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %5) #64
  %20 = load i8, ptr %2, align 1, !tbaa !1
  switch i8 %20, label %450 [
    i8 98, label %21
    i8 115, label %27
    i8 102, label %33
    i8 110, label %39
    i8 105, label %45
    i8 83, label %102
    i8 84, label %134
    i8 70, label %134
    i8 73, label %144
    i8 97, label %161
    i8 109, label %246
    i8 111, label %275
    i8 117, label %319
    i8 69, label %337
    i8 78, label %375
  ]

21:                                               ; preds = %4
  %22 = load i32, ptr %1, align 8, !tbaa !390
  %23 = icmp eq i32 %22, 1
  br i1 %23, label %25, label %24

24:                                               ; preds = %21
  tail call fastcc void @json_mismatch(ptr noundef %0, ptr noundef nonnull %1, ptr noundef nonnull @.str.77, ptr noundef null, i32 noundef %3) #61
  br label %25

25:                                               ; preds = %24, %21
  %26 = zext i1 %23 to i32
  br label %456

27:                                               ; preds = %4
  %28 = load i32, ptr %1, align 8, !tbaa !390
  %29 = icmp eq i32 %28, 3
  br i1 %29, label %31, label %30

30:                                               ; preds = %27
  tail call fastcc void @json_mismatch(ptr noundef %0, ptr noundef nonnull %1, ptr noundef nonnull @.str.78, ptr noundef null, i32 noundef %3) #61
  br label %31

31:                                               ; preds = %30, %27
  %32 = zext i1 %29 to i32
  br label %456

33:                                               ; preds = %4
  %34 = load i32, ptr %1, align 8, !tbaa !390
  %35 = icmp eq i32 %34, 2
  br i1 %35, label %37, label %36

36:                                               ; preds = %33
  tail call fastcc void @json_mismatch(ptr noundef %0, ptr noundef nonnull %1, ptr noundef nonnull @.str.79, ptr noundef null, i32 noundef %3) #61
  br label %37

37:                                               ; preds = %36, %33
  %38 = zext i1 %35 to i32
  br label %456

39:                                               ; preds = %4
  %40 = load i32, ptr %1, align 8, !tbaa !390
  %41 = icmp eq i32 %40, 0
  br i1 %41, label %43, label %42

42:                                               ; preds = %39
  tail call fastcc void @json_mismatch(ptr noundef %0, ptr noundef nonnull %1, ptr noundef nonnull @.str.36, ptr noundef null, i32 noundef %3) #61
  br label %43

43:                                               ; preds = %42, %39
  %44 = zext i1 %41 to i32
  br label %456

45:                                               ; preds = %4
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %6) #64
  %46 = getelementptr inbounds i8, ptr %2, i32 1
  store ptr %46, ptr %6, align 4, !tbaa !6
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %7) #64
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %8) #64
  %47 = call fastcc i64 @json_schema_number(ptr noundef nonnull %6, ptr noundef nonnull %7) #61
  %48 = call fastcc i64 @json_schema_number(ptr noundef nonnull %6, ptr noundef nonnull %8) #61
  %49 = load i32, ptr %1, align 8, !tbaa !390
  %50 = icmp eq i32 %49, 2
  br i1 %50, label %51, label %56

51:                                               ; preds = %45
  %52 = getelementptr inbounds %struct.FaJson, ptr %1, i32 0, i32 1
  %53 = load i32, ptr %52, align 4, !tbaa !398
  %54 = and i32 %53, 1
  %55 = icmp eq i32 %54, 0
  br i1 %55, label %56, label %57

56:                                               ; preds = %51, %45
  tail call fastcc void @json_mismatch(ptr noundef %0, ptr noundef nonnull %1, ptr noundef nonnull @.str.80, ptr noundef null, i32 noundef %3) #61
  br label %100

57:                                               ; preds = %51
  %58 = and i32 %53, 2
  %59 = icmp eq i32 %58, 0
  br i1 %59, label %60, label %63

60:                                               ; preds = %57
  %61 = icmp eq i32 %3, 0
  br i1 %61, label %100, label %62

62:                                               ; preds = %60
  tail call fastcc void @json_error_start(ptr noundef %0) #61
  tail call fastcc void @append_c(ptr noundef nonnull @.str.81) #61
  tail call fastcc void @json_error_end(ptr noundef %0) #61
  br label %100

63:                                               ; preds = %57
  %64 = load i32, ptr %7, align 4, !tbaa !23
  %65 = icmp eq i32 %64, 0
  br i1 %65, label %70, label %66

66:                                               ; preds = %63
  %67 = getelementptr inbounds %struct.FaJson, ptr %1, i32 0, i32 2
  %68 = load i64, ptr %67, align 8, !tbaa !397
  %69 = icmp slt i64 %68, %47
  br i1 %69, label %79, label %70

70:                                               ; preds = %66, %63
  %71 = load i32, ptr %8, align 4, !tbaa !23
  %72 = icmp eq i32 %71, 0
  br i1 %72, label %100, label %73

73:                                               ; preds = %70
  %74 = getelementptr inbounds %struct.FaJson, ptr %1, i32 0, i32 2
  %75 = load i64, ptr %74, align 8, !tbaa !397
  %76 = icmp sgt i64 %75, %48
  br i1 %76, label %77, label %100

77:                                               ; preds = %73
  %78 = icmp eq i32 %3, 0
  br i1 %78, label %100, label %81

79:                                               ; preds = %66
  %80 = icmp eq i32 %3, 0
  br i1 %80, label %100, label %83

81:                                               ; preds = %77
  br i1 %65, label %82, label %83

82:                                               ; preds = %81
  tail call fastcc void @json_error_start(ptr noundef %0) #61
  br label %89

83:                                               ; preds = %79, %81
  %84 = phi i64 [ %68, %79 ], [ %75, %81 ]
  %85 = icmp slt i64 %84, %47
  %86 = freeze i1 %85
  tail call fastcc void @json_error_start(ptr noundef %0) #61
  %87 = select i1 %86, ptr @.str.82, ptr @.str.83
  %88 = select i1 %86, i64 %47, i64 %48
  br label %89

89:                                               ; preds = %83, %82
  %90 = phi ptr [ @.str.83, %82 ], [ %87, %83 ]
  %91 = phi i64 [ %48, %82 ], [ %88, %83 ]
  tail call fastcc void @append_c(ptr noundef nonnull %90) #61
  %92 = tail call i64 @llvm.abs.i64(i64 %91, i1 false)
  %93 = lshr i64 %91, 63
  %94 = trunc i64 %93 to i32
  tail call fastcc void @append_u64(i64 noundef %92, i32 noundef %94) #61
  tail call fastcc void @append_c(ptr noundef nonnull @.str.84) #61
  %95 = getelementptr inbounds %struct.FaJson, ptr %1, i32 0, i32 2
  %96 = load i64, ptr %95, align 8, !tbaa !397
  %97 = tail call i64 @llvm.abs.i64(i64 %96, i1 false)
  %98 = lshr i64 %96, 63
  %99 = trunc i64 %98 to i32
  tail call fastcc void @append_u64(i64 noundef %97, i32 noundef %99) #61
  tail call fastcc void @json_error_end(ptr noundef %0) #61
  br label %100

100:                                              ; preds = %79, %70, %73, %77, %89, %60, %62, %56
  %101 = phi i32 [ 0, %56 ], [ 0, %62 ], [ 0, %60 ], [ 0, %89 ], [ 0, %77 ], [ 1, %73 ], [ 1, %70 ], [ 0, %79 ]
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %8) #64
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %7) #64
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %6) #64
  br label %456

102:                                              ; preds = %4
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %9) #64
  %103 = getelementptr inbounds i8, ptr %2, i32 1
  store ptr %103, ptr %9, align 4, !tbaa !6
  %104 = call fastcc i64 @json_schema_number(ptr noundef nonnull %9, ptr noundef nonnull %5) #61
  %105 = load i32, ptr %1, align 8, !tbaa !390
  %106 = icmp eq i32 %105, 3
  br i1 %106, label %107, label %131

107:                                              ; preds = %102
  %108 = getelementptr inbounds %struct.FaJson, ptr %1, i32 0, i32 5
  %109 = load i32, ptr %108, align 4, !tbaa !403
  %110 = trunc i64 %104 to i32
  %111 = icmp eq i32 %109, %110
  br i1 %111, label %112, label %131

112:                                              ; preds = %107
  %113 = icmp eq i32 %109, 0
  br i1 %113, label %132, label %114

114:                                              ; preds = %112
  %115 = getelementptr inbounds %struct.FaJson, ptr %1, i32 0, i32 4
  %116 = load ptr, ptr %115, align 8, !tbaa !404
  %117 = load ptr, ptr %9, align 4, !tbaa !6
  br label %120

118:                                              ; preds = %120
  %119 = icmp eq i32 %128, 0
  br i1 %119, label %131, label %132

120:                                              ; preds = %114, %120
  %121 = phi i32 [ 1, %114 ], [ %128, %120 ]
  %122 = phi i32 [ 0, %114 ], [ %129, %120 ]
  %123 = getelementptr inbounds i8, ptr %116, i32 %122
  %124 = load i8, ptr %123, align 1, !tbaa !1
  %125 = getelementptr inbounds i8, ptr %117, i32 %122
  %126 = load i8, ptr %125, align 1, !tbaa !1
  %127 = icmp eq i8 %124, %126
  %128 = select i1 %127, i32 %121, i32 0
  %129 = add nuw i32 %122, 1
  %130 = icmp eq i32 %129, %109
  br i1 %130, label %118, label %120, !llvm.loop !405

131:                                              ; preds = %118, %107, %102
  tail call fastcc void @json_mismatch(ptr noundef %0, ptr noundef nonnull %1, ptr noundef null, ptr noundef nonnull %2, i32 noundef %3) #61
  br label %132

132:                                              ; preds = %112, %118, %131
  %133 = phi i32 [ 0, %131 ], [ 1, %118 ], [ 1, %112 ]
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %9) #64
  br label %456

134:                                              ; preds = %4, %4
  %135 = load i32, ptr %1, align 8, !tbaa !390
  %136 = icmp eq i32 %135, 1
  br i1 %136, label %137, label %143

137:                                              ; preds = %134
  %138 = getelementptr inbounds %struct.FaJson, ptr %1, i32 0, i32 1
  %139 = load i32, ptr %138, align 4, !tbaa !398
  %140 = icmp eq i8 %20, 84
  %141 = zext i1 %140 to i32
  %142 = icmp eq i32 %139, %141
  br i1 %142, label %456, label %143

143:                                              ; preds = %137, %134
  tail call fastcc void @json_mismatch(ptr noundef %0, ptr noundef nonnull %1, ptr noundef null, ptr noundef nonnull %2, i32 noundef %3) #61
  br label %456

144:                                              ; preds = %4
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %10) #64
  %145 = getelementptr inbounds i8, ptr %2, i32 1
  store ptr %145, ptr %10, align 4, !tbaa !6
  %146 = call fastcc i64 @json_schema_number(ptr noundef nonnull %10, ptr noundef nonnull %5) #61
  %147 = load i32, ptr %1, align 8, !tbaa !390
  %148 = icmp eq i32 %147, 2
  br i1 %148, label %149, label %158

149:                                              ; preds = %144
  %150 = getelementptr inbounds %struct.FaJson, ptr %1, i32 0, i32 1
  %151 = load i32, ptr %150, align 4, !tbaa !398
  %152 = and i32 %151, 2
  %153 = icmp eq i32 %152, 0
  br i1 %153, label %158, label %154

154:                                              ; preds = %149
  %155 = getelementptr inbounds %struct.FaJson, ptr %1, i32 0, i32 2
  %156 = load i64, ptr %155, align 8, !tbaa !397
  %157 = icmp eq i64 %156, %146
  br i1 %157, label %159, label %158

158:                                              ; preds = %154, %149, %144
  tail call fastcc void @json_mismatch(ptr noundef %0, ptr noundef nonnull %1, ptr noundef null, ptr noundef nonnull %2, i32 noundef %3) #61
  br label %159

159:                                              ; preds = %158, %154
  %160 = phi i32 [ 1, %154 ], [ 0, %158 ]
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %10) #64
  br label %456

161:                                              ; preds = %4
  %162 = load i32, ptr %1, align 8, !tbaa !390
  %163 = icmp eq i32 %162, 4
  br i1 %163, label %165, label %164

164:                                              ; preds = %161
  tail call fastcc void @json_mismatch(ptr noundef %0, ptr noundef nonnull %1, ptr noundef nonnull @.str.85, ptr noundef null, i32 noundef %3) #61
  br label %456

165:                                              ; preds = %161
  %166 = getelementptr inbounds %struct.JsonCheck, ptr %0, i32 0, i32 1
  %167 = load i32, ptr %166, align 4, !tbaa !401
  %168 = getelementptr inbounds %struct.FaJson, ptr %1, i32 0, i32 9
  %169 = load i32, ptr %168, align 4, !tbaa !406
  %170 = icmp eq i32 %169, 0
  br i1 %170, label %456, label %171

171:                                              ; preds = %165
  %172 = getelementptr inbounds %struct.FaJson, ptr %1, i32 0, i32 8
  %173 = getelementptr inbounds i8, ptr %2, i32 1
  br label %178

174:                                              ; preds = %236
  %175 = add nuw i32 %179, 1
  %176 = load i32, ptr %168, align 4, !tbaa !406
  %177 = icmp ult i32 %175, %176
  br i1 %177, label %178, label %244, !llvm.loop !407

178:                                              ; preds = %171, %174
  %179 = phi i32 [ 0, %171 ], [ %175, %174 ]
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %11) #64
  %180 = icmp ugt i32 %179, 9999
  br i1 %180, label %181, label %204

181:                                              ; preds = %178, %181
  %182 = phi i32 [ %188, %181 ], [ 24, %178 ]
  %183 = phi i32 [ %185, %181 ], [ %179, %178 ]
  %184 = freeze i32 %183
  %185 = udiv i32 %184, 10000
  %186 = mul i32 %185, 10000
  %187 = sub i32 %184, %186
  %188 = add i32 %182, -4
  %189 = getelementptr inbounds i8, ptr %11, i32 %188
  %190 = trunc i32 %187 to i16
  %191 = udiv i16 %190, 100
  %192 = shl nuw nsw i16 %191, 1
  %193 = zext nneg i16 %192 to i32
  %194 = getelementptr inbounds i8, ptr @decimal_u64.pairs, i32 %193
  %195 = load i16, ptr %194, align 2
  store i16 %195, ptr %189, align 4
  %196 = getelementptr inbounds i8, ptr %189, i32 2
  %197 = mul i16 %191, 100
  %198 = sub i16 %190, %197
  %199 = shl nuw nsw i16 %198, 1
  %200 = zext nneg i16 %199 to i32
  %201 = getelementptr inbounds i8, ptr @decimal_u64.pairs, i32 %200
  %202 = load i16, ptr %201, align 2
  store i16 %202, ptr %196, align 2
  %203 = icmp ugt i32 %183, 99999999
  br i1 %203, label %181, label %204, !llvm.loop !22

204:                                              ; preds = %181, %178
  %205 = phi i32 [ %179, %178 ], [ %185, %181 ]
  %206 = phi i32 [ 24, %178 ], [ %188, %181 ]
  %207 = icmp ugt i32 %205, 99
  br i1 %207, label %208, label %221

208:                                              ; preds = %204
  %209 = trunc i32 %205 to i16
  %210 = freeze i16 %209
  %211 = udiv i16 %210, 100
  %212 = mul i16 %211, 100
  %213 = sub i16 %210, %212
  %214 = zext nneg i16 %211 to i32
  %215 = add i32 %206, -2
  %216 = getelementptr inbounds i8, ptr %11, i32 %215
  %217 = shl nuw nsw i16 %213, 1
  %218 = zext nneg i16 %217 to i32
  %219 = getelementptr inbounds i8, ptr @decimal_u64.pairs, i32 %218
  %220 = load i16, ptr %219, align 2
  store i16 %220, ptr %216, align 1
  br label %221

221:                                              ; preds = %208, %204
  %222 = phi i32 [ %214, %208 ], [ %205, %204 ]
  %223 = phi i32 [ %215, %208 ], [ %206, %204 ]
  %224 = icmp ugt i32 %222, 9
  br i1 %224, label %225, label %231

225:                                              ; preds = %221
  %226 = add i32 %223, -2
  %227 = getelementptr inbounds i8, ptr %11, i32 %226
  %228 = shl nuw nsw i32 %222, 1
  %229 = getelementptr inbounds i8, ptr @decimal_u64.pairs, i32 %228
  %230 = load i16, ptr %229, align 2
  store i16 %230, ptr %227, align 1
  br label %236

231:                                              ; preds = %221
  %232 = trunc i32 %222 to i8
  %233 = or disjoint i8 %232, 48
  %234 = add i32 %223, -1
  %235 = getelementptr inbounds i8, ptr %11, i32 %234
  store i8 %233, ptr %235, align 1, !tbaa !1
  br label %236

236:                                              ; preds = %225, %231
  %237 = phi i32 [ %226, %225 ], [ %234, %231 ]
  tail call fastcc void @json_path(ptr noundef %0, ptr noundef nonnull @.str.86, i32 noundef 1) #61
  %238 = getelementptr inbounds i8, ptr %11, i32 %237
  %239 = sub i32 24, %237
  call fastcc void @json_path(ptr noundef %0, ptr noundef nonnull %238, i32 noundef %239) #61
  tail call fastcc void @json_path(ptr noundef %0, ptr noundef nonnull @.str.87, i32 noundef 1) #61
  %240 = load ptr, ptr %172, align 8, !tbaa !408
  %241 = getelementptr inbounds %struct.FaJson, ptr %240, i32 %179
  %242 = tail call fastcc i32 @json_check(ptr noundef %0, ptr noundef %241, ptr noundef nonnull %173, i32 noundef %3) #61
  store i32 %167, ptr %166, align 4, !tbaa !401
  %243 = icmp ne i32 %242, 0
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %11) #64
  br i1 %243, label %174, label %244

244:                                              ; preds = %174, %236
  %245 = zext i1 %243 to i32
  br label %456

246:                                              ; preds = %4
  %247 = load i32, ptr %1, align 8, !tbaa !390
  %248 = icmp eq i32 %247, 5
  br i1 %248, label %250, label %249

249:                                              ; preds = %246
  tail call fastcc void @json_mismatch(ptr noundef %0, ptr noundef nonnull %1, ptr noundef nonnull @.str.88, ptr noundef null, i32 noundef %3) #61
  br label %456

250:                                              ; preds = %246
  %251 = getelementptr inbounds %struct.JsonCheck, ptr %0, i32 0, i32 1
  %252 = load i32, ptr %251, align 4, !tbaa !401
  %253 = getelementptr inbounds %struct.FaJson, ptr %1, i32 0, i32 8
  %254 = getelementptr inbounds %struct.FaJson, ptr %1, i32 0, i32 9
  %255 = load i32, ptr %254, align 4, !tbaa !406
  %256 = icmp eq i32 %255, 0
  br i1 %256, label %456, label %257

257:                                              ; preds = %250
  %258 = getelementptr inbounds i8, ptr %2, i32 1
  br label %263

259:                                              ; preds = %263
  %260 = add nuw i32 %264, 1
  %261 = load i32, ptr %254, align 4, !tbaa !406
  %262 = icmp ult i32 %260, %261
  br i1 %262, label %263, label %273, !llvm.loop !409

263:                                              ; preds = %257, %259
  %264 = phi i32 [ 0, %257 ], [ %260, %259 ]
  %265 = load ptr, ptr %253, align 8, !tbaa !408
  %266 = getelementptr inbounds %struct.FaJson, ptr %265, i32 %264
  tail call fastcc void @json_path(ptr noundef nonnull %0, ptr noundef nonnull @.str.89, i32 noundef 1) #61
  %267 = getelementptr inbounds %struct.FaJson, ptr %265, i32 %264, i32 6
  %268 = load ptr, ptr %267, align 8, !tbaa !392
  %269 = getelementptr inbounds %struct.FaJson, ptr %265, i32 %264, i32 7
  %270 = load i32, ptr %269, align 4, !tbaa !393
  tail call fastcc void @json_path(ptr noundef nonnull %0, ptr noundef %268, i32 noundef %270) #61
  %271 = tail call fastcc i32 @json_check(ptr noundef nonnull %0, ptr noundef %266, ptr noundef nonnull %258, i32 noundef %3) #61
  store i32 %252, ptr %251, align 4, !tbaa !401
  %272 = icmp ne i32 %271, 0
  br i1 %272, label %259, label %273

273:                                              ; preds = %259, %263
  %274 = zext i1 %272 to i32
  br label %456

275:                                              ; preds = %4
  %276 = load i32, ptr %1, align 8, !tbaa !390
  %277 = icmp eq i32 %276, 5
  br i1 %277, label %279, label %278

278:                                              ; preds = %275
  tail call fastcc void @json_mismatch(ptr noundef %0, ptr noundef nonnull %1, ptr noundef nonnull @.str.88, ptr noundef null, i32 noundef %3) #61
  br label %456

279:                                              ; preds = %275
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %12) #64
  %280 = getelementptr inbounds i8, ptr %2, i32 1
  store ptr %280, ptr %12, align 4, !tbaa !6
  %281 = call fastcc i64 @json_schema_number(ptr noundef nonnull %12, ptr noundef nonnull %5) #61
  %282 = getelementptr inbounds %struct.JsonCheck, ptr %0, i32 0, i32 1
  %283 = load i32, ptr %282, align 4, !tbaa !401
  %284 = icmp slt i64 %281, 1
  br i1 %284, label %316, label %285

285:                                              ; preds = %279, %312
  %286 = phi i1 [ %314, %312 ], [ false, %279 ]
  %287 = phi i64 [ %313, %312 ], [ 0, %279 ]
  %288 = call fastcc i64 @json_schema_number(ptr noundef nonnull %12, ptr noundef nonnull %5) #61
  %289 = load ptr, ptr %12, align 4, !tbaa !6
  %290 = trunc i64 %288 to i32
  %291 = getelementptr inbounds i8, ptr %289, i32 %290
  %292 = load i8, ptr %291, align 1, !tbaa !1
  %293 = getelementptr inbounds i8, ptr %291, i32 1
  %294 = tail call fastcc ptr @json_skip(ptr noundef nonnull %293) #61
  store ptr %294, ptr %12, align 4, !tbaa !6
  %295 = tail call ptr @fa_json_field(ptr noundef nonnull %1, ptr noundef %289, i32 noundef %290) #61
  %296 = icmp eq ptr %295, null
  br i1 %296, label %297, label %309

297:                                              ; preds = %285
  %298 = icmp eq i8 %292, 49
  br i1 %298, label %312, label %299

299:                                              ; preds = %297
  %300 = icmp eq i32 %3, 0
  br i1 %300, label %316, label %301

301:                                              ; preds = %299
  tail call fastcc void @json_error_start(ptr noundef %0) #61
  tail call fastcc void @append_c(ptr noundef nonnull @.str.90) #61
  tail call fastcc void @reserve(i32 noundef %290) #61
  %302 = load ptr, ptr @builder, align 4, !tbaa !6
  %303 = load i32, ptr @builder_length, align 4, !tbaa !4
  %304 = getelementptr inbounds i8, ptr %302, i32 %303
  tail call fastcc void @copy_bytes(ptr noundef %304, ptr noundef nonnull %289, i32 noundef %290) #61
  %305 = add i32 %303, %290
  store i32 %305, ptr @builder_length, align 4, !tbaa !4
  tail call fastcc void @append_c(ptr noundef nonnull @.str.91) #61
  %306 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #62
  store i64 10, ptr %306, align 8, !tbaa !233
  %307 = getelementptr inbounds %struct.MessageError, ptr %306, i32 0, i32 1
  tail call void @fa_b_take(ptr noundef nonnull %307) #61
  %308 = getelementptr inbounds %struct.JsonCheck, ptr %0, i32 0, i32 3
  store ptr %306, ptr %308, align 4, !tbaa !388
  br label %316

309:                                              ; preds = %285
  tail call fastcc void @json_path(ptr noundef %0, ptr noundef nonnull @.str.89, i32 noundef 1) #61
  tail call fastcc void @json_path(ptr noundef %0, ptr noundef nonnull %289, i32 noundef %290) #61
  %310 = tail call fastcc i32 @json_check(ptr noundef %0, ptr noundef nonnull %295, ptr noundef nonnull %293, i32 noundef %3) #61
  store i32 %283, ptr %282, align 4, !tbaa !401
  %311 = icmp eq i32 %310, 0
  br i1 %311, label %316, label %312

312:                                              ; preds = %297, %309
  %313 = add nuw nsw i64 %287, 1
  %314 = icmp sge i64 %313, %281
  %315 = icmp eq i64 %313, %281
  br i1 %315, label %316, label %285, !llvm.loop !410

316:                                              ; preds = %312, %309, %279, %299, %301
  %317 = phi i1 [ %286, %299 ], [ %286, %301 ], [ true, %279 ], [ %314, %312 ], [ %286, %309 ]
  %318 = zext i1 %317 to i32
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %12) #64
  br label %456

319:                                              ; preds = %4
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %13) #64
  %320 = getelementptr inbounds i8, ptr %2, i32 1
  store ptr %320, ptr %13, align 4, !tbaa !6
  %321 = call fastcc i64 @json_schema_number(ptr noundef nonnull %13, ptr noundef nonnull %5) #61
  %322 = icmp sgt i64 %321, 0
  br i1 %322, label %323, label %334

323:                                              ; preds = %319
  %324 = load ptr, ptr %13, align 4, !tbaa !6
  br label %325

325:                                              ; preds = %323, %330
  %326 = phi i64 [ %332, %330 ], [ 0, %323 ]
  %327 = phi ptr [ %331, %330 ], [ %324, %323 ]
  %328 = tail call fastcc i32 @json_check(ptr noundef %0, ptr noundef %1, ptr noundef %327, i32 noundef 0) #61
  %329 = icmp eq i32 %328, 0
  br i1 %329, label %330, label %335

330:                                              ; preds = %325
  %331 = tail call fastcc ptr @json_skip(ptr noundef %327) #61
  %332 = add nuw nsw i64 %326, 1
  %333 = icmp eq i64 %332, %321
  br i1 %333, label %334, label %325, !llvm.loop !411

334:                                              ; preds = %330, %319
  tail call fastcc void @json_mismatch(ptr noundef %0, ptr noundef %1, ptr noundef nonnull @.str.92, ptr noundef null, i32 noundef %3) #61
  br label %335

335:                                              ; preds = %325, %334
  %336 = phi i32 [ 0, %334 ], [ 1, %325 ]
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %13) #64
  br label %456

337:                                              ; preds = %4
  %338 = tail call fastcc i64 @json_enum_of(ptr noundef %1, ptr noundef nonnull %2) #61
  %339 = icmp sgt i64 %338, -1
  br i1 %339, label %456, label %340

340:                                              ; preds = %337
  %341 = icmp eq i32 %3, 0
  br i1 %341, label %456, label %342

342:                                              ; preds = %340
  tail call fastcc void @json_error_start(ptr noundef %0) #61
  tail call fastcc void @append_c(ptr noundef nonnull @.str.93) #61
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %14) #64
  %343 = getelementptr inbounds i8, ptr %2, i32 1
  store ptr %343, ptr %14, align 4, !tbaa !6
  %344 = call fastcc i64 @json_schema_number(ptr noundef nonnull %14, ptr noundef nonnull %5) #61
  %345 = icmp sgt i64 %344, 0
  br i1 %345, label %354, label %346

346:                                              ; preds = %372, %342
  tail call fastcc void @append_c(ptr noundef nonnull @.str.84) #61
  %347 = load i32, ptr %1, align 8, !tbaa !390
  %348 = icmp ult i32 %347, 5
  br i1 %348, label %349, label %352

349:                                              ; preds = %346
  %350 = getelementptr inbounds [5 x ptr], ptr @switch.table.json_mismatch, i32 0, i32 %347
  %351 = load ptr, ptr %350, align 4
  br label %352

352:                                              ; preds = %346, %349
  %353 = phi ptr [ %351, %349 ], [ @.str.103, %346 ]
  tail call fastcc void @append_c(ptr noundef nonnull %353) #61
  tail call fastcc void @json_error_end(ptr noundef %0) #61
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %14) #64
  br label %456

354:                                              ; preds = %342, %372
  %355 = phi i64 [ %373, %372 ], [ 0, %342 ]
  %356 = call fastcc i64 @json_schema_number(ptr noundef nonnull %14, ptr noundef nonnull %5) #61
  %357 = load ptr, ptr %14, align 4, !tbaa !6
  %358 = trunc i64 %356 to i32
  %359 = getelementptr inbounds i8, ptr %357, i32 %358
  %360 = icmp eq i64 %355, 0
  br i1 %360, label %362, label %361

361:                                              ; preds = %354
  tail call fastcc void @append_c(ptr noundef nonnull @.str.11) #61
  br label %362

362:                                              ; preds = %361, %354
  %363 = load i8, ptr %359, align 1, !tbaa !1
  %364 = icmp eq i8 %363, 112
  %365 = getelementptr inbounds i8, ptr %359, i32 1
  store ptr %365, ptr %14, align 4, !tbaa !6
  br i1 %364, label %366, label %371

366:                                              ; preds = %362
  %367 = call fastcc i64 @json_schema_number(ptr noundef nonnull %14, ptr noundef nonnull %5) #61
  %368 = tail call i64 @llvm.abs.i64(i64 %367, i1 false)
  %369 = lshr i64 %367, 63
  %370 = trunc i64 %369 to i32
  tail call fastcc void @append_u64(i64 noundef %368, i32 noundef %370) #61
  br label %372

371:                                              ; preds = %362
  tail call void @fa_b_json_string(ptr noundef nonnull %357, i32 noundef %358) #61
  br label %372

372:                                              ; preds = %371, %366
  %373 = add nuw nsw i64 %355, 1
  %374 = icmp eq i64 %373, %344
  br i1 %374, label %346, label %354, !llvm.loop !412

375:                                              ; preds = %4
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %15) #64
  %376 = call fastcc i64 @json_member_of(ptr noundef %1, ptr noundef nonnull %2, ptr noundef nonnull %15) #61
  %377 = icmp slt i64 %376, 0
  br i1 %377, label %378, label %417

378:                                              ; preds = %375
  %379 = load ptr, ptr %15, align 4, !tbaa !6
  %380 = icmp eq ptr %379, null
  br i1 %380, label %384, label %381

381:                                              ; preds = %378
  %382 = load i32, ptr %379, align 8, !tbaa !390
  %383 = icmp eq i32 %382, 3
  br i1 %383, label %385, label %384

384:                                              ; preds = %381, %378
  tail call fastcc void @json_mismatch(ptr noundef %0, ptr noundef %1, ptr noundef nonnull @.str.92, ptr noundef null, i32 noundef %3) #61
  br label %448

385:                                              ; preds = %381
  %386 = icmp eq i32 %3, 0
  br i1 %386, label %448, label %387

387:                                              ; preds = %385
  tail call fastcc void @json_error_start(ptr noundef %0) #61
  tail call fastcc void @append_c(ptr noundef nonnull @.str.94) #61
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %16) #64
  %388 = getelementptr inbounds i8, ptr %2, i32 1
  store ptr %388, ptr %16, align 4, !tbaa !6
  %389 = call fastcc i64 @json_schema_number(ptr noundef nonnull %16, ptr noundef nonnull %5) #61
  %390 = icmp sgt i64 %389, 0
  br i1 %390, label %391, label %395

391:                                              ; preds = %387
  %392 = load ptr, ptr %16, align 4, !tbaa !6
  %393 = getelementptr inbounds %struct.JsonMember, ptr %17, i32 0, i32 2
  %394 = getelementptr inbounds %struct.JsonMember, ptr %17, i32 0, i32 1
  br label %400

395:                                              ; preds = %413, %387
  tail call fastcc void @append_c(ptr noundef nonnull @.str.84) #61
  %396 = getelementptr inbounds %struct.FaJson, ptr %379, i32 0, i32 4
  %397 = load ptr, ptr %396, align 8, !tbaa !404
  %398 = getelementptr inbounds %struct.FaJson, ptr %379, i32 0, i32 5
  %399 = load i32, ptr %398, align 4, !tbaa !403
  tail call void @fa_b_json_string(ptr noundef %397, i32 noundef %399) #61
  tail call fastcc void @json_error_end(ptr noundef %0) #61
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %16) #64
  br label %448

400:                                              ; preds = %391, %413
  %401 = phi i64 [ 0, %391 ], [ %415, %413 ]
  %402 = phi i32 [ 1, %391 ], [ %414, %413 ]
  %403 = phi ptr [ %392, %391 ], [ %404, %413 ]
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %17) #64
  %404 = call fastcc ptr @json_member_at(ptr noundef %403, ptr noundef nonnull %17) #61
  %405 = load i8, ptr %393, align 4, !tbaa !413
  %406 = icmp eq i8 %405, 108
  br i1 %406, label %413, label %407

407:                                              ; preds = %400
  %408 = icmp eq i32 %402, 0
  br i1 %408, label %409, label %410

409:                                              ; preds = %407
  tail call fastcc void @append_c(ptr noundef nonnull @.str.11) #61
  br label %410

410:                                              ; preds = %409, %407
  %411 = load ptr, ptr %17, align 4, !tbaa !415
  %412 = load i32, ptr %394, align 4, !tbaa !416
  tail call void @fa_b_json_string(ptr noundef %411, i32 noundef %412) #61
  br label %413

413:                                              ; preds = %400, %410
  %414 = phi i32 [ 0, %410 ], [ %402, %400 ]
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %17) #64
  %415 = add nuw nsw i64 %401, 1
  %416 = icmp eq i64 %415, %389
  br i1 %416, label %395, label %400, !llvm.loop !417

417:                                              ; preds = %375
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %18) #64
  %418 = getelementptr inbounds i8, ptr %2, i32 1
  store ptr %418, ptr %18, align 4, !tbaa !6
  %419 = call fastcc i64 @json_schema_number(ptr noundef nonnull %18, ptr noundef nonnull %5) #61
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %19) #64
  %420 = load ptr, ptr %18, align 4, !tbaa !6
  br label %424

421:                                              ; preds = %424
  %422 = getelementptr inbounds %struct.JsonMember, ptr %19, i32 0, i32 2
  %423 = load i8, ptr %422, align 4, !tbaa !413
  switch i8 %423, label %434 [
    i8 101, label %446
    i8 108, label %446
    i8 114, label %430
  ]

424:                                              ; preds = %417, %424
  %425 = phi i64 [ 0, %417 ], [ %428, %424 ]
  %426 = phi ptr [ %420, %417 ], [ %427, %424 ]
  %427 = call fastcc ptr @json_member_at(ptr noundef %426, ptr noundef nonnull %19) #61
  %428 = add nuw i64 %425, 1
  %429 = icmp eq i64 %425, %376
  br i1 %429, label %421, label %424, !llvm.loop !418

430:                                              ; preds = %421
  %431 = getelementptr inbounds %struct.JsonMember, ptr %19, i32 0, i32 3
  %432 = load ptr, ptr %431, align 4, !tbaa !419
  %433 = tail call fastcc i32 @json_check(ptr noundef %0, ptr noundef %1, ptr noundef %432, i32 noundef %3) #61
  br label %446

434:                                              ; preds = %421
  %435 = tail call ptr @fa_json_field(ptr noundef %1, ptr noundef nonnull @.str.95, i32 noundef 5) #61
  %436 = icmp eq ptr %435, null
  br i1 %436, label %437, label %440

437:                                              ; preds = %434
  %438 = icmp eq i32 %3, 0
  br i1 %438, label %446, label %439

439:                                              ; preds = %437
  tail call fastcc void @json_error_start(ptr noundef %0) #61
  tail call fastcc void @append_c(ptr noundef nonnull @.str.96) #61
  tail call fastcc void @json_error_end(ptr noundef %0) #61
  br label %446

440:                                              ; preds = %434
  %441 = getelementptr inbounds %struct.JsonCheck, ptr %0, i32 0, i32 1
  %442 = load i32, ptr %441, align 4, !tbaa !401
  tail call fastcc void @json_path(ptr noundef %0, ptr noundef nonnull @.str.97, i32 noundef 6) #61
  %443 = getelementptr inbounds %struct.JsonMember, ptr %19, i32 0, i32 3
  %444 = load ptr, ptr %443, align 4, !tbaa !419
  %445 = tail call fastcc i32 @json_check(ptr noundef %0, ptr noundef nonnull %435, ptr noundef %444, i32 noundef %3) #61
  store i32 %442, ptr %441, align 4, !tbaa !401
  br label %446

446:                                              ; preds = %440, %439, %437, %421, %421, %430
  %447 = phi i32 [ %433, %430 ], [ 1, %421 ], [ 1, %421 ], [ %445, %440 ], [ 0, %439 ], [ 0, %437 ]
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %19) #64
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %18) #64
  br label %448

448:                                              ; preds = %385, %395, %446, %384
  %449 = phi i32 [ 0, %384 ], [ %447, %446 ], [ 0, %395 ], [ 0, %385 ]
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %15) #64
  br label %456

450:                                              ; preds = %4
  %451 = icmp eq i32 %3, 0
  br i1 %451, label %456, label %452

452:                                              ; preds = %450
  tail call fastcc void @json_error_start(ptr noundef %0) #61
  tail call fastcc void @append_c(ptr noundef nonnull @.str.98) #61
  %453 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #62
  store i64 10, ptr %453, align 8, !tbaa !233
  %454 = getelementptr inbounds %struct.MessageError, ptr %453, i32 0, i32 1
  tail call void @fa_b_take(ptr noundef nonnull %454) #61
  %455 = getelementptr inbounds %struct.JsonCheck, ptr %0, i32 0, i32 3
  store ptr %453, ptr %455, align 4, !tbaa !388
  br label %456

456:                                              ; preds = %250, %273, %165, %244, %450, %452, %340, %352, %337, %137, %143, %448, %335, %316, %278, %249, %164, %159, %132, %100, %43, %37, %31, %25
  %457 = phi i32 [ %449, %448 ], [ %336, %335 ], [ 0, %278 ], [ %318, %316 ], [ 0, %249 ], [ 0, %164 ], [ %160, %159 ], [ %133, %132 ], [ %101, %100 ], [ %44, %43 ], [ %38, %37 ], [ %32, %31 ], [ %26, %25 ], [ 1, %137 ], [ 0, %143 ], [ 1, %337 ], [ 0, %352 ], [ 0, %340 ], [ 0, %452 ], [ 0, %450 ], [ 1, %165 ], [ %245, %244 ], [ 1, %250 ], [ %274, %273 ]
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %5) #64
  ret i32 %457
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(read, inaccessiblemem: none)
define hidden i64 @fa_json_enum(ptr nocapture noundef readonly %0, ptr noundef readonly %1) local_unnamed_addr #34 {
  %3 = tail call fastcc i64 @json_enum_of(ptr noundef %0, ptr noundef %1) #61
  ret i64 %3
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(read, inaccessiblemem: none)
define internal fastcc i64 @json_enum_of(ptr nocapture noundef readonly %0, ptr noundef %1) unnamed_addr #34 {
  %3 = alloca i32, align 4
  %4 = alloca ptr, align 4
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %3) #64
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %4) #64
  %5 = getelementptr inbounds i8, ptr %1, i32 1
  store ptr %5, ptr %4, align 4, !tbaa !6
  %6 = call fastcc i64 @json_schema_number(ptr noundef nonnull %4, ptr noundef nonnull %3) #61
  %7 = getelementptr inbounds %struct.FaJson, ptr %0, i32 0, i32 2
  %8 = icmp sgt i64 %6, 0
  br i1 %8, label %9, label %59

9:                                                ; preds = %2
  %10 = getelementptr inbounds %struct.FaJson, ptr %0, i32 0, i32 5
  %11 = getelementptr inbounds %struct.FaJson, ptr %0, i32 0, i32 4
  %12 = getelementptr inbounds %struct.FaJson, ptr %0, i32 0, i32 1
  br label %13

13:                                               ; preds = %9, %56
  %14 = phi i64 [ 0, %9 ], [ %57, %56 ]
  %15 = call fastcc i64 @json_schema_number(ptr noundef nonnull %4, ptr noundef nonnull %3) #61
  %16 = load ptr, ptr %4, align 4, !tbaa !6
  %17 = trunc i64 %15 to i32
  %18 = getelementptr inbounds i8, ptr %16, i32 %17
  %19 = load i8, ptr %18, align 1, !tbaa !1
  %20 = icmp eq i8 %19, 112
  %21 = getelementptr inbounds i8, ptr %18, i32 1
  store ptr %21, ptr %4, align 4, !tbaa !6
  br i1 %20, label %22, label %33

22:                                               ; preds = %13
  %23 = call fastcc i64 @json_schema_number(ptr noundef nonnull %4, ptr noundef nonnull %3) #61
  %24 = load i32, ptr %0, align 8, !tbaa !390
  %25 = icmp eq i32 %24, 2
  br i1 %25, label %26, label %56

26:                                               ; preds = %22
  %27 = load i32, ptr %12, align 4, !tbaa !398
  %28 = and i32 %27, 2
  %29 = icmp eq i32 %28, 0
  br i1 %29, label %56, label %30

30:                                               ; preds = %26
  %31 = load i64, ptr %7, align 8, !tbaa !397
  %32 = icmp eq i64 %31, %23
  br i1 %32, label %59, label %56

33:                                               ; preds = %13
  %34 = load i32, ptr %0, align 8, !tbaa !390
  %35 = icmp eq i32 %34, 3
  br i1 %35, label %36, label %56

36:                                               ; preds = %33
  %37 = load i32, ptr %10, align 4, !tbaa !403
  %38 = icmp eq i32 %37, %17
  br i1 %38, label %39, label %56

39:                                               ; preds = %36
  %40 = icmp eq i32 %17, 0
  br i1 %40, label %59, label %41

41:                                               ; preds = %39
  %42 = load ptr, ptr %11, align 8, !tbaa !404
  br label %45

43:                                               ; preds = %45
  %44 = icmp eq i32 %53, 0
  br i1 %44, label %56, label %59

45:                                               ; preds = %41, %45
  %46 = phi i32 [ 0, %41 ], [ %54, %45 ]
  %47 = phi i32 [ 1, %41 ], [ %53, %45 ]
  %48 = getelementptr inbounds i8, ptr %42, i32 %46
  %49 = load i8, ptr %48, align 1, !tbaa !1
  %50 = getelementptr inbounds i8, ptr %16, i32 %46
  %51 = load i8, ptr %50, align 1, !tbaa !1
  %52 = icmp eq i8 %49, %51
  %53 = select i1 %52, i32 %47, i32 0
  %54 = add nuw i32 %46, 1
  %55 = icmp eq i32 %54, %17
  br i1 %55, label %43, label %45, !llvm.loop !420

56:                                               ; preds = %43, %22, %26, %30, %36, %33
  %57 = add nuw nsw i64 %14, 1
  %58 = icmp eq i64 %57, %6
  br i1 %58, label %59, label %13, !llvm.loop !421

59:                                               ; preds = %39, %56, %43, %30, %2
  %60 = phi i64 [ -1, %2 ], [ %14, %39 ], [ %14, %30 ], [ %14, %43 ], [ -1, %56 ]
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %4) #64
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %3) #64
  ret i64 %60
}

; Function Attrs: nounwind optsize
define hidden i64 @fa_json_member(ptr nocapture noundef readonly %0, ptr noundef %1) local_unnamed_addr #3 {
  %3 = alloca ptr, align 4
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %3) #64
  %4 = call fastcc i64 @json_member_of(ptr noundef %0, ptr noundef %1, ptr noundef nonnull %3) #61
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %3) #64
  ret i64 %4
}

; Function Attrs: nounwind optsize
define internal fastcc i64 @json_member_of(ptr nocapture noundef readonly %0, ptr noundef %1, ptr nocapture noundef %2) unnamed_addr #3 {
  %4 = alloca i32, align 4
  %5 = alloca ptr, align 4
  %6 = alloca %struct.JsonMember, align 4
  %7 = alloca %struct.JsonCheck, align 4
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %4) #64
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %5) #64
  %8 = getelementptr inbounds i8, ptr %1, i32 1
  store ptr %8, ptr %5, align 4, !tbaa !6
  %9 = call fastcc i64 @json_schema_number(ptr noundef nonnull %5, ptr noundef nonnull %4) #61
  %10 = load i32, ptr %0, align 8, !tbaa !390
  %11 = icmp eq i32 %10, 5
  br i1 %11, label %12, label %14

12:                                               ; preds = %3
  %13 = tail call ptr @fa_json_field(ptr noundef nonnull %0, ptr noundef nonnull @.str.104, i32 noundef 4) #61
  br label %14

14:                                               ; preds = %3, %12
  %15 = phi ptr [ %13, %12 ], [ null, %3 ]
  store ptr %15, ptr %2, align 4, !tbaa !6
  %16 = icmp sgt i64 %9, 0
  br i1 %16, label %17, label %22

17:                                               ; preds = %14
  %18 = load ptr, ptr %5, align 4, !tbaa !6
  %19 = getelementptr inbounds %struct.JsonMember, ptr %6, i32 0, i32 2
  %20 = getelementptr inbounds %struct.JsonMember, ptr %6, i32 0, i32 1
  %21 = getelementptr inbounds %struct.JsonMember, ptr %6, i32 0, i32 3
  br label %24

22:                                               ; preds = %68, %67, %14
  %23 = phi i64 [ -1, %14 ], [ %25, %67 ], [ %69, %68 ]
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %5) #64
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %4) #64
  ret i64 %23

24:                                               ; preds = %17, %68
  %25 = phi i64 [ 0, %17 ], [ %70, %68 ]
  %26 = phi i64 [ -1, %17 ], [ %69, %68 ]
  %27 = phi ptr [ %18, %17 ], [ %28, %68 ]
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %6) #64
  %28 = call fastcc ptr @json_member_at(ptr noundef %27, ptr noundef nonnull %6) #61
  %29 = load i8, ptr %19, align 4, !tbaa !413
  %30 = icmp eq i8 %29, 108
  br i1 %30, label %31, label %36

31:                                               ; preds = %24
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %7) #64
  call void @llvm.memset.p0.i32(ptr noundef nonnull align 4 dereferenceable(16) %7, i8 0, i32 16, i1 false)
  %32 = load ptr, ptr %21, align 4, !tbaa !419
  %33 = call fastcc i32 @json_check(ptr noundef nonnull %7, ptr noundef nonnull %0, ptr noundef %32, i32 noundef 0) #61
  %34 = icmp eq i32 %33, 0
  %35 = select i1 %34, i64 %26, i64 %25
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %7) #64
  br label %68

36:                                               ; preds = %24
  %37 = load ptr, ptr %2, align 4, !tbaa !6
  %38 = icmp eq ptr %37, null
  br i1 %38, label %68, label %39

39:                                               ; preds = %36
  %40 = load i32, ptr %37, align 8, !tbaa !390
  %41 = icmp eq i32 %40, 3
  br i1 %41, label %42, label %68

42:                                               ; preds = %39
  %43 = getelementptr inbounds %struct.FaJson, ptr %37, i32 0, i32 5
  %44 = load i32, ptr %43, align 4, !tbaa !403
  %45 = load i32, ptr %20, align 4, !tbaa !416
  %46 = icmp eq i32 %44, %45
  br i1 %46, label %47, label %68

47:                                               ; preds = %42
  %48 = icmp eq i32 %44, 0
  br i1 %48, label %67, label %49

49:                                               ; preds = %47
  %50 = getelementptr inbounds %struct.FaJson, ptr %37, i32 0, i32 4
  %51 = load ptr, ptr %50, align 8, !tbaa !404
  %52 = load ptr, ptr %6, align 4, !tbaa !415
  br label %55

53:                                               ; preds = %55
  %54 = icmp eq i32 %64, 0
  br i1 %54, label %68, label %67

55:                                               ; preds = %49, %55
  %56 = phi i32 [ 0, %49 ], [ %65, %55 ]
  %57 = phi i32 [ 1, %49 ], [ %64, %55 ]
  %58 = getelementptr inbounds i8, ptr %51, i32 %56
  %59 = load i8, ptr %58, align 1, !tbaa !1
  %60 = getelementptr inbounds i8, ptr %52, i32 %56
  %61 = load i8, ptr %60, align 1, !tbaa !1
  %62 = icmp eq i8 %59, %61
  %63 = freeze i1 %62
  %64 = select i1 %63, i32 %57, i32 0
  %65 = add nuw i32 %56, 1
  %66 = icmp eq i32 %65, %44
  br i1 %66, label %53, label %55, !llvm.loop !422

67:                                               ; preds = %53, %47
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %6) #64
  br label %22

68:                                               ; preds = %53, %36, %39, %42, %31
  %69 = phi i64 [ %35, %31 ], [ %26, %42 ], [ %26, %39 ], [ %26, %36 ], [ %26, %53 ]
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %6) #64
  %70 = add nuw nsw i64 %25, 1
  %71 = icmp slt i64 %70, %9
  %72 = icmp slt i64 %69, 0
  %73 = select i1 %71, i1 %72, i1 false
  br i1 %73, label %24, label %22, !llvm.loop !423
}

; Function Attrs: nounwind optsize
define hidden i32 @fa_json_fits(ptr nocapture noundef readonly %0, ptr noundef %1) local_unnamed_addr #3 {
  %3 = alloca %struct.JsonCheck, align 4
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %3) #64
  call void @llvm.memset.p0.i32(ptr noundef nonnull align 4 dereferenceable(16) %3, i8 0, i32 16, i1 false)
  %4 = call fastcc i32 @json_check(ptr noundef nonnull %3, ptr noundef %0, ptr noundef %1, i32 noundef 0) #61
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %3) #64
  ret i32 %4
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read)
define hidden i32 @fa_json_boolean(ptr nocapture noundef readonly %0) local_unnamed_addr #37 {
  %2 = getelementptr inbounds %struct.FaJson, ptr %0, i32 0, i32 1
  %3 = load i32, ptr %2, align 4, !tbaa !398
  ret i32 %3
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read)
define hidden i64 @fa_json_integer(ptr nocapture noundef readonly %0) local_unnamed_addr #37 {
  %2 = getelementptr inbounds %struct.FaJson, ptr %0, i32 0, i32 2
  %3 = load i64, ptr %2, align 8, !tbaa !397
  ret i64 %3
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read)
define hidden double @fa_json_number(ptr nocapture noundef readonly %0) local_unnamed_addr #37 {
  %2 = getelementptr inbounds %struct.FaJson, ptr %0, i32 0, i32 3
  %3 = load double, ptr %2, align 8, !tbaa !399
  ret double %3
}

; Function Attrs: nounwind optsize
define hidden void @fa_json_string(ptr nocapture noundef readonly %0, ptr nocapture noundef writeonly %1) local_unnamed_addr #3 {
  %3 = alloca %struct.FaString, align 4
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %3) #64
  %4 = getelementptr inbounds %struct.FaJson, ptr %0, i32 0, i32 4
  %5 = load ptr, ptr %4, align 8, !tbaa !404
  %6 = getelementptr inbounds %struct.FaJson, ptr %0, i32 0, i32 5
  %7 = load i32, ptr %6, align 4, !tbaa !403
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %3, ptr noundef %5, i32 noundef %7, i32 noundef 0) #61
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %1, ptr noundef nonnull align 4 dereferenceable(12) %3, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %3) #64
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read)
define hidden i64 @fa_json_count(ptr nocapture noundef readonly %0) local_unnamed_addr #37 {
  %2 = getelementptr inbounds %struct.FaJson, ptr %0, i32 0, i32 9
  %3 = load i32, ptr %2, align 4, !tbaa !406
  %4 = zext i32 %3 to i64
  ret i64 %4
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read)
define hidden ptr @fa_json_item(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #37 {
  %3 = getelementptr inbounds %struct.FaJson, ptr %0, i32 0, i32 8
  %4 = load ptr, ptr %3, align 8, !tbaa !408
  %5 = trunc i64 %1 to i32
  %6 = getelementptr inbounds %struct.FaJson, ptr %4, i32 %5
  ret ptr %6
}

; Function Attrs: nounwind optsize
define hidden void @fa_json_key(ptr nocapture noundef readonly %0, ptr nocapture noundef writeonly %1) local_unnamed_addr #3 {
  %3 = alloca %struct.FaString, align 4
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %3) #64
  %4 = getelementptr inbounds %struct.FaJson, ptr %0, i32 0, i32 6
  %5 = load ptr, ptr %4, align 8, !tbaa !392
  %6 = getelementptr inbounds %struct.FaJson, ptr %0, i32 0, i32 7
  %7 = load i32, ptr %6, align 4, !tbaa !393
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %3, ptr noundef %5, i32 noundef %7, i32 noundef 0) #61
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %1, ptr noundef nonnull align 4 dereferenceable(12) %3, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %3) #64
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(read, inaccessiblemem: none)
define hidden ptr @fa_json_field(ptr nocapture noundef readonly %0, ptr nocapture noundef readonly %1, i32 noundef %2) local_unnamed_addr #34 {
  %4 = getelementptr inbounds %struct.FaJson, ptr %0, i32 0, i32 9
  %5 = load i32, ptr %4, align 4, !tbaa !406
  %6 = icmp eq i32 %5, 0
  br i1 %6, label %38, label %7

7:                                                ; preds = %3
  %8 = getelementptr inbounds %struct.FaJson, ptr %0, i32 0, i32 8
  %9 = load ptr, ptr %8, align 8, !tbaa !408
  %10 = icmp eq i32 %2, 0
  br label %11

11:                                               ; preds = %7, %23
  %12 = phi i32 [ %5, %7 ], [ %13, %23 ]
  %13 = add i32 %12, -1
  %14 = getelementptr inbounds %struct.FaJson, ptr %9, i32 %13, i32 7
  %15 = load i32, ptr %14, align 4, !tbaa !393
  %16 = icmp eq i32 %15, %2
  br i1 %16, label %17, label %23

17:                                               ; preds = %11
  br i1 %10, label %36, label %18

18:                                               ; preds = %17
  %19 = getelementptr inbounds %struct.FaJson, ptr %9, i32 %13, i32 6
  %20 = load ptr, ptr %19, align 8, !tbaa !392
  br label %25

21:                                               ; preds = %25
  %22 = icmp eq i32 %33, 0
  br i1 %22, label %23, label %36

23:                                               ; preds = %21, %11
  %24 = icmp eq i32 %13, 0
  br i1 %24, label %38, label %11, !llvm.loop !424

25:                                               ; preds = %18, %25
  %26 = phi i32 [ 0, %18 ], [ %34, %25 ]
  %27 = phi i32 [ 1, %18 ], [ %33, %25 ]
  %28 = getelementptr inbounds i8, ptr %20, i32 %26
  %29 = load i8, ptr %28, align 1, !tbaa !1
  %30 = getelementptr inbounds i8, ptr %1, i32 %26
  %31 = load i8, ptr %30, align 1, !tbaa !1
  %32 = icmp eq i8 %29, %31
  %33 = select i1 %32, i32 %27, i32 0
  %34 = add nuw i32 %26, 1
  %35 = icmp eq i32 %34, %2
  br i1 %35, label %21, label %25, !llvm.loop !425

36:                                               ; preds = %17, %21
  %37 = getelementptr inbounds %struct.FaJson, ptr %9, i32 %13
  br label %38

38:                                               ; preds = %23, %3, %36
  %39 = phi ptr [ %37, %36 ], [ null, %3 ], [ null, %23 ]
  ret ptr %39
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_json_string(ptr nocapture noundef readonly %0, i32 noundef %1) local_unnamed_addr #3 {
  tail call fastcc void @reserve(i32 noundef 1) #61
  %3 = load ptr, ptr @builder, align 4, !tbaa !6
  %4 = load i32, ptr @builder_length, align 4, !tbaa !4
  %5 = getelementptr inbounds i8, ptr %3, i32 %4
  store i8 34, ptr %5, align 1, !tbaa !1
  %6 = add i32 %4, 1
  store i32 %6, ptr @builder_length, align 4, !tbaa !4
  %7 = icmp eq i32 %1, 0
  br i1 %7, label %8, label %20

8:                                                ; preds = %61, %2
  %9 = phi i32 [ 0, %2 ], [ %63, %61 ]
  %10 = getelementptr inbounds i8, ptr %0, i32 %9
  %11 = sub i32 %1, %9
  tail call fastcc void @reserve(i32 noundef %11) #61
  %12 = load ptr, ptr @builder, align 4, !tbaa !6
  %13 = load i32, ptr @builder_length, align 4, !tbaa !4
  %14 = getelementptr inbounds i8, ptr %12, i32 %13
  tail call fastcc void @copy_bytes(ptr noundef %14, ptr noundef %10, i32 noundef %11) #61
  %15 = add i32 %13, %11
  store i32 %15, ptr @builder_length, align 4, !tbaa !4
  tail call fastcc void @reserve(i32 noundef 1) #61
  %16 = load ptr, ptr @builder, align 4, !tbaa !6
  %17 = load i32, ptr @builder_length, align 4, !tbaa !4
  %18 = getelementptr inbounds i8, ptr %16, i32 %17
  store i8 34, ptr %18, align 1, !tbaa !1
  %19 = add i32 %17, 1
  store i32 %19, ptr @builder_length, align 4, !tbaa !4
  ret void

20:                                               ; preds = %2, %61
  %21 = phi i32 [ %63, %61 ], [ 0, %2 ]
  %22 = phi i32 [ %62, %61 ], [ 0, %2 ]
  %23 = getelementptr inbounds i8, ptr %0, i32 %22
  %24 = load i8, ptr %23, align 1, !tbaa !1
  %25 = freeze i8 %24
  %26 = zext i8 %25 to i32
  %27 = icmp ugt i8 %25, 31
  br i1 %27, label %28, label %31

28:                                               ; preds = %20
  switch i8 %25, label %29 [
    i8 92, label %31
    i8 34, label %31
  ]

29:                                               ; preds = %28
  %30 = add nuw i32 %22, 1
  br label %61

31:                                               ; preds = %28, %28, %20
  %32 = getelementptr inbounds i8, ptr %0, i32 %21
  %33 = sub i32 %22, %21
  tail call fastcc void @reserve(i32 noundef %33) #61
  %34 = load ptr, ptr @builder, align 4, !tbaa !6
  %35 = load i32, ptr @builder_length, align 4, !tbaa !4
  %36 = getelementptr inbounds i8, ptr %34, i32 %35
  tail call fastcc void @copy_bytes(ptr noundef %36, ptr noundef %32, i32 noundef %33) #61
  %37 = add i32 %35, %33
  store i32 %37, ptr @builder_length, align 4, !tbaa !4
  %38 = add nuw i32 %22, 1
  switch i8 %25, label %46 [
    i8 34, label %39
    i8 92, label %40
    i8 8, label %41
    i8 12, label %42
    i8 10, label %43
    i8 13, label %44
    i8 9, label %45
  ]

39:                                               ; preds = %31
  tail call fastcc void @append_c(ptr noundef nonnull @.str.3) #61
  br label %61

40:                                               ; preds = %31
  tail call fastcc void @append_c(ptr noundef nonnull @.str.2) #61
  br label %61

41:                                               ; preds = %31
  tail call fastcc void @append_c(ptr noundef nonnull @.str.33) #61
  br label %61

42:                                               ; preds = %31
  tail call fastcc void @append_c(ptr noundef nonnull @.str.34) #61
  br label %61

43:                                               ; preds = %31
  tail call fastcc void @append_c(ptr noundef nonnull @.str.4) #61
  br label %61

44:                                               ; preds = %31
  tail call fastcc void @append_c(ptr noundef nonnull @.str.5) #61
  br label %61

45:                                               ; preds = %31
  tail call fastcc void @append_c(ptr noundef nonnull @.str.6) #61
  br label %61

46:                                               ; preds = %31
  tail call fastcc void @append_c(ptr noundef nonnull @.str.35) #61
  %47 = lshr i32 %26, 4
  %48 = getelementptr inbounds [17 x i8], ptr @fa_b_json_string.hex, i32 0, i32 %47
  %49 = load i8, ptr %48, align 1, !tbaa !1
  tail call fastcc void @reserve(i32 noundef 1) #61
  %50 = load ptr, ptr @builder, align 4, !tbaa !6
  %51 = load i32, ptr @builder_length, align 4, !tbaa !4
  %52 = getelementptr inbounds i8, ptr %50, i32 %51
  store i8 %49, ptr %52, align 1, !tbaa !1
  %53 = add i32 %51, 1
  store i32 %53, ptr @builder_length, align 4, !tbaa !4
  %54 = and i32 %26, 15
  %55 = getelementptr inbounds [17 x i8], ptr @fa_b_json_string.hex, i32 0, i32 %54
  %56 = load i8, ptr %55, align 1, !tbaa !1
  tail call fastcc void @reserve(i32 noundef 1) #61
  %57 = load ptr, ptr @builder, align 4, !tbaa !6
  %58 = load i32, ptr @builder_length, align 4, !tbaa !4
  %59 = getelementptr inbounds i8, ptr %57, i32 %58
  store i8 %56, ptr %59, align 1, !tbaa !1
  %60 = add i32 %58, 1
  store i32 %60, ptr @builder_length, align 4, !tbaa !4
  br label %61

61:                                               ; preds = %29, %39, %40, %41, %42, %43, %44, %45, %46
  %62 = phi i32 [ %30, %29 ], [ %38, %39 ], [ %38, %40 ], [ %38, %41 ], [ %38, %42 ], [ %38, %43 ], [ %38, %44 ], [ %38, %45 ], [ %38, %46 ]
  %63 = phi i32 [ %21, %29 ], [ %38, %39 ], [ %38, %40 ], [ %38, %41 ], [ %38, %42 ], [ %38, %43 ], [ %38, %44 ], [ %38, %45 ], [ %38, %46 ]
  %64 = icmp eq i32 %62, %1
  br i1 %64, label %8, label %20, !llvm.loop !426
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_json_number(double noundef %0) local_unnamed_addr #3 {
  %2 = fsub double %0, %0
  %3 = fcmp une double %2, 0.000000e+00
  br i1 %3, label %4, label %5

4:                                                ; preds = %1
  tail call fastcc void @append_c(ptr noundef nonnull @.str.36) #61
  br label %6

5:                                                ; preds = %1
  tail call void @fa_b_f64(double noundef %0) #61
  br label %6

6:                                                ; preds = %5, %4
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_worker_stop(ptr noundef %0) local_unnamed_addr #3 {
  %2 = icmp eq ptr %0, null
  br i1 %2, label %73, label %3

3:                                                ; preds = %1
  %4 = getelementptr inbounds %struct.FaWorker, ptr %0, i32 0, i32 10
  %5 = load i32, ptr %4, align 4, !tbaa !427
  %6 = icmp eq i32 %5, 0
  br i1 %6, label %7, label %73

7:                                                ; preds = %3
  store i32 1, ptr %4, align 4, !tbaa !427
  %8 = getelementptr inbounds %struct.FaWorker, ptr %0, i32 0, i32 8
  %9 = load i32, ptr %8, align 4, !tbaa !430
  %10 = icmp eq i32 %9, 0
  br i1 %10, label %73, label %11

11:                                               ; preds = %7
  %12 = getelementptr inbounds %struct.FaWorker, ptr %0, i32 0, i32 12
  %13 = load ptr, ptr %12, align 4, !tbaa !431
  %14 = icmp eq ptr %13, null
  br i1 %14, label %15, label %71

15:                                               ; preds = %11
  %16 = getelementptr inbounds %struct.FaWorker, ptr %0, i32 0, i32 3
  %17 = load ptr, ptr %16, align 4, !tbaa !6
  %18 = icmp eq ptr %17, null
  br i1 %18, label %19, label %22

19:                                               ; preds = %15
  %20 = getelementptr inbounds %struct.FaWorker, ptr %0, i32 0, i32 4
  store ptr null, ptr %20, align 4, !tbaa !432
  %21 = getelementptr inbounds %struct.FaWorker, ptr %0, i32 0, i32 9
  store i32 1, ptr %21, align 8, !tbaa !433
  br label %71

22:                                               ; preds = %15, %33
  %23 = phi ptr [ %37, %33 ], [ %17, %15 ]
  %24 = phi ptr [ %36, %33 ], [ null, %15 ]
  %25 = phi ptr [ %35, %33 ], [ %16, %15 ]
  %26 = phi ptr [ %34, %33 ], [ null, %15 ]
  %27 = getelementptr inbounds i8, ptr %23, i32 176
  %28 = load i32, ptr %27, align 4, !tbaa !286
  %29 = and i32 %28, 1
  %30 = icmp eq i32 %29, 0
  br i1 %30, label %33, label %31

31:                                               ; preds = %22
  %32 = load ptr, ptr %23, align 8, !tbaa !434
  store ptr %32, ptr %25, align 4, !tbaa !6
  store ptr %24, ptr %23, align 8, !tbaa !434
  br label %33

33:                                               ; preds = %22, %31
  %34 = phi ptr [ %26, %31 ], [ %23, %22 ]
  %35 = phi ptr [ %25, %31 ], [ %23, %22 ]
  %36 = phi ptr [ %23, %31 ], [ %24, %22 ]
  %37 = load ptr, ptr %35, align 4, !tbaa !6
  %38 = icmp eq ptr %37, null
  br i1 %38, label %39, label %22, !llvm.loop !436

39:                                               ; preds = %33
  %40 = getelementptr inbounds %struct.FaWorker, ptr %0, i32 0, i32 4
  store ptr %34, ptr %40, align 4, !tbaa !432
  %41 = getelementptr inbounds %struct.FaWorker, ptr %0, i32 0, i32 9
  store i32 1, ptr %41, align 8, !tbaa !433
  %42 = icmp eq ptr %36, null
  br i1 %42, label %71, label %43

43:                                               ; preds = %39, %69
  %44 = phi ptr [ %45, %69 ], [ %36, %39 ]
  %45 = load ptr, ptr %44, align 8, !tbaa !434
  %46 = getelementptr inbounds %struct.FaCall, ptr %44, i32 0, i32 2
  %47 = load ptr, ptr %46, align 8, !tbaa !437
  %48 = getelementptr inbounds %struct.FaCall, ptr %44, i32 0, i32 4
  %49 = load ptr, ptr %48, align 8, !tbaa !438
  %50 = getelementptr inbounds %struct.FaCall, ptr %44, i32 0, i32 6
  %51 = load ptr, ptr %50, align 8, !tbaa !439
  tail call void %49(ptr noundef %51) #62
  %52 = icmp eq ptr %47, null
  br i1 %52, label %65, label %53

53:                                               ; preds = %43
  %54 = getelementptr inbounds %struct.FaExecutor, ptr %47, i32 0, i32 11
  %55 = load i64, ptr %54, align 8, !tbaa !288
  %56 = add nsw i64 %55, -1
  store i64 %56, ptr %54, align 8, !tbaa !288
  %57 = getelementptr inbounds %struct.FaCall, ptr %44, i32 0, i32 7
  %58 = load i64, ptr %57, align 8, !tbaa !440
  %59 = trunc i64 %58 to i32
  tail call void @fa_free(ptr noundef nonnull %44, i32 noundef %59, i32 noundef 16) #62
  %60 = getelementptr inbounds %struct.FaExecutor, ptr %47, i32 0, i32 14
  %61 = atomicrmw add ptr %60, i64 1 seq_cst, align 8
  %62 = getelementptr inbounds %struct.FaExecutor, ptr %47, i32 0, i32 12
  %63 = atomicrmw sub ptr %62, i64 1 seq_cst, align 8
  %64 = atomicrmw sub ptr %60, i64 1 seq_cst, align 8
  br label %69

65:                                               ; preds = %43
  %66 = getelementptr inbounds %struct.FaCall, ptr %44, i32 0, i32 7
  %67 = load i64, ptr %66, align 8, !tbaa !440
  %68 = trunc i64 %67 to i32
  tail call void @fa_free(ptr noundef nonnull %44, i32 noundef %68, i32 noundef 16) #62
  br label %69

69:                                               ; preds = %65, %53
  tail call void @fa_shared_release(ptr noundef nonnull %0) #61
  %70 = icmp eq ptr %45, null
  br i1 %70, label %71, label %43, !llvm.loop !441

71:                                               ; preds = %69, %39, %19, %11
  %72 = atomicrmw sub ptr @workers_alive, i64 1 acq_rel, align 8
  br label %73

73:                                               ; preds = %71, %7, %1, %3
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_worker_detach(ptr nocapture noundef %0) local_unnamed_addr #3 {
  %2 = getelementptr inbounds %struct.FaWorker, ptr %0, i32 0, i32 8
  %3 = load i32, ptr %2, align 4, !tbaa !430
  %4 = icmp eq i32 %3, 0
  br i1 %4, label %17, label %5

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaWorker, ptr %0, i32 0, i32 10
  %7 = load i32, ptr %6, align 4, !tbaa !427
  %8 = icmp eq i32 %7, 0
  br i1 %8, label %9, label %17

9:                                                ; preds = %5
  %10 = atomicrmw add ptr %0, i64 1 monotonic, align 8
  %11 = getelementptr inbounds %struct.FaWorker, ptr %0, i32 0, i32 11
  store i32 1, ptr %11, align 16, !tbaa !442
  %12 = getelementptr inbounds %struct.FaWorker, ptr %0, i32 0, i32 9
  store i32 1, ptr %12, align 8, !tbaa !433
  store i32 1, ptr %6, align 4, !tbaa !427
  br i1 icmp ne (ptr @pthread_detach, ptr null), label %13, label %17

13:                                               ; preds = %9
  %14 = getelementptr inbounds %struct.FaWorker, ptr %0, i32 0, i32 7
  %15 = load i32, ptr %14, align 16, !tbaa !443
  %16 = tail call i32 @pthread_detach(i32 noundef %15) #62
  br label %17

17:                                               ; preds = %1, %5, %13, %9
  ret void
}

; Function Attrs: optsize
declare extern_weak i32 @pthread_detach(i32 noundef) #4

; Function Attrs: nounwind optsize
define hidden ptr @fa_worker_new(i64 noundef %0, ptr noundef %1) local_unnamed_addr #3 {
  %3 = trunc i64 %0 to i32
  %4 = add i32 %3, 256
  %5 = tail call fastcc ptr @shared_new(i32 noundef %4, ptr noundef nonnull @worker_drop) #61
  %6 = getelementptr inbounds %struct.FaWorker, ptr %5, i32 0, i32 6
  store ptr %1, ptr %6, align 4, !tbaa !444
  ret ptr %5
}

; Function Attrs: nounwind optsize
define internal void @worker_drop(ptr noundef %0) #3 {
  tail call void @fa_worker_stop(ptr noundef %0) #61
  %2 = getelementptr inbounds %struct.FaWorker, ptr %0, i32 0, i32 6
  %3 = load ptr, ptr %2, align 4, !tbaa !444
  %4 = icmp eq ptr %3, null
  br i1 %4, label %6, label %5

5:                                                ; preds = %1
  tail call void %3(ptr noundef nonnull %0) #62
  br label %6

6:                                                ; preds = %5, %1
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(none)
define hidden void @fa_worker_start(ptr nocapture noundef readnone %0) local_unnamed_addr #23 {
  ret void
}

; Function Attrs: nounwind optsize
define hidden nonnull ptr @fa_call_new(i64 noundef %0) local_unnamed_addr #3 {
  %2 = trunc i64 %0 to i32
  %3 = add i32 %2, 144
  %4 = tail call ptr @fa_alloc(i32 noundef %3, i32 noundef 16) #62
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(180) %4, i8 0, i32 180, i1 false)
  %5 = zext i32 %3 to i64
  %6 = getelementptr inbounds %struct.FaCall, ptr %4, i32 0, i32 7
  store i64 %5, ptr %6, align 8, !tbaa !440
  %7 = getelementptr inbounds i8, ptr %4, i32 144
  ret ptr %7
}

; Function Attrs: nounwind optsize
define hidden void @fa_call_free(ptr noundef %0) local_unnamed_addr #3 {
  %2 = getelementptr inbounds i8, ptr %0, i32 -144
  %3 = getelementptr inbounds i8, ptr %0, i32 -112
  %4 = load i64, ptr %3, align 8, !tbaa !440
  %5 = trunc i64 %4 to i32
  tail call void @fa_free(ptr noundef nonnull %2, i32 noundef %5, i32 noundef 16) #62
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_worker_call(ptr noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef %3, ptr noundef %4, ptr noundef %5, i32 noundef %6) local_unnamed_addr #3 {
  %8 = getelementptr inbounds i8, ptr %1, i32 -144
  %9 = getelementptr inbounds i8, ptr %1, i32 -140
  store ptr %0, ptr %9, align 4, !tbaa !445
  %10 = getelementptr inbounds i8, ptr %1, i32 -132
  store ptr %2, ptr %10, align 4, !tbaa !446
  %11 = getelementptr inbounds i8, ptr %1, i32 -120
  store ptr %3, ptr %11, align 8, !tbaa !439
  %12 = getelementptr inbounds i8, ptr %1, i32 -128
  store ptr %4, ptr %12, align 8, !tbaa !438
  %13 = getelementptr inbounds i8, ptr %1, i32 -124
  store ptr %5, ptr %13, align 4, !tbaa !447
  store ptr null, ptr %8, align 8, !tbaa !434
  %14 = icmp eq ptr %0, null
  br i1 %14, label %17, label %15

15:                                               ; preds = %7
  %16 = atomicrmw add ptr %0, i64 1 monotonic, align 8
  br label %17

17:                                               ; preds = %7, %15
  %18 = getelementptr inbounds %struct.FaWorker, ptr %0, i32 0, i32 8
  %19 = load i32, ptr %18, align 4, !tbaa !430
  %20 = icmp eq i32 %19, 0
  br i1 %20, label %26, label %21

21:                                               ; preds = %17
  %22 = getelementptr inbounds %struct.FaWorker, ptr %0, i32 0, i32 10
  %23 = load i32, ptr %22, align 4, !tbaa !427
  %24 = icmp ne i32 %23, 0
  %25 = or i1 %14, %24
  br i1 %25, label %26, label %37

26:                                               ; preds = %21, %17
  %27 = getelementptr inbounds i8, ptr %1, i32 -104
  store i32 1, ptr %27, align 8, !tbaa !448
  %28 = load i32, ptr @fa_blocking_depth, align 4, !tbaa !23
  store i32 1, ptr @fa_blocking_depth, align 4, !tbaa !23
  %29 = getelementptr inbounds i8, ptr %0, i32 256
  tail call void %2(ptr noundef nonnull %29, ptr noundef %3, ptr noundef nonnull %1) #62
  store i32 %28, ptr @fa_blocking_depth, align 4, !tbaa !23
  %30 = getelementptr inbounds i8, ptr %1, i32 32
  %31 = load i32, ptr %30, align 4, !tbaa !286
  %32 = or i32 %31, 2
  store i32 %32, ptr %30, align 4, !tbaa !286
  store i32 2, ptr %27, align 8, !tbaa !448
  store ptr null, ptr %1, align 4, !tbaa !6
  %33 = getelementptr inbounds i8, ptr %1, i32 28
  store atomic i32 3, ptr %33 release, align 4
  %34 = icmp eq i32 %6, 2
  br i1 %34, label %35, label %62

35:                                               ; preds = %26
  %36 = load ptr, ptr %13, align 4, !tbaa !447
  tail call void %36(ptr noundef nonnull %1) #62
  tail call void @fa_shared_release(ptr noundef nonnull %0) #61
  br label %62

37:                                               ; preds = %21
  %38 = icmp ne i32 %6, 0
  br i1 %38, label %39, label %53

39:                                               ; preds = %37
  %40 = tail call fastcc ptr @executor() #61
  %41 = getelementptr inbounds i8, ptr %1, i32 -136
  store ptr %40, ptr %41, align 8, !tbaa !437
  %42 = getelementptr inbounds %struct.FaExecutor, ptr %40, i32 0, i32 12
  %43 = atomicrmw add ptr %42, i64 1 acq_rel, align 8
  %44 = getelementptr inbounds i8, ptr %1, i32 28
  store atomic i32 2, ptr %44 release, align 4
  %45 = icmp eq i32 %6, 2
  br i1 %45, label %46, label %53

46:                                               ; preds = %39
  %47 = getelementptr inbounds i8, ptr %1, i32 32
  %48 = load i32, ptr %47, align 4, !tbaa !286
  %49 = or i32 %48, 1
  store i32 %49, ptr %47, align 4, !tbaa !286
  %50 = getelementptr inbounds %struct.FaExecutor, ptr %40, i32 0, i32 11
  %51 = load i64, ptr %50, align 8, !tbaa !288
  %52 = add nsw i64 %51, 1
  store i64 %52, ptr %50, align 8, !tbaa !288
  br label %53

53:                                               ; preds = %39, %46, %37
  %54 = getelementptr inbounds i8, ptr %1, i32 -104
  store i32 0, ptr %54, align 8, !tbaa !448
  %55 = getelementptr inbounds %struct.FaWorker, ptr %0, i32 0, i32 4
  %56 = load ptr, ptr %55, align 4, !tbaa !432
  %57 = icmp eq ptr %56, null
  br i1 %57, label %59, label %58

58:                                               ; preds = %53
  store ptr %8, ptr %56, align 8, !tbaa !434
  br label %61

59:                                               ; preds = %53
  %60 = getelementptr inbounds %struct.FaWorker, ptr %0, i32 0, i32 3
  store ptr %8, ptr %60, align 16, !tbaa !449
  br label %61

61:                                               ; preds = %59, %58
  store ptr %8, ptr %55, align 4, !tbaa !432
  tail call void @llvm.assume(i1 %38)
  br label %62

62:                                               ; preds = %61, %26, %35
  ret void
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define hidden void @fa_call_resume(ptr noundef %0) local_unnamed_addr #46 {
  store ptr null, ptr %0, align 4, !tbaa !6
  tail call void @fa_task_finish(ptr noundef nonnull %0) #61
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_call_destroy(ptr noundef %0) local_unnamed_addr #3 {
  %2 = getelementptr inbounds i8, ptr %0, i32 -144
  %3 = getelementptr inbounds i8, ptr %0, i32 -140
  %4 = load ptr, ptr %3, align 4, !tbaa !445
  %5 = icmp eq ptr %4, null
  br i1 %5, label %10, label %6

6:                                                ; preds = %1
  %7 = getelementptr inbounds %struct.FaWorker, ptr %4, i32 0, i32 8
  %8 = load i32, ptr %7, align 4, !tbaa !430
  %9 = icmp eq i32 %8, 0
  br i1 %9, label %10, label %13

10:                                               ; preds = %6, %1
  %11 = getelementptr inbounds i8, ptr %0, i32 -124
  %12 = load ptr, ptr %11, align 4, !tbaa !447
  tail call void %12(ptr noundef nonnull %0) #62
  tail call void @fa_shared_release(ptr noundef %4) #61
  br label %77

13:                                               ; preds = %6
  %14 = getelementptr inbounds i8, ptr %0, i32 -104
  %15 = load i32, ptr %14, align 8, !tbaa !448
  switch i32 %15, label %58 [
    i32 0, label %16
    i32 1, label %47
  ]

16:                                               ; preds = %13
  %17 = getelementptr inbounds %struct.FaWorker, ptr %4, i32 0, i32 3
  br label %18

18:                                               ; preds = %18, %16
  %19 = phi ptr [ null, %16 ], [ %21, %18 ]
  %20 = phi ptr [ %17, %16 ], [ %21, %18 ]
  %21 = load ptr, ptr %20, align 4, !tbaa !6
  %22 = icmp eq ptr %21, %2
  br i1 %22, label %23, label %18, !llvm.loop !450

23:                                               ; preds = %18
  %24 = load ptr, ptr %2, align 8, !tbaa !434
  store ptr %24, ptr %20, align 4, !tbaa !6
  %25 = getelementptr inbounds %struct.FaWorker, ptr %4, i32 0, i32 4
  %26 = load ptr, ptr %25, align 4, !tbaa !432
  %27 = icmp eq ptr %26, %2
  br i1 %27, label %28, label %29

28:                                               ; preds = %23
  store ptr %19, ptr %25, align 4, !tbaa !432
  br label %29

29:                                               ; preds = %28, %23
  %30 = getelementptr inbounds i8, ptr %0, i32 -128
  %31 = load ptr, ptr %30, align 8, !tbaa !438
  %32 = getelementptr inbounds i8, ptr %0, i32 -120
  %33 = load ptr, ptr %32, align 8, !tbaa !439
  tail call void %31(ptr noundef %33) #62
  %34 = getelementptr inbounds i8, ptr %0, i32 -136
  %35 = load ptr, ptr %34, align 8, !tbaa !437
  %36 = getelementptr inbounds i8, ptr %0, i32 -112
  %37 = load i64, ptr %36, align 8, !tbaa !440
  %38 = trunc i64 %37 to i32
  tail call void @fa_free(ptr noundef nonnull %2, i32 noundef %38, i32 noundef 16) #62
  %39 = icmp eq ptr %35, null
  br i1 %39, label %46, label %40

40:                                               ; preds = %29
  %41 = getelementptr inbounds %struct.FaExecutor, ptr %35, i32 0, i32 14
  %42 = atomicrmw add ptr %41, i64 1 seq_cst, align 8
  %43 = getelementptr inbounds %struct.FaExecutor, ptr %35, i32 0, i32 12
  %44 = atomicrmw sub ptr %43, i64 1 seq_cst, align 8
  %45 = atomicrmw sub ptr %41, i64 1 seq_cst, align 8
  br label %46

46:                                               ; preds = %40, %29
  tail call void @fa_shared_release(ptr noundef nonnull %4) #61
  br label %77

47:                                               ; preds = %13
  store i32 3, ptr %14, align 8, !tbaa !448
  %48 = getelementptr inbounds i8, ptr %0, i32 -136
  %49 = load ptr, ptr %48, align 8, !tbaa !437
  %50 = icmp eq ptr %49, null
  br i1 %50, label %57, label %51

51:                                               ; preds = %47
  %52 = getelementptr inbounds %struct.FaExecutor, ptr %49, i32 0, i32 14
  %53 = atomicrmw add ptr %52, i64 1 seq_cst, align 8
  %54 = getelementptr inbounds %struct.FaExecutor, ptr %49, i32 0, i32 12
  %55 = atomicrmw sub ptr %54, i64 1 seq_cst, align 8
  %56 = atomicrmw sub ptr %52, i64 1 seq_cst, align 8
  br label %57

57:                                               ; preds = %51, %47
  tail call void @fa_shared_release(ptr noundef nonnull %4) #61
  br label %77

58:                                               ; preds = %13
  %59 = load ptr, ptr @current_executor, align 4, !tbaa !6
  %60 = getelementptr inbounds i8, ptr %0, i32 16
  %61 = icmp eq ptr %59, null
  br i1 %61, label %74, label %62

62:                                               ; preds = %58
  %63 = getelementptr inbounds i8, ptr %0, i32 -136
  %64 = load ptr, ptr %63, align 8, !tbaa !437
  %65 = icmp eq ptr %64, %59
  br i1 %65, label %66, label %74

66:                                               ; preds = %62
  %67 = getelementptr inbounds i8, ptr %0, i32 28
  %68 = load atomic i32, ptr %67 acquire, align 4
  %69 = icmp eq i32 %68, 5
  br i1 %69, label %70, label %74

70:                                               ; preds = %66
  tail call fastcc void @drain_remote(ptr noundef nonnull %59) #61
  %71 = load atomic i32, ptr %67 acquire, align 4
  %72 = icmp eq i32 %71, 1
  br i1 %72, label %73, label %74

73:                                               ; preds = %70
  tail call fastcc void @ready_unlink(ptr noundef nonnull %59, ptr noundef nonnull %60) #61
  br label %74

74:                                               ; preds = %70, %73, %66, %62, %58
  %75 = getelementptr inbounds i8, ptr %0, i32 -124
  %76 = load ptr, ptr %75, align 4, !tbaa !447
  tail call void %76(ptr noundef nonnull %0) #62
  tail call void @fa_shared_release(ptr noundef nonnull %4) #61
  br label %77

77:                                               ; preds = %74, %57, %46, %10
  ret void
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_workers_new(i64 noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3, ptr noundef %4, ptr noundef %5) local_unnamed_addr #3 {
  %7 = tail call i64 @llvm.smax.i64(i64 %0, i64 1)
  %8 = trunc i64 %3 to i32
  %9 = add i32 %8, 384
  %10 = tail call fastcc ptr @shared_new(i32 noundef %9, ptr noundef nonnull @workers_drop) #61
  %11 = getelementptr inbounds %struct.FaWorkerPool, ptr %10, i32 0, i32 13
  store ptr %4, ptr %11, align 8, !tbaa !451
  %12 = getelementptr inbounds %struct.FaWorkerPool, ptr %10, i32 0, i32 12
  store ptr %5, ptr %12, align 4, !tbaa !453
  %13 = tail call i64 @llvm.smax.i64(i64 %1, i64 0)
  %14 = getelementptr inbounds %struct.FaWorkerPool, ptr %10, i32 0, i32 10
  store i64 %13, ptr %14, align 8, !tbaa !454
  %15 = tail call i64 @llvm.smax.i64(i64 %2, i64 0)
  %16 = getelementptr inbounds %struct.FaWorkerPool, ptr %10, i32 0, i32 9
  store i64 %15, ptr %16, align 16, !tbaa !455
  %17 = trunc i64 %7 to i32
  %18 = shl i32 %17, 2
  %19 = tail call ptr @fa_alloc(i32 noundef %18, i32 noundef 4) #62
  %20 = getelementptr inbounds %struct.FaWorkerPool, ptr %10, i32 0, i32 4
  store ptr %19, ptr %20, align 16, !tbaa !456
  %21 = getelementptr inbounds %struct.FaWorkerPool, ptr %10, i32 0, i32 5
  store i64 %7, ptr %21, align 8, !tbaa !457
  ret ptr %10
}

; Function Attrs: nounwind optsize
define internal void @workers_drop(ptr noundef %0) #3 {
  tail call fastcc void @workers_shut(ptr noundef %0) #61
  %2 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i32 0, i32 5
  %3 = load i64, ptr %2, align 8, !tbaa !457
  %4 = icmp sgt i64 %3, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i32 0, i32 4
  br label %16

7:                                                ; preds = %16, %1
  %8 = phi i64 [ %3, %1 ], [ %23, %16 ]
  %9 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i32 0, i32 4
  %10 = load ptr, ptr %9, align 16, !tbaa !456
  %11 = trunc i64 %8 to i32
  %12 = shl i32 %11, 2
  tail call void @fa_free(ptr noundef %10, i32 noundef %12, i32 noundef 4) #62
  %13 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i32 0, i32 13
  %14 = load ptr, ptr %13, align 8, !tbaa !451
  %15 = icmp eq ptr %14, null
  br i1 %15, label %26, label %25

16:                                               ; preds = %5, %16
  %17 = phi i64 [ 0, %5 ], [ %22, %16 ]
  %18 = load ptr, ptr %6, align 16, !tbaa !456
  %19 = trunc i64 %17 to i32
  %20 = getelementptr inbounds ptr, ptr %18, i32 %19
  %21 = load ptr, ptr %20, align 4, !tbaa !6
  tail call void @fa_shared_release(ptr noundef %21) #61
  %22 = add nuw nsw i64 %17, 1
  %23 = load i64, ptr %2, align 8, !tbaa !457
  %24 = icmp slt i64 %22, %23
  br i1 %24, label %16, label %7, !llvm.loop !458

25:                                               ; preds = %7
  tail call void %14(ptr noundef nonnull %0) #62
  br label %26

26:                                               ; preds = %25, %7
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_workers_fill(ptr noundef %0) local_unnamed_addr #3 {
  %2 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i32 0, i32 5
  %3 = load i64, ptr %2, align 8, !tbaa !457
  %4 = icmp sgt i64 %3, 0
  br i1 %4, label %5, label %8

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i32 0, i32 12
  %7 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i32 0, i32 4
  br label %9

8:                                                ; preds = %9, %1
  ret void

9:                                                ; preds = %5, %9
  %10 = phi i64 [ 0, %5 ], [ %16, %9 ]
  %11 = load ptr, ptr %6, align 4, !tbaa !453
  %12 = tail call ptr %11(ptr noundef nonnull %0) #62
  %13 = load ptr, ptr %7, align 16, !tbaa !456
  %14 = trunc i64 %10 to i32
  %15 = getelementptr inbounds ptr, ptr %13, i32 %14
  store ptr %12, ptr %15, align 4, !tbaa !6
  %16 = add nuw nsw i64 %10, 1
  %17 = load i64, ptr %2, align 8, !tbaa !457
  %18 = icmp slt i64 %16, %17
  br i1 %18, label %9, label %8, !llvm.loop !459
}

; Function Attrs: nounwind optsize
define hidden noundef i32 @fa_workers_reserve(ptr nocapture noundef %0, ptr noundef %1, ptr noundef %2) local_unnamed_addr #3 {
  %4 = tail call fastcc ptr @executor() #61
  %5 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i32 0, i32 9
  %6 = load i64, ptr %5, align 16, !tbaa !455
  %7 = icmp eq i64 %6, 0
  %8 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i32 0, i32 8
  %9 = load i64, ptr %8, align 8, !tbaa !460
  %10 = icmp slt i64 %9, %6
  %11 = select i1 %7, i1 true, i1 %10
  br i1 %11, label %12, label %15

12:                                               ; preds = %3
  %13 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i32 0, i32 8
  %14 = add nsw i64 %9, 1
  store i64 %14, ptr %13, align 8, !tbaa !460
  br label %17

15:                                               ; preds = %3
  %16 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i32 0, i32 11
  tail call fastcc void @line_push(ptr noundef nonnull %16, ptr noundef %1, ptr noundef %2, ptr noundef nonnull @.str.37) #61
  br label %17

17:                                               ; preds = %15, %12
  %18 = phi i32 [ 0, %12 ], [ 1, %15 ]
  ret i32 %18
}

; Function Attrs: nounwind optsize
define internal fastcc void @line_push(ptr nocapture noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef %3) unnamed_addr #3 {
  %5 = tail call fastcc ptr @executor() #61
  store ptr %2, ptr %1, align 8, !tbaa !309
  %6 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 1
  store ptr %5, ptr %6, align 4, !tbaa !311
  %7 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 5
  store ptr %3, ptr %7, align 4, !tbaa !461
  %8 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 9
  store i32 0, ptr %8, align 8, !tbaa !328
  %9 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 10
  store i32 0, ptr %9, align 4, !tbaa !329
  %10 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 8
  store i64 -1, ptr %10, align 8, !tbaa !312
  %11 = load ptr, ptr %0, align 4, !tbaa !6
  %12 = icmp eq ptr %11, null
  %13 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 2
  br i1 %12, label %14, label %16

14:                                               ; preds = %4
  store ptr %1, ptr %13, align 8, !tbaa !462
  %15 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 3
  store ptr %1, ptr %15, align 4, !tbaa !463
  br label %21

16:                                               ; preds = %4
  store ptr %11, ptr %13, align 8, !tbaa !462
  %17 = getelementptr inbounds %struct.FaWait, ptr %11, i32 0, i32 3
  %18 = load ptr, ptr %17, align 4, !tbaa !463
  %19 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 3
  store ptr %18, ptr %19, align 4, !tbaa !463
  %20 = getelementptr inbounds %struct.FaWait, ptr %18, i32 0, i32 2
  store ptr %1, ptr %20, align 8, !tbaa !462
  br label %21

21:                                               ; preds = %16, %14
  %22 = phi ptr [ %17, %16 ], [ %0, %14 ]
  store ptr %1, ptr %22, align 4, !tbaa !6
  %23 = getelementptr inbounds i8, ptr %2, i32 28
  store atomic i32 2, ptr %23 release, align 4
  %24 = getelementptr inbounds %struct.FaExecutor, ptr %5, i32 0, i32 26
  store ptr %3, ptr %24, align 4, !tbaa !346
  %25 = getelementptr inbounds %struct.FaExecutor, ptr %5, i32 0, i32 10
  %26 = load ptr, ptr %25, align 8, !tbaa !299
  %27 = icmp eq ptr %26, %2
  %28 = icmp eq ptr %26, null
  %29 = or i1 %27, %28
  br i1 %29, label %30, label %32

30:                                               ; preds = %21
  %31 = getelementptr inbounds %struct.FaExecutor, ptr %5, i32 0, i32 25
  store ptr %3, ptr %31, align 8, !tbaa !300
  br label %32

32:                                               ; preds = %21, %30
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define hidden void @fa_workers_reserve_now(ptr nocapture noundef %0) local_unnamed_addr #19 {
  %2 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i32 0, i32 9
  %3 = load i64, ptr %2, align 16, !tbaa !455
  %4 = icmp slt i64 %3, 1
  %5 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i32 0, i32 8
  %6 = load i64, ptr %5, align 8, !tbaa !460
  %7 = icmp slt i64 %6, %3
  %8 = select i1 %4, i1 true, i1 %7
  br i1 %8, label %10, label %9, !llvm.loop !464

9:                                                ; preds = %1, %9
  br label %9

10:                                               ; preds = %1
  %11 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i32 0, i32 8
  %12 = add nsw i64 %6, 1
  store i64 %12, ptr %11, align 8, !tbaa !460
  ret void
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define hidden void @fa_workers_unreserve(ptr nocapture noundef %0, ptr noundef %1) local_unnamed_addr #46 {
  %3 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 9
  %4 = load i32, ptr %3, align 8, !tbaa !328
  %5 = icmp eq i32 %4, 1
  br i1 %5, label %6, label %7

6:                                                ; preds = %2
  tail call fastcc void @workers_room(ptr noundef %0) #61
  br label %24

7:                                                ; preds = %2
  %8 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i32 0, i32 11
  %9 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 2
  %10 = load ptr, ptr %9, align 8, !tbaa !462
  %11 = icmp eq ptr %10, %1
  br i1 %11, label %20, label %12

12:                                               ; preds = %7
  %13 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 3
  %14 = load ptr, ptr %13, align 4, !tbaa !463
  %15 = getelementptr inbounds %struct.FaWait, ptr %14, i32 0, i32 2
  store ptr %10, ptr %15, align 8, !tbaa !462
  %16 = load ptr, ptr %9, align 8, !tbaa !462
  %17 = getelementptr inbounds %struct.FaWait, ptr %16, i32 0, i32 3
  store ptr %14, ptr %17, align 4, !tbaa !463
  %18 = load ptr, ptr %8, align 4, !tbaa !6
  %19 = icmp eq ptr %18, %1
  br i1 %19, label %20, label %22

20:                                               ; preds = %12, %7
  %21 = phi ptr [ null, %7 ], [ %16, %12 ]
  store ptr %21, ptr %8, align 4, !tbaa !6
  br label %22

22:                                               ; preds = %12, %20
  store ptr null, ptr %9, align 8, !tbaa !462
  %23 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 3
  store ptr null, ptr %23, align 4, !tbaa !463
  br label %24

24:                                               ; preds = %22, %6
  ret void
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define internal fastcc void @workers_room(ptr nocapture noundef %0) unnamed_addr #46 {
  %2 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i32 0, i32 11
  %3 = load ptr, ptr %2, align 4, !tbaa !6
  %4 = icmp eq ptr %3, null
  br i1 %4, label %22, label %5

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaWait, ptr %3, i32 0, i32 2
  %7 = load ptr, ptr %6, align 8, !tbaa !462
  %8 = icmp eq ptr %7, %3
  br i1 %8, label %17, label %9

9:                                                ; preds = %5
  %10 = getelementptr inbounds %struct.FaWait, ptr %3, i32 0, i32 3
  %11 = load ptr, ptr %10, align 4, !tbaa !463
  %12 = getelementptr inbounds %struct.FaWait, ptr %11, i32 0, i32 2
  store ptr %7, ptr %12, align 8, !tbaa !462
  %13 = load ptr, ptr %6, align 8, !tbaa !462
  %14 = getelementptr inbounds %struct.FaWait, ptr %13, i32 0, i32 3
  store ptr %11, ptr %14, align 4, !tbaa !463
  %15 = load ptr, ptr %2, align 4, !tbaa !6
  %16 = icmp eq ptr %15, %3
  br i1 %16, label %17, label %19

17:                                               ; preds = %9, %5
  %18 = phi ptr [ null, %5 ], [ %13, %9 ]
  store ptr %18, ptr %2, align 4, !tbaa !6
  br label %19

19:                                               ; preds = %17, %9
  store ptr null, ptr %6, align 8, !tbaa !462
  %20 = getelementptr inbounds %struct.FaWait, ptr %3, i32 0, i32 3
  store ptr null, ptr %20, align 4, !tbaa !463
  %21 = getelementptr inbounds %struct.FaWait, ptr %3, i32 0, i32 9
  store i32 1, ptr %21, align 8, !tbaa !328
  tail call fastcc void @wake(ptr noundef nonnull %3) #61
  br label %26

22:                                               ; preds = %1
  %23 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i32 0, i32 8
  %24 = load i64, ptr %23, align 8, !tbaa !460
  %25 = add nsw i64 %24, -1
  store i64 %25, ptr %23, align 8, !tbaa !460
  br label %26

26:                                               ; preds = %22, %19
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: write)
define hidden void @fa_worker_start_in_pool(ptr nocapture noundef writeonly %0, ptr noundef %1) local_unnamed_addr #24 {
  %3 = getelementptr inbounds %struct.FaWorker, ptr %0, i32 0, i32 12
  store ptr %1, ptr %3, align 4, !tbaa !431
  ret void
}

; Function Attrs: nounwind optsize
define hidden nonnull ptr @fa_job_new(i64 noundef %0, i64 noundef %1) local_unnamed_addr #3 {
  %3 = trunc i64 %0 to i32
  %4 = add i32 %3, 15
  %5 = and i32 %4, -16
  %6 = trunc i64 %1 to i32
  %7 = add i32 %6, 15
  %8 = and i32 %7, -16
  %9 = add i32 %5, 208
  %10 = add i32 %9, %8
  %11 = tail call ptr @fa_alloc(i32 noundef %10, i32 noundef 16) #62
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(180) %11, i8 0, i32 180, i1 false)
  %12 = zext i32 %10 to i64
  %13 = getelementptr inbounds %struct.FaCall, ptr %11, i32 0, i32 7
  store i64 %12, ptr %13, align 8, !tbaa !440
  %14 = getelementptr inbounds i8, ptr %11, i32 144
  %15 = getelementptr inbounds i8, ptr %14, i32 %5
  %16 = getelementptr inbounds %struct.FaCall, ptr %11, i32 0, i32 12
  store ptr %15, ptr %16, align 8, !tbaa !465
  %17 = getelementptr inbounds i8, ptr %15, i32 %8
  %18 = getelementptr inbounds %struct.FaCall, ptr %11, i32 0, i32 14
  store ptr %17, ptr %18, align 8, !tbaa !466
  %19 = getelementptr inbounds %struct.FaWait, ptr %17, i32 0, i32 8
  store i64 -1, ptr %19, align 8, !tbaa !312
  %20 = getelementptr inbounds %struct.FaWait, ptr %17, i32 0, i32 9
  store i32 2, ptr %20, align 8, !tbaa !328
  %21 = getelementptr inbounds %struct.FaCall, ptr %11, i32 0, i32 13
  store i64 %1, ptr %21, align 8, !tbaa !467
  %22 = getelementptr inbounds %struct.FaCall, ptr %11, i32 0, i32 9
  store i32 2, ptr %22, align 4, !tbaa !468
  ret ptr %14
}

; Function Attrs: nounwind optsize
define hidden void @fa_workers_enqueue(ptr noundef %0, ptr nocapture noundef %1, ptr noundef %2, ptr noundef %3, ptr noundef %4, ptr noundef %5, ptr noundef %6, ptr noundef %7) local_unnamed_addr #3 {
  %9 = tail call fastcc ptr @executor() #61
  %10 = getelementptr inbounds i8, ptr %1, i32 -96
  store ptr %0, ptr %10, align 8, !tbaa !469
  %11 = getelementptr inbounds i8, ptr %1, i32 -132
  store ptr %2, ptr %11, align 4, !tbaa !446
  %12 = getelementptr inbounds i8, ptr %1, i32 -120
  store ptr %3, ptr %12, align 8, !tbaa !439
  %13 = getelementptr inbounds i8, ptr %1, i32 -128
  store ptr %4, ptr %13, align 8, !tbaa !438
  %14 = getelementptr inbounds i8, ptr %1, i32 -124
  store ptr %5, ptr %14, align 4, !tbaa !447
  %15 = getelementptr inbounds i8, ptr %1, i32 -60
  store ptr %6, ptr %15, align 4, !tbaa !470
  %16 = getelementptr inbounds i8, ptr %1, i32 -56
  store ptr %7, ptr %16, align 8, !tbaa !471
  %17 = getelementptr inbounds i8, ptr %1, i32 -136
  store ptr %9, ptr %17, align 8, !tbaa !437
  %18 = icmp eq ptr %0, null
  br i1 %18, label %21, label %19

19:                                               ; preds = %8
  %20 = atomicrmw add ptr %0, i64 1 monotonic, align 8
  br label %21

21:                                               ; preds = %8, %19
  %22 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i32 0, i32 4
  %23 = load ptr, ptr %22, align 16, !tbaa !456
  %24 = load ptr, ptr %23, align 4, !tbaa !6
  %25 = getelementptr inbounds i8, ptr %1, i32 -80
  %26 = load ptr, ptr %25, align 8, !tbaa !465
  tail call void %2(ptr noundef %24, ptr noundef %3, ptr noundef %26) #62
  %27 = getelementptr inbounds i8, ptr %1, i32 36
  %28 = load ptr, ptr %25, align 8, !tbaa !465
  %29 = getelementptr inbounds i8, ptr %1, i32 -72
  %30 = load i64, ptr %29, align 8, !tbaa !467
  %31 = trunc i64 %30 to i32
  tail call fastcc void @copy_bytes(ptr noundef nonnull %27, ptr noundef %28, i32 noundef %31) #61
  %32 = getelementptr inbounds i8, ptr %1, i32 32
  %33 = load i32, ptr %32, align 4, !tbaa !286
  %34 = or i32 %33, 2
  store i32 %34, ptr %32, align 4, !tbaa !286
  %35 = getelementptr inbounds i8, ptr %1, i32 -104
  store i32 2, ptr %35, align 8, !tbaa !448
  store ptr null, ptr %1, align 4, !tbaa !6
  %36 = getelementptr inbounds i8, ptr %1, i32 28
  store atomic i32 3, ptr %36 release, align 4
  %37 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i32 0, i32 8
  %38 = load i64, ptr %37, align 8, !tbaa !460
  %39 = add nsw i64 %38, -1
  store i64 %39, ptr %37, align 8, !tbaa !460
  %40 = getelementptr inbounds i8, ptr %1, i32 -100
  store i32 1, ptr %40, align 4, !tbaa !468
  ret void
}

; Function Attrs: nofree norecurse nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden void @fa_job_resume(ptr noundef %0) local_unnamed_addr #33 {
  %2 = getelementptr inbounds i8, ptr %0, i32 -144
  tail call fastcc void @job_disarm(ptr noundef nonnull %2) #61
  store ptr null, ptr %0, align 4, !tbaa !6
  tail call void @fa_task_finish(ptr noundef nonnull %0) #61
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc void @job_disarm(ptr nocapture noundef readonly %0) unnamed_addr #20 {
  %2 = getelementptr inbounds %struct.FaCall, ptr %0, i32 0, i32 14
  %3 = load ptr, ptr %2, align 8, !tbaa !466
  %4 = getelementptr inbounds %struct.FaWait, ptr %3, i32 0, i32 9
  %5 = load i32, ptr %4, align 8, !tbaa !328
  %6 = icmp eq i32 %5, 0
  br i1 %6, label %7, label %14

7:                                                ; preds = %1
  %8 = getelementptr inbounds %struct.FaWait, ptr %3, i32 0, i32 8
  %9 = load i64, ptr %8, align 8, !tbaa !312
  %10 = icmp sgt i64 %9, -1
  br i1 %10, label %11, label %14

11:                                               ; preds = %7
  %12 = getelementptr inbounds %struct.FaWait, ptr %3, i32 0, i32 1
  %13 = load ptr, ptr %12, align 4, !tbaa !311
  tail call fastcc void @timer_remove(ptr noundef %13, i64 noundef %9) #61
  br label %14

14:                                               ; preds = %11, %7, %1
  store i32 2, ptr %4, align 8, !tbaa !328
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_job_destroy(ptr noundef %0) local_unnamed_addr #3 {
  %2 = getelementptr inbounds i8, ptr %0, i32 -144
  %3 = getelementptr inbounds i8, ptr %0, i32 -96
  %4 = load ptr, ptr %3, align 8, !tbaa !469
  tail call fastcc void @job_disarm(ptr noundef nonnull %2) #61
  %5 = getelementptr inbounds i8, ptr %0, i32 -104
  %6 = load i32, ptr %5, align 8, !tbaa !448
  switch i32 %6, label %46 [
    i32 0, label %7
    i32 1, label %38
  ]

7:                                                ; preds = %1
  %8 = getelementptr inbounds %struct.FaWorkerPool, ptr %4, i32 0, i32 6
  br label %9

9:                                                ; preds = %9, %7
  %10 = phi ptr [ null, %7 ], [ %12, %9 ]
  %11 = phi ptr [ %8, %7 ], [ %12, %9 ]
  %12 = load ptr, ptr %11, align 4, !tbaa !6
  %13 = icmp eq ptr %12, %2
  br i1 %13, label %14, label %9, !llvm.loop !472

14:                                               ; preds = %9
  %15 = load ptr, ptr %2, align 8, !tbaa !434
  store ptr %15, ptr %11, align 4, !tbaa !6
  %16 = getelementptr inbounds %struct.FaWorkerPool, ptr %4, i32 0, i32 7
  %17 = load ptr, ptr %16, align 4, !tbaa !473
  %18 = icmp eq ptr %17, %2
  br i1 %18, label %19, label %20

19:                                               ; preds = %14
  store ptr %10, ptr %16, align 4, !tbaa !473
  br label %20

20:                                               ; preds = %19, %14
  tail call fastcc void @workers_room(ptr noundef nonnull %4) #61
  %21 = getelementptr inbounds i8, ptr %0, i32 -128
  %22 = load ptr, ptr %21, align 8, !tbaa !438
  %23 = getelementptr inbounds i8, ptr %0, i32 -120
  %24 = load ptr, ptr %23, align 8, !tbaa !439
  tail call void %22(ptr noundef %24) #62
  %25 = getelementptr inbounds i8, ptr %0, i32 -136
  %26 = load ptr, ptr %25, align 8, !tbaa !437
  %27 = getelementptr inbounds %struct.FaExecutor, ptr %26, i32 0, i32 14
  %28 = atomicrmw add ptr %27, i64 1 seq_cst, align 8
  %29 = getelementptr inbounds %struct.FaExecutor, ptr %26, i32 0, i32 12
  %30 = atomicrmw sub ptr %29, i64 1 seq_cst, align 8
  %31 = atomicrmw sub ptr %27, i64 1 seq_cst, align 8
  %32 = getelementptr inbounds i8, ptr %0, i32 -100
  %33 = atomicrmw sub ptr %32, i32 1 acq_rel, align 4
  %34 = icmp eq i32 %33, 1
  br i1 %34, label %35, label %62

35:                                               ; preds = %20
  %36 = getelementptr inbounds i8, ptr %0, i32 -124
  %37 = load ptr, ptr %36, align 4, !tbaa !447
  tail call void %37(ptr noundef nonnull %0) #62
  br label %62

38:                                               ; preds = %1
  store i32 3, ptr %5, align 8, !tbaa !448
  %39 = getelementptr inbounds i8, ptr %0, i32 -136
  %40 = load ptr, ptr %39, align 8, !tbaa !437
  %41 = getelementptr inbounds %struct.FaExecutor, ptr %40, i32 0, i32 14
  %42 = atomicrmw add ptr %41, i64 1 seq_cst, align 8
  %43 = getelementptr inbounds %struct.FaExecutor, ptr %40, i32 0, i32 12
  %44 = atomicrmw sub ptr %43, i64 1 seq_cst, align 8
  %45 = atomicrmw sub ptr %41, i64 1 seq_cst, align 8
  br label %62

46:                                               ; preds = %1
  %47 = load ptr, ptr @current_executor, align 4, !tbaa !6
  %48 = getelementptr inbounds i8, ptr %0, i32 16
  %49 = icmp eq ptr %47, null
  br i1 %49, label %62, label %50

50:                                               ; preds = %46
  %51 = getelementptr inbounds i8, ptr %0, i32 -136
  %52 = load ptr, ptr %51, align 8, !tbaa !437
  %53 = icmp eq ptr %52, %47
  br i1 %53, label %54, label %62

54:                                               ; preds = %50
  %55 = getelementptr inbounds i8, ptr %0, i32 28
  %56 = load atomic i32, ptr %55 acquire, align 4
  %57 = icmp eq i32 %56, 5
  br i1 %57, label %58, label %62

58:                                               ; preds = %54
  tail call fastcc void @drain_remote(ptr noundef nonnull %47) #61
  %59 = load atomic i32, ptr %55 acquire, align 4
  %60 = icmp eq i32 %59, 1
  br i1 %60, label %61, label %62

61:                                               ; preds = %58
  tail call fastcc void @ready_unlink(ptr noundef nonnull %47, ptr noundef nonnull %48) #61
  br label %62

62:                                               ; preds = %35, %20, %46, %50, %54, %61, %58, %38
  %63 = getelementptr inbounds i8, ptr %0, i32 -100
  %64 = atomicrmw sub ptr %63, i32 1 acq_rel, align 4
  %65 = icmp eq i32 %64, 1
  br i1 %65, label %66, label %69

66:                                               ; preds = %62
  %67 = getelementptr inbounds i8, ptr %0, i32 -124
  %68 = load ptr, ptr %67, align 4, !tbaa !447
  tail call void %68(ptr noundef nonnull %0) #62
  br label %69

69:                                               ; preds = %62, %66
  tail call void @fa_shared_release(ptr noundef %4) #61
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_workers_shutdown(ptr nocapture noundef %0) local_unnamed_addr #3 {
  tail call fastcc void @workers_shut(ptr noundef %0) #61
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @workers_shut(ptr nocapture noundef %0) unnamed_addr #3 {
  %2 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i32 0, i32 14
  %3 = load i32, ptr %2, align 4, !tbaa !474
  %4 = icmp eq i32 %3, 0
  br i1 %4, label %5, label %20

5:                                                ; preds = %1
  store i32 1, ptr %2, align 4, !tbaa !474
  %6 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i32 0, i32 5
  %7 = load i64, ptr %6, align 8, !tbaa !457
  %8 = icmp sgt i64 %7, 0
  br i1 %8, label %9, label %20

9:                                                ; preds = %5
  %10 = getelementptr inbounds %struct.FaWorkerPool, ptr %0, i32 0, i32 4
  br label %11

11:                                               ; preds = %9, %11
  %12 = phi i64 [ 0, %9 ], [ %17, %11 ]
  %13 = load ptr, ptr %10, align 16, !tbaa !456
  %14 = trunc i64 %12 to i32
  %15 = getelementptr inbounds ptr, ptr %13, i32 %14
  %16 = load ptr, ptr %15, align 4, !tbaa !6
  tail call void @fa_worker_stop(ptr noundef %16) #61
  %17 = add nuw nsw i64 %12, 1
  %18 = load i64, ptr %6, align 8, !tbaa !457
  %19 = icmp slt i64 %17, %18
  br i1 %19, label %11, label %20, !llvm.loop !475

20:                                               ; preds = %11, %5, %1
  ret void
}

; Function Attrs: cold mustprogress nofree noinline norecurse nosync nounwind optsize willreturn memory(none)
define hidden noundef nonnull ptr @fa_error_worker_timeout() local_unnamed_addr #39 {
  ret ptr @worker_timeout
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_mutex_new(i64 noundef %0, ptr noundef %1) local_unnamed_addr #3 {
  %3 = trunc i64 %0 to i32
  %4 = add i32 %3, 176
  %5 = tail call fastcc ptr @shared_new(i32 noundef %4, ptr noundef %1) #61
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
  tail call fastcc void @deadlock(ptr noundef nonnull @.str.38) #67
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
  br i1 %11, label %12, label %46

12:                                               ; preds = %8
  %13 = getelementptr inbounds %struct.FaMutex, ptr %0, i32 0, i32 1
  %14 = atomicrmw xchg ptr %13, i32 1 acquire, align 16
  %15 = icmp eq i32 %14, 0
  br i1 %15, label %16, label %46

16:                                               ; preds = %12
  %17 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 9
  store i32 1, ptr %17, align 8, !tbaa !328
  %18 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 10
  store i32 0, ptr %18, align 4, !tbaa !329
  %19 = load i32, ptr @fa_coop_budget, align 4, !tbaa !23
  %20 = add nsw i32 %19, -1
  store i32 %20, ptr @fa_coop_budget, align 4, !tbaa !23
  %21 = icmp sgt i32 %19, 1
  br i1 %21, label %47, label %22

22:                                               ; preds = %16
  %23 = load ptr, ptr @current_executor, align 4, !tbaa !6
  %24 = icmp eq ptr %23, null
  br i1 %24, label %27, label %25

25:                                               ; preds = %22
  %26 = getelementptr inbounds %struct.FaExecutor, ptr %23, i32 0, i32 23
  store i32 1, ptr %26, align 8, !tbaa !307
  br label %27

27:                                               ; preds = %25, %22
  %28 = tail call fastcc ptr @executor() #61
  %29 = getelementptr inbounds %struct.FaExecutor, ptr %28, i32 0, i32 23
  store i32 1, ptr %29, align 8, !tbaa !307
  %30 = getelementptr inbounds i8, ptr %2, i32 16
  %31 = getelementptr inbounds i8, ptr %2, i32 32
  %32 = load i32, ptr %31, align 4, !tbaa !286
  %33 = or i32 %32, 4
  store i32 %33, ptr %31, align 4, !tbaa !286
  %34 = getelementptr inbounds %struct.FaExecutor, ptr %28, i32 0, i32 2
  %35 = getelementptr inbounds %struct.FaExecutor, ptr %28, i32 0, i32 3
  %36 = getelementptr inbounds i8, ptr %2, i32 20
  store ptr null, ptr %36, align 4, !tbaa !290
  %37 = load ptr, ptr %35, align 4, !tbaa !6
  %38 = getelementptr inbounds i8, ptr %2, i32 24
  store ptr %37, ptr %38, align 4, !tbaa !291
  %39 = icmp eq ptr %37, null
  %40 = getelementptr inbounds %struct.FaTask, ptr %37, i32 0, i32 1
  %41 = select i1 %39, ptr %34, ptr %40
  store ptr %30, ptr %41, align 4, !tbaa !6
  store ptr %30, ptr %35, align 4, !tbaa !6
  %42 = getelementptr inbounds %struct.FaExecutor, ptr %28, i32 0, i32 4
  %43 = load i64, ptr %42, align 8, !tbaa !292
  %44 = add nsw i64 %43, 1
  store i64 %44, ptr %42, align 8, !tbaa !292
  %45 = getelementptr inbounds i8, ptr %2, i32 28
  store atomic i32 1, ptr %45 release, align 4
  br label %47

46:                                               ; preds = %12, %8
  tail call fastcc void @line_push(ptr noundef nonnull %9, ptr noundef %1, ptr noundef %2, ptr noundef %3) #61
  br label %47

47:                                               ; preds = %16, %46, %27
  %48 = phi i32 [ 2, %27 ], [ 1, %46 ], [ 0, %16 ]
  ret i32 %48
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(argmem: readwrite)
define hidden noundef ptr @fa_mutex_try_lock(ptr noundef %0) local_unnamed_addr #45 {
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
  %9 = load ptr, ptr %8, align 8, !tbaa !462
  %10 = icmp eq ptr %9, %5
  br i1 %10, label %19, label %11

11:                                               ; preds = %7
  %12 = getelementptr inbounds %struct.FaWait, ptr %5, i32 0, i32 3
  %13 = load ptr, ptr %12, align 4, !tbaa !463
  %14 = getelementptr inbounds %struct.FaWait, ptr %13, i32 0, i32 2
  store ptr %9, ptr %14, align 8, !tbaa !462
  %15 = load ptr, ptr %8, align 8, !tbaa !462
  %16 = getelementptr inbounds %struct.FaWait, ptr %15, i32 0, i32 3
  store ptr %13, ptr %16, align 4, !tbaa !463
  %17 = load ptr, ptr %4, align 4, !tbaa !6
  %18 = icmp eq ptr %17, %5
  br i1 %18, label %19, label %21

19:                                               ; preds = %11, %7
  %20 = phi ptr [ null, %7 ], [ %15, %11 ]
  store ptr %20, ptr %4, align 4, !tbaa !6
  br label %21

21:                                               ; preds = %19, %11
  store ptr null, ptr %8, align 8, !tbaa !462
  %22 = getelementptr inbounds %struct.FaWait, ptr %5, i32 0, i32 3
  store ptr null, ptr %22, align 4, !tbaa !463
  %23 = getelementptr inbounds %struct.FaWait, ptr %5, i32 0, i32 9
  store i32 1, ptr %23, align 8, !tbaa !328
  tail call fastcc void @wake(ptr noundef nonnull %5) #61
  br label %26

24:                                               ; preds = %3
  %25 = getelementptr inbounds %struct.FaMutex, ptr %0, i32 0, i32 1
  store atomic i32 0, ptr %25 release, align 16
  br label %26

26:                                               ; preds = %24, %21
  tail call void @fa_shared_release(ptr noundef nonnull %0) #61
  br label %27

27:                                               ; preds = %1, %26
  ret void
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define internal fastcc void @wake(ptr nocapture noundef %0) unnamed_addr #46 {
  %2 = load ptr, ptr %0, align 8, !tbaa !309
  %3 = getelementptr inbounds i8, ptr %2, i32 16
  %4 = getelementptr inbounds %struct.FaWait, ptr %0, i32 0, i32 1
  %5 = load ptr, ptr %4, align 4, !tbaa !311
  %6 = load ptr, ptr @current_executor, align 4, !tbaa !6
  %7 = icmp eq ptr %5, %6
  br i1 %7, label %8, label %26

8:                                                ; preds = %1
  %9 = getelementptr inbounds i8, ptr %2, i32 32
  %10 = load i32, ptr %9, align 4, !tbaa !286
  %11 = and i32 %10, -5
  store i32 %11, ptr %9, align 4, !tbaa !286
  %12 = getelementptr inbounds %struct.FaExecutor, ptr %5, i32 0, i32 1
  %13 = getelementptr inbounds i8, ptr %2, i32 20
  store ptr null, ptr %13, align 4, !tbaa !290
  %14 = load ptr, ptr %12, align 4, !tbaa !6
  %15 = getelementptr inbounds i8, ptr %2, i32 24
  store ptr %14, ptr %15, align 4, !tbaa !291
  %16 = icmp eq ptr %14, null
  %17 = getelementptr inbounds %struct.FaTask, ptr %14, i32 0, i32 1
  %18 = select i1 %16, ptr %5, ptr %17
  store ptr %3, ptr %18, align 4, !tbaa !6
  store ptr %3, ptr %12, align 4, !tbaa !6
  %19 = getelementptr inbounds %struct.FaExecutor, ptr %5, i32 0, i32 4
  %20 = load i64, ptr %19, align 8, !tbaa !292
  %21 = add nsw i64 %20, 1
  store i64 %21, ptr %19, align 8, !tbaa !292
  %22 = getelementptr inbounds %struct.FaExecutor, ptr %5, i32 0, i32 5
  %23 = load i64, ptr %22, align 8, !tbaa !293
  %24 = add nsw i64 %23, 1
  store i64 %24, ptr %22, align 8, !tbaa !293
  %25 = getelementptr inbounds i8, ptr %2, i32 28
  store atomic i32 1, ptr %25 release, align 4
  br label %35

26:                                               ; preds = %1
  %27 = getelementptr inbounds %struct.FaWait, ptr %0, i32 0, i32 10
  store i32 1, ptr %27, align 4, !tbaa !329
  %28 = getelementptr inbounds i8, ptr %2, i32 28
  store atomic i32 5, ptr %28 release, align 4
  %29 = getelementptr inbounds i8, ptr %2, i32 20
  store atomic i32 0, ptr %29 monotonic, align 4
  %30 = getelementptr inbounds %struct.FaExecutor, ptr %5, i32 0, i32 16
  %31 = ptrtoint ptr %3 to i32
  %32 = atomicrmw xchg ptr %30, i32 %31 acq_rel, align 4
  %33 = inttoptr i32 %32 to ptr
  %34 = getelementptr inbounds %struct.FaTask, ptr %33, i32 0, i32 1
  store atomic i32 %31, ptr %34 release, align 4
  br label %35

35:                                               ; preds = %26, %8
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_mutex_cancel(ptr noundef %0, ptr noundef %1) local_unnamed_addr #3 {
  %3 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 9
  %4 = load i32, ptr %3, align 8, !tbaa !328
  %5 = icmp eq i32 %4, 0
  br i1 %5, label %6, label %23

6:                                                ; preds = %2
  %7 = getelementptr inbounds %struct.FaMutex, ptr %0, i32 0, i32 3
  %8 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 2
  %9 = load ptr, ptr %8, align 8, !tbaa !462
  %10 = icmp eq ptr %9, %1
  br i1 %10, label %19, label %11

11:                                               ; preds = %6
  %12 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 3
  %13 = load ptr, ptr %12, align 4, !tbaa !463
  %14 = getelementptr inbounds %struct.FaWait, ptr %13, i32 0, i32 2
  store ptr %9, ptr %14, align 8, !tbaa !462
  %15 = load ptr, ptr %8, align 8, !tbaa !462
  %16 = getelementptr inbounds %struct.FaWait, ptr %15, i32 0, i32 3
  store ptr %13, ptr %16, align 4, !tbaa !463
  %17 = load ptr, ptr %7, align 4, !tbaa !6
  %18 = icmp eq ptr %17, %1
  br i1 %18, label %19, label %21

19:                                               ; preds = %11, %6
  %20 = phi ptr [ null, %6 ], [ %15, %11 ]
  store ptr %20, ptr %7, align 4, !tbaa !6
  br label %21

21:                                               ; preds = %11, %19
  store ptr null, ptr %8, align 8, !tbaa !462
  %22 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 3
  store ptr null, ptr %22, align 4, !tbaa !463
  tail call void @fa_shared_release(ptr noundef nonnull %0) #61
  br label %24

23:                                               ; preds = %2
  tail call fastcc void @forget_remote_wake(ptr noundef nonnull %1) #61
  tail call void @fa_mutex_unlock(ptr noundef %0) #61
  br label %24

24:                                               ; preds = %23, %21
  ret void
}

; Function Attrs: nofree norecurse nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc void @forget_remote_wake(ptr nocapture noundef readonly %0) unnamed_addr #33 {
  %2 = getelementptr inbounds %struct.FaWait, ptr %0, i32 0, i32 10
  %3 = load i32, ptr %2, align 4, !tbaa !329
  %4 = icmp eq i32 %3, 0
  br i1 %4, label %13, label %5

5:                                                ; preds = %1
  %6 = load ptr, ptr @current_executor, align 4, !tbaa !6
  tail call fastcc void @drain_remote(ptr noundef %6) #61
  %7 = load ptr, ptr %0, align 8, !tbaa !309
  %8 = getelementptr inbounds i8, ptr %7, i32 28
  %9 = load atomic i32, ptr %8 acquire, align 4
  %10 = icmp eq i32 %9, 1
  br i1 %10, label %11, label %13

11:                                               ; preds = %5
  %12 = getelementptr inbounds i8, ptr %7, i32 16
  tail call fastcc void @ready_unlink(ptr noundef %6, ptr noundef nonnull %12) #61
  br label %13

13:                                               ; preds = %5, %11, %1
  ret void
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_channel_new(i64 noundef %0, i64 noundef %1, ptr noundef %2) local_unnamed_addr #3 {
  %4 = tail call fastcc ptr @shared_new(i32 noundef 240, ptr noundef nonnull @channel_drop) #61
  %5 = tail call i64 @llvm.umin.i64(i64 %0, i64 9223372036854775807)
  %6 = getelementptr inbounds %struct.FaChannel, ptr %4, i32 0, i32 3
  store i64 %5, ptr %6, align 16, !tbaa !476
  %7 = tail call i64 @llvm.smax.i64(i64 %1, i64 1)
  %8 = getelementptr inbounds %struct.FaChannel, ptr %4, i32 0, i32 4
  store i64 %7, ptr %8, align 8, !tbaa !478
  %9 = getelementptr inbounds %struct.FaChannel, ptr %4, i32 0, i32 11
  store ptr %2, ptr %9, align 8, !tbaa !479
  ret ptr %4
}

; Function Attrs: nounwind optsize
define internal void @channel_drop(ptr nocapture noundef readonly %0) #3 {
  %2 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 7
  %3 = load i64, ptr %2, align 16, !tbaa !480
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
  %13 = load i64, ptr %12, align 8, !tbaa !481
  %14 = icmp sgt i64 %13, 0
  br i1 %14, label %35, label %42

15:                                               ; preds = %5, %31
  %16 = phi i64 [ %3, %5 ], [ %32, %31 ]
  %17 = phi i64 [ 0, %5 ], [ %33, %31 ]
  %18 = load ptr, ptr %6, align 8, !tbaa !479
  %19 = icmp eq ptr %18, null
  br i1 %19, label %31, label %20

20:                                               ; preds = %15
  %21 = load i64, ptr %7, align 8, !tbaa !482
  %22 = add nsw i64 %21, %17
  %23 = load i64, ptr %8, align 8, !tbaa !481
  %24 = srem i64 %22, %23
  %25 = load ptr, ptr %9, align 16, !tbaa !483
  %26 = load i64, ptr %10, align 8, !tbaa !478
  %27 = mul nsw i64 %26, %24
  %28 = trunc i64 %27 to i32
  %29 = getelementptr inbounds i8, ptr %25, i32 %28
  tail call void %18(ptr noundef %29) #62
  %30 = load i64, ptr %2, align 16, !tbaa !480
  br label %31

31:                                               ; preds = %20, %15
  %32 = phi i64 [ %30, %20 ], [ %16, %15 ]
  %33 = add nuw nsw i64 %17, 1
  %34 = icmp slt i64 %33, %32
  br i1 %34, label %15, label %11, !llvm.loop !484

35:                                               ; preds = %11
  %36 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 5
  %37 = load ptr, ptr %36, align 16, !tbaa !483
  %38 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 4
  %39 = load i64, ptr %38, align 8, !tbaa !478
  %40 = mul nsw i64 %39, %13
  %41 = trunc i64 %40 to i32
  tail call void @fa_free(ptr noundef %37, i32 noundef %41, i32 noundef 16) #62
  br label %42

42:                                               ; preds = %35, %11
  ret void
}

; Function Attrs: nounwind optsize
define hidden noundef i32 @fa_channel_send(ptr nocapture noundef %0, ptr nocapture noundef readonly %1, i32 noundef %2) local_unnamed_addr #3 {
  %4 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 9
  %5 = load i32, ptr %4, align 16, !tbaa !485
  %6 = icmp eq i32 %5, 0
  br i1 %6, label %7, label %13

7:                                                ; preds = %3
  %8 = tail call fastcc i32 @channel_send_now(ptr noundef nonnull %0, ptr noundef %1) #61, !range !50
  %9 = icmp eq i32 %8, 0
  br i1 %9, label %10, label %13

10:                                               ; preds = %7
  %11 = icmp eq i32 %2, 0
  br i1 %11, label %13, label %12

12:                                               ; preds = %10
  tail call fastcc void @deadlock(ptr noundef nonnull @.str.39) #67
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
  %8 = load ptr, ptr %7, align 8, !tbaa !462
  %9 = icmp eq ptr %8, %4
  br i1 %9, label %18, label %10

10:                                               ; preds = %6
  %11 = getelementptr inbounds %struct.FaWait, ptr %4, i32 0, i32 3
  %12 = load ptr, ptr %11, align 4, !tbaa !463
  %13 = getelementptr inbounds %struct.FaWait, ptr %12, i32 0, i32 2
  store ptr %8, ptr %13, align 8, !tbaa !462
  %14 = load ptr, ptr %7, align 8, !tbaa !462
  %15 = getelementptr inbounds %struct.FaWait, ptr %14, i32 0, i32 3
  store ptr %12, ptr %15, align 4, !tbaa !463
  %16 = load ptr, ptr %3, align 4, !tbaa !6
  %17 = icmp eq ptr %16, %4
  br i1 %17, label %18, label %20

18:                                               ; preds = %10, %6
  %19 = phi ptr [ null, %6 ], [ %14, %10 ]
  store ptr %19, ptr %3, align 4, !tbaa !6
  br label %20

20:                                               ; preds = %18, %10
  store ptr null, ptr %7, align 8, !tbaa !462
  %21 = getelementptr inbounds %struct.FaWait, ptr %4, i32 0, i32 3
  store ptr null, ptr %21, align 4, !tbaa !463
  %22 = getelementptr inbounds %struct.FaWait, ptr %4, i32 0, i32 4
  %23 = load ptr, ptr %22, align 8, !tbaa !486
  %24 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 4
  %25 = load i64, ptr %24, align 8, !tbaa !478
  %26 = trunc i64 %25 to i32
  tail call fastcc void @copy_bytes(ptr noundef %23, ptr noundef %1, i32 noundef %26) #61
  %27 = getelementptr inbounds %struct.FaWait, ptr %4, i32 0, i32 9
  store i32 1, ptr %27, align 8, !tbaa !328
  tail call fastcc void @wake(ptr noundef nonnull %4) #61
  br label %35

28:                                               ; preds = %2
  %29 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 7
  %30 = load i64, ptr %29, align 16, !tbaa !480
  %31 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 3
  %32 = load i64, ptr %31, align 16, !tbaa !476
  %33 = icmp slt i64 %30, %32
  br i1 %33, label %34, label %35

34:                                               ; preds = %28
  tail call fastcc void @channel_push(ptr noundef nonnull %0, ptr noundef %1) #61
  br label %35

35:                                               ; preds = %28, %34, %20
  %36 = phi i32 [ 1, %20 ], [ 1, %34 ], [ 0, %28 ]
  ret i32 %36
}

; Function Attrs: nounwind optsize
define hidden noundef i32 @fa_channel_send_async(ptr nocapture noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef %3, ptr noundef %4) local_unnamed_addr #3 {
  %6 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 9
  %7 = load i32, ptr %6, align 16, !tbaa !485
  %8 = icmp eq i32 %7, 0
  br i1 %8, label %9, label %45

9:                                                ; preds = %5
  %10 = tail call fastcc i32 @channel_send_now(ptr noundef nonnull %0, ptr noundef %1) #61, !range !50
  %11 = icmp eq i32 %10, 0
  br i1 %11, label %42, label %12

12:                                               ; preds = %9
  %13 = getelementptr inbounds %struct.FaWait, ptr %2, i32 0, i32 9
  store i32 1, ptr %13, align 8, !tbaa !328
  %14 = getelementptr inbounds %struct.FaWait, ptr %2, i32 0, i32 10
  store i32 0, ptr %14, align 4, !tbaa !329
  %15 = load i32, ptr @fa_coop_budget, align 4, !tbaa !23
  %16 = add nsw i32 %15, -1
  store i32 %16, ptr @fa_coop_budget, align 4, !tbaa !23
  %17 = icmp sgt i32 %15, 1
  br i1 %17, label %45, label %18

18:                                               ; preds = %12
  %19 = load ptr, ptr @current_executor, align 4, !tbaa !6
  %20 = icmp eq ptr %19, null
  br i1 %20, label %23, label %21

21:                                               ; preds = %18
  %22 = getelementptr inbounds %struct.FaExecutor, ptr %19, i32 0, i32 23
  store i32 1, ptr %22, align 8, !tbaa !307
  br label %23

23:                                               ; preds = %21, %18
  %24 = tail call fastcc ptr @executor() #61
  %25 = getelementptr inbounds %struct.FaExecutor, ptr %24, i32 0, i32 23
  store i32 1, ptr %25, align 8, !tbaa !307
  %26 = getelementptr inbounds i8, ptr %3, i32 16
  %27 = getelementptr inbounds i8, ptr %3, i32 32
  %28 = load i32, ptr %27, align 4, !tbaa !286
  %29 = or i32 %28, 4
  store i32 %29, ptr %27, align 4, !tbaa !286
  %30 = getelementptr inbounds %struct.FaExecutor, ptr %24, i32 0, i32 2
  %31 = getelementptr inbounds %struct.FaExecutor, ptr %24, i32 0, i32 3
  %32 = getelementptr inbounds i8, ptr %3, i32 20
  store ptr null, ptr %32, align 4, !tbaa !290
  %33 = load ptr, ptr %31, align 4, !tbaa !6
  %34 = getelementptr inbounds i8, ptr %3, i32 24
  store ptr %33, ptr %34, align 4, !tbaa !291
  %35 = icmp eq ptr %33, null
  %36 = getelementptr inbounds %struct.FaTask, ptr %33, i32 0, i32 1
  %37 = select i1 %35, ptr %30, ptr %36
  store ptr %26, ptr %37, align 4, !tbaa !6
  store ptr %26, ptr %31, align 4, !tbaa !6
  %38 = getelementptr inbounds %struct.FaExecutor, ptr %24, i32 0, i32 4
  %39 = load i64, ptr %38, align 8, !tbaa !292
  %40 = add nsw i64 %39, 1
  store i64 %40, ptr %38, align 8, !tbaa !292
  %41 = getelementptr inbounds i8, ptr %3, i32 28
  store atomic i32 1, ptr %41 release, align 4
  br label %45

42:                                               ; preds = %9
  %43 = getelementptr inbounds %struct.FaWait, ptr %2, i32 0, i32 4
  store ptr %1, ptr %43, align 8, !tbaa !486
  %44 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 12
  tail call fastcc void @line_push(ptr noundef nonnull %44, ptr noundef %2, ptr noundef %3, ptr noundef %4) #61
  br label %45

45:                                               ; preds = %12, %5, %42, %23
  %46 = phi i32 [ 2, %23 ], [ 1, %42 ], [ 3, %5 ], [ 0, %12 ]
  ret i32 %46
}

; Function Attrs: nounwind optsize
define hidden noundef i32 @fa_channel_receive(ptr nocapture noundef %0, ptr nocapture noundef writeonly %1, i32 noundef %2) local_unnamed_addr #3 {
  %4 = tail call fastcc i32 @channel_receive_now(ptr noundef %0, ptr noundef %1) #61, !range !50
  %5 = icmp eq i32 %4, 0
  br i1 %5, label %6, label %13

6:                                                ; preds = %3
  %7 = icmp eq i32 %2, 0
  br i1 %7, label %13, label %8

8:                                                ; preds = %6
  %9 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 9
  %10 = load i32, ptr %9, align 16, !tbaa !485
  %11 = icmp eq i32 %10, 0
  br i1 %11, label %12, label %13

12:                                               ; preds = %8
  tail call fastcc void @deadlock(ptr noundef nonnull @.str.40) #67
  unreachable

13:                                               ; preds = %6, %8, %3
  %14 = phi i32 [ 1, %3 ], [ 0, %8 ], [ 0, %6 ]
  ret i32 %14
}

; Function Attrs: nounwind optsize
define internal fastcc noundef i32 @channel_receive_now(ptr nocapture noundef %0, ptr nocapture noundef writeonly %1) unnamed_addr #3 {
  %3 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 7
  %4 = load i64, ptr %3, align 16, !tbaa !480
  %5 = icmp sgt i64 %4, 0
  br i1 %5, label %6, label %45

6:                                                ; preds = %2
  %7 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 5
  %8 = load ptr, ptr %7, align 16, !tbaa !483
  %9 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 6
  %10 = load i64, ptr %9, align 8, !tbaa !482
  %11 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 4
  %12 = load i64, ptr %11, align 8, !tbaa !478
  %13 = mul nsw i64 %12, %10
  %14 = trunc i64 %13 to i32
  %15 = getelementptr inbounds i8, ptr %8, i32 %14
  %16 = trunc i64 %12 to i32
  tail call fastcc void @copy_bytes(ptr noundef %1, ptr noundef %15, i32 noundef %16) #61
  %17 = load i64, ptr %9, align 8, !tbaa !482
  %18 = add nsw i64 %17, 1
  %19 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 8
  %20 = load i64, ptr %19, align 8, !tbaa !481
  %21 = srem i64 %18, %20
  store i64 %21, ptr %9, align 8, !tbaa !482
  %22 = load i64, ptr %3, align 16, !tbaa !480
  %23 = add nsw i64 %22, -1
  store i64 %23, ptr %3, align 16, !tbaa !480
  %24 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 12
  %25 = load ptr, ptr %24, align 4, !tbaa !6
  %26 = icmp eq ptr %25, null
  br i1 %26, label %73, label %27

27:                                               ; preds = %6
  %28 = getelementptr inbounds %struct.FaWait, ptr %25, i32 0, i32 2
  %29 = load ptr, ptr %28, align 8, !tbaa !462
  %30 = icmp eq ptr %29, %25
  br i1 %30, label %39, label %31

31:                                               ; preds = %27
  %32 = getelementptr inbounds %struct.FaWait, ptr %25, i32 0, i32 3
  %33 = load ptr, ptr %32, align 4, !tbaa !463
  %34 = getelementptr inbounds %struct.FaWait, ptr %33, i32 0, i32 2
  store ptr %29, ptr %34, align 8, !tbaa !462
  %35 = load ptr, ptr %28, align 8, !tbaa !462
  %36 = getelementptr inbounds %struct.FaWait, ptr %35, i32 0, i32 3
  store ptr %33, ptr %36, align 4, !tbaa !463
  %37 = load ptr, ptr %24, align 4, !tbaa !6
  %38 = icmp eq ptr %37, %25
  br i1 %38, label %39, label %41

39:                                               ; preds = %31, %27
  %40 = phi ptr [ null, %27 ], [ %35, %31 ]
  store ptr %40, ptr %24, align 4, !tbaa !6
  br label %41

41:                                               ; preds = %39, %31
  store ptr null, ptr %28, align 8, !tbaa !462
  %42 = getelementptr inbounds %struct.FaWait, ptr %25, i32 0, i32 3
  store ptr null, ptr %42, align 4, !tbaa !463
  %43 = getelementptr inbounds %struct.FaWait, ptr %25, i32 0, i32 4
  %44 = load ptr, ptr %43, align 8, !tbaa !486
  tail call fastcc void @channel_push(ptr noundef nonnull %0, ptr noundef %44) #61
  br label %70

45:                                               ; preds = %2
  %46 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 12
  %47 = load ptr, ptr %46, align 4, !tbaa !6
  %48 = icmp eq ptr %47, null
  br i1 %48, label %73, label %49

49:                                               ; preds = %45
  %50 = getelementptr inbounds %struct.FaWait, ptr %47, i32 0, i32 2
  %51 = load ptr, ptr %50, align 8, !tbaa !462
  %52 = icmp eq ptr %51, %47
  br i1 %52, label %61, label %53

53:                                               ; preds = %49
  %54 = getelementptr inbounds %struct.FaWait, ptr %47, i32 0, i32 3
  %55 = load ptr, ptr %54, align 4, !tbaa !463
  %56 = getelementptr inbounds %struct.FaWait, ptr %55, i32 0, i32 2
  store ptr %51, ptr %56, align 8, !tbaa !462
  %57 = load ptr, ptr %50, align 8, !tbaa !462
  %58 = getelementptr inbounds %struct.FaWait, ptr %57, i32 0, i32 3
  store ptr %55, ptr %58, align 4, !tbaa !463
  %59 = load ptr, ptr %46, align 4, !tbaa !6
  %60 = icmp eq ptr %59, %47
  br i1 %60, label %61, label %63

61:                                               ; preds = %53, %49
  %62 = phi ptr [ null, %49 ], [ %57, %53 ]
  store ptr %62, ptr %46, align 4, !tbaa !6
  br label %63

63:                                               ; preds = %61, %53
  store ptr null, ptr %50, align 8, !tbaa !462
  %64 = getelementptr inbounds %struct.FaWait, ptr %47, i32 0, i32 3
  store ptr null, ptr %64, align 4, !tbaa !463
  %65 = getelementptr inbounds %struct.FaWait, ptr %47, i32 0, i32 4
  %66 = load ptr, ptr %65, align 8, !tbaa !486
  %67 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 4
  %68 = load i64, ptr %67, align 8, !tbaa !478
  %69 = trunc i64 %68 to i32
  tail call fastcc void @copy_bytes(ptr noundef %1, ptr noundef %66, i32 noundef %69) #61
  br label %70

70:                                               ; preds = %63, %41
  %71 = phi ptr [ %25, %41 ], [ %47, %63 ]
  %72 = getelementptr inbounds %struct.FaWait, ptr %71, i32 0, i32 9
  store i32 1, ptr %72, align 8, !tbaa !328
  tail call fastcc void @wake(ptr noundef nonnull %71) #61
  br label %73

73:                                               ; preds = %70, %45, %6
  %74 = phi i32 [ 1, %6 ], [ 0, %45 ], [ 1, %70 ]
  ret i32 %74
}

; Function Attrs: nounwind optsize
define hidden noundef i32 @fa_channel_receive_async(ptr nocapture noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef %3, ptr noundef %4) local_unnamed_addr #3 {
  %6 = tail call fastcc i32 @channel_receive_now(ptr noundef %0, ptr noundef %1) #61, !range !50
  %7 = icmp eq i32 %6, 0
  br i1 %7, label %38, label %8

8:                                                ; preds = %5
  %9 = getelementptr inbounds %struct.FaWait, ptr %2, i32 0, i32 9
  store i32 1, ptr %9, align 8, !tbaa !328
  %10 = getelementptr inbounds %struct.FaWait, ptr %2, i32 0, i32 10
  store i32 0, ptr %10, align 4, !tbaa !329
  %11 = load i32, ptr @fa_coop_budget, align 4, !tbaa !23
  %12 = add nsw i32 %11, -1
  store i32 %12, ptr @fa_coop_budget, align 4, !tbaa !23
  %13 = icmp sgt i32 %11, 1
  br i1 %13, label %45, label %14

14:                                               ; preds = %8
  %15 = load ptr, ptr @current_executor, align 4, !tbaa !6
  %16 = icmp eq ptr %15, null
  br i1 %16, label %19, label %17

17:                                               ; preds = %14
  %18 = getelementptr inbounds %struct.FaExecutor, ptr %15, i32 0, i32 23
  store i32 1, ptr %18, align 8, !tbaa !307
  br label %19

19:                                               ; preds = %17, %14
  %20 = tail call fastcc ptr @executor() #61
  %21 = getelementptr inbounds %struct.FaExecutor, ptr %20, i32 0, i32 23
  store i32 1, ptr %21, align 8, !tbaa !307
  %22 = getelementptr inbounds i8, ptr %3, i32 16
  %23 = getelementptr inbounds i8, ptr %3, i32 32
  %24 = load i32, ptr %23, align 4, !tbaa !286
  %25 = or i32 %24, 4
  store i32 %25, ptr %23, align 4, !tbaa !286
  %26 = getelementptr inbounds %struct.FaExecutor, ptr %20, i32 0, i32 2
  %27 = getelementptr inbounds %struct.FaExecutor, ptr %20, i32 0, i32 3
  %28 = getelementptr inbounds i8, ptr %3, i32 20
  store ptr null, ptr %28, align 4, !tbaa !290
  %29 = load ptr, ptr %27, align 4, !tbaa !6
  %30 = getelementptr inbounds i8, ptr %3, i32 24
  store ptr %29, ptr %30, align 4, !tbaa !291
  %31 = icmp eq ptr %29, null
  %32 = getelementptr inbounds %struct.FaTask, ptr %29, i32 0, i32 1
  %33 = select i1 %31, ptr %26, ptr %32
  store ptr %22, ptr %33, align 4, !tbaa !6
  store ptr %22, ptr %27, align 4, !tbaa !6
  %34 = getelementptr inbounds %struct.FaExecutor, ptr %20, i32 0, i32 4
  %35 = load i64, ptr %34, align 8, !tbaa !292
  %36 = add nsw i64 %35, 1
  store i64 %36, ptr %34, align 8, !tbaa !292
  %37 = getelementptr inbounds i8, ptr %3, i32 28
  store atomic i32 1, ptr %37 release, align 4
  br label %45

38:                                               ; preds = %5
  %39 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 9
  %40 = load i32, ptr %39, align 16, !tbaa !485
  %41 = icmp eq i32 %40, 0
  br i1 %41, label %42, label %45

42:                                               ; preds = %38
  %43 = getelementptr inbounds %struct.FaWait, ptr %2, i32 0, i32 4
  store ptr %1, ptr %43, align 8, !tbaa !486
  %44 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 13
  tail call fastcc void @line_push(ptr noundef nonnull %44, ptr noundef %2, ptr noundef %3, ptr noundef %4) #61
  br label %45

45:                                               ; preds = %38, %8, %42, %19
  %46 = phi i32 [ 2, %19 ], [ 1, %42 ], [ 0, %8 ], [ 3, %38 ]
  ret i32 %46
}

; Function Attrs: nofree norecurse nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden i32 @fa_channel_cancel(ptr nocapture noundef %0, ptr noundef %1) local_unnamed_addr #33 {
  %3 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 9
  %4 = load i32, ptr %3, align 8, !tbaa !328
  %5 = icmp eq i32 %4, 0
  br i1 %5, label %6, label %36

6:                                                ; preds = %2
  %7 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 12
  %8 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 13
  %9 = load ptr, ptr %8, align 16, !tbaa !487
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
  %17 = load ptr, ptr %16, align 8, !tbaa !462
  %18 = icmp eq ptr %17, %9
  br i1 %18, label %19, label %10, !llvm.loop !488

19:                                               ; preds = %15, %13, %10
  %20 = phi ptr [ %7, %10 ], [ %8, %13 ], [ %7, %15 ]
  %21 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 2
  %22 = load ptr, ptr %21, align 8, !tbaa !462
  %23 = icmp eq ptr %22, %1
  br i1 %23, label %32, label %24

24:                                               ; preds = %19
  %25 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 3
  %26 = load ptr, ptr %25, align 4, !tbaa !463
  %27 = getelementptr inbounds %struct.FaWait, ptr %26, i32 0, i32 2
  store ptr %22, ptr %27, align 8, !tbaa !462
  %28 = load ptr, ptr %21, align 8, !tbaa !462
  %29 = getelementptr inbounds %struct.FaWait, ptr %28, i32 0, i32 3
  store ptr %26, ptr %29, align 4, !tbaa !463
  %30 = load ptr, ptr %20, align 4, !tbaa !6
  %31 = icmp eq ptr %30, %1
  br i1 %31, label %32, label %34

32:                                               ; preds = %24, %19
  %33 = phi ptr [ null, %19 ], [ %28, %24 ]
  store ptr %33, ptr %20, align 4, !tbaa !6
  br label %34

34:                                               ; preds = %32, %24
  store ptr null, ptr %21, align 8, !tbaa !462
  %35 = getelementptr inbounds %struct.FaWait, ptr %1, i32 0, i32 3
  store ptr null, ptr %35, align 4, !tbaa !463
  br label %37

36:                                               ; preds = %2
  tail call fastcc void @forget_remote_wake(ptr noundef nonnull %1) #61
  br label %37

37:                                               ; preds = %34, %36
  ret i32 %4
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(argmem: readwrite)
define hidden i32 @fa_wait_status(ptr nocapture noundef readonly %0) local_unnamed_addr #45 {
  %2 = getelementptr inbounds %struct.FaWait, ptr %0, i32 0, i32 9
  %3 = load atomic i32, ptr %2 acquire, align 8
  ret i32 %3
}

; Function Attrs: nofree norecurse nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden void @fa_channel_close(ptr nocapture noundef %0) local_unnamed_addr #33 {
  %2 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 9
  store i32 1, ptr %2, align 16, !tbaa !485
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
  %13 = load ptr, ptr %12, align 8, !tbaa !462
  %14 = icmp eq ptr %13, %11
  br i1 %14, label %23, label %15

15:                                               ; preds = %10
  %16 = getelementptr inbounds %struct.FaWait, ptr %11, i32 0, i32 3
  %17 = load ptr, ptr %16, align 4, !tbaa !463
  %18 = getelementptr inbounds %struct.FaWait, ptr %17, i32 0, i32 2
  store ptr %13, ptr %18, align 8, !tbaa !462
  %19 = load ptr, ptr %12, align 8, !tbaa !462
  %20 = getelementptr inbounds %struct.FaWait, ptr %19, i32 0, i32 3
  store ptr %17, ptr %20, align 4, !tbaa !463
  %21 = load ptr, ptr %3, align 4, !tbaa !6
  %22 = icmp eq ptr %21, %11
  br i1 %22, label %23, label %25

23:                                               ; preds = %15, %10
  %24 = phi ptr [ null, %10 ], [ %19, %15 ]
  store ptr %24, ptr %3, align 4, !tbaa !6
  br label %25

25:                                               ; preds = %15, %23
  store ptr null, ptr %12, align 8, !tbaa !462
  %26 = getelementptr inbounds %struct.FaWait, ptr %11, i32 0, i32 3
  store ptr null, ptr %26, align 4, !tbaa !463
  %27 = getelementptr inbounds %struct.FaWait, ptr %11, i32 0, i32 9
  store i32 2, ptr %27, align 8, !tbaa !328
  tail call fastcc void @wake(ptr noundef nonnull %11) #61
  %28 = load ptr, ptr %3, align 4, !tbaa !6
  %29 = icmp eq ptr %28, null
  br i1 %29, label %6, label %10, !llvm.loop !489

30:                                               ; preds = %6, %45
  %31 = phi ptr [ %48, %45 ], [ %8, %6 ]
  %32 = getelementptr inbounds %struct.FaWait, ptr %31, i32 0, i32 2
  %33 = load ptr, ptr %32, align 8, !tbaa !462
  %34 = icmp eq ptr %33, %31
  br i1 %34, label %43, label %35

35:                                               ; preds = %30
  %36 = getelementptr inbounds %struct.FaWait, ptr %31, i32 0, i32 3
  %37 = load ptr, ptr %36, align 4, !tbaa !463
  %38 = getelementptr inbounds %struct.FaWait, ptr %37, i32 0, i32 2
  store ptr %33, ptr %38, align 8, !tbaa !462
  %39 = load ptr, ptr %32, align 8, !tbaa !462
  %40 = getelementptr inbounds %struct.FaWait, ptr %39, i32 0, i32 3
  store ptr %37, ptr %40, align 4, !tbaa !463
  %41 = load ptr, ptr %7, align 4, !tbaa !6
  %42 = icmp eq ptr %41, %31
  br i1 %42, label %43, label %45

43:                                               ; preds = %35, %30
  %44 = phi ptr [ null, %30 ], [ %39, %35 ]
  store ptr %44, ptr %7, align 4, !tbaa !6
  br label %45

45:                                               ; preds = %43, %35
  store ptr null, ptr %32, align 8, !tbaa !462
  %46 = getelementptr inbounds %struct.FaWait, ptr %31, i32 0, i32 3
  store ptr null, ptr %46, align 4, !tbaa !463
  %47 = getelementptr inbounds %struct.FaWait, ptr %31, i32 0, i32 9
  store i32 2, ptr %47, align 8, !tbaa !328
  tail call fastcc void @wake(ptr noundef nonnull %31) #61
  %48 = load ptr, ptr %7, align 4, !tbaa !6
  %49 = icmp eq ptr %48, null
  br i1 %49, label %50, label %30, !llvm.loop !490

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
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %3) #64
  %4 = call fastcc i128 @divide_u128(i128 noundef %0, i128 noundef %1, ptr noundef nonnull %3) #61
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %3) #64
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
  store i128 %14, ptr %2, align 16, !tbaa !491
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
  br i1 %58, label %10, label %19, !llvm.loop !493
}

; Function Attrs: nounwind optsize
define weak hidden i128 @__umodti3(i128 noundef %0, i128 noundef %1) local_unnamed_addr #3 {
  %3 = alloca i128, align 16
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %3) #64
  %4 = call fastcc i128 @divide_u128(i128 noundef %0, i128 noundef %1, ptr noundef nonnull %3) #61
  %5 = load i128, ptr %3, align 16, !tbaa !491
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %3) #64
  ret i128 %5
}

; Function Attrs: nounwind optsize
define weak hidden i128 @__divti3(i128 noundef %0, i128 noundef %1) local_unnamed_addr #3 {
  %3 = alloca i128, align 16
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %3) #64
  %4 = tail call noundef i128 @llvm.abs.i128(i128 %0, i1 false)
  %5 = tail call noundef i128 @llvm.abs.i128(i128 %1, i1 false)
  %6 = call fastcc i128 @divide_u128(i128 noundef %4, i128 noundef %5, ptr noundef nonnull %3) #61
  %7 = lshr i128 %0, 127
  %8 = trunc i128 %7 to i32
  %9 = lshr i128 %1, 127
  %10 = trunc i128 %9 to i32
  %11 = icmp eq i32 %8, %10
  %12 = sub i128 0, %6
  %13 = select i1 %11, i128 %6, i128 %12
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %3) #64
  ret i128 %13
}

; Function Attrs: nounwind optsize
define weak hidden i128 @__modti3(i128 noundef %0, i128 noundef %1) local_unnamed_addr #3 {
  %3 = alloca i128, align 16
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %3) #64
  %4 = tail call noundef i128 @llvm.abs.i128(i128 %0, i1 false)
  %5 = tail call noundef i128 @llvm.abs.i128(i128 %1, i1 false)
  %6 = call fastcc i128 @divide_u128(i128 noundef %4, i128 noundef %5, ptr noundef nonnull %3) #61
  %7 = icmp slt i128 %0, 0
  %8 = load i128, ptr %3, align 16, !tbaa !491
  %9 = sub i128 0, %8
  %10 = select i1 %7, i128 %9, i128 %8
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %3) #64
  ret i128 %10
}

; Function Attrs: optsize
declare i32 @fa_host_format_float(double noundef, i32 noundef, ptr noundef) local_unnamed_addr #50

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: write)
define internal fastcc i32 @size_class(i32 noundef %0, ptr nocapture noundef writeonly %1) unnamed_addr #10 {
  %3 = icmp ult i32 %0, 129
  br i1 %3, label %4, label %11

4:                                                ; preds = %2
  %5 = add nuw nsw i32 %0, 7
  %6 = and i32 %5, 504
  %7 = icmp ugt i32 %0, 8
  %8 = select i1 %7, i32 %6, i32 8
  store i32 %8, ptr %1, align 4, !tbaa !4
  %9 = lshr exact i32 %8, 3
  %10 = add nsw i32 %9, -1
  br label %29

11:                                               ; preds = %2, %27
  %12 = phi i32 [ %15, %27 ], [ 16, %2 ]
  %13 = phi i32 [ %28, %27 ], [ 128, %2 ]
  %14 = lshr exact i32 %13, 2
  %15 = add i32 %12, 4
  br label %16

16:                                               ; preds = %11, %23
  %17 = phi i32 [ 1, %11 ], [ %25, %23 ]
  %18 = phi i32 [ %12, %11 ], [ %24, %23 ]
  %19 = mul nuw i32 %17, %14
  %20 = add i32 %19, %13
  %21 = icmp ult i32 %20, %0
  br i1 %21, label %23, label %22

22:                                               ; preds = %16
  store i32 %20, ptr %1, align 4, !tbaa !4
  br label %29

23:                                               ; preds = %16
  %24 = add i32 %18, 1
  %25 = add nuw nsw i32 %17, 1
  %26 = icmp eq i32 %25, 5
  br i1 %26, label %27, label %16, !llvm.loop !494

27:                                               ; preds = %23
  %28 = shl i32 %13, 1
  br label %11

29:                                               ; preds = %22, %4
  %30 = phi i32 [ %10, %4 ], [ %18, %22 ]
  ret i32 %30
}

; Function Attrs: noinline nounwind optsize
define internal fastcc void @string_grow(ptr nocapture noundef %0, ptr nocapture noundef readonly %1, i32 noundef %2, i32 noundef %3) unnamed_addr #15 {
  %5 = alloca i32, align 4
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
  %19 = icmp ult i32 %18, %17
  %20 = icmp ugt i32 %18, 1073741822
  %21 = or i1 %19, %20
  br i1 %21, label %22, label %23

22:                                               ; preds = %16
  tail call fastcc void @out_of_memory() #65
  unreachable

23:                                               ; preds = %16
  %24 = and i32 %7, 1073741823
  %25 = icmp ne i32 %24, 0
  %26 = icmp ne i32 %24, 1073741823
  %27 = and i1 %25, %26
  %28 = and i1 %9, %27
  %29 = shl nuw nsw i32 %24, 1
  %30 = tail call i32 @llvm.umax.i32(i32 %29, i32 %18)
  %31 = select i1 %28, i32 %30, i32 %18
  %32 = tail call i32 @llvm.umin.i32(i32 %31, i32 1073741822)
  %33 = tail call i32 @llvm.umax.i32(i32 %32, i32 64)
  %34 = icmp ult i32 %31, 129
  br i1 %34, label %35, label %38

35:                                               ; preds = %23
  %36 = add nuw nsw i32 %33, 7
  %37 = and i32 %36, 504
  br label %43

38:                                               ; preds = %23
  %39 = icmp ugt i32 %31, 8192
  br i1 %39, label %43, label %40

40:                                               ; preds = %38
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %5) #64
  %41 = call fastcc i32 @size_class(i32 noundef %33, ptr noundef nonnull %5) #61
  %42 = load i32, ptr %5, align 4, !tbaa !4
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %5) #64
  br label %43

43:                                               ; preds = %35, %38, %40
  %44 = phi i32 [ %37, %35 ], [ %42, %40 ], [ %33, %38 ]
  %45 = tail call ptr @fa_alloc(i32 noundef %44, i32 noundef 1) #62
  %46 = load i32, ptr %6, align 4, !tbaa !19
  %47 = and i32 %46, 1073741824
  %48 = icmp eq i32 %47, 0
  br i1 %48, label %49, label %51

49:                                               ; preds = %43
  %50 = load ptr, ptr %0, align 4, !tbaa !13
  br label %51

51:                                               ; preds = %43, %49
  %52 = phi ptr [ %50, %49 ], [ %0, %43 ]
  tail call fastcc void @copy_bytes(ptr noundef %45, ptr noundef %52, i32 noundef %17) #61
  %53 = getelementptr inbounds i8, ptr %45, i32 %17
  tail call fastcc void @copy_bytes(ptr noundef %53, ptr noundef %1, i32 noundef %2) #61
  br i1 %9, label %54, label %57

54:                                               ; preds = %51
  switch i32 %24, label %55 [
    i32 1073741823, label %57
    i32 0, label %57
  ]

55:                                               ; preds = %54
  %56 = load ptr, ptr %0, align 4, !tbaa !13
  tail call void @fa_free(ptr noundef %56, i32 noundef %24, i32 noundef 1) #62
  br label %57

57:                                               ; preds = %54, %54, %51, %55
  store ptr %45, ptr %0, align 4, !tbaa !13
  %58 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 1
  store i32 %18, ptr %58, align 4, !tbaa !18
  %59 = or i32 %44, %3
  store i32 %59, ptr %6, align 4, !tbaa !19
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(none)
define internal fastcc i32 @break_kind(i32 noundef %0) unnamed_addr #51 {
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
  %59 = load i32, ptr %58, align 8, !tbaa !121
  %60 = icmp ugt i32 %59, %0
  br i1 %60, label %52, label %61, !llvm.loop !495

61:                                               ; preds = %55
  %62 = getelementptr inbounds %struct.Range, ptr @EXTEND, i32 %57, i32 1
  %63 = load i32, ptr %62, align 4, !tbaa !123
  %64 = icmp ult i32 %63, %0
  %65 = add nuw i32 %57, 1
  br i1 %64, label %49, label %83, !llvm.loop !495

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
  %76 = load i32, ptr %75, align 8, !tbaa !121
  %77 = icmp ugt i32 %76, %0
  br i1 %77, label %69, label %78, !llvm.loop !495

78:                                               ; preds = %72
  %79 = getelementptr inbounds %struct.Range, ptr @PICTOGRAPHIC, i32 %74, i32 1
  %80 = load i32, ptr %79, align 4, !tbaa !123
  %81 = icmp ult i32 %80, %0
  %82 = add nuw i32 %74, 1
  br i1 %81, label %66, label %83, !llvm.loop !495

83:                                               ; preds = %61, %78, %69, %35, %29, %23, %20, %18, %3, %5, %1, %44, %13, %2
  %84 = phi i32 [ 2, %2 ], [ %17, %13 ], [ %48, %44 ], [ 1, %1 ], [ 3, %5 ], [ 3, %3 ], [ 5, %18 ], [ 6, %20 ], [ 8, %23 ], [ 9, %29 ], [ 10, %35 ], [ 0, %69 ], [ 7, %78 ], [ 4, %61 ]
  ret i32 %84
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
  br i1 %23, label %9, label %24, !llvm.loop !496

24:                                               ; preds = %13
  %25 = select i1 %21, i32 8, i32 0
  %26 = add i32 %25, %10
  %27 = tail call i64 @llvm.cttz.i64(i64 %22, i1 true), !range !112
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
  br i1 %45, label %31, label %54, !llvm.loop !497

46:                                               ; preds = %35, %51
  %47 = phi i32 [ %52, %51 ], [ %32, %35 ]
  %48 = getelementptr inbounds i8, ptr %0, i32 %47
  %49 = load i8, ptr %48, align 1, !tbaa !1
  %50 = icmp eq i8 %49, %3
  br i1 %50, label %59, label %51

51:                                               ; preds = %46
  %52 = add nuw i32 %47, 1
  %53 = icmp ult i32 %52, %1
  br i1 %53, label %46, label %59, !llvm.loop !498

54:                                               ; preds = %37
  %55 = tail call i64 @llvm.cttz.i64(i64 %44, i1 true), !range !499
  %56 = trunc i64 %55 to i32
  %57 = lshr i32 %56, 3
  %58 = add i32 %57, %32
  br label %59

59:                                               ; preds = %46, %51, %35, %54, %24
  %60 = phi i32 [ %30, %24 ], [ %58, %54 ], [ %1, %35 ], [ %47, %46 ], [ %1, %51 ]
  ret i32 %60
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc i32 @at_boundary(ptr nocapture noundef %0, i32 noundef %1) unnamed_addr #20 {
  %3 = load ptr, ptr %0, align 4, !tbaa !45
  %4 = getelementptr inbounds %struct.Boundaries, ptr %0, i32 0, i32 3
  %5 = load i32, ptr %4, align 4, !tbaa !49
  %6 = icmp ne i32 %5, 0
  %7 = icmp eq i32 %1, 0
  %8 = or i1 %7, %6
  br i1 %8, label %37, label %9

9:                                                ; preds = %2
  %10 = getelementptr inbounds %struct.Boundaries, ptr %0, i32 0, i32 1
  %11 = load i32, ptr %10, align 4, !tbaa !47
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
  %27 = load i32, ptr %26, align 4, !tbaa !48
  %28 = icmp ult i32 %27, %1
  br i1 %28, label %29, label %33

29:                                               ; preds = %25, %29
  %30 = phi i32 [ %31, %29 ], [ %27, %25 ]
  %31 = tail call fastcc i32 @next_boundary(ptr noundef %3, i32 noundef %11, i32 noundef %30) #61
  store i32 %31, ptr %26, align 4, !tbaa !48
  %32 = icmp ult i32 %31, %1
  br i1 %32, label %29, label %33, !llvm.loop !500

33:                                               ; preds = %29, %25
  %34 = phi i32 [ %27, %25 ], [ %31, %29 ]
  %35 = icmp eq i32 %34, %1
  %36 = zext i1 %35 to i32
  br label %37

37:                                               ; preds = %2, %9, %21, %33
  %38 = phi i32 [ %36, %33 ], [ 1, %21 ], [ 1, %9 ], [ 1, %2 ]
  ret i32 %38
}

; Function Attrs: nounwind optsize
define internal void @split_one(ptr nocapture noundef %0, i32 noundef %1) #3 {
  %3 = alloca %struct.FaString, align 4
  %4 = getelementptr inbounds %struct.Splitting, ptr %0, i32 0, i32 3
  %5 = load ptr, ptr %4, align 4, !tbaa !77
  %6 = tail call ptr @fa_array_push_slot(ptr noundef %5, i64 noundef 12) #61
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %3) #64
  %7 = getelementptr inbounds %struct.Splitting, ptr %0, i32 0, i32 1
  %8 = load i32, ptr %7, align 4, !tbaa !74
  call fastcc void @split_part(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %3, ptr noundef %0, i32 noundef %8, i32 noundef %1) #61
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %6, ptr noundef nonnull align 4 dereferenceable(12) %3, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %3) #64
  %9 = getelementptr inbounds %struct.Splitting, ptr %0, i32 0, i32 2
  %10 = load i32, ptr %9, align 4, !tbaa !75
  %11 = add i32 %10, %1
  store i32 %11, ptr %7, align 4, !tbaa !74
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @split_part(ptr dead_on_unwind noalias nocapture writable writeonly sret(%struct.FaString) align 4 %0, ptr nocapture noundef readonly %1, i32 noundef %2, i32 noundef %3) unnamed_addr #3 {
  %5 = getelementptr inbounds %struct.Splitting, ptr %1, i32 0, i32 5
  %6 = load i32, ptr %5, align 4, !tbaa !79
  %7 = icmp eq i32 %6, 0
  %8 = load ptr, ptr %1, align 4, !tbaa !72
  %9 = getelementptr inbounds i8, ptr %8, i32 %2
  br i1 %7, label %17, label %10

10:                                               ; preds = %4
  store ptr %9, ptr %0, align 4, !tbaa !13
  %11 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 1
  %12 = sub i32 %3, %2
  store i32 %12, ptr %11, align 4, !tbaa !18
  %13 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 2
  %14 = getelementptr inbounds %struct.Splitting, ptr %1, i32 0, i32 4
  %15 = load i32, ptr %14, align 4, !tbaa !78
  %16 = or i32 %15, 1073741823
  store i32 %16, ptr %13, align 4, !tbaa !19
  br label %21

17:                                               ; preds = %4
  %18 = sub i32 %3, %2
  %19 = getelementptr inbounds %struct.Splitting, ptr %1, i32 0, i32 4
  %20 = load i32, ptr %19, align 4, !tbaa !78
  tail call fastcc void @owned_copy_with(ptr dead_on_unwind writable sret(%struct.FaString) align 4 %0, ptr noundef %9, i32 noundef %18, i32 noundef %20) #61
  br label %21

21:                                               ; preds = %17, %10
  ret void
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
  %8 = load i32, ptr %7, align 4, !tbaa !124
  %9 = icmp ult i32 %8, %0
  br i1 %9, label %2, label %10

10:                                               ; preds = %6
  %11 = icmp eq i32 %8, %0
  br i1 %11, label %12, label %14

12:                                               ; preds = %10
  %13 = load ptr, ptr %4, align 4, !tbaa !127
  br label %20

14:                                               ; preds = %10
  %15 = shl i32 %0, 16
  %16 = getelementptr inbounds i8, ptr %4, i32 %15
  %17 = load ptr, ptr %4, align 4, !tbaa !127
  store ptr %17, ptr %16, align 4, !tbaa !127
  %18 = sub i32 %8, %0
  %19 = getelementptr inbounds %struct.FreeSpan, ptr %16, i32 0, i32 1
  store i32 %18, ptr %19, align 4, !tbaa !124
  br label %20

20:                                               ; preds = %14, %12
  %21 = phi ptr [ %16, %14 ], [ %13, %12 ]
  store ptr %21, ptr %3, align 4, !tbaa !6
  br label %40

22:                                               ; preds = %2
  %23 = tail call i32 @llvm.umax.i32(i32 %0, i32 16)
  %24 = tail call ptr @fa_pages(i32 noundef %23) #62
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
  store i32 %31, ptr %32, align 4, !tbaa !124
  store ptr null, ptr %30, align 4, !tbaa !127
  br label %33

33:                                               ; preds = %33, %28
  %34 = phi ptr [ @free_spans, %28 ], [ %35, %33 ]
  %35 = load ptr, ptr %34, align 4, !tbaa !6
  %36 = icmp ne ptr %35, null
  %37 = icmp ult ptr %35, %30
  %38 = and i1 %36, %37
  br i1 %38, label %33, label %39, !llvm.loop !501

39:                                               ; preds = %33
  store ptr %35, ptr %30, align 4, !tbaa !127
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
  %3 = load i32, ptr %2, align 8, !tbaa !218
  %4 = icmp eq i32 %3, 0
  br i1 %4, label %21, label %5

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 14
  %7 = load i32, ptr %6, align 4, !tbaa !226
  %8 = icmp eq i32 %7, 0
  br i1 %8, label %9, label %40

9:                                                ; preds = %5
  %10 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 15
  %11 = load i32, ptr %10, align 8, !tbaa !220
  %12 = icmp eq i32 %11, 0
  br i1 %12, label %13, label %40

13:                                               ; preds = %9
  store i32 1, ptr %6, align 4, !tbaa !226
  %14 = load ptr, ptr @reactive_pending, align 4, !tbaa !6
  %15 = load i64, ptr @reactive_pending_count, align 8, !tbaa !89
  %16 = tail call fastcc ptr @grow_items(ptr noundef %14, i64 noundef %15, ptr noundef nonnull @reactive_pending_capacity, i32 noundef 4) #61
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
  %23 = load i32, ptr %22, align 8, !tbaa !207
  %24 = icmp eq i32 %23, 0
  br i1 %24, label %25, label %40

25:                                               ; preds = %21
  store i32 1, ptr %22, align 8, !tbaa !207
  %26 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 3
  %27 = load i64, ptr %26, align 8, !tbaa !502
  %28 = icmp sgt i64 %27, 0
  br i1 %28, label %29, label %40

29:                                               ; preds = %25
  %30 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 2
  br label %31

31:                                               ; preds = %29, %31
  %32 = phi i64 [ 0, %29 ], [ %37, %31 ]
  %33 = load ptr, ptr %30, align 8, !tbaa !503
  %34 = trunc i64 %32 to i32
  %35 = getelementptr inbounds ptr, ptr %33, i32 %34
  %36 = load ptr, ptr %35, align 4, !tbaa !6
  tail call fastcc void @mark(ptr noundef %36) #61
  %37 = add nuw nsw i64 %32, 1
  %38 = load i64, ptr %26, align 8, !tbaa !502
  %39 = icmp slt i64 %37, %38
  br i1 %39, label %31, label %40, !llvm.loop !504

40:                                               ; preds = %31, %25, %21, %5, %9, %13
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc void @timer_up(ptr nocapture noundef readonly %0, i64 noundef %1) unnamed_addr #20 {
  %3 = getelementptr inbounds %struct.FaExecutor, ptr %0, i32 0, i32 6
  %4 = load ptr, ptr %3, align 8, !tbaa !316
  %5 = trunc i64 %1 to i32
  %6 = getelementptr inbounds %struct.FaTimer, ptr %4, i32 %5
  %7 = load i64, ptr %6, align 8, !tbaa.struct !190
  %8 = getelementptr inbounds i8, ptr %6, i32 8
  %9 = load ptr, ptr %8, align 8, !tbaa.struct !191
  %10 = getelementptr inbounds i8, ptr %6, i32 12
  %11 = load i32, ptr %10, align 4, !tbaa.struct !323
  %12 = getelementptr inbounds %struct.FaWait, ptr %9, i32 0, i32 7
  %13 = icmp sgt i64 %1, 0
  br i1 %13, label %14, label %45

14:                                               ; preds = %2, %33
  %15 = phi i64 [ %17, %33 ], [ %1, %2 ]
  %16 = add nsw i64 %15, -1
  %17 = lshr i64 %16, 2
  %18 = load ptr, ptr %3, align 8, !tbaa !316
  %19 = trunc i64 %17 to i32
  %20 = getelementptr inbounds %struct.FaTimer, ptr %18, i32 %19
  %21 = load i64, ptr %20, align 1
  %22 = getelementptr inbounds i8, ptr %20, i32 8
  %23 = load ptr, ptr %22, align 1
  %24 = getelementptr inbounds i8, ptr %20, i32 12
  %25 = icmp slt i64 %7, %21
  br i1 %25, label %33, label %26

26:                                               ; preds = %14
  %27 = icmp eq i64 %7, %21
  br i1 %27, label %28, label %43

28:                                               ; preds = %26
  %29 = load i64, ptr %12, align 8, !tbaa !320
  %30 = getelementptr inbounds %struct.FaWait, ptr %23, i32 0, i32 7
  %31 = load i64, ptr %30, align 8, !tbaa !320
  %32 = icmp slt i64 %29, %31
  br i1 %32, label %33, label %43

33:                                               ; preds = %14, %28
  %34 = load i32, ptr %24, align 1
  %35 = trunc i64 %15 to i32
  %36 = getelementptr inbounds %struct.FaTimer, ptr %18, i32 %35
  store i64 %21, ptr %36, align 8, !tbaa.struct !190
  %37 = getelementptr inbounds i8, ptr %36, i32 8
  store ptr %23, ptr %37, align 8, !tbaa.struct !191
  %38 = getelementptr inbounds i8, ptr %36, i32 12
  store i32 %34, ptr %38, align 4, !tbaa.struct !323
  %39 = getelementptr inbounds %struct.FaWait, ptr %23, i32 0, i32 8
  store i64 %15, ptr %39, align 8, !tbaa !312
  %40 = icmp ult i64 %16, 4
  br i1 %40, label %41, label %14

41:                                               ; preds = %33
  %42 = load ptr, ptr %3, align 8, !tbaa !316
  br label %45

43:                                               ; preds = %28, %26
  %44 = trunc i64 %15 to i32
  br label %45

45:                                               ; preds = %41, %43, %2
  %46 = phi i32 [ %5, %2 ], [ %44, %43 ], [ %19, %41 ]
  %47 = phi ptr [ %4, %2 ], [ %18, %43 ], [ %42, %41 ]
  %48 = phi i64 [ %1, %2 ], [ %15, %43 ], [ %17, %41 ]
  %49 = getelementptr inbounds %struct.FaTimer, ptr %47, i32 %46
  store i64 %7, ptr %49, align 8, !tbaa.struct !190
  %50 = getelementptr inbounds i8, ptr %49, i32 8
  store ptr %9, ptr %50, align 8, !tbaa.struct !191
  %51 = getelementptr inbounds i8, ptr %49, i32 12
  store i32 %11, ptr %51, align 4, !tbaa.struct !323
  %52 = getelementptr inbounds %struct.FaWait, ptr %9, i32 0, i32 8
  store i64 %48, ptr %52, align 8, !tbaa !312
  ret void
}

; Function Attrs: optsize
declare void @fa_resume(ptr noundef) local_unnamed_addr #4

; Function Attrs: nounwind optsize
define internal void @socket_drop(ptr nocapture noundef %0) #3 {
  %2 = getelementptr inbounds %struct.FaSocket, ptr %0, i32 0, i32 4
  %3 = load ptr, ptr %2, align 8, !tbaa !351
  %4 = icmp eq ptr %3, null
  br i1 %4, label %6, label %5

5:                                                ; preds = %1
  store ptr null, ptr %2, align 8, !tbaa !351
  tail call void @fa_free(ptr noundef nonnull %3, i32 noundef 20, i32 noundef 16) #62
  br label %6

6:                                                ; preds = %1, %5
  %7 = getelementptr inbounds %struct.FaSocket, ptr %0, i32 0, i32 1
  store i32 -1, ptr %7, align 8, !tbaa !353
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc noundef ptr @net_error(i32 noundef %0, ptr nocapture noundef readonly %1) unnamed_addr #3 {
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  store i32 0, ptr @builder_simple, align 4, !tbaa !4
  tail call fastcc void @append_c(ptr noundef %1) #61
  tail call fastcc void @append_c(ptr noundef nonnull @.str.56) #61
  tail call fastcc void @append_c(ptr noundef nonnull @.str.57) #61
  %3 = sext i32 %0 to i64
  %4 = tail call i64 @llvm.abs.i64(i64 %3, i1 true)
  %5 = lshr i64 %3, 63
  %6 = trunc i64 %5 to i32
  tail call fastcc void @append_u64(i64 noundef %4, i32 noundef %6) #61
  %7 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #62
  store i64 11, ptr %7, align 8, !tbaa !233
  %8 = getelementptr inbounds %struct.MessageError, ptr %7, i32 0, i32 1
  tail call void @fa_b_take(ptr noundef nonnull %8) #61
  ret ptr %7
}

; Function Attrs: optsize
declare i32 @fa_host_file_run(i32 noundef, ptr noundef, i32 noundef, ptr noundef, i32 noundef, ptr noundef) local_unnamed_addr #52

; Function Attrs: optsize
declare void @fa_host_file_take(ptr noundef) local_unnamed_addr #53

; Function Attrs: nounwind optsize
define internal fastcc noundef i32 @json_string(ptr nocapture noundef %0, ptr nocapture noundef writeonly %1, ptr nocapture noundef writeonly %2) unnamed_addr #3 {
  %4 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 2
  %5 = load i32, ptr %4, align 4, !tbaa !378
  %6 = add i32 %5, 1
  %7 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 1
  %8 = load i32, ptr %7, align 4, !tbaa !376
  %9 = icmp ult i32 %6, %8
  br i1 %9, label %10, label %12

10:                                               ; preds = %3
  %11 = load ptr, ptr %0, align 4, !tbaa !374
  br label %17

12:                                               ; preds = %30, %3
  %13 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 8
  store ptr @.str.71, ptr %13, align 4, !tbaa !379
  %14 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 9
  store ptr null, ptr %14, align 4, !tbaa !380
  %15 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 10
  store i32 0, ptr %15, align 4, !tbaa !381
  %16 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 11
  store i8 0, ptr %16, align 4, !tbaa !382
  br label %183

17:                                               ; preds = %10, %30
  %18 = phi i32 [ %6, %10 ], [ %34, %30 ]
  %19 = phi i32 [ 0, %10 ], [ %32, %30 ]
  %20 = getelementptr inbounds i8, ptr %11, i32 %18
  %21 = load i8, ptr %20, align 1, !tbaa !1
  %22 = icmp eq i8 %21, 34
  br i1 %22, label %36, label %23

23:                                               ; preds = %17
  %24 = icmp ult i8 %21, 32
  br i1 %24, label %25, label %30

25:                                               ; preds = %23
  %26 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 8
  store ptr @.str.71, ptr %26, align 4, !tbaa !379
  %27 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 9
  store ptr null, ptr %27, align 4, !tbaa !380
  %28 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 10
  store i32 0, ptr %28, align 4, !tbaa !381
  %29 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 11
  store i8 0, ptr %29, align 4, !tbaa !382
  br label %183

30:                                               ; preds = %23
  %31 = icmp eq i8 %21, 92
  %32 = select i1 %31, i32 1, i32 %19
  %33 = select i1 %31, i32 2, i32 1
  %34 = add i32 %33, %18
  %35 = icmp ult i32 %34, %8
  br i1 %35, label %17, label %12

36:                                               ; preds = %17
  %37 = add nuw i32 %18, 1
  store i32 %37, ptr %4, align 4, !tbaa !378
  %38 = icmp eq i32 %19, 0
  %39 = sub i32 %18, %6
  br i1 %38, label %40, label %42

40:                                               ; preds = %36
  %41 = getelementptr inbounds i8, ptr %11, i32 %6
  store ptr %41, ptr %1, align 4, !tbaa !6
  store i32 %39, ptr %2, align 4, !tbaa !4
  br label %183

42:                                               ; preds = %36
  %43 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 3
  %44 = load ptr, ptr %43, align 4, !tbaa !377
  %45 = tail call fastcc ptr @json_alloc(ptr noundef %44, i32 noundef %39) #61
  %46 = icmp ult i32 %6, %18
  br i1 %46, label %47, label %181

47:                                               ; preds = %42, %177
  %48 = phi i32 [ %179, %177 ], [ %6, %42 ]
  %49 = phi i32 [ %178, %177 ], [ 0, %42 ]
  %50 = load ptr, ptr %0, align 4, !tbaa !374
  %51 = getelementptr inbounds i8, ptr %50, i32 %48
  %52 = load i8, ptr %51, align 1, !tbaa !1
  %53 = icmp eq i8 %52, 92
  br i1 %53, label %58, label %54

54:                                               ; preds = %47
  %55 = getelementptr inbounds i8, ptr %45, i32 %49
  store i8 %52, ptr %55, align 1, !tbaa !1
  %56 = add i32 %49, 1
  %57 = add nuw i32 %48, 1
  br label %177, !llvm.loop !505

58:                                               ; preds = %47
  %59 = getelementptr i8, ptr %51, i32 1
  %60 = load i8, ptr %59, align 1, !tbaa !1
  %61 = add i32 %48, 2
  switch i8 %60, label %170 [
    i8 34, label %62
    i8 92, label %62
    i8 47, label %62
    i8 98, label %65
    i8 102, label %68
    i8 110, label %71
    i8 114, label %74
    i8 116, label %77
    i8 117, label %80
  ]

62:                                               ; preds = %58, %58, %58
  %63 = add i32 %49, 1
  %64 = getelementptr inbounds i8, ptr %45, i32 %49
  store i8 %60, ptr %64, align 1, !tbaa !1
  br label %177

65:                                               ; preds = %58
  %66 = add i32 %49, 1
  %67 = getelementptr inbounds i8, ptr %45, i32 %49
  store i8 8, ptr %67, align 1, !tbaa !1
  br label %177

68:                                               ; preds = %58
  %69 = add i32 %49, 1
  %70 = getelementptr inbounds i8, ptr %45, i32 %49
  store i8 12, ptr %70, align 1, !tbaa !1
  br label %177

71:                                               ; preds = %58
  %72 = add i32 %49, 1
  %73 = getelementptr inbounds i8, ptr %45, i32 %49
  store i8 10, ptr %73, align 1, !tbaa !1
  br label %177

74:                                               ; preds = %58
  %75 = add i32 %49, 1
  %76 = getelementptr inbounds i8, ptr %45, i32 %49
  store i8 13, ptr %76, align 1, !tbaa !1
  br label %177

77:                                               ; preds = %58
  %78 = add i32 %49, 1
  %79 = getelementptr inbounds i8, ptr %45, i32 %49
  store i8 9, ptr %79, align 1, !tbaa !1
  br label %177

80:                                               ; preds = %58
  %81 = tail call fastcc i32 @json_code_unit(ptr noundef nonnull %0, i32 noundef %61) #61
  %82 = icmp slt i32 %81, 0
  br i1 %82, label %162, label %83

83:                                               ; preds = %80
  %84 = add i32 %48, 6
  %85 = icmp ugt i32 %84, %18
  br i1 %85, label %162, label %86

86:                                               ; preds = %83
  %87 = and i32 %81, 2147482624
  %88 = icmp eq i32 %87, 55296
  br i1 %88, label %89, label %110

89:                                               ; preds = %86
  %90 = add i32 %48, 12
  %91 = icmp ugt i32 %90, %18
  br i1 %91, label %110, label %92

92:                                               ; preds = %89
  %93 = getelementptr inbounds i8, ptr %50, i32 %84
  %94 = load i8, ptr %93, align 1, !tbaa !1
  %95 = icmp eq i8 %94, 92
  br i1 %95, label %96, label %110

96:                                               ; preds = %92
  %97 = getelementptr i8, ptr %51, i32 7
  %98 = load i8, ptr %97, align 1, !tbaa !1
  %99 = icmp eq i8 %98, 117
  br i1 %99, label %100, label %110

100:                                              ; preds = %96
  %101 = add i32 %48, 8
  %102 = tail call fastcc i32 @json_code_unit(ptr noundef nonnull %0, i32 noundef %101) #61
  %103 = and i32 %102, -1024
  %104 = icmp eq i32 %103, 56320
  %105 = shl nuw nsw i32 %81, 10
  %106 = add nsw i32 %105, -56613888
  %107 = add i32 %106, %102
  %108 = select i1 %104, i32 %90, i32 %84
  %109 = select i1 %104, i32 %107, i32 %81
  br label %110

110:                                              ; preds = %100, %96, %92, %89, %86
  %111 = phi i32 [ %108, %100 ], [ %84, %96 ], [ %84, %92 ], [ %84, %89 ], [ %84, %86 ]
  %112 = phi i32 [ %109, %100 ], [ %81, %96 ], [ %81, %92 ], [ %81, %89 ], [ %81, %86 ]
  %113 = and i32 %112, -2048
  %114 = icmp eq i32 %113, 55296
  %115 = select i1 %114, i32 65533, i32 %112
  %116 = getelementptr inbounds i8, ptr %45, i32 %49
  %117 = icmp ult i32 %115, 128
  br i1 %117, label %118, label %120

118:                                              ; preds = %110
  %119 = trunc i32 %115 to i8
  store i8 %119, ptr %116, align 1, !tbaa !1
  br label %167

120:                                              ; preds = %110
  %121 = icmp ult i32 %115, 2048
  br i1 %121, label %122, label %130

122:                                              ; preds = %120
  %123 = lshr i32 %115, 6
  %124 = trunc i32 %123 to i8
  %125 = or disjoint i8 %124, -64
  store i8 %125, ptr %116, align 1, !tbaa !1
  %126 = trunc i32 %115 to i8
  %127 = and i8 %126, 63
  %128 = or disjoint i8 %127, -128
  %129 = getelementptr inbounds i8, ptr %116, i32 1
  store i8 %128, ptr %129, align 1, !tbaa !1
  br label %167

130:                                              ; preds = %120
  %131 = icmp ult i32 %115, 65536
  %132 = getelementptr inbounds i8, ptr %116, i32 1
  br i1 %131, label %133, label %145

133:                                              ; preds = %130
  %134 = lshr i32 %115, 12
  %135 = trunc i32 %134 to i8
  %136 = or disjoint i8 %135, -32
  store i8 %136, ptr %116, align 1, !tbaa !1
  %137 = lshr i32 %115, 6
  %138 = trunc i32 %137 to i8
  %139 = and i8 %138, 63
  %140 = or disjoint i8 %139, -128
  store i8 %140, ptr %132, align 1, !tbaa !1
  %141 = trunc i32 %115 to i8
  %142 = and i8 %141, 63
  %143 = or disjoint i8 %142, -128
  %144 = getelementptr inbounds i8, ptr %116, i32 2
  store i8 %143, ptr %144, align 1, !tbaa !1
  br label %167

145:                                              ; preds = %130
  %146 = lshr i32 %115, 18
  %147 = trunc i32 %146 to i8
  %148 = or i8 %147, -16
  store i8 %148, ptr %116, align 1, !tbaa !1
  %149 = lshr i32 %115, 12
  %150 = trunc i32 %149 to i8
  %151 = and i8 %150, 63
  %152 = or disjoint i8 %151, -128
  store i8 %152, ptr %132, align 1, !tbaa !1
  %153 = lshr i32 %115, 6
  %154 = trunc i32 %153 to i8
  %155 = and i8 %154, 63
  %156 = or disjoint i8 %155, -128
  %157 = getelementptr inbounds i8, ptr %116, i32 2
  store i8 %156, ptr %157, align 1, !tbaa !1
  %158 = trunc i32 %115 to i8
  %159 = and i8 %158, 63
  %160 = or disjoint i8 %159, -128
  %161 = getelementptr inbounds i8, ptr %116, i32 3
  store i8 %160, ptr %161, align 1, !tbaa !1
  br label %167

162:                                              ; preds = %80, %83
  %163 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 8
  store ptr @.str.72, ptr %163, align 4, !tbaa !379
  %164 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 9
  store ptr null, ptr %164, align 4, !tbaa !380
  %165 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 10
  store i32 0, ptr %165, align 4, !tbaa !381
  %166 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 11
  store i8 0, ptr %166, align 4, !tbaa !382
  br label %183

167:                                              ; preds = %145, %133, %122, %118
  %168 = phi i32 [ 1, %118 ], [ 2, %122 ], [ 3, %133 ], [ 4, %145 ]
  %169 = add i32 %168, %49
  br label %177

170:                                              ; preds = %58
  %171 = getelementptr inbounds i8, ptr %50, i32 %61
  %172 = getelementptr inbounds i8, ptr %171, i32 -1
  %173 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 8
  store ptr @.str.73, ptr %173, align 4, !tbaa !379
  %174 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 9
  store ptr %172, ptr %174, align 4, !tbaa !380
  %175 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 10
  store i32 1, ptr %175, align 4, !tbaa !381
  %176 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 11
  store i8 0, ptr %176, align 4, !tbaa !382
  br label %183

177:                                              ; preds = %167, %77, %74, %71, %68, %65, %62, %54
  %178 = phi i32 [ %56, %54 ], [ %78, %77 ], [ %75, %74 ], [ %72, %71 ], [ %69, %68 ], [ %66, %65 ], [ %63, %62 ], [ %169, %167 ]
  %179 = phi i32 [ %57, %54 ], [ %61, %77 ], [ %61, %74 ], [ %61, %71 ], [ %61, %68 ], [ %61, %65 ], [ %61, %62 ], [ %111, %167 ]
  %180 = icmp ult i32 %179, %18
  br i1 %180, label %47, label %181

181:                                              ; preds = %177, %42
  %182 = phi i32 [ 0, %42 ], [ %178, %177 ]
  store ptr %45, ptr %1, align 4, !tbaa !6
  store i32 %182, ptr %2, align 4, !tbaa !4
  br label %183

183:                                              ; preds = %162, %170, %25, %181, %40, %12
  %184 = phi i32 [ 0, %12 ], [ 1, %40 ], [ 1, %181 ], [ 0, %25 ], [ 0, %170 ], [ 0, %162 ]
  ret i32 %184
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: read)
define internal fastcc i32 @json_same(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef readonly %2) unnamed_addr #12 {
  br label %4

4:                                                ; preds = %4, %3
  %5 = phi i32 [ 0, %3 ], [ %9, %4 ]
  %6 = getelementptr inbounds i8, ptr %2, i32 %5
  %7 = load i8, ptr %6, align 1, !tbaa !1
  %8 = icmp eq i8 %7, 0
  %9 = add i32 %5, 1
  br i1 %8, label %10, label %4, !llvm.loop !28

10:                                               ; preds = %4
  %11 = icmp eq i32 %5, %1
  br i1 %11, label %12, label %31

12:                                               ; preds = %10
  %13 = icmp eq i32 %1, 0
  br i1 %13, label %31, label %14

14:                                               ; preds = %12
  %15 = load i8, ptr %0, align 1, !tbaa !1
  %16 = load i8, ptr %2, align 1, !tbaa !1
  %17 = icmp eq i8 %15, %16
  br i1 %17, label %18, label %31

18:                                               ; preds = %14, %22
  %19 = phi i32 [ %20, %22 ], [ 0, %14 ]
  %20 = add nuw i32 %19, 1
  %21 = icmp eq i32 %20, %1
  br i1 %21, label %28, label %22, !llvm.loop !506

22:                                               ; preds = %18
  %23 = getelementptr inbounds i8, ptr %0, i32 %20
  %24 = load i8, ptr %23, align 1, !tbaa !1
  %25 = getelementptr inbounds i8, ptr %2, i32 %20
  %26 = load i8, ptr %25, align 1, !tbaa !1
  %27 = icmp eq i8 %24, %26
  br i1 %27, label %18, label %28, !llvm.loop !506

28:                                               ; preds = %22, %18
  %29 = icmp uge i32 %20, %1
  %30 = zext i1 %29 to i32
  br label %31

31:                                               ; preds = %14, %28, %12, %10
  %32 = phi i32 [ 0, %10 ], [ 1, %12 ], [ 0, %14 ], [ %30, %28 ]
  ret i32 %32
}

; Function Attrs: nounwind optsize
define internal fastcc void @json_collect(ptr nocapture noundef %0, i32 noundef %1, ptr nocapture noundef writeonly %2) unnamed_addr #3 {
  %4 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 5
  %5 = load i32, ptr %4, align 4, !tbaa !391
  %6 = sub i32 %5, %1
  %7 = getelementptr inbounds %struct.FaJson, ptr %2, i32 0, i32 9
  store i32 %6, ptr %7, align 4, !tbaa !406
  %8 = getelementptr inbounds %struct.FaJson, ptr %2, i32 0, i32 8
  store ptr null, ptr %8, align 8, !tbaa !408
  %9 = icmp eq i32 %5, %1
  br i1 %9, label %18, label %10

10:                                               ; preds = %3
  %11 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 3
  %12 = load ptr, ptr %11, align 4, !tbaa !377
  %13 = mul i32 %6, 48
  %14 = tail call fastcc ptr @json_alloc(ptr noundef %12, i32 noundef %13) #61
  store ptr %14, ptr %8, align 8, !tbaa !408
  %15 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 4
  %16 = load ptr, ptr %15, align 4, !tbaa !384
  %17 = getelementptr inbounds %struct.FaJson, ptr %16, i32 %1
  tail call fastcc void @copy_bytes(ptr noundef %14, ptr noundef %17, i32 noundef %13) #61
  br label %18

18:                                               ; preds = %10, %3
  store i32 %1, ptr %4, align 4, !tbaa !391
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @json_push(ptr nocapture noundef %0, ptr nocapture noundef readonly %1) unnamed_addr #3 {
  %3 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 5
  %4 = load i32, ptr %3, align 4, !tbaa !391
  %5 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 6
  %6 = load i32, ptr %5, align 4, !tbaa !383
  %7 = icmp eq i32 %4, %6
  br i1 %7, label %11, label %8

8:                                                ; preds = %2
  %9 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 4
  %10 = load ptr, ptr %9, align 4, !tbaa !384
  br label %30

11:                                               ; preds = %2
  %12 = icmp ult i32 %4, 64
  %13 = shl i32 %4, 1
  %14 = select i1 %12, i32 64, i32 %13
  %15 = mul i32 %14, 48
  %16 = tail call ptr @fa_alloc(i32 noundef %15, i32 noundef 16) #62
  %17 = load i32, ptr %5, align 4, !tbaa !383
  %18 = icmp eq i32 %17, 0
  br i1 %18, label %27, label %19

19:                                               ; preds = %11
  %20 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 4
  %21 = load ptr, ptr %20, align 4, !tbaa !384
  %22 = load i32, ptr %3, align 4, !tbaa !391
  %23 = mul i32 %22, 48
  tail call fastcc void @copy_bytes(ptr noundef %16, ptr noundef %21, i32 noundef %23) #61
  %24 = load ptr, ptr %20, align 4, !tbaa !384
  %25 = load i32, ptr %5, align 4, !tbaa !383
  %26 = mul i32 %25, 48
  tail call void @fa_free(ptr noundef %24, i32 noundef %26, i32 noundef 16) #62
  br label %27

27:                                               ; preds = %19, %11
  %28 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 4
  store ptr %16, ptr %28, align 4, !tbaa !384
  store i32 %14, ptr %5, align 4, !tbaa !383
  %29 = load i32, ptr %3, align 4, !tbaa !391
  br label %30

30:                                               ; preds = %8, %27
  %31 = phi i32 [ %4, %8 ], [ %29, %27 ]
  %32 = phi ptr [ %10, %8 ], [ %16, %27 ]
  %33 = getelementptr inbounds %struct.FaJson, ptr %32, i32 %31
  tail call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 8 dereferenceable(48) %33, ptr noundef nonnull align 8 dereferenceable(48) %1, i32 48, i1 false), !tbaa.struct !507
  %34 = load i32, ptr %3, align 4, !tbaa !391
  %35 = add i32 %34, 1
  store i32 %35, ptr %3, align 4, !tbaa !391
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @json_order(ptr nocapture noundef readonly %0, ptr nocapture noundef %1) unnamed_addr #3 {
  %3 = getelementptr inbounds %struct.FaJson, ptr %1, i32 0, i32 9
  %4 = load i32, ptr %3, align 4, !tbaa !406
  %5 = icmp eq i32 %4, 0
  br i1 %5, label %71, label %6

6:                                                ; preds = %2
  %7 = getelementptr inbounds %struct.FaJson, ptr %1, i32 0, i32 8
  %8 = load ptr, ptr %7, align 8, !tbaa !408
  br label %11

9:                                                ; preds = %11
  %10 = icmp eq i32 %21, 0
  br i1 %10, label %71, label %24

11:                                               ; preds = %6, %11
  %12 = phi i32 [ 0, %6 ], [ %21, %11 ]
  %13 = phi i32 [ 0, %6 ], [ %22, %11 ]
  %14 = getelementptr inbounds %struct.FaJson, ptr %8, i32 %13, i32 6
  %15 = load ptr, ptr %14, align 8, !tbaa !392
  %16 = getelementptr inbounds %struct.FaJson, ptr %8, i32 %13, i32 7
  %17 = load i32, ptr %16, align 4, !tbaa !393
  %18 = tail call fastcc i64 @json_index(ptr noundef %15, i32 noundef %17) #61
  %19 = icmp sgt i64 %18, -1
  %20 = zext i1 %19 to i32
  %21 = add i32 %12, %20
  %22 = add nuw i32 %13, 1
  %23 = icmp eq i32 %22, %4
  br i1 %23, label %9, label %11, !llvm.loop !509

24:                                               ; preds = %9
  %25 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 3
  %26 = load ptr, ptr %25, align 4, !tbaa !377
  %27 = mul i32 %4, 48
  %28 = tail call fastcc ptr @json_alloc(ptr noundef %26, i32 noundef %27) #61
  %29 = getelementptr inbounds %struct.FaJson, ptr %1, i32 0, i32 8
  br label %32

30:                                               ; preds = %66
  %31 = getelementptr inbounds %struct.FaJson, ptr %1, i32 0, i32 8
  store ptr %28, ptr %31, align 8, !tbaa !408
  br label %71

32:                                               ; preds = %24, %66
  %33 = phi i32 [ 0, %24 ], [ %69, %66 ]
  %34 = phi i32 [ %21, %24 ], [ %68, %66 ]
  %35 = phi i32 [ 0, %24 ], [ %67, %66 ]
  %36 = load ptr, ptr %29, align 8, !tbaa !408
  %37 = getelementptr inbounds %struct.FaJson, ptr %36, i32 %33
  %38 = getelementptr inbounds %struct.FaJson, ptr %36, i32 %33, i32 6
  %39 = load ptr, ptr %38, align 8, !tbaa !392
  %40 = getelementptr inbounds %struct.FaJson, ptr %36, i32 %33, i32 7
  %41 = load i32, ptr %40, align 4, !tbaa !393
  %42 = tail call fastcc i64 @json_index(ptr noundef %39, i32 noundef %41) #61
  %43 = icmp slt i64 %42, 0
  br i1 %43, label %46, label %44

44:                                               ; preds = %32
  %45 = icmp eq i32 %35, 0
  br i1 %45, label %62, label %49

46:                                               ; preds = %32
  %47 = getelementptr inbounds %struct.FaJson, ptr %28, i32 %34
  tail call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 8 dereferenceable(48) %47, ptr noundef nonnull align 8 dereferenceable(48) %37, i32 48, i1 false), !tbaa.struct !507
  %48 = add i32 %34, 1
  br label %66

49:                                               ; preds = %44, %58
  %50 = phi i32 [ %51, %58 ], [ %35, %44 ]
  %51 = add i32 %50, -1
  %52 = getelementptr inbounds %struct.FaJson, ptr %28, i32 %51, i32 6
  %53 = load ptr, ptr %52, align 8, !tbaa !392
  %54 = getelementptr inbounds %struct.FaJson, ptr %28, i32 %51, i32 7
  %55 = load i32, ptr %54, align 4, !tbaa !393
  %56 = tail call fastcc i64 @json_index(ptr noundef %53, i32 noundef %55) #61
  %57 = icmp sgt i64 %56, %42
  br i1 %57, label %58, label %62

58:                                               ; preds = %49
  %59 = getelementptr inbounds %struct.FaJson, ptr %28, i32 %51
  %60 = getelementptr inbounds %struct.FaJson, ptr %28, i32 %50
  tail call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 8 dereferenceable(48) %60, ptr noundef nonnull align 8 dereferenceable(48) %59, i32 48, i1 false), !tbaa.struct !507
  %61 = icmp eq i32 %51, 0
  br i1 %61, label %62, label %49, !llvm.loop !510

62:                                               ; preds = %49, %58, %44
  %63 = phi i32 [ 0, %44 ], [ 0, %58 ], [ %50, %49 ]
  %64 = getelementptr inbounds %struct.FaJson, ptr %28, i32 %63
  tail call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 8 dereferenceable(48) %64, ptr noundef nonnull align 8 dereferenceable(48) %37, i32 48, i1 false), !tbaa.struct !507
  %65 = add i32 %35, 1
  br label %66

66:                                               ; preds = %62, %46
  %67 = phi i32 [ %35, %46 ], [ %65, %62 ]
  %68 = phi i32 [ %48, %46 ], [ %34, %62 ]
  %69 = add nuw i32 %33, 1
  %70 = icmp eq i32 %69, %4
  br i1 %70, label %30, label %32, !llvm.loop !511

71:                                               ; preds = %2, %9, %30
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc ptr @json_alloc(ptr nocapture noundef %0, i32 noundef %1) unnamed_addr #3 {
  %3 = add i32 %1, 15
  %4 = and i32 %3, -16
  %5 = getelementptr inbounds %struct.FaJsonDocument, ptr %0, i32 0, i32 3
  %6 = load i32, ptr %5, align 8, !tbaa !512
  %7 = icmp ugt i32 %4, %6
  br i1 %7, label %11, label %8

8:                                                ; preds = %2
  %9 = getelementptr inbounds %struct.FaJsonDocument, ptr %0, i32 0, i32 2
  %10 = load ptr, ptr %9, align 4, !tbaa !513
  br label %20

11:                                               ; preds = %2
  %12 = add i32 %4, 16
  %13 = tail call i32 @llvm.umax.i32(i32 %12, i32 65536)
  %14 = tail call ptr @fa_alloc(i32 noundef %13, i32 noundef 16) #62
  %15 = getelementptr inbounds %struct.FaJsonDocument, ptr %0, i32 0, i32 1
  %16 = load ptr, ptr %15, align 8, !tbaa !366
  store ptr %16, ptr %14, align 4, !tbaa !370
  %17 = getelementptr inbounds %struct.JsonChunk, ptr %14, i32 0, i32 1
  store i32 %13, ptr %17, align 4, !tbaa !372
  store ptr %14, ptr %15, align 8, !tbaa !366
  %18 = getelementptr inbounds i8, ptr %14, i32 16
  %19 = add i32 %13, -16
  br label %20

20:                                               ; preds = %8, %11
  %21 = phi i32 [ %6, %8 ], [ %19, %11 ]
  %22 = phi ptr [ %10, %8 ], [ %18, %11 ]
  %23 = getelementptr inbounds %struct.FaJsonDocument, ptr %0, i32 0, i32 2
  %24 = getelementptr inbounds i8, ptr %22, i32 %4
  store ptr %24, ptr %23, align 4, !tbaa !513
  %25 = sub i32 %21, %4
  store i32 %25, ptr %5, align 8, !tbaa !512
  ret ptr %22
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: read)
define internal fastcc i64 @json_index(ptr nocapture noundef readonly %0, i32 noundef %1) unnamed_addr #12 {
  %3 = add i32 %1, -11
  %4 = icmp ult i32 %3, -10
  br i1 %4, label %28, label %5

5:                                                ; preds = %2
  %6 = icmp ugt i32 %1, 1
  br i1 %6, label %7, label %10

7:                                                ; preds = %5
  %8 = load i8, ptr %0, align 1, !tbaa !1
  %9 = icmp eq i8 %8, 48
  br i1 %9, label %28, label %10

10:                                               ; preds = %5, %7
  br label %11

11:                                               ; preds = %10, %18
  %12 = phi i32 [ %23, %18 ], [ 0, %10 ]
  %13 = phi i64 [ %22, %18 ], [ 0, %10 ]
  %14 = getelementptr inbounds i8, ptr %0, i32 %12
  %15 = load i8, ptr %14, align 1, !tbaa !1
  %16 = add i8 %15, -58
  %17 = icmp ult i8 %16, -10
  br i1 %17, label %28, label %18

18:                                               ; preds = %11
  %19 = mul nsw i64 %13, 10
  %20 = zext nneg i8 %15 to i64
  %21 = add i64 %19, -48
  %22 = add i64 %21, %20
  %23 = add nuw i32 %12, 1
  %24 = icmp eq i32 %23, %1
  br i1 %24, label %25, label %11, !llvm.loop !514

25:                                               ; preds = %18
  %26 = icmp slt i64 %22, 4294967295
  %27 = select i1 %26, i64 %22, i64 -1
  br label %28

28:                                               ; preds = %11, %25, %2, %7
  %29 = phi i64 [ -1, %7 ], [ -1, %2 ], [ %27, %25 ], [ -1, %11 ]
  ret i64 %29
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(read, inaccessiblemem: none)
define internal fastcc i32 @json_code_unit(ptr nocapture noundef readonly %0, i32 noundef %1) unnamed_addr #34 {
  %3 = add i32 %1, 4
  %4 = getelementptr inbounds %struct.JsonReader, ptr %0, i32 0, i32 1
  %5 = load i32, ptr %4, align 4, !tbaa !376
  %6 = icmp ugt i32 %3, %5
  br i1 %6, label %37, label %7

7:                                                ; preds = %2
  %8 = load ptr, ptr %0, align 4, !tbaa !374
  %9 = getelementptr i8, ptr %8, i32 %1
  br label %10

10:                                               ; preds = %7, %32
  %11 = phi i32 [ 0, %7 ], [ %35, %32 ]
  %12 = phi i32 [ 0, %7 ], [ %34, %32 ]
  %13 = getelementptr i8, ptr %9, i32 %11
  %14 = load i8, ptr %13, align 1, !tbaa !1
  %15 = zext i8 %14 to i32
  %16 = add i8 %14, -48
  %17 = icmp ult i8 %16, 10
  br i1 %17, label %18, label %20

18:                                               ; preds = %10
  %19 = add nsw i32 %15, -48
  br label %29

20:                                               ; preds = %10
  %21 = add i8 %14, -97
  %22 = icmp ult i8 %21, 6
  br i1 %22, label %23, label %25

23:                                               ; preds = %20
  %24 = add nsw i32 %15, -87
  br label %29

25:                                               ; preds = %20
  %26 = add i8 %14, -65
  %27 = icmp ult i8 %26, 6
  %28 = add nsw i32 %15, -55
  br i1 %27, label %29, label %37

29:                                               ; preds = %25, %18, %23
  %30 = phi i32 [ %19, %18 ], [ %24, %23 ], [ %28, %25 ]
  %31 = icmp sgt i32 %30, -1
  br i1 %31, label %32, label %37

32:                                               ; preds = %29
  %33 = shl nsw i32 %12, 4
  %34 = add nuw nsw i32 %30, %33
  %35 = add nuw nsw i32 %11, 1
  %36 = icmp eq i32 %35, 4
  br i1 %36, label %37, label %10, !llvm.loop !515

37:                                               ; preds = %29, %32, %25, %2
  %38 = phi i32 [ -1, %2 ], [ -1, %29 ], [ %34, %32 ], [ -1, %25 ]
  ret i32 %38
}

; Function Attrs: optsize
declare double @fa_host_parse_number(ptr noundef, i32 noundef) local_unnamed_addr #54

; Function Attrs: nounwind optsize
define internal fastcc void @json_mismatch(ptr nocapture noundef %0, ptr nocapture noundef readonly %1, ptr nocapture noundef readonly %2, ptr noundef %3, i32 noundef %4) unnamed_addr #3 {
  %6 = alloca i32, align 4
  %7 = alloca ptr, align 4
  %8 = alloca ptr, align 4
  %9 = icmp eq i32 %4, 0
  br i1 %9, label %41, label %10

10:                                               ; preds = %5
  tail call fastcc void @json_error_start(ptr noundef %0) #61
  tail call fastcc void @append_c(ptr noundef nonnull @.str.99) #61
  %11 = icmp eq ptr %3, null
  br i1 %11, label %29, label %12

12:                                               ; preds = %10
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %6) #64
  %13 = load i8, ptr %3, align 1, !tbaa !1
  switch i8 %13, label %25 [
    i8 83, label %14
    i8 73, label %19
  ]

14:                                               ; preds = %12
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %7) #64
  %15 = getelementptr inbounds i8, ptr %3, i32 1
  store ptr %15, ptr %7, align 4, !tbaa !6
  %16 = call fastcc i64 @json_schema_number(ptr noundef nonnull %7, ptr noundef nonnull %6) #61
  %17 = load ptr, ptr %7, align 4, !tbaa !6
  %18 = trunc i64 %16 to i32
  tail call void @fa_b_json_string(ptr noundef %17, i32 noundef %18) #61
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %7) #64
  br label %28

19:                                               ; preds = %12
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %8) #64
  %20 = getelementptr inbounds i8, ptr %3, i32 1
  store ptr %20, ptr %8, align 4, !tbaa !6
  %21 = call fastcc i64 @json_schema_number(ptr noundef nonnull %8, ptr noundef nonnull %6) #61
  %22 = tail call i64 @llvm.abs.i64(i64 %21, i1 false)
  %23 = lshr i64 %21, 63
  %24 = trunc i64 %23 to i32
  tail call fastcc void @append_u64(i64 noundef %22, i32 noundef %24) #61
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %8) #64
  br label %28

25:                                               ; preds = %12
  %26 = icmp eq i8 %13, 84
  %27 = select i1 %26, ptr @.str, ptr @.str.1
  tail call fastcc void @append_c(ptr noundef nonnull %27) #61
  br label %28

28:                                               ; preds = %25, %19, %14
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %6) #64
  br label %30

29:                                               ; preds = %10
  tail call fastcc void @append_c(ptr noundef %2) #61
  br label %30

30:                                               ; preds = %29, %28
  tail call fastcc void @append_c(ptr noundef nonnull @.str.84) #61
  %31 = load i32, ptr %1, align 8, !tbaa !390
  %32 = icmp ult i32 %31, 5
  br i1 %32, label %33, label %36

33:                                               ; preds = %30
  %34 = getelementptr inbounds [5 x ptr], ptr @switch.table.json_mismatch, i32 0, i32 %31
  %35 = load ptr, ptr %34, align 4
  br label %36

36:                                               ; preds = %30, %33
  %37 = phi ptr [ %35, %33 ], [ @.str.103, %30 ]
  tail call fastcc void @append_c(ptr noundef nonnull %37) #61
  %38 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #62
  store i64 10, ptr %38, align 8, !tbaa !233
  %39 = getelementptr inbounds %struct.MessageError, ptr %38, i32 0, i32 1
  tail call void @fa_b_take(ptr noundef nonnull %39) #61
  %40 = getelementptr inbounds %struct.JsonCheck, ptr %0, i32 0, i32 3
  store ptr %38, ptr %40, align 4, !tbaa !388
  br label %41

41:                                               ; preds = %5, %36
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(read, argmem: readwrite, inaccessiblemem: none)
define internal fastcc i64 @json_schema_number(ptr nocapture noundef %0, ptr nocapture noundef writeonly %1) unnamed_addr #14 {
  %3 = load ptr, ptr %0, align 4, !tbaa !6
  %4 = load i8, ptr %3, align 1, !tbaa !1
  %5 = icmp eq i8 %4, 45
  %6 = zext i1 %5 to i32
  %7 = getelementptr inbounds i8, ptr %3, i32 %6
  %8 = load i8, ptr %7, align 1, !tbaa !1
  %9 = icmp ne i8 %8, 59
  %10 = zext i1 %9 to i32
  store i32 %10, ptr %1, align 4, !tbaa !23
  %11 = load i8, ptr %7, align 1, !tbaa !1
  %12 = icmp eq i8 %11, 59
  br i1 %12, label %24, label %13

13:                                               ; preds = %2, %13
  %14 = phi i8 [ %22, %13 ], [ %11, %2 ]
  %15 = phi i64 [ %20, %13 ], [ 0, %2 ]
  %16 = phi ptr [ %21, %13 ], [ %7, %2 ]
  %17 = zext i8 %14 to i64
  %18 = mul i64 %15, 10
  %19 = add i64 %18, -48
  %20 = add i64 %19, %17
  %21 = getelementptr inbounds i8, ptr %16, i32 1
  %22 = load i8, ptr %21, align 1, !tbaa !1
  %23 = icmp eq i8 %22, 59
  br i1 %23, label %24, label %13, !llvm.loop !516

24:                                               ; preds = %13, %2
  %25 = phi ptr [ %7, %2 ], [ %21, %13 ]
  %26 = phi i64 [ 0, %2 ], [ %20, %13 ]
  %27 = getelementptr inbounds i8, ptr %25, i32 1
  store ptr %27, ptr %0, align 4, !tbaa !6
  %28 = sub i64 0, %26
  %29 = select i1 %5, i64 %28, i64 %26
  ret i64 %29
}

; Function Attrs: nounwind optsize
define internal fastcc void @json_error_start(ptr nocapture noundef readonly %0) unnamed_addr #3 {
  store i32 0, ptr @builder_length, align 4, !tbaa !4
  store i32 0, ptr @builder_simple, align 4, !tbaa !4
  %2 = load ptr, ptr %0, align 4, !tbaa !387
  %3 = getelementptr inbounds %struct.JsonCheck, ptr %0, i32 0, i32 1
  %4 = load i32, ptr %3, align 4, !tbaa !401
  tail call fastcc void @reserve(i32 noundef %4) #61
  %5 = load ptr, ptr @builder, align 4, !tbaa !6
  %6 = load i32, ptr @builder_length, align 4, !tbaa !4
  %7 = getelementptr inbounds i8, ptr %5, i32 %6
  tail call fastcc void @copy_bytes(ptr noundef %7, ptr noundef %2, i32 noundef %4) #61
  %8 = add i32 %6, %4
  store i32 %8, ptr @builder_length, align 4, !tbaa !4
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @json_error_end(ptr nocapture noundef writeonly %0) unnamed_addr #3 {
  %2 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #62
  store i64 10, ptr %2, align 8, !tbaa !233
  %3 = getelementptr inbounds %struct.MessageError, ptr %2, i32 0, i32 1
  tail call void @fa_b_take(ptr noundef nonnull %3) #61
  %4 = getelementptr inbounds %struct.JsonCheck, ptr %0, i32 0, i32 3
  store ptr %2, ptr %4, align 4, !tbaa !388
  ret void
}

; Function Attrs: nofree nosync nounwind optsize memory(read, inaccessiblemem: none)
define internal fastcc ptr @json_skip(ptr noundef %0) unnamed_addr #55 {
  %2 = alloca ptr, align 4
  %3 = alloca i32, align 4
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %3) #64
  %4 = load i8, ptr %0, align 1, !tbaa !1
  switch i8 %4, label %91 [
    i8 105, label %5
    i8 83, label %10
    i8 73, label %16
    i8 97, label %20
    i8 109, label %20
    i8 111, label %23
    i8 117, label %39
    i8 78, label %50
    i8 69, label %71
  ]

5:                                                ; preds = %1
  %6 = getelementptr inbounds i8, ptr %0, i32 1
  store ptr %6, ptr %2, align 4, !tbaa !6
  %7 = call fastcc i64 @json_schema_number(ptr noundef nonnull %2, ptr noundef nonnull %3) #61
  %8 = call fastcc i64 @json_schema_number(ptr noundef nonnull %2, ptr noundef nonnull %3) #61
  %9 = load ptr, ptr %2, align 4, !tbaa !6
  br label %93

10:                                               ; preds = %1
  %11 = getelementptr inbounds i8, ptr %0, i32 1
  store ptr %11, ptr %2, align 4, !tbaa !6
  %12 = call fastcc i64 @json_schema_number(ptr noundef nonnull %2, ptr noundef nonnull %3) #61
  %13 = load ptr, ptr %2, align 4, !tbaa !6
  %14 = trunc i64 %12 to i32
  %15 = getelementptr inbounds i8, ptr %13, i32 %14
  br label %93

16:                                               ; preds = %1
  %17 = getelementptr inbounds i8, ptr %0, i32 1
  store ptr %17, ptr %2, align 4, !tbaa !6
  %18 = call fastcc i64 @json_schema_number(ptr noundef nonnull %2, ptr noundef nonnull %3) #61
  %19 = load ptr, ptr %2, align 4, !tbaa !6
  br label %93

20:                                               ; preds = %1, %1
  %21 = getelementptr inbounds i8, ptr %0, i32 1
  %22 = tail call fastcc ptr @json_skip(ptr noundef nonnull %21) #61
  br label %93

23:                                               ; preds = %1
  %24 = getelementptr inbounds i8, ptr %0, i32 1
  store ptr %24, ptr %2, align 4, !tbaa !6
  %25 = call fastcc i64 @json_schema_number(ptr noundef nonnull %2, ptr noundef nonnull %3) #61
  %26 = icmp sgt i64 %25, 0
  br i1 %26, label %29, label %27

27:                                               ; preds = %23
  %28 = load ptr, ptr %2, align 4, !tbaa !6
  br label %93

29:                                               ; preds = %23, %29
  %30 = phi i64 [ %37, %29 ], [ 0, %23 ]
  %31 = call fastcc i64 @json_schema_number(ptr noundef nonnull %2, ptr noundef nonnull %3) #61
  %32 = load ptr, ptr %2, align 4, !tbaa !6
  %33 = trunc i64 %31 to i32
  %34 = getelementptr inbounds i8, ptr %32, i32 %33
  %35 = getelementptr inbounds i8, ptr %34, i32 1
  %36 = tail call fastcc ptr @json_skip(ptr noundef nonnull %35) #61
  store ptr %36, ptr %2, align 4, !tbaa !6
  %37 = add nuw nsw i64 %30, 1
  %38 = icmp eq i64 %37, %25
  br i1 %38, label %93, label %29, !llvm.loop !517

39:                                               ; preds = %1
  %40 = getelementptr inbounds i8, ptr %0, i32 1
  store ptr %40, ptr %2, align 4, !tbaa !6
  %41 = call fastcc i64 @json_schema_number(ptr noundef nonnull %2, ptr noundef nonnull %3) #61
  %42 = load ptr, ptr %2, align 4, !tbaa !6
  %43 = icmp sgt i64 %41, 0
  br i1 %43, label %44, label %93

44:                                               ; preds = %39, %44
  %45 = phi i64 [ %48, %44 ], [ 0, %39 ]
  %46 = phi ptr [ %47, %44 ], [ %42, %39 ]
  %47 = tail call fastcc ptr @json_skip(ptr noundef %46) #61
  %48 = add nuw nsw i64 %45, 1
  %49 = icmp eq i64 %48, %41
  br i1 %49, label %93, label %44, !llvm.loop !518

50:                                               ; preds = %1
  %51 = getelementptr inbounds i8, ptr %0, i32 1
  store ptr %51, ptr %2, align 4, !tbaa !6
  %52 = call fastcc i64 @json_schema_number(ptr noundef nonnull %2, ptr noundef nonnull %3) #61
  %53 = icmp sgt i64 %52, 0
  br i1 %53, label %56, label %54

54:                                               ; preds = %50
  %55 = load ptr, ptr %2, align 4, !tbaa !6
  br label %93

56:                                               ; preds = %50, %67
  %57 = phi i64 [ %69, %67 ], [ 0, %50 ]
  %58 = call fastcc i64 @json_schema_number(ptr noundef nonnull %2, ptr noundef nonnull %3) #61
  %59 = load ptr, ptr %2, align 4, !tbaa !6
  %60 = trunc i64 %58 to i32
  %61 = getelementptr inbounds i8, ptr %59, i32 %60
  %62 = load i8, ptr %61, align 1, !tbaa !1
  %63 = getelementptr i8, ptr %61, i32 1
  store ptr %63, ptr %2, align 4, !tbaa !6
  %64 = icmp eq i8 %62, 101
  br i1 %64, label %67, label %65

65:                                               ; preds = %56
  %66 = tail call fastcc ptr @json_skip(ptr noundef %63) #61
  store ptr %66, ptr %2, align 4, !tbaa !6
  br label %67

67:                                               ; preds = %65, %56
  %68 = phi ptr [ %66, %65 ], [ %63, %56 ]
  %69 = add nuw nsw i64 %57, 1
  %70 = icmp eq i64 %69, %52
  br i1 %70, label %93, label %56, !llvm.loop !519

71:                                               ; preds = %1
  %72 = getelementptr inbounds i8, ptr %0, i32 1
  store ptr %72, ptr %2, align 4, !tbaa !6
  %73 = call fastcc i64 @json_schema_number(ptr noundef nonnull %2, ptr noundef nonnull %3) #61
  %74 = icmp sgt i64 %73, 0
  br i1 %74, label %77, label %75

75:                                               ; preds = %88, %71
  %76 = load ptr, ptr %2, align 4, !tbaa !6
  br label %93

77:                                               ; preds = %71, %88
  %78 = phi i64 [ %89, %88 ], [ 0, %71 ]
  %79 = call fastcc i64 @json_schema_number(ptr noundef nonnull %2, ptr noundef nonnull %3) #61
  %80 = load ptr, ptr %2, align 4, !tbaa !6
  %81 = trunc i64 %79 to i32
  %82 = getelementptr inbounds i8, ptr %80, i32 %81
  %83 = load i8, ptr %82, align 1, !tbaa !1
  %84 = icmp eq i8 %83, 112
  %85 = getelementptr inbounds i8, ptr %82, i32 1
  store ptr %85, ptr %2, align 4, !tbaa !6
  br i1 %84, label %86, label %88

86:                                               ; preds = %77
  %87 = call fastcc i64 @json_schema_number(ptr noundef nonnull %2, ptr noundef nonnull %3) #61
  br label %88

88:                                               ; preds = %77, %86
  %89 = add nuw nsw i64 %78, 1
  %90 = icmp eq i64 %89, %73
  br i1 %90, label %75, label %77, !llvm.loop !520

91:                                               ; preds = %1
  %92 = getelementptr inbounds i8, ptr %0, i32 1
  br label %93

93:                                               ; preds = %67, %44, %29, %39, %54, %27, %91, %75, %20, %16, %10, %5
  %94 = phi ptr [ %92, %91 ], [ %76, %75 ], [ %22, %20 ], [ %19, %16 ], [ %15, %10 ], [ %9, %5 ], [ %28, %27 ], [ %55, %54 ], [ %42, %39 ], [ %36, %29 ], [ %47, %44 ], [ %68, %67 ]
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %3) #64
  ret ptr %94
}

; Function Attrs: nofree nosync nounwind optsize memory(read, argmem: readwrite, inaccessiblemem: none)
define internal fastcc ptr @json_member_at(ptr noundef %0, ptr nocapture noundef writeonly %1) unnamed_addr #56 {
  %3 = alloca ptr, align 4
  %4 = alloca i32, align 4
  store ptr %0, ptr %3, align 4, !tbaa !6
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %4) #64
  %5 = call fastcc i64 @json_schema_number(ptr noundef nonnull %3, ptr noundef nonnull %4) #61
  %6 = load ptr, ptr %3, align 4, !tbaa !6
  store ptr %6, ptr %1, align 4, !tbaa !415
  %7 = trunc i64 %5 to i32
  %8 = getelementptr inbounds %struct.JsonMember, ptr %1, i32 0, i32 1
  store i32 %7, ptr %8, align 4, !tbaa !416
  %9 = getelementptr inbounds i8, ptr %6, i32 %7
  %10 = load i8, ptr %9, align 1, !tbaa !1
  %11 = getelementptr inbounds %struct.JsonMember, ptr %1, i32 0, i32 2
  store i8 %10, ptr %11, align 4, !tbaa !413
  %12 = getelementptr inbounds i8, ptr %9, i32 1
  %13 = getelementptr inbounds %struct.JsonMember, ptr %1, i32 0, i32 3
  store ptr %12, ptr %13, align 4, !tbaa !419
  %14 = icmp eq i8 %10, 101
  br i1 %14, label %17, label %15

15:                                               ; preds = %2
  %16 = tail call fastcc ptr @json_skip(ptr noundef nonnull %12) #61
  br label %17

17:                                               ; preds = %2, %15
  %18 = phi ptr [ %16, %15 ], [ %12, %2 ]
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %4) #64
  ret ptr %18
}

; Function Attrs: nounwind optsize
define internal fastcc void @channel_push(ptr nocapture noundef %0, ptr nocapture noundef readonly %1) unnamed_addr #3 {
  %3 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 7
  %4 = load i64, ptr %3, align 16, !tbaa !480
  %5 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 8
  %6 = load i64, ptr %5, align 8, !tbaa !481
  %7 = icmp eq i64 %4, %6
  br i1 %7, label %13, label %8

8:                                                ; preds = %2
  %9 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 6
  %10 = load i64, ptr %9, align 8, !tbaa !482
  %11 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 5
  %12 = load ptr, ptr %11, align 16, !tbaa !483
  br label %63

13:                                               ; preds = %2
  %14 = icmp slt i64 %4, 4
  %15 = shl nuw nsw i64 %4, 1
  %16 = select i1 %14, i64 4, i64 %15
  %17 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 4
  %18 = load i64, ptr %17, align 8, !tbaa !478
  %19 = mul nsw i64 %18, %16
  %20 = trunc i64 %19 to i32
  %21 = tail call ptr @fa_alloc(i32 noundef %20, i32 noundef 16) #62
  %22 = icmp eq ptr %21, null
  br i1 %22, label %29, label %23

23:                                               ; preds = %13
  %24 = load i64, ptr %3, align 16, !tbaa !480
  %25 = icmp sgt i64 %24, 0
  br i1 %25, label %26, label %30

26:                                               ; preds = %23
  %27 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 6
  %28 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 5
  br label %34

29:                                               ; preds = %13
  tail call fastcc void @out_of_memory() #65
  unreachable

30:                                               ; preds = %34, %23
  %31 = phi i64 [ %24, %23 ], [ %50, %34 ]
  %32 = load i64, ptr %5, align 8, !tbaa !481
  %33 = icmp sgt i64 %32, 0
  br i1 %33, label %52, label %59

34:                                               ; preds = %26, %34
  %35 = phi i64 [ 0, %26 ], [ %49, %34 ]
  %36 = load i64, ptr %27, align 8, !tbaa !482
  %37 = add nsw i64 %36, %35
  %38 = load i64, ptr %5, align 8, !tbaa !481
  %39 = srem i64 %37, %38
  %40 = load i64, ptr %17, align 8, !tbaa !478
  %41 = mul nsw i64 %40, %35
  %42 = trunc i64 %41 to i32
  %43 = getelementptr inbounds i8, ptr %21, i32 %42
  %44 = load ptr, ptr %28, align 16, !tbaa !483
  %45 = mul nsw i64 %40, %39
  %46 = trunc i64 %45 to i32
  %47 = getelementptr inbounds i8, ptr %44, i32 %46
  %48 = trunc i64 %40 to i32
  tail call fastcc void @copy_bytes(ptr noundef nonnull %43, ptr noundef %47, i32 noundef %48) #61
  %49 = add nuw nsw i64 %35, 1
  %50 = load i64, ptr %3, align 16, !tbaa !480
  %51 = icmp slt i64 %49, %50
  br i1 %51, label %34, label %30, !llvm.loop !521

52:                                               ; preds = %30
  %53 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 5
  %54 = load ptr, ptr %53, align 16, !tbaa !483
  %55 = load i64, ptr %17, align 8, !tbaa !478
  %56 = mul nsw i64 %55, %32
  %57 = trunc i64 %56 to i32
  tail call void @fa_free(ptr noundef %54, i32 noundef %57, i32 noundef 16) #62
  %58 = load i64, ptr %3, align 16, !tbaa !480
  br label %59

59:                                               ; preds = %52, %30
  %60 = phi i64 [ %58, %52 ], [ %31, %30 ]
  %61 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 5
  store ptr %21, ptr %61, align 16, !tbaa !483
  %62 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 6
  store i64 0, ptr %62, align 8, !tbaa !482
  store i64 %16, ptr %5, align 8, !tbaa !481
  br label %63

63:                                               ; preds = %8, %59
  %64 = phi ptr [ %12, %8 ], [ %21, %59 ]
  %65 = phi i64 [ %6, %8 ], [ %16, %59 ]
  %66 = phi i64 [ %4, %8 ], [ %60, %59 ]
  %67 = phi i64 [ %10, %8 ], [ 0, %59 ]
  %68 = add nsw i64 %66, %67
  %69 = srem i64 %68, %65
  %70 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 4
  %71 = load i64, ptr %70, align 8, !tbaa !478
  %72 = mul nsw i64 %71, %69
  %73 = trunc i64 %72 to i32
  %74 = getelementptr inbounds i8, ptr %64, i32 %73
  %75 = trunc i64 %71 to i32
  tail call fastcc void @copy_bytes(ptr noundef %74, ptr noundef %1, i32 noundef %75) #61
  %76 = load i64, ptr %3, align 16, !tbaa !480
  %77 = add nsw i64 %76, 1
  store i64 %77, ptr %3, align 16, !tbaa !480
  ret void
}

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.abs.i64(i64, i1 immarg) #57

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.umax.i32(i32, i32) #57

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.umin.i32(i32, i32) #57

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.smin.i64(i64, i64) #57

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.smax.i64(i64, i64) #57

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.umax.i64(i64, i64) #57

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.umin.i64(i64, i64) #57

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.fshl.i64(i64, i64, i64) #57

; Function Attrs: nocallback nofree nosync nounwind willreturn memory(inaccessiblemem: readwrite)
declare void @llvm.experimental.noalias.scope.decl(metadata) #58

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.usub.sat.i64(i64, i64) #57

; Function Attrs: nocallback nofree nounwind willreturn memory(argmem: write)
declare void @llvm.memset.p0.i64(ptr nocapture writeonly, i8, i64, i1 immarg) #59

; Function Attrs: nocallback nofree nosync nounwind willreturn memory(inaccessiblemem: write)
declare void @llvm.assume(i1 noundef) #60

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i128 @llvm.abs.i128(i128, i1 immarg) #57

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
attributes #16 = { alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #17 = { alwaysinline nofree norecurse nosync nounwind optsize memory(argmem: read) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #18 = { nofree noinline norecurse nosync nounwind optsize memory(argmem: read) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #19 = { nofree norecurse nosync nounwind optsize memory(argmem: readwrite) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #20 = { nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #21 = { nofree noinline norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #22 = { cold noinline nounwind optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #23 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #24 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: write) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #25 = { optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" "wasm-import-module"="fa" "wasm-import-name"="argumentsSize" }
attributes #26 = { optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" "wasm-import-module"="fa" "wasm-import-name"="argumentsCopy" }
attributes #27 = { alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(readwrite, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #28 = { mustprogress nofree noinline norecurse nosync nounwind optsize willreturn memory(none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #29 = { alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
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
attributes #42 = { alwaysinline nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #43 = { nofree norecurse nosync nounwind optsize memory(write, argmem: readwrite, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #44 = { alwaysinline nofree norecurse nosync nounwind optsize memory(read, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #45 = { mustprogress nofree norecurse nounwind optsize willreturn memory(argmem: readwrite) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #46 = { mustprogress nofree norecurse nounwind optsize willreturn memory(readwrite, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #47 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, argmem: readwrite, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #48 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(readwrite, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #49 = { optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" "wasm-import-module"="fa" "wasm-import-name"="timer" }
attributes #50 = { optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" "wasm-import-module"="fa" "wasm-import-name"="formatFloat" }
attributes #51 = { nofree norecurse nosync nounwind optsize memory(none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #52 = { optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" "wasm-import-module"="fa" "wasm-import-name"="fileRun" }
attributes #53 = { optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" "wasm-import-module"="fa" "wasm-import-name"="fileTake" }
attributes #54 = { optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" "wasm-import-module"="fa" "wasm-import-name"="parseNumber" }
attributes #55 = { nofree nosync nounwind optsize memory(read, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #56 = { nofree nosync nounwind optsize memory(read, argmem: readwrite, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #57 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #58 = { nocallback nofree nosync nounwind willreturn memory(inaccessiblemem: readwrite) }
attributes #59 = { nocallback nofree nounwind willreturn memory(argmem: write) }
attributes #60 = { nocallback nofree nosync nounwind willreturn memory(inaccessiblemem: write) }
attributes #61 = { optsize }
attributes #62 = { nounwind optsize }
attributes #63 = { noreturn nounwind optsize }
attributes #64 = { nounwind }
attributes #65 = { noreturn optsize }
attributes #66 = { cold optsize }
attributes #67 = { cold noreturn optsize }

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
!36 = !{!"branch_weights", i32 2000, i32 1}
!37 = !{!"branch_weights", i32 2000, i32 2002}
!38 = distinct !{!38, !9}
!39 = distinct !{!39, !9}
!40 = !{i32 0, i32 13}
!41 = distinct !{!41, !9}
!42 = distinct !{!42, !9}
!43 = distinct !{!43, !9}
!44 = distinct !{!44, !9}
!45 = !{!46, !7, i64 0}
!46 = !{!"", !7, i64 0, !5, i64 4, !5, i64 8, !5, i64 12}
!47 = !{!46, !5, i64 4}
!48 = !{!46, !5, i64 8}
!49 = !{!46, !5, i64 12}
!50 = !{i32 0, i32 2}
!51 = distinct !{!51, !9}
!52 = distinct !{!52, !9}
!53 = distinct !{!53, !9}
!54 = distinct !{!54, !9}
!55 = !{!56, !7, i64 0}
!56 = !{!"", !7, i64 0, !5, i64 4, !5, i64 8, !7, i64 12, !5, i64 16, !24, i64 20, !24, i64 24}
!57 = !{!56, !5, i64 4}
!58 = !{!56, !5, i64 8}
!59 = !{!56, !7, i64 12}
!60 = !{!56, !5, i64 16}
!61 = !{!56, !24, i64 20}
!62 = !{!56, !24, i64 24}
!63 = !{ptr @replace_one, ptr @split_one}
!64 = distinct !{!64, !9}
!65 = distinct !{!65, !9}
!66 = !{!67, !7, i64 0}
!67 = !{!"", !7, i64 0, !68, i64 8, !68, i64 16}
!68 = !{!"long long", !2, i64 0}
!69 = distinct !{!69, !9}
!70 = !{!67, !68, i64 16}
!71 = distinct !{!71, !9}
!72 = !{!73, !7, i64 0}
!73 = !{!"", !7, i64 0, !5, i64 4, !5, i64 8, !7, i64 12, !5, i64 16, !24, i64 20}
!74 = !{!73, !5, i64 4}
!75 = !{!73, !5, i64 8}
!76 = !{!67, !68, i64 8}
!77 = !{!73, !7, i64 12}
!78 = !{!73, !5, i64 16}
!79 = !{!73, !24, i64 20}
!80 = !{!81}
!81 = distinct !{!81, !82, !"split_part: argument 0"}
!82 = distinct !{!82, !"split_part"}
!83 = distinct !{!83, !9}
!84 = distinct !{!84, !9}
!85 = distinct !{!85, !9}
!86 = distinct !{!86, !9}
!87 = distinct !{!87, !9}
!88 = distinct !{!88, !9}
!89 = !{!68, !68, i64 0}
!90 = distinct !{!90, !9}
!91 = distinct !{!91, !9}
!92 = distinct !{!92, !9}
!93 = !{!94, !68, i64 0}
!94 = !{!"", !68, i64 0, !68, i64 8, !68, i64 16}
!95 = !{!94, !68, i64 8}
!96 = !{!94, !68, i64 16}
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
!112 = !{i64 0, i64 65}
!113 = !{!114, !68, i64 0}
!114 = !{!"", !68, i64 0, !68, i64 8, !2, i64 16}
!115 = !{!114, !68, i64 8}
!116 = distinct !{!116, !9}
!117 = distinct !{!117, !9}
!118 = distinct !{!118, !9}
!119 = !{!120, !7, i64 0}
!120 = !{!"FreeBlock", !7, i64 0}
!121 = !{!122, !24, i64 0}
!122 = !{!"", !24, i64 0, !24, i64 4}
!123 = !{!122, !24, i64 4}
!124 = !{!125, !5, i64 4}
!125 = !{!"FreeSpan", !7, i64 0, !5, i64 4}
!126 = distinct !{!126, !9}
!127 = !{!125, !7, i64 0}
!128 = !{!129, !7, i64 0}
!129 = !{!"FaArena", !7, i64 0, !7, i64 4, !7, i64 8, !7, i64 12, !7, i64 16}
!130 = !{!131, !7, i64 4}
!131 = !{!"FaArenaCleanup", !7, i64 0, !7, i64 4, !7, i64 8}
!132 = !{!131, !7, i64 8}
!133 = distinct !{!133, !9}
!134 = !{!129, !7, i64 4}
!135 = distinct !{!135, !9}
!136 = !{!129, !7, i64 8}
!137 = !{!129, !7, i64 12}
!138 = !{!129, !7, i64 16}
!139 = !{!131, !7, i64 0}
!140 = !{!141, !24, i64 28}
!141 = !{!"", !68, i64 0, !7, i64 8, !7, i64 12, !7, i64 16, !24, i64 20, !24, i64 24, !24, i64 28, !24, i64 32, !24, i64 36, !24, i64 40}
!142 = distinct !{!142, !9}
!143 = !{!141, !24, i64 32}
!144 = !{!141, !24, i64 36}
!145 = !{!141, !24, i64 40}
!146 = !{!141, !24, i64 20}
!147 = !{!141, !7, i64 12}
!148 = distinct !{!148, !9}
!149 = !{!141, !7, i64 8}
!150 = !{!141, !68, i64 0}
!151 = !{!141, !7, i64 16}
!152 = distinct !{!152, !9}
!153 = !{!141, !24, i64 24}
!154 = distinct !{!154, !9}
!155 = distinct !{!155, !9}
!156 = distinct !{!156, !9}
!157 = !{!158, !7, i64 32}
!158 = !{!"", !68, i64 0, !7, i64 8, !68, i64 16, !68, i64 24, !7, i64 32, !68, i64 40}
!159 = !{!158, !68, i64 16}
!160 = !{!158, !7, i64 8}
!161 = distinct !{!161, !9}
!162 = !{!158, !68, i64 40}
!163 = !{!158, !68, i64 24}
!164 = !{!158, !68, i64 0}
!165 = distinct !{!165, !9}
!166 = distinct !{!166, !9}
!167 = distinct !{!167, !9}
!168 = distinct !{!168, !9}
!169 = distinct !{!169, !9}
!170 = distinct !{!170, !9}
!171 = !{!172, !68, i64 0}
!172 = !{!"", !68, i64 0, !68, i64 8, !7, i64 16}
!173 = !{!172, !68, i64 8}
!174 = !{!172, !7, i64 16}
!175 = distinct !{!175, !9}
!176 = distinct !{!176, !9}
!177 = distinct !{!177, !9}
!178 = !{!179, !68, i64 8}
!179 = !{!"", !7, i64 0, !7, i64 4, !68, i64 8}
!180 = !{!179, !7, i64 0}
!181 = !{!179, !7, i64 4}
!182 = distinct !{!182, !9}
!183 = distinct !{!183, !9}
!184 = !{!185, !68, i64 48}
!185 = !{!"FaSource", !68, i64 0, !68, i64 8, !7, i64 16, !68, i64 24, !68, i64 32, !7, i64 40, !7, i64 44, !68, i64 48, !68, i64 56}
!186 = !{!185, !7, i64 44}
!187 = distinct !{!187, !9}
!188 = !{!185, !7, i64 40}
!189 = !{i64 0, i64 8, !89, i64 8, i64 8, !89, i64 16, i64 4, !6}
!190 = !{i64 0, i64 8, !89, i64 8, i64 4, !6}
!191 = !{i64 0, i64 4, !6}
!192 = distinct !{!192, !9}
!193 = !{!194, !68, i64 96}
!194 = !{!"FaObserver", !185, i64 0, !7, i64 64, !7, i64 68, !7, i64 72, !7, i64 76, !7, i64 80, !7, i64 84, !7, i64 88, !68, i64 96, !68, i64 104, !68, i64 112, !24, i64 120, !24, i64 124, !24, i64 128, !24, i64 132, !24, i64 136, !24, i64 140, !24, i64 144, !24, i64 148, !68, i64 152, !2, i64 160}
!195 = !{!194, !7, i64 84}
!196 = distinct !{!196, !9}
!197 = !{!194, !68, i64 104}
!198 = !{!194, !7, i64 88}
!199 = !{!185, !68, i64 8}
!200 = !{!185, !68, i64 24}
!201 = !{!185, !7, i64 16}
!202 = distinct !{!202, !9}
!203 = distinct !{!203, !9}
!204 = distinct !{!204, !9}
!205 = distinct !{!205, !9}
!206 = !{!194, !68, i64 0}
!207 = !{!194, !24, i64 120}
!208 = !{!194, !7, i64 72}
!209 = !{!194, !7, i64 76}
!210 = !{!194, !7, i64 80}
!211 = !{!194, !7, i64 68}
!212 = !{!194, !7, i64 64}
!213 = !{!194, !68, i64 152}
!214 = !{!194, !24, i64 124}
!215 = !{!194, !24, i64 128}
!216 = !{!194, !24, i64 140}
!217 = !{!194, !68, i64 8}
!218 = !{!194, !24, i64 144}
!219 = !{!194, !68, i64 112}
!220 = !{!194, !24, i64 136}
!221 = distinct !{!221, !9}
!222 = distinct !{!222, !9}
!223 = distinct !{!223, !9}
!224 = distinct !{!224, !9}
!225 = distinct !{!225, !9}
!226 = !{!194, !24, i64 132}
!227 = distinct !{!227, !9}
!228 = distinct !{!228, !9}
!229 = !{!185, !68, i64 0}
!230 = !{!185, !68, i64 32}
!231 = !{!185, !68, i64 56}
!232 = distinct !{!232, !9}
!233 = !{!234, !68, i64 0}
!234 = !{!"", !68, i64 0, !14, i64 8}
!235 = distinct !{!235, !9}
!236 = distinct !{!236, !9}
!237 = distinct !{!237, !9}
!238 = !{!239, !68, i64 0}
!239 = !{!"", !68, i64 0, !14, i64 8, !14, i64 20}
!240 = !{!241, !7, i64 32}
!241 = !{!"", !7, i64 0, !68, i64 8, !68, i64 16, !68, i64 24, !7, i64 32, !68, i64 40, !68, i64 48, !68, i64 56, !7, i64 64, !68, i64 72}
!242 = !{!241, !7, i64 64}
!243 = !{!241, !7, i64 0}
!244 = !{!245, !7, i64 0}
!245 = !{!"", !7, i64 0, !24, i64 4, !24, i64 8}
!246 = !{!245, !24, i64 4}
!247 = !{!241, !68, i64 40}
!248 = distinct !{!248, !9}
!249 = !{i64 0, i64 8, !89}
!250 = !{i64 0, i64 8, !89, i64 8, i64 8, !89}
!251 = !{!252, !68, i64 0}
!252 = !{!"", !68, i64 0, !68, i64 8}
!253 = !{!252, !68, i64 8}
!254 = !{!241, !68, i64 8}
!255 = !{!241, !68, i64 16}
!256 = !{!241, !68, i64 24}
!257 = !{!241, !68, i64 48}
!258 = !{!241, !68, i64 56}
!259 = !{!241, !68, i64 72}
!260 = distinct !{!260, !9}
!261 = distinct !{!261, !9}
!262 = distinct !{!262, !9}
!263 = distinct !{!263, !9}
!264 = distinct !{!264, !9}
!265 = distinct !{!265, !9}
!266 = distinct !{!266, !9}
!267 = distinct !{!267, !9}
!268 = distinct !{!268, !9}
!269 = !{i32 0, i32 33}
!270 = !{!245, !24, i64 8}
!271 = distinct !{!271, !9}
!272 = distinct !{!272, !9}
!273 = !{!"branch_weights", i32 1073203, i32 2146410445}
!274 = distinct !{!274, !9}
!275 = distinct !{!275, !9}
!276 = !{i64 0, i64 4, !6, i64 8, i64 8, !89, i64 16, i64 8, !89, i64 24, i64 8, !89, i64 32, i64 4, !6, i64 40, i64 8, !89, i64 48, i64 8, !89, i64 56, i64 8, !89, i64 64, i64 4, !6, i64 72, i64 8, !89}
!277 = !{i64 0, i64 4, !6, i64 4, i64 4, !23, i64 8, i64 4, !23}
!278 = distinct !{!278, !9}
!279 = distinct !{!279, !9}
!280 = !{!281, !7, i64 16}
!281 = !{!"", !68, i64 0, !68, i64 8, !7, i64 16, !68, i64 24}
!282 = !{!281, !68, i64 8}
!283 = !{!284, !68, i64 32}
!284 = !{!"", !281, i64 0, !68, i64 32}
!285 = !{!281, !68, i64 0}
!286 = !{!287, !24, i64 16}
!287 = !{!"FaTask", !7, i64 0, !7, i64 4, !7, i64 8, !24, i64 12, !24, i64 16}
!288 = !{!289, !68, i64 72}
!289 = !{!"FaExecutor", !7, i64 0, !7, i64 4, !7, i64 8, !7, i64 12, !68, i64 16, !68, i64 24, !7, i64 32, !68, i64 40, !68, i64 48, !68, i64 56, !7, i64 64, !68, i64 72, !68, i64 80, !68, i64 88, !68, i64 96, !7, i64 104, !7, i64 108, !287, i64 112, !24, i64 132, !24, i64 136, !24, i64 140, !24, i64 144, !68, i64 152, !24, i64 160, !68, i64 168, !7, i64 176, !7, i64 180, !2, i64 184, !24, i64 312}
!290 = !{!287, !7, i64 4}
!291 = !{!287, !7, i64 8}
!292 = !{!289, !68, i64 16}
!293 = !{!289, !68, i64 24}
!294 = !{!289, !7, i64 104}
!295 = !{!289, !7, i64 108}
!296 = !{!289, !24, i64 136}
!297 = !{!289, !24, i64 140}
!298 = !{!287, !7, i64 0}
!299 = !{!289, !7, i64 64}
!300 = !{!289, !7, i64 176}
!301 = !{!302, !7, i64 20}
!302 = !{!"", !7, i64 0, !68, i64 8, !7, i64 16, !7, i64 20}
!303 = !{!302, !7, i64 16}
!304 = !{!302, !68, i64 8}
!305 = !{!302, !7, i64 0}
!306 = distinct !{!306, !9}
!307 = !{!289, !24, i64 160}
!308 = !{!289, !68, i64 168}
!309 = !{!310, !7, i64 0}
!310 = !{!"FaWait", !7, i64 0, !7, i64 4, !7, i64 8, !7, i64 12, !7, i64 16, !7, i64 20, !68, i64 24, !68, i64 32, !68, i64 40, !24, i64 48, !24, i64 52, !7, i64 56}
!311 = !{!310, !7, i64 4}
!312 = !{!310, !68, i64 40}
!313 = !{!310, !7, i64 56}
!314 = !{!289, !68, i64 40}
!315 = !{!289, !68, i64 48}
!316 = !{!289, !7, i64 32}
!317 = distinct !{!317, !9}
!318 = !{!310, !68, i64 24}
!319 = !{!289, !68, i64 56}
!320 = !{!310, !68, i64 32}
!321 = !{!322, !7, i64 8}
!322 = !{!"", !68, i64 0, !7, i64 8}
!323 = !{}
!324 = distinct !{!324, !9}
!325 = !{!326, !24, i64 4}
!326 = !{!"FaIo", !24, i64 0, !24, i64 4, !7, i64 8, !7, i64 12, !7, i64 16}
!327 = !{!326, !7, i64 16}
!328 = !{!310, !24, i64 48}
!329 = !{!310, !24, i64 52}
!330 = !{!289, !68, i64 88}
!331 = !{!326, !7, i64 8}
!332 = !{!326, !7, i64 12}
!333 = !{!289, !24, i64 144}
!334 = !{!289, !7, i64 8}
!335 = !{!289, !68, i64 152}
!336 = distinct !{!336, !9}
!337 = !{!322, !68, i64 0}
!338 = distinct !{!338, !9}
!339 = distinct !{!339, !9}
!340 = distinct !{!340, !9}
!341 = distinct !{!341, !9}
!342 = !{!289, !7, i64 0}
!343 = distinct !{!343, !9}
!344 = !{!289, !24, i64 312}
!345 = distinct !{!345, !9}
!346 = !{!289, !7, i64 180}
!347 = !{!348, !7, i64 0}
!348 = !{!"FaJob", !7, i64 0, !7, i64 4, !7, i64 8, !7, i64 12, !7, i64 16, !24, i64 20}
!349 = !{!348, !24, i64 20}
!350 = !{!348, !7, i64 4}
!351 = !{!352, !7, i64 48}
!352 = !{!"", !281, i64 0, !24, i64 32, !24, i64 36, !68, i64 40, !7, i64 48}
!353 = !{!352, !24, i64 32}
!354 = !{!352, !24, i64 36}
!355 = !{!352, !68, i64 40}
!356 = !{!357, !7, i64 0}
!357 = !{!"", !348, i64 0, !24, i64 24, !24, i64 28, !7, i64 32, !5, i64 36, !7, i64 40, !5, i64 44, !5, i64 48}
!358 = !{!357, !7, i64 4}
!359 = !{!357, !24, i64 24}
!360 = !{!357, !7, i64 32}
!361 = !{!357, !5, i64 36}
!362 = !{!357, !7, i64 40}
!363 = !{!357, !5, i64 44}
!364 = !{!357, !5, i64 48}
!365 = !{!357, !24, i64 28}
!366 = !{!367, !7, i64 48}
!367 = !{!"", !368, i64 0, !7, i64 48, !7, i64 52, !5, i64 56}
!368 = !{!"FaJson", !24, i64 0, !24, i64 4, !68, i64 8, !369, i64 16, !7, i64 24, !5, i64 28, !7, i64 32, !5, i64 36, !7, i64 40, !5, i64 44}
!369 = !{!"double", !2, i64 0}
!370 = !{!371, !7, i64 0}
!371 = !{!"JsonChunk", !7, i64 0, !5, i64 4}
!372 = !{!371, !5, i64 4}
!373 = distinct !{!373, !9}
!374 = !{!375, !7, i64 0}
!375 = !{!"", !7, i64 0, !5, i64 4, !5, i64 8, !7, i64 12, !7, i64 16, !5, i64 20, !5, i64 24, !24, i64 28, !7, i64 32, !7, i64 36, !5, i64 40, !2, i64 44}
!376 = !{!375, !5, i64 4}
!377 = !{!375, !7, i64 12}
!378 = !{!375, !5, i64 8}
!379 = !{!375, !7, i64 32}
!380 = !{!375, !7, i64 36}
!381 = !{!375, !5, i64 40}
!382 = !{!375, !2, i64 44}
!383 = !{!375, !5, i64 24}
!384 = !{!375, !7, i64 16}
!385 = !{!386, !5, i64 8}
!386 = !{!"", !7, i64 0, !5, i64 4, !5, i64 8, !7, i64 12}
!387 = !{!386, !7, i64 0}
!388 = !{!386, !7, i64 12}
!389 = !{!375, !24, i64 28}
!390 = !{!368, !24, i64 0}
!391 = !{!375, !5, i64 20}
!392 = !{!368, !7, i64 32}
!393 = !{!368, !5, i64 36}
!394 = distinct !{!394, !9}
!395 = distinct !{!395, !9}
!396 = distinct !{!396, !9}
!397 = !{!368, !68, i64 8}
!398 = !{!368, !24, i64 4}
!399 = !{!368, !369, i64 16}
!400 = distinct !{!400, !9}
!401 = !{!386, !5, i64 4}
!402 = distinct !{!402, !9}
!403 = !{!368, !5, i64 28}
!404 = !{!368, !7, i64 24}
!405 = distinct !{!405, !9}
!406 = !{!368, !5, i64 44}
!407 = distinct !{!407, !9}
!408 = !{!368, !7, i64 40}
!409 = distinct !{!409, !9}
!410 = distinct !{!410, !9}
!411 = distinct !{!411, !9}
!412 = distinct !{!412, !9}
!413 = !{!414, !2, i64 8}
!414 = !{!"", !7, i64 0, !5, i64 4, !2, i64 8, !7, i64 12}
!415 = !{!414, !7, i64 0}
!416 = !{!414, !5, i64 4}
!417 = distinct !{!417, !9}
!418 = distinct !{!418, !9}
!419 = !{!414, !7, i64 12}
!420 = distinct !{!420, !9}
!421 = distinct !{!421, !9}
!422 = distinct !{!422, !9}
!423 = distinct !{!423, !9}
!424 = distinct !{!424, !9}
!425 = distinct !{!425, !9}
!426 = distinct !{!426, !9}
!427 = !{!428, !24, i64 188}
!428 = !{!"FaWorker", !281, i64 0, !429, i64 32, !429, i64 96, !7, i64 160, !7, i64 164, !7, i64 168, !7, i64 172, !5, i64 176, !24, i64 180, !24, i64 184, !24, i64 188, !24, i64 192, !7, i64 196}
!429 = !{!"", !2, i64 0}
!430 = !{!428, !24, i64 180}
!431 = !{!428, !7, i64 196}
!432 = !{!428, !7, i64 164}
!433 = !{!428, !24, i64 184}
!434 = !{!435, !7, i64 0}
!435 = !{!"FaCall", !7, i64 0, !7, i64 4, !7, i64 8, !7, i64 12, !7, i64 16, !7, i64 20, !7, i64 24, !68, i64 32, !24, i64 40, !24, i64 44, !7, i64 48, !68, i64 56, !7, i64 64, !68, i64 72, !7, i64 80, !7, i64 84, !7, i64 88}
!436 = distinct !{!436, !9}
!437 = !{!435, !7, i64 8}
!438 = !{!435, !7, i64 16}
!439 = !{!435, !7, i64 24}
!440 = !{!435, !68, i64 32}
!441 = distinct !{!441, !9}
!442 = !{!428, !24, i64 192}
!443 = !{!428, !5, i64 176}
!444 = !{!428, !7, i64 172}
!445 = !{!435, !7, i64 4}
!446 = !{!435, !7, i64 12}
!447 = !{!435, !7, i64 20}
!448 = !{!435, !24, i64 40}
!449 = !{!428, !7, i64 160}
!450 = distinct !{!450, !9}
!451 = !{!452, !7, i64 280}
!452 = !{!"FaWorkerPool", !281, i64 0, !429, i64 32, !429, i64 96, !429, i64 160, !7, i64 224, !68, i64 232, !7, i64 240, !7, i64 244, !68, i64 248, !68, i64 256, !68, i64 264, !7, i64 272, !7, i64 276, !7, i64 280, !24, i64 284}
!453 = !{!452, !7, i64 276}
!454 = !{!452, !68, i64 264}
!455 = !{!452, !68, i64 256}
!456 = !{!452, !7, i64 224}
!457 = !{!452, !68, i64 232}
!458 = distinct !{!458, !9}
!459 = distinct !{!459, !9}
!460 = !{!452, !68, i64 248}
!461 = !{!310, !7, i64 20}
!462 = !{!310, !7, i64 8}
!463 = !{!310, !7, i64 12}
!464 = distinct !{!464, !9}
!465 = !{!435, !7, i64 64}
!466 = !{!435, !7, i64 80}
!467 = !{!435, !68, i64 72}
!468 = !{!435, !24, i64 44}
!469 = !{!435, !7, i64 48}
!470 = !{!435, !7, i64 84}
!471 = !{!435, !7, i64 88}
!472 = distinct !{!472, !9}
!473 = !{!452, !7, i64 244}
!474 = !{!452, !24, i64 284}
!475 = distinct !{!475, !9}
!476 = !{!477, !68, i64 160}
!477 = !{!"", !281, i64 0, !429, i64 32, !429, i64 96, !68, i64 160, !68, i64 168, !7, i64 176, !68, i64 184, !68, i64 192, !68, i64 200, !24, i64 208, !24, i64 212, !7, i64 216, !7, i64 220, !7, i64 224}
!478 = !{!477, !68, i64 168}
!479 = !{!477, !7, i64 216}
!480 = !{!477, !68, i64 192}
!481 = !{!477, !68, i64 200}
!482 = !{!477, !68, i64 184}
!483 = !{!477, !7, i64 176}
!484 = distinct !{!484, !9}
!485 = !{!477, !24, i64 208}
!486 = !{!310, !7, i64 16}
!487 = !{!477, !7, i64 224}
!488 = distinct !{!488, !9}
!489 = distinct !{!489, !9}
!490 = distinct !{!490, !9}
!491 = !{!492, !492, i64 0}
!492 = !{!"__int128", !2, i64 0}
!493 = distinct !{!493, !9}
!494 = distinct !{!494, !9}
!495 = distinct !{!495, !9}
!496 = distinct !{!496, !9}
!497 = distinct !{!497, !9}
!498 = distinct !{!498, !9}
!499 = !{i64 7, i64 65}
!500 = distinct !{!500, !9}
!501 = distinct !{!501, !9}
!502 = !{!194, !68, i64 24}
!503 = !{!194, !7, i64 16}
!504 = distinct !{!504, !9}
!505 = distinct !{!505, !9}
!506 = distinct !{!506, !9}
!507 = !{i64 0, i64 4, !23, i64 4, i64 4, !23, i64 8, i64 8, !89, i64 16, i64 8, !508, i64 24, i64 4, !6, i64 28, i64 4, !4, i64 32, i64 4, !6, i64 36, i64 4, !4, i64 40, i64 4, !6, i64 44, i64 4, !4}
!508 = !{!369, !369, i64 0}
!509 = distinct !{!509, !9}
!510 = distinct !{!510, !9}
!511 = distinct !{!511, !9}
!512 = !{!367, !5, i64 56}
!513 = !{!367, !7, i64 52}
!514 = distinct !{!514, !9}
!515 = distinct !{!515, !9}
!516 = distinct !{!516, !9}
!517 = distinct !{!517, !9}
!518 = distinct !{!518, !9}
!519 = distinct !{!519, !9}
!520 = distinct !{!520, !9}
!521 = distinct !{!521, !9}
