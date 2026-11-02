source_filename = "fa_runtime.c"

%struct.BareError = type { i64 }
%struct.Range = type { i32, i32 }
%struct.FaString = type { ptr, i32, i32 }
%struct.Boundaries = type { ptr, i32, i32, i32 }
%struct.Replacing = type { ptr, i32, i32, ptr, i32, i32, i32 }
%struct.Splitting = type { ptr, i32, i32, ptr, i32 }
%struct.FaArrayHeader = type { ptr, i64, i64 }
%struct.IndexError = type { i64, i64, i64 }
%struct.FaSegmented = type { i64, i64, [48 x ptr] }
%struct.PageHeader = type { i32, i32 }
%struct.FreeSpan = type { ptr, i32 }
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
%struct.FaMutex = type { %struct.FaShared, i32, i32, [8 x i8], %struct.FaSystemLock, %struct.FaSystemLock, [0 x i8] }
%struct.FaSystemLock = type { [64 x i8] }
%struct.FaChannel = type { %struct.FaShared, %struct.FaSystemLock, %struct.FaSystemLock, i64, i64, ptr, i64, i64, i64, i32, i32, ptr }

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
@.str.12 = private unnamed_addr constant [3 x i8] c"SS\00", align 1
@argument_bytes = internal unnamed_addr global ptr null, align 4
@argument_count = internal unnamed_addr global i32 0, align 4
@argument_items = internal unnamed_addr global ptr null, align 4
@free_blocks = internal unnamed_addr global [40 x ptr] zeroinitializer, align 16
@builder_capacity = internal unnamed_addr global i32 0, align 4
@fa_live_blocks = hidden global i32 0, align 4
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
@.str.30 = private unnamed_addr constant [38 x i8] c"`lock()` on a mutex this thread holds\00", align 1
@.str.31 = private unnamed_addr constant [60 x i8] c"`send` on a full channel that no other thread receives from\00", align 1
@.str.32 = private unnamed_addr constant [60 x i8] c"`receive` on an empty channel that no other thread sends to\00", align 1
@EXTEND = internal unnamed_addr constant [153 x %struct.Range] [%struct.Range { i32 768, i32 879 }, %struct.Range { i32 1155, i32 1161 }, %struct.Range { i32 1425, i32 1469 }, %struct.Range { i32 1471, i32 1471 }, %struct.Range { i32 1473, i32 1474 }, %struct.Range { i32 1476, i32 1477 }, %struct.Range { i32 1479, i32 1479 }, %struct.Range { i32 1552, i32 1562 }, %struct.Range { i32 1611, i32 1631 }, %struct.Range { i32 1648, i32 1648 }, %struct.Range { i32 1750, i32 1756 }, %struct.Range { i32 1759, i32 1764 }, %struct.Range { i32 1767, i32 1768 }, %struct.Range { i32 1770, i32 1773 }, %struct.Range { i32 1809, i32 1809 }, %struct.Range { i32 1840, i32 1866 }, %struct.Range { i32 1958, i32 1968 }, %struct.Range { i32 2027, i32 2035 }, %struct.Range { i32 2070, i32 2093 }, %struct.Range { i32 2137, i32 2139 }, %struct.Range { i32 2200, i32 2207 }, %struct.Range { i32 2250, i32 2273 }, %struct.Range { i32 2275, i32 2307 }, %struct.Range { i32 2362, i32 2364 }, %struct.Range { i32 2366, i32 2383 }, %struct.Range { i32 2385, i32 2391 }, %struct.Range { i32 2402, i32 2403 }, %struct.Range { i32 2433, i32 2435 }, %struct.Range { i32 2492, i32 2492 }, %struct.Range { i32 2494, i32 2509 }, %struct.Range { i32 2519, i32 2519 }, %struct.Range { i32 2530, i32 2531 }, %struct.Range { i32 2561, i32 2563 }, %struct.Range { i32 2620, i32 2641 }, %struct.Range { i32 2672, i32 2673 }, %struct.Range { i32 2677, i32 2677 }, %struct.Range { i32 2689, i32 2691 }, %struct.Range { i32 2748, i32 2748 }, %struct.Range { i32 2750, i32 2765 }, %struct.Range { i32 2786, i32 2787 }, %struct.Range { i32 2817, i32 2819 }, %struct.Range { i32 2876, i32 2876 }, %struct.Range { i32 2878, i32 2903 }, %struct.Range { i32 2914, i32 2915 }, %struct.Range { i32 2946, i32 2946 }, %struct.Range { i32 3006, i32 3021 }, %struct.Range { i32 3031, i32 3031 }, %struct.Range { i32 3072, i32 3076 }, %struct.Range { i32 3132, i32 3132 }, %struct.Range { i32 3134, i32 3158 }, %struct.Range { i32 3170, i32 3171 }, %struct.Range { i32 3201, i32 3203 }, %struct.Range { i32 3260, i32 3260 }, %struct.Range { i32 3262, i32 3286 }, %struct.Range { i32 3298, i32 3299 }, %struct.Range { i32 3328, i32 3331 }, %struct.Range { i32 3387, i32 3388 }, %struct.Range { i32 3390, i32 3405 }, %struct.Range { i32 3415, i32 3415 }, %struct.Range { i32 3426, i32 3427 }, %struct.Range { i32 3457, i32 3459 }, %struct.Range { i32 3530, i32 3551 }, %struct.Range { i32 3570, i32 3571 }, %struct.Range { i32 3633, i32 3633 }, %struct.Range { i32 3635, i32 3642 }, %struct.Range { i32 3655, i32 3662 }, %struct.Range { i32 3761, i32 3761 }, %struct.Range { i32 3763, i32 3772 }, %struct.Range { i32 3784, i32 3790 }, %struct.Range { i32 3864, i32 3865 }, %struct.Range { i32 3893, i32 3893 }, %struct.Range { i32 3895, i32 3895 }, %struct.Range { i32 3897, i32 3897 }, %struct.Range { i32 3902, i32 3903 }, %struct.Range { i32 3953, i32 3972 }, %struct.Range { i32 3974, i32 3975 }, %struct.Range { i32 3981, i32 4028 }, %struct.Range { i32 4038, i32 4038 }, %struct.Range { i32 4139, i32 4158 }, %struct.Range { i32 4182, i32 4185 }, %struct.Range { i32 4190, i32 4192 }, %struct.Range { i32 4194, i32 4196 }, %struct.Range { i32 4199, i32 4205 }, %struct.Range { i32 4209, i32 4212 }, %struct.Range { i32 4226, i32 4237 }, %struct.Range { i32 4239, i32 4239 }, %struct.Range { i32 4250, i32 4253 }, %struct.Range { i32 4957, i32 4959 }, %struct.Range { i32 5906, i32 5909 }, %struct.Range { i32 5938, i32 5940 }, %struct.Range { i32 5970, i32 5971 }, %struct.Range { i32 6002, i32 6003 }, %struct.Range { i32 6068, i32 6099 }, %struct.Range { i32 6109, i32 6109 }, %struct.Range { i32 6155, i32 6157 }, %struct.Range { i32 6159, i32 6159 }, %struct.Range { i32 6277, i32 6278 }, %struct.Range { i32 6313, i32 6313 }, %struct.Range { i32 6432, i32 6459 }, %struct.Range { i32 6679, i32 6683 }, %struct.Range { i32 6741, i32 6783 }, %struct.Range { i32 6832, i32 6911 }, %struct.Range { i32 6912, i32 6916 }, %struct.Range { i32 6964, i32 6980 }, %struct.Range { i32 7019, i32 7027 }, %struct.Range { i32 7040, i32 7042 }, %struct.Range { i32 7073, i32 7085 }, %struct.Range { i32 7142, i32 7155 }, %struct.Range { i32 7204, i32 7223 }, %struct.Range { i32 7376, i32 7378 }, %struct.Range { i32 7380, i32 7400 }, %struct.Range { i32 7405, i32 7405 }, %struct.Range { i32 7412, i32 7412 }, %struct.Range { i32 7415, i32 7417 }, %struct.Range { i32 7616, i32 7679 }, %struct.Range { i32 8204, i32 8204 }, %struct.Range { i32 8400, i32 8432 }, %struct.Range { i32 11503, i32 11505 }, %struct.Range { i32 11647, i32 11647 }, %struct.Range { i32 11744, i32 11775 }, %struct.Range { i32 12330, i32 12335 }, %struct.Range { i32 12441, i32 12442 }, %struct.Range { i32 42607, i32 42610 }, %struct.Range { i32 42612, i32 42621 }, %struct.Range { i32 42654, i32 42655 }, %struct.Range { i32 42736, i32 42737 }, %struct.Range { i32 43010, i32 43010 }, %struct.Range { i32 43014, i32 43014 }, %struct.Range { i32 43019, i32 43019 }, %struct.Range { i32 43043, i32 43047 }, %struct.Range { i32 43052, i32 43052 }, %struct.Range { i32 43136, i32 43137 }, %struct.Range { i32 43188, i32 43205 }, %struct.Range { i32 43232, i32 43249 }, %struct.Range { i32 43263, i32 43263 }, %struct.Range { i32 43302, i32 43309 }, %struct.Range { i32 43335, i32 43347 }, %struct.Range { i32 43392, i32 43395 }, %struct.Range { i32 43443, i32 43456 }, %struct.Range { i32 43561, i32 43574 }, %struct.Range { i32 43587, i32 43587 }, %struct.Range { i32 43596, i32 43597 }, %struct.Range { i32 43755, i32 43759 }, %struct.Range { i32 43765, i32 43766 }, %struct.Range { i32 44003, i32 44010 }, %struct.Range { i32 44012, i32 44013 }, %struct.Range { i32 64286, i32 64286 }, %struct.Range { i32 65024, i32 65039 }, %struct.Range { i32 65056, i32 65071 }, %struct.Range { i32 65438, i32 65439 }, %struct.Range { i32 127995, i32 127999 }, %struct.Range { i32 917536, i32 917631 }, %struct.Range { i32 917760, i32 917999 }], align 16
@PICTOGRAPHIC = internal unnamed_addr constant [22 x %struct.Range] [%struct.Range { i32 169, i32 169 }, %struct.Range { i32 174, i32 174 }, %struct.Range { i32 8252, i32 8252 }, %struct.Range { i32 8265, i32 8265 }, %struct.Range { i32 8482, i32 8482 }, %struct.Range { i32 8505, i32 8505 }, %struct.Range { i32 8596, i32 8618 }, %struct.Range { i32 8986, i32 9215 }, %struct.Range { i32 9410, i32 9410 }, %struct.Range { i32 9642, i32 9726 }, %struct.Range { i32 9728, i32 10175 }, %struct.Range { i32 10548, i32 10549 }, %struct.Range { i32 11013, i32 11093 }, %struct.Range { i32 12336, i32 12336 }, %struct.Range { i32 12349, i32 12349 }, %struct.Range { i32 12951, i32 12951 }, %struct.Range { i32 12953, i32 12953 }, %struct.Range { i32 126976, i32 127231 }, %struct.Range { i32 127245, i32 127405 }, %struct.Range { i32 127489, i32 127994 }, %struct.Range { i32 128000, i32 129791 }, %struct.Range { i32 130048, i32 131069 }], align 16
@.str.33 = private unnamed_addr constant [1 x i8] zeroinitializer, align 1
@ascii_table = internal constant [128 x i8] c"\00\01\02\03\04\05\06\07\08\09\0A\0B\0C\0D\0E\0F\10\11\12\13\14\15\16\17\18\19\1A\1B\1C\1D\1E\1F !\22#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\7F", align 16
@out_of_memory.message = internal constant [21 x i8] c"stop: out of memory\0A\00", align 16
@class_next = internal unnamed_addr global [40 x ptr] zeroinitializer, align 16
@class_end = internal unnamed_addr global [40 x ptr] zeroinitializer, align 16
@free_spans = internal unnamed_addr global ptr null, align 4
@.str.34 = private unnamed_addr constant [34 x i8] c"a derived value depends on itself\00", align 1
@.str.35 = private unnamed_addr constant [7 x i8] c"stop: \00", align 1
@.str.36 = private unnamed_addr constant [7 x i8] c"\0A  at \00", align 1
@.str.37 = private unnamed_addr constant [17 x i8] c"stop: deadlock: \00", align 1
@llvm.used = appending global [1 x ptr] [ptr @fa_flush], section "llvm.metadata"

; Function Attrs: nounwind optsize
define hidden void @fa_flush() #0 {
  %1 = load i32, ptr @output_length, align 4, !tbaa !1
  %2 = icmp eq i32 %1, 0
  br i1 %2, label %4, label %3

3:                                                ; preds = %0
  tail call void @fa_write(i32 noundef 1, ptr noundef nonnull @output, i32 noundef %1) #41
  store i32 0, ptr @output_length, align 4, !tbaa !1
  br label %4

4:                                                ; preds = %0, %3
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_out(i32 noundef %0, ptr noundef %1, i32 noundef %2) local_unnamed_addr #1 {
  %4 = icmp eq i32 %0, 1
  %5 = load i32, ptr @output_length, align 4, !tbaa !1
  br i1 %4, label %10, label %6

6:                                                ; preds = %3
  %7 = icmp eq i32 %5, 0
  br i1 %7, label %9, label %8

8:                                                ; preds = %6
  tail call void @fa_write(i32 noundef 1, ptr noundef nonnull @output, i32 noundef %5) #41
  store i32 0, ptr @output_length, align 4, !tbaa !1
  br label %9

9:                                                ; preds = %6, %8
  tail call void @fa_write(i32 noundef %0, ptr noundef %1, i32 noundef %2) #41
  br label %23

10:                                               ; preds = %3
  %11 = add i32 %5, %2
  %12 = icmp ugt i32 %11, 8192
  br i1 %12, label %13, label %19

13:                                               ; preds = %10
  %14 = icmp eq i32 %5, 0
  br i1 %14, label %16, label %15

15:                                               ; preds = %13
  tail call void @fa_write(i32 noundef 1, ptr noundef nonnull @output, i32 noundef %5) #41
  store i32 0, ptr @output_length, align 4, !tbaa !1
  br label %16

16:                                               ; preds = %13, %15
  %17 = icmp ugt i32 %2, 8192
  br i1 %17, label %18, label %19

18:                                               ; preds = %16
  tail call void @fa_write(i32 noundef 1, ptr noundef %1, i32 noundef %2) #41
  br label %23

19:                                               ; preds = %16, %10
  %20 = phi i32 [ %5, %10 ], [ 0, %16 ]
  %21 = getelementptr inbounds i8, ptr @output, i32 %20
  tail call fastcc void @copy_bytes(ptr noundef nonnull %21, ptr noundef %1, i32 noundef %2) #42
  %22 = add i32 %20, %2
  store i32 %22, ptr @output_length, align 4, !tbaa !1
  br label %23

23:                                               ; preds = %19, %18, %9
  ret void
}

; Function Attrs: optsize
declare void @fa_write(i32 noundef, ptr noundef, i32 noundef) local_unnamed_addr #2

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: readwrite)
define internal fastcc void @copy_bytes(ptr nocapture noundef writeonly %0, ptr nocapture noundef readonly %1, i32 noundef %2) unnamed_addr #3 {
  %4 = icmp ult i32 %2, 17
  br i1 %4, label %5, label %35

5:                                                ; preds = %3
  %6 = icmp ugt i32 %2, 7
  br i1 %6, label %7, label %14

7:                                                ; preds = %5
  %8 = load i64, ptr %1, align 1
  %9 = getelementptr inbounds i8, ptr %1, i32 %2
  %10 = getelementptr inbounds i8, ptr %9, i32 -8
  %11 = load i64, ptr %10, align 1
  store i64 %8, ptr %0, align 1
  %12 = getelementptr inbounds i8, ptr %0, i32 %2
  %13 = getelementptr inbounds i8, ptr %12, i32 -8
  store i64 %11, ptr %13, align 1
  br label %36

14:                                               ; preds = %5
  %15 = icmp ugt i32 %2, 3
  br i1 %15, label %16, label %23

16:                                               ; preds = %14
  %17 = load i32, ptr %1, align 1
  %18 = getelementptr inbounds i8, ptr %1, i32 %2
  %19 = getelementptr inbounds i8, ptr %18, i32 -4
  %20 = load i32, ptr %19, align 1
  store i32 %17, ptr %0, align 1
  %21 = getelementptr inbounds i8, ptr %0, i32 %2
  %22 = getelementptr inbounds i8, ptr %21, i32 -4
  store i32 %20, ptr %22, align 1
  br label %36

23:                                               ; preds = %14
  %24 = icmp eq i32 %2, 0
  br i1 %24, label %36, label %25

25:                                               ; preds = %23
  %26 = load i8, ptr %1, align 1, !tbaa !5
  %27 = lshr i32 %2, 1
  %28 = getelementptr inbounds i8, ptr %1, i32 %27
  %29 = load i8, ptr %28, align 1, !tbaa !5
  %30 = add nsw i32 %2, -1
  %31 = getelementptr inbounds i8, ptr %1, i32 %30
  %32 = load i8, ptr %31, align 1, !tbaa !5
  store i8 %26, ptr %0, align 1, !tbaa !5
  %33 = getelementptr inbounds i8, ptr %0, i32 %27
  store i8 %29, ptr %33, align 1, !tbaa !5
  %34 = getelementptr inbounds i8, ptr %0, i32 %30
  store i8 %32, ptr %34, align 1, !tbaa !5
  br label %36

35:                                               ; preds = %3
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %0, ptr align 1 %1, i32 %2, i1 false)
  br label %36

36:                                               ; preds = %7, %23, %25, %16, %35
  ret void
}

; Function Attrs: noreturn nounwind optsize
define hidden void @fa_terminate(i32 noundef %0) local_unnamed_addr #4 {
  tail call void @fa_flush() #42
  tail call void @fa_exit(i32 noundef %0) #43
  unreachable
}

; Function Attrs: noreturn optsize
declare void @fa_exit(i32 noundef) local_unnamed_addr #5

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, argmem: none, inaccessiblemem: none)
define hidden void @fa_b_reset() local_unnamed_addr #6 {
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_bytes(ptr nocapture noundef readonly %0, i32 noundef %1) local_unnamed_addr #1 {
  tail call fastcc void @reserve(i32 noundef %1) #42
  %3 = load ptr, ptr @builder, align 4, !tbaa !6
  %4 = load i32, ptr @builder_length, align 4, !tbaa !1
  %5 = getelementptr inbounds i8, ptr %3, i32 %4
  tail call fastcc void @copy_bytes(ptr noundef %5, ptr noundef %0, i32 noundef %1) #42
  %6 = add i32 %4, %1
  store i32 %6, ptr @builder_length, align 4, !tbaa !1
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @reserve(i32 noundef %0) unnamed_addr #1 {
  %2 = load i32, ptr @builder_length, align 4, !tbaa !1
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
  %15 = tail call ptr @fa_alloc(i32 noundef %11, i32 noundef 1) #41
  %16 = load ptr, ptr @builder, align 4, !tbaa !6
  %17 = load i32, ptr @builder_length, align 4, !tbaa !1
  tail call fastcc void @copy_bytes(ptr noundef %15, ptr noundef %16, i32 noundef %17) #42
  %18 = icmp eq ptr %16, null
  br i1 %18, label %21, label %19

19:                                               ; preds = %14
  %20 = load i32, ptr @builder_capacity, align 4, !tbaa !1
  tail call void @fa_free(ptr noundef nonnull %16, i32 noundef %20, i32 noundef 1) #41
  br label %21

21:                                               ; preds = %19, %14
  store ptr %15, ptr @builder, align 4, !tbaa !6
  store i32 %11, ptr @builder_capacity, align 4, !tbaa !1
  br label %22

22:                                               ; preds = %1, %21
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_line(i32 noundef %0) local_unnamed_addr #1 {
  tail call fastcc void @reserve(i32 noundef 1) #42
  %2 = load ptr, ptr @builder, align 4, !tbaa !6
  %3 = load i32, ptr @builder_length, align 4, !tbaa !1
  %4 = getelementptr inbounds i8, ptr %2, i32 %3
  store i8 10, ptr %4, align 1, !tbaa !5
  %5 = add i32 %3, 1
  store i32 %5, ptr @builder_length, align 4, !tbaa !1
  tail call void @fa_out(i32 noundef %0, ptr noundef %2, i32 noundef %5) #42
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_take(ptr nocapture noundef writeonly %0) local_unnamed_addr #1 {
  %2 = alloca %struct.FaString, align 4
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %2) #44
  %3 = load ptr, ptr @builder, align 4, !tbaa !6
  %4 = load i32, ptr @builder_length, align 4, !tbaa !1
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %2, ptr noundef %3, i32 noundef %4, i32 noundef 0) #42
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %0, ptr noundef nonnull align 4 dereferenceable(12) %2, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %2) #44
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  ret void
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.start.p0(i64 immarg, ptr nocapture) #7

; Function Attrs: mustprogress nocallback nofree nounwind willreturn memory(argmem: readwrite)
declare void @llvm.memcpy.p0.p0.i32(ptr noalias nocapture writeonly, ptr noalias nocapture readonly, i32, i1 immarg) #8

; Function Attrs: mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.end.p0(i64 immarg, ptr nocapture) #7

; Function Attrs: nounwind optsize
define hidden void @fa_b_i64(i64 noundef %0) local_unnamed_addr #1 {
  %2 = tail call i64 @llvm.abs.i64(i64 %0, i1 false)
  %3 = lshr i64 %0, 63
  %4 = trunc i64 %3 to i32
  tail call fastcc void @append_u64(i64 noundef %2, i32 noundef %4) #42
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @append_u64(i64 noundef %0, i32 noundef %1) unnamed_addr #1 {
  %3 = alloca [24 x i8], align 16
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %3) #44
  br label %4

4:                                                ; preds = %4, %2
  %5 = phi i64 [ %0, %2 ], [ %9, %4 ]
  %6 = phi i32 [ 24, %2 ], [ %7, %4 ]
  %7 = add i32 %6, -1
  %8 = freeze i64 %5
  %9 = udiv i64 %8, 10
  %10 = mul i64 %9, 10
  %11 = sub i64 %8, %10
  %12 = trunc i64 %11 to i8
  %13 = or disjoint i8 %12, 48
  %14 = getelementptr inbounds [24 x i8], ptr %3, i32 0, i32 %7
  store i8 %13, ptr %14, align 1, !tbaa !5
  %15 = icmp ult i64 %5, 10
  br i1 %15, label %16, label %4, !llvm.loop !11

16:                                               ; preds = %4
  %17 = icmp eq i32 %1, 0
  br i1 %17, label %21, label %18

18:                                               ; preds = %16
  %19 = add i32 %6, -2
  %20 = getelementptr inbounds [24 x i8], ptr %3, i32 0, i32 %19
  store i8 45, ptr %20, align 1, !tbaa !5
  br label %21

21:                                               ; preds = %18, %16
  %22 = phi i32 [ %19, %18 ], [ %7, %16 ]
  %23 = getelementptr inbounds i8, ptr %3, i32 %22
  %24 = sub i32 24, %22
  tail call fastcc void @reserve(i32 noundef %24) #42
  %25 = load ptr, ptr @builder, align 4, !tbaa !6
  %26 = load i32, ptr @builder_length, align 4, !tbaa !1
  %27 = getelementptr inbounds i8, ptr %25, i32 %26
  call fastcc void @copy_bytes(ptr noundef %27, ptr noundef nonnull %23, i32 noundef %24) #42
  %28 = add i32 %26, %24
  store i32 %28, ptr @builder_length, align 4, !tbaa !1
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %3) #44
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_u64(i64 noundef %0) local_unnamed_addr #1 {
  tail call fastcc void @append_u64(i64 noundef %0, i32 noundef 0) #42
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_i128(i64 noundef %0, i64 noundef %1) local_unnamed_addr #1 {
  %3 = zext i64 %1 to i128
  %4 = shl nuw i128 %3, 64
  %5 = zext i64 %0 to i128
  %6 = or disjoint i128 %4, %5
  %7 = icmp slt i64 %1, 0
  %8 = sub i128 0, %6
  %9 = select i1 %7, i128 %8, i128 %6
  %10 = lshr i64 %1, 63
  %11 = trunc i64 %10 to i32
  tail call fastcc void @append_u128(i128 noundef %9, i32 noundef %11) #42
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @append_u128(i128 noundef %0, i32 noundef %1) unnamed_addr #1 {
  %3 = alloca [4 x i32], align 16
  %4 = alloca [48 x i8], align 16
  %5 = icmp ult i128 %0, 18446744073709551616
  br i1 %5, label %6, label %8

6:                                                ; preds = %2
  %7 = trunc i128 %0 to i64
  tail call fastcc void @append_u64(i64 noundef %7, i32 noundef %1) #42
  br label %80

8:                                                ; preds = %2
  %9 = lshr i128 %0, 64
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %3) #44
  %10 = lshr i128 %0, 96
  %11 = trunc i128 %10 to i32
  store i32 %11, ptr %3, align 16, !tbaa !12
  %12 = getelementptr inbounds i32, ptr %3, i32 1
  %13 = trunc i128 %9 to i32
  store i32 %13, ptr %12, align 4, !tbaa !12
  %14 = getelementptr inbounds i32, ptr %3, i32 2
  %15 = lshr i128 %0, 32
  %16 = trunc i128 %15 to i32
  store i32 %16, ptr %14, align 8, !tbaa !12
  %17 = getelementptr inbounds i32, ptr %3, i32 3
  %18 = trunc i128 %0 to i32
  store i32 %18, ptr %17, align 4, !tbaa !12
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %4) #44
  br label %19

19:                                               ; preds = %48, %8
  %20 = phi i32 [ 48, %8 ], [ %53, %48 ]
  br label %21

21:                                               ; preds = %21, %19
  %22 = phi i64 [ 0, %19 ], [ %33, %21 ]
  %23 = phi i32 [ 0, %19 ], [ %34, %21 ]
  %24 = shl nuw nsw i64 %22, 32
  %25 = getelementptr inbounds i32, ptr %3, i32 %23
  %26 = load i32, ptr %25, align 4, !tbaa !12
  %27 = zext i32 %26 to i64
  %28 = or disjoint i64 %24, %27
  %29 = freeze i64 %28
  %30 = udiv i64 %29, 1000000000
  %31 = trunc i64 %30 to i32
  store i32 %31, ptr %25, align 4, !tbaa !12
  %32 = mul i64 %30, 1000000000
  %33 = sub i64 %29, %32
  %34 = add nuw nsw i32 %23, 1
  %35 = icmp eq i32 %34, 4
  br i1 %35, label %36, label %21, !llvm.loop !14

36:                                               ; preds = %21
  %37 = trunc i64 %33 to i32
  %38 = load i32, ptr %3, align 16, !tbaa !12
  %39 = load i32, ptr %12, align 4, !tbaa !12
  %40 = or i32 %39, %38
  %41 = load i32, ptr %14, align 8, !tbaa !12
  %42 = or i32 %40, %41
  %43 = load i32, ptr %17, align 4, !tbaa !12
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
  store i8 %59, ptr %60, align 1, !tbaa !5
  %61 = add nuw nsw i32 %50, 1
  %62 = icmp ult i32 %50, 8
  %63 = icmp ugt i32 %51, 9
  %64 = select i1 %45, i1 true, i1 %63
  %65 = select i1 %62, i1 %64, i1 false
  br i1 %65, label %49, label %48, !llvm.loop !15

66:                                               ; preds = %36, %48
  %67 = phi i32 [ %53, %48 ], [ %20, %36 ]
  %68 = icmp eq i32 %1, 0
  br i1 %68, label %72, label %69

69:                                               ; preds = %66
  %70 = add i32 %67, -1
  %71 = getelementptr inbounds [48 x i8], ptr %4, i32 0, i32 %70
  store i8 45, ptr %71, align 1, !tbaa !5
  br label %72

72:                                               ; preds = %69, %66
  %73 = phi i32 [ %70, %69 ], [ %67, %66 ]
  %74 = getelementptr inbounds i8, ptr %4, i32 %73
  %75 = sub i32 48, %73
  tail call fastcc void @reserve(i32 noundef %75) #42
  %76 = load ptr, ptr @builder, align 4, !tbaa !6
  %77 = load i32, ptr @builder_length, align 4, !tbaa !1
  %78 = getelementptr inbounds i8, ptr %76, i32 %77
  call fastcc void @copy_bytes(ptr noundef %78, ptr noundef nonnull %74, i32 noundef %75) #42
  %79 = add i32 %77, %75
  store i32 %79, ptr @builder_length, align 4, !tbaa !1
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %4) #44
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %3) #44
  br label %80

80:                                               ; preds = %72, %6
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_u128(i64 noundef %0, i64 noundef %1) local_unnamed_addr #1 {
  %3 = zext i64 %1 to i128
  %4 = shl nuw i128 %3, 64
  %5 = zext i64 %0 to i128
  %6 = or disjoint i128 %4, %5
  tail call fastcc void @append_u128(i128 noundef %6, i32 noundef 0) #42
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_bool(i32 noundef %0) local_unnamed_addr #1 {
  %2 = icmp eq i32 %0, 0
  %3 = select i1 %2, ptr @.str.1, ptr @.str
  tail call fastcc void @append_c(ptr noundef nonnull %3) #42
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @append_c(ptr nocapture noundef readonly %0) unnamed_addr #1 {
  br label %2

2:                                                ; preds = %2, %1
  %3 = phi i32 [ 0, %1 ], [ %7, %2 ]
  %4 = getelementptr inbounds i8, ptr %0, i32 %3
  %5 = load i8, ptr %4, align 1, !tbaa !5
  %6 = icmp eq i8 %5, 0
  %7 = add i32 %3, 1
  br i1 %6, label %8, label %2, !llvm.loop !16

8:                                                ; preds = %2
  tail call fastcc void @reserve(i32 noundef %3) #42
  %9 = load ptr, ptr @builder, align 4, !tbaa !6
  %10 = load i32, ptr @builder_length, align 4, !tbaa !1
  %11 = getelementptr inbounds i8, ptr %9, i32 %10
  tail call fastcc void @copy_bytes(ptr noundef %11, ptr noundef nonnull %0, i32 noundef %3) #42
  %12 = add i32 %10, %3
  store i32 %12, ptr @builder_length, align 4, !tbaa !1
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_f64(double noundef %0) local_unnamed_addr #1 {
  tail call fastcc void @reserve(i32 noundef 40) #42
  %2 = load ptr, ptr @builder, align 4, !tbaa !6
  %3 = load i32, ptr @builder_length, align 4, !tbaa !1
  %4 = getelementptr inbounds i8, ptr %2, i32 %3
  %5 = tail call i32 @fa_host_format_float(double noundef %0, i32 noundef 0, ptr noundef %4) #41
  %6 = load i32, ptr @builder_length, align 4, !tbaa !1
  %7 = add i32 %6, %5
  store i32 %7, ptr @builder_length, align 4, !tbaa !1
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_f32(float noundef %0) local_unnamed_addr #1 {
  %2 = fpext float %0 to double
  tail call fastcc void @reserve(i32 noundef 40) #42
  %3 = load ptr, ptr @builder, align 4, !tbaa !6
  %4 = load i32, ptr @builder_length, align 4, !tbaa !1
  %5 = getelementptr inbounds i8, ptr %3, i32 %4
  %6 = tail call i32 @fa_host_format_float(double noundef %2, i32 noundef 1, ptr noundef %5) #41
  %7 = load i32, ptr @builder_length, align 4, !tbaa !1
  %8 = add i32 %7, %6
  store i32 %8, ptr @builder_length, align 4, !tbaa !1
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_percentage(double noundef %0) local_unnamed_addr #1 {
  tail call fastcc void @reserve(i32 noundef 40) #42
  %2 = load ptr, ptr @builder, align 4, !tbaa !6
  %3 = load i32, ptr @builder_length, align 4, !tbaa !1
  %4 = getelementptr inbounds i8, ptr %2, i32 %3
  %5 = tail call i32 @fa_host_format_float(double noundef %0, i32 noundef 2, ptr noundef %4) #41
  %6 = load i32, ptr @builder_length, align 4, !tbaa !1
  %7 = add i32 %6, %5
  store i32 %7, ptr @builder_length, align 4, !tbaa !1
  tail call fastcc void @reserve(i32 noundef 1) #42
  %8 = load ptr, ptr @builder, align 4, !tbaa !6
  %9 = load i32, ptr @builder_length, align 4, !tbaa !1
  %10 = getelementptr inbounds i8, ptr %8, i32 %9
  store i8 37, ptr %10, align 1, !tbaa !5
  %11 = add i32 %9, 1
  store i32 %11, ptr @builder_length, align 4, !tbaa !1
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_quoted(ptr nocapture noundef readonly %0, i32 noundef %1) local_unnamed_addr #1 {
  tail call fastcc void @reserve(i32 noundef 1) #42
  %3 = load ptr, ptr @builder, align 4, !tbaa !6
  %4 = load i32, ptr @builder_length, align 4, !tbaa !1
  %5 = getelementptr inbounds i8, ptr %3, i32 %4
  store i8 34, ptr %5, align 1, !tbaa !5
  %6 = add i32 %4, 1
  store i32 %6, ptr @builder_length, align 4, !tbaa !1
  %7 = icmp eq i32 %1, 0
  br i1 %7, label %8, label %13

8:                                                ; preds = %50, %2
  tail call fastcc void @reserve(i32 noundef 1) #42
  %9 = load ptr, ptr @builder, align 4, !tbaa !6
  %10 = load i32, ptr @builder_length, align 4, !tbaa !1
  %11 = getelementptr inbounds i8, ptr %9, i32 %10
  store i8 34, ptr %11, align 1, !tbaa !5
  %12 = add i32 %10, 1
  store i32 %12, ptr @builder_length, align 4, !tbaa !1
  ret void

13:                                               ; preds = %2, %50
  %14 = phi i32 [ %51, %50 ], [ 0, %2 ]
  %15 = getelementptr inbounds i8, ptr %0, i32 %14
  %16 = load i8, ptr %15, align 1, !tbaa !5
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
  tail call fastcc void @append_c(ptr noundef nonnull @.str.2) #42
  br label %50

19:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.3) #42
  br label %50

20:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.4) #42
  br label %50

21:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.5) #42
  br label %50

22:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.6) #42
  br label %50

23:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.7) #42
  br label %50

24:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.8) #42
  br label %50

25:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.9) #42
  br label %50

26:                                               ; preds = %13
  %27 = icmp ult i8 %16, 32
  %28 = icmp eq i8 %16, 127
  %29 = or i1 %27, %28
  br i1 %29, label %30, label %45

30:                                               ; preds = %26
  tail call fastcc void @append_c(ptr noundef nonnull @.str.10) #42
  %31 = lshr i32 %17, 4
  %32 = getelementptr inbounds [17 x i8], ptr @fa_b_quoted.hex, i32 0, i32 %31
  %33 = load i8, ptr %32, align 1, !tbaa !5
  tail call fastcc void @reserve(i32 noundef 1) #42
  %34 = load ptr, ptr @builder, align 4, !tbaa !6
  %35 = load i32, ptr @builder_length, align 4, !tbaa !1
  %36 = getelementptr inbounds i8, ptr %34, i32 %35
  store i8 %33, ptr %36, align 1, !tbaa !5
  %37 = add i32 %35, 1
  store i32 %37, ptr @builder_length, align 4, !tbaa !1
  %38 = and i32 %17, 15
  %39 = getelementptr inbounds [17 x i8], ptr @fa_b_quoted.hex, i32 0, i32 %38
  %40 = load i8, ptr %39, align 1, !tbaa !5
  tail call fastcc void @reserve(i32 noundef 1) #42
  %41 = load ptr, ptr @builder, align 4, !tbaa !6
  %42 = load i32, ptr @builder_length, align 4, !tbaa !1
  %43 = getelementptr inbounds i8, ptr %41, i32 %42
  store i8 %40, ptr %43, align 1, !tbaa !5
  %44 = add i32 %42, 1
  store i32 %44, ptr @builder_length, align 4, !tbaa !1
  br label %50

45:                                               ; preds = %26
  tail call fastcc void @reserve(i32 noundef 1) #42
  %46 = load ptr, ptr @builder, align 4, !tbaa !6
  %47 = load i32, ptr @builder_length, align 4, !tbaa !1
  %48 = getelementptr inbounds i8, ptr %46, i32 %47
  store i8 %16, ptr %48, align 1, !tbaa !5
  %49 = add i32 %47, 1
  store i32 %49, ptr @builder_length, align 4, !tbaa !1
  br label %50

50:                                               ; preds = %30, %45, %25, %24, %23, %22, %21, %20, %19, %18
  %51 = add nuw i32 %14, 1
  %52 = icmp eq i32 %51, %1
  br i1 %52, label %8, label %13, !llvm.loop !17
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_string_array(ptr nocapture noundef readonly %0, i32 noundef %1) local_unnamed_addr #1 {
  tail call fastcc void @reserve(i32 noundef 1) #42
  %3 = load ptr, ptr @builder, align 4, !tbaa !6
  %4 = load i32, ptr @builder_length, align 4, !tbaa !1
  %5 = getelementptr inbounds i8, ptr %3, i32 %4
  store i8 91, ptr %5, align 1, !tbaa !5
  %6 = add i32 %4, 1
  store i32 %6, ptr @builder_length, align 4, !tbaa !1
  %7 = icmp eq i32 %1, 0
  br i1 %7, label %8, label %13

8:                                                ; preds = %17, %2
  tail call fastcc void @reserve(i32 noundef 1) #42
  %9 = load ptr, ptr @builder, align 4, !tbaa !6
  %10 = load i32, ptr @builder_length, align 4, !tbaa !1
  %11 = getelementptr inbounds i8, ptr %9, i32 %10
  store i8 93, ptr %11, align 1, !tbaa !5
  %12 = add i32 %10, 1
  store i32 %12, ptr @builder_length, align 4, !tbaa !1
  ret void

13:                                               ; preds = %2, %17
  %14 = phi i32 [ %22, %17 ], [ 0, %2 ]
  %15 = icmp eq i32 %14, 0
  br i1 %15, label %17, label %16

16:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.11) #42
  br label %17

17:                                               ; preds = %16, %13
  %18 = getelementptr inbounds %struct.FaString, ptr %0, i32 %14
  %19 = load ptr, ptr %18, align 4, !tbaa !18
  %20 = getelementptr inbounds %struct.FaString, ptr %0, i32 %14, i32 1
  %21 = load i32, ptr %20, align 4, !tbaa !20
  tail call void @fa_b_quoted(ptr noundef %19, i32 noundef %21) #42
  %22 = add nuw i32 %14, 1
  %23 = icmp eq i32 %22, %1
  br i1 %23, label %8, label %13, !llvm.loop !21
}

; Function Attrs: nounwind optsize
define hidden void @fa_concat(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef readonly %2, i32 noundef %3, ptr nocapture noundef writeonly %4) local_unnamed_addr #1 {
  %6 = alloca %struct.FaString, align 4
  %7 = add i32 %3, %1
  %8 = icmp ult i32 %7, 2
  br i1 %8, label %9, label %12

9:                                                ; preds = %5
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %6) #44
  %10 = icmp eq i32 %1, 0
  %11 = select i1 %10, ptr %2, ptr %0
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %6, ptr noundef %11, i32 noundef %7, i32 noundef 0) #42
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %4, ptr noundef nonnull align 4 dereferenceable(12) %6, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %6) #44
  br label %19

12:                                               ; preds = %5
  %13 = tail call ptr @fa_alloc(i32 noundef %7, i32 noundef 1) #41
  tail call fastcc void @copy_bytes(ptr noundef %13, ptr noundef %0, i32 noundef %1) #42
  %14 = getelementptr inbounds i8, ptr %13, i32 %1
  tail call fastcc void @copy_bytes(ptr noundef %14, ptr noundef %2, i32 noundef %3) #42
  store ptr %13, ptr %4, align 4, !tbaa !18
  %15 = getelementptr inbounds %struct.FaString, ptr %4, i32 0, i32 1
  store i32 %7, ptr %15, align 4, !tbaa !20
  %16 = tail call fastcc i32 @simple_flag(ptr noundef %13, i32 noundef %7) #42, !range !22
  %17 = or i32 %16, %7
  %18 = getelementptr inbounds %struct.FaString, ptr %4, i32 0, i32 2
  store i32 %17, ptr %18, align 4, !tbaa !23
  br label %19

19:                                               ; preds = %12, %9
  ret void
}

; Function Attrs: optsize
declare ptr @fa_alloc(i32 noundef, i32 noundef) local_unnamed_addr #2

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: read)
define internal fastcc i32 @simple_flag(ptr nocapture noundef readonly %0, i32 noundef %1) unnamed_addr #9 {
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
  br i1 %21, label %4, label %8, !llvm.loop !24

22:                                               ; preds = %4, %22
  %23 = phi i64 [ %30, %22 ], [ %6, %4 ]
  %24 = phi i32 [ %31, %22 ], [ %5, %4 ]
  %25 = getelementptr inbounds i8, ptr %0, i32 %24
  %26 = load i8, ptr %25, align 1, !tbaa !5
  %27 = icmp eq i8 %26, 13
  %28 = select i1 %27, i8 -128, i8 %26
  %29 = zext i8 %28 to i64
  %30 = or i64 %23, %29
  %31 = add nuw i32 %24, 1
  %32 = icmp eq i32 %31, %1
  br i1 %32, label %33, label %22, !llvm.loop !25

33:                                               ; preds = %22, %4
  %34 = phi i64 [ %6, %4 ], [ %30, %22 ]
  %35 = and i64 %34, -9187201950435737472
  %36 = icmp eq i64 %35, 0
  %37 = select i1 %36, i32 -2147483648, i32 0
  ret i32 %37
}

; Function Attrs: nounwind optsize
define hidden void @fa_string_append(ptr nocapture noundef %0, ptr nocapture noundef readonly %1, i32 noundef %2, i32 noundef %3) local_unnamed_addr #1 {
  %5 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 2
  %6 = load i32, ptr %5, align 4, !tbaa !23
  %7 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 1
  %8 = load i32, ptr %7, align 4, !tbaa !20
  %9 = add i32 %8, %2
  %10 = and i32 %6, 2147483647
  %11 = icmp slt i32 %3, 0
  %12 = icmp ule i32 %9, %10
  %13 = select i1 %11, i1 %12, i1 false
  %14 = icmp ne i32 %10, 2147483647
  %15 = select i1 %13, i1 %14, i1 false
  br i1 %15, label %16, label %19, !prof !26

16:                                               ; preds = %4
  %17 = load ptr, ptr %0, align 4, !tbaa !18
  %18 = getelementptr inbounds i8, ptr %17, i32 %8
  tail call fastcc void @copy_bytes(ptr noundef %18, ptr noundef %1, i32 noundef %2) #42
  store i32 %9, ptr %7, align 4, !tbaa !20
  br label %20

19:                                               ; preds = %4
  tail call fastcc void @string_append_slow(ptr noundef nonnull %0, ptr noundef %1, i32 noundef %2, i32 noundef %3) #42
  br label %20

20:                                               ; preds = %19, %16
  ret void
}

; Function Attrs: noinline nounwind optsize
define internal fastcc void @string_append_slow(ptr nocapture noundef %0, ptr nocapture noundef readonly %1, i32 noundef %2, i32 noundef %3) unnamed_addr #10 {
  %5 = icmp eq i32 %2, 0
  br i1 %5, label %37, label %6

6:                                                ; preds = %4
  %7 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 2
  %8 = load i32, ptr %7, align 4, !tbaa !23
  %9 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 1
  %10 = load i32, ptr %9, align 4, !tbaa !20
  %11 = add i32 %10, %2
  %12 = icmp sgt i32 %3, -1
  br i1 %12, label %13, label %23

13:                                               ; preds = %6
  %14 = icmp eq i32 %2, 1
  br i1 %14, label %15, label %21

15:                                               ; preds = %13
  %16 = load i8, ptr %1, align 1, !tbaa !5
  %17 = icmp sgt i8 %16, -1
  %18 = icmp eq i8 %16, 13
  %19 = select i1 %18, i32 0, i32 -2147483648
  %20 = select i1 %17, i32 %19, i32 0
  br label %23

21:                                               ; preds = %13
  %22 = tail call fastcc i32 @simple_flag(ptr noundef %1, i32 noundef %2) #42, !range !22
  br label %23

23:                                               ; preds = %15, %21, %6
  %24 = phi i32 [ -2147483648, %6 ], [ %20, %15 ], [ %22, %21 ]
  %25 = icmp eq i32 %10, 0
  %26 = select i1 %25, i32 -1, i32 %8
  %27 = and i32 %24, %26
  %28 = and i32 %8, 2147483647
  %29 = icmp ugt i32 %11, %28
  %30 = freeze i1 %29
  br i1 %30, label %36, label %31

31:                                               ; preds = %23
  switch i32 %8, label %32 [
    i32 -1, label %36
    i32 -2147483648, label %36
    i32 2147483647, label %36
    i32 0, label %36
  ]

32:                                               ; preds = %31
  %33 = load ptr, ptr %0, align 4, !tbaa !18
  %34 = getelementptr inbounds i8, ptr %33, i32 %10
  tail call fastcc void @copy_bytes(ptr noundef %34, ptr noundef %1, i32 noundef %2) #42
  store i32 %11, ptr %9, align 4, !tbaa !20
  %35 = or i32 %27, %28
  store i32 %35, ptr %7, align 4, !tbaa !23
  br label %37

36:                                               ; preds = %31, %31, %31, %31, %23
  tail call fastcc void @string_grow(ptr noundef nonnull %0, ptr noundef %1, i32 noundef %2, i32 noundef %27) #42
  br label %37

37:                                               ; preds = %32, %36, %4
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: read)
define hidden i32 @fa_string_equal(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef readonly %2, i32 noundef %3) local_unnamed_addr #9 {
  %5 = icmp eq i32 %1, %3
  br i1 %5, label %6, label %25

6:                                                ; preds = %4
  %7 = icmp eq i32 %1, 0
  br i1 %7, label %25, label %8

8:                                                ; preds = %6
  %9 = load i8, ptr %0, align 1, !tbaa !5
  %10 = load i8, ptr %2, align 1, !tbaa !5
  %11 = icmp eq i8 %9, %10
  br i1 %11, label %12, label %25

12:                                               ; preds = %8, %16
  %13 = phi i32 [ %14, %16 ], [ 0, %8 ]
  %14 = add nuw i32 %13, 1
  %15 = icmp eq i32 %14, %1
  br i1 %15, label %22, label %16, !llvm.loop !27

16:                                               ; preds = %12
  %17 = getelementptr inbounds i8, ptr %0, i32 %14
  %18 = load i8, ptr %17, align 1, !tbaa !5
  %19 = getelementptr inbounds i8, ptr %2, i32 %14
  %20 = load i8, ptr %19, align 1, !tbaa !5
  %21 = icmp eq i8 %18, %20
  br i1 %21, label %12, label %22, !llvm.loop !27

22:                                               ; preds = %16, %12
  %23 = icmp uge i32 %14, %1
  %24 = zext i1 %23 to i32
  br label %25

25:                                               ; preds = %8, %22, %6, %4
  %26 = phi i32 [ 0, %4 ], [ 1, %6 ], [ 0, %8 ], [ %24, %22 ]
  ret i32 %26
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: read)
define hidden i32 @fa_string_compare(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef readonly %2, i32 noundef %3) local_unnamed_addr #9 {
  %5 = tail call i32 @llvm.umin.i32(i32 %1, i32 %3)
  %6 = icmp eq i32 %5, 0
  br i1 %6, label %11, label %7

7:                                                ; preds = %4
  %8 = load i8, ptr %0, align 1, !tbaa !5
  %9 = load i8, ptr %2, align 1, !tbaa !5
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
  br i1 %19, label %39, label %20, !llvm.loop !28

20:                                               ; preds = %16
  %21 = getelementptr inbounds i8, ptr %0, i32 %18
  %22 = load i8, ptr %21, align 1, !tbaa !5
  %23 = getelementptr inbounds i8, ptr %2, i32 %18
  %24 = load i8, ptr %23, align 1, !tbaa !5
  %25 = icmp eq i8 %22, %24
  br i1 %25, label %16, label %26, !llvm.loop !28

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
define hidden i32 @fa_next_grapheme(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2) local_unnamed_addr #11 {
  %4 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %2) #42
  ret i32 %4
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define internal fastcc i32 @next_boundary(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2) unnamed_addr #11 {
  %4 = alloca i32, align 4
  %5 = alloca i32, align 4
  store i32 %2, ptr %4, align 4, !tbaa !1
  %6 = getelementptr inbounds i8, ptr %0, i32 %2
  %7 = load i8, ptr %6, align 1, !tbaa !5
  %8 = icmp sgt i8 %7, -1
  br i1 %8, label %9, label %24

9:                                                ; preds = %3
  %10 = add i32 %2, 1
  %11 = icmp ult i32 %10, %1
  br i1 %11, label %12, label %21

12:                                               ; preds = %9
  %13 = getelementptr inbounds i8, ptr %0, i32 %10
  %14 = load i8, ptr %13, align 1, !tbaa !5
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
  %25 = call fastcc i32 @decode(ptr noundef nonnull %0, i32 noundef %1, ptr noundef nonnull %4) #42
  %26 = tail call fastcc i32 @break_kind(i32 noundef %25) #42, !range !29
  %27 = load i32, ptr %4, align 4, !tbaa !1
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
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %5) #44
  store i32 %38, ptr %5, align 4, !tbaa !1
  %39 = call fastcc i32 @decode(ptr noundef %0, i32 noundef %1, ptr noundef nonnull %5) #42
  %40 = tail call fastcc i32 @break_kind(i32 noundef %39) #42, !range !29
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
  %75 = load i32, ptr %5, align 4, !tbaa !1
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
  %84 = load i32, ptr %5, align 4, !tbaa !1
  %85 = add nuw nsw i32 %36, 1
  br label %94

86:                                               ; preds = %53, %52, %52, %52, %52, %34
  %87 = load i32, ptr %5, align 4, !tbaa !1
  br label %94

88:                                               ; preds = %59
  %89 = load i32, ptr %5, align 4, !tbaa !1
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
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %5) #44
  br label %106

101:                                              ; preds = %74, %94, %88
  %102 = phi i32 [ %89, %88 ], [ %95, %94 ], [ %75, %74 ]
  %103 = phi i32 [ %92, %88 ], [ %96, %94 ], [ 0, %74 ]
  %104 = phi i32 [ 1, %88 ], [ %99, %94 ], [ 1, %74 ]
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %5) #44
  %105 = icmp ult i32 %102, %1
  br i1 %105, label %34, label %106

106:                                              ; preds = %101, %24, %100, %21
  %107 = phi i32 [ %23, %21 ], [ %38, %100 ], [ %27, %24 ], [ %102, %101 ]
  ret i32 %107
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define hidden i32 @fa_graphemes(ptr nocapture noundef readonly %0, i32 noundef %1) local_unnamed_addr #11 {
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
  %11 = load i8, ptr %10, align 1, !tbaa !5
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
  %23 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %6) #42
  %24 = add i32 %5, 1
  br label %25

25:                                               ; preds = %22, %13
  %26 = phi i32 [ %7, %13 ], [ %23, %22 ]
  %27 = phi i32 [ %21, %13 ], [ %24, %22 ]
  %28 = icmp ult i32 %26, %1
  br i1 %28, label %4, label %29, !llvm.loop !30

29:                                               ; preds = %25, %2
  %30 = phi i32 [ 0, %2 ], [ %27, %25 ]
  ret i32 %30
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_append_into(ptr nocapture noundef %0) local_unnamed_addr #1 {
  %2 = load ptr, ptr @builder, align 4, !tbaa !6
  %3 = load i32, ptr @builder_length, align 4, !tbaa !1
  %4 = tail call fastcc i32 @simple_flag(ptr noundef %2, i32 noundef %3) #42, !range !22
  tail call void @fa_string_append(ptr noundef %0, ptr noundef %2, i32 noundef %3, i32 noundef %4) #42
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden i32 @fa_string_includes(ptr noundef %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3, i32 noundef %4) local_unnamed_addr #12 {
  %6 = and i32 %2, -2147483648
  %7 = tail call fastcc i64 @find(ptr noundef %0, i32 noundef %1, i32 noundef %6, ptr noundef %3, i32 noundef %4, i32 noundef 0) #42
  %8 = icmp sgt i64 %7, -1
  %9 = zext i1 %8 to i32
  ret i32 %9
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc i64 @find(ptr noundef %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3, i32 noundef %4, i32 noundef %5) unnamed_addr #12 {
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
  %17 = load i8, ptr %3, align 1, !tbaa !5
  %18 = tail call fastcc i32 @next_byte(ptr noundef %0, i32 noundef %1, i32 noundef 0, i8 noundef zeroext %17) #42
  %19 = add i32 %18, %4
  %20 = icmp ugt i32 %19, %1
  br i1 %20, label %120, label %21

21:                                               ; preds = %16, %40
  %22 = phi i32 [ %42, %40 ], [ %18, %16 ]
  %23 = getelementptr i8, ptr %0, i32 %22
  %24 = load i8, ptr %23, align 1, !tbaa !5
  %25 = icmp eq i8 %24, %17
  br i1 %25, label %26, label %40

26:                                               ; preds = %21, %30
  %27 = phi i32 [ %28, %30 ], [ 0, %21 ]
  %28 = add nuw i32 %27, 1
  %29 = icmp eq i32 %28, %4
  br i1 %29, label %36, label %30, !llvm.loop !31

30:                                               ; preds = %26
  %31 = getelementptr i8, ptr %23, i32 %28
  %32 = load i8, ptr %31, align 1, !tbaa !5
  %33 = getelementptr inbounds i8, ptr %3, i32 %28
  %34 = load i8, ptr %33, align 1, !tbaa !5
  %35 = icmp eq i8 %32, %34
  br i1 %35, label %26, label %36, !llvm.loop !31

36:                                               ; preds = %30, %26
  %37 = icmp ult i32 %28, %4
  br i1 %37, label %40, label %38

38:                                               ; preds = %36
  %39 = zext i32 %22 to i64
  br label %120

40:                                               ; preds = %21, %36
  %41 = add i32 %22, 1
  %42 = tail call fastcc i32 @next_byte(ptr noundef nonnull %0, i32 noundef %1, i32 noundef %41, i8 noundef zeroext %17) #42
  %43 = add i32 %42, %4
  %44 = icmp ugt i32 %43, %1
  br i1 %44, label %120, label %21, !llvm.loop !32

45:                                               ; preds = %10
  br i1 %13, label %75, label %46

46:                                               ; preds = %45
  %47 = load i8, ptr %3, align 1, !tbaa !5
  %48 = sub i32 %1, %4
  %49 = icmp eq i32 %48, -1
  br i1 %49, label %120, label %50

50:                                               ; preds = %46, %72
  %51 = phi i32 [ %73, %72 ], [ 0, %46 ]
  %52 = sub i32 %48, %51
  %53 = select i1 %14, i32 %52, i32 %51
  %54 = getelementptr inbounds i8, ptr %0, i32 %53
  %55 = load i8, ptr %54, align 1, !tbaa !5
  %56 = icmp ne i8 %55, %47
  %57 = add i32 %53, %4
  %58 = icmp ugt i32 %57, %1
  %59 = or i1 %58, %56
  br i1 %59, label %72, label %60

60:                                               ; preds = %50, %64
  %61 = phi i32 [ %62, %64 ], [ 0, %50 ]
  %62 = add nuw i32 %61, 1
  %63 = icmp eq i32 %62, %4
  br i1 %63, label %70, label %64, !llvm.loop !31

64:                                               ; preds = %60
  %65 = getelementptr i8, ptr %54, i32 %62
  %66 = load i8, ptr %65, align 1, !tbaa !5
  %67 = getelementptr inbounds i8, ptr %3, i32 %62
  %68 = load i8, ptr %67, align 1, !tbaa !5
  %69 = icmp eq i8 %66, %68
  br i1 %69, label %60, label %70, !llvm.loop !31

70:                                               ; preds = %64, %60
  %71 = icmp ult i32 %62, %4
  br i1 %71, label %72, label %118

72:                                               ; preds = %50, %70
  %73 = add nuw i32 %51, 1
  %74 = icmp eq i32 %51, %48
  br i1 %74, label %120, label %50, !llvm.loop !33

75:                                               ; preds = %45
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %7) #44
  store ptr %0, ptr %7, align 4, !tbaa !34
  %76 = getelementptr inbounds %struct.Boundaries, ptr %7, i32 0, i32 1
  store i32 %1, ptr %76, align 4, !tbaa !36
  %77 = getelementptr inbounds %struct.Boundaries, ptr %7, i32 0, i32 2
  store i32 0, ptr %77, align 4, !tbaa !37
  %78 = getelementptr inbounds %struct.Boundaries, ptr %7, i32 0, i32 3
  store i32 %2, ptr %78, align 4, !tbaa !38
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %8) #44
  store ptr %0, ptr %8, align 4, !tbaa !34
  %79 = getelementptr inbounds %struct.Boundaries, ptr %8, i32 0, i32 1
  store i32 %1, ptr %79, align 4, !tbaa !36
  %80 = getelementptr inbounds %struct.Boundaries, ptr %8, i32 0, i32 2
  store i32 0, ptr %80, align 4, !tbaa !37
  %81 = getelementptr inbounds %struct.Boundaries, ptr %8, i32 0, i32 3
  store i32 %2, ptr %81, align 4, !tbaa !38
  br label %82

82:                                               ; preds = %75, %111
  %83 = phi i32 [ %114, %111 ], [ %4, %75 ]
  %84 = phi i32 [ %113, %111 ], [ 0, %75 ]
  %85 = phi i64 [ %112, %111 ], [ -1, %75 ]
  %86 = getelementptr i8, ptr %0, i32 %84
  br i1 %12, label %103, label %87

87:                                               ; preds = %82
  %88 = load i8, ptr %86, align 1, !tbaa !5
  %89 = load i8, ptr %3, align 1, !tbaa !5
  %90 = icmp eq i8 %88, %89
  br i1 %90, label %91, label %111

91:                                               ; preds = %87, %95
  %92 = phi i32 [ %93, %95 ], [ 0, %87 ]
  %93 = add nuw i32 %92, 1
  %94 = icmp eq i32 %93, %4
  br i1 %94, label %101, label %95, !llvm.loop !31

95:                                               ; preds = %91
  %96 = getelementptr i8, ptr %86, i32 %93
  %97 = load i8, ptr %96, align 1, !tbaa !5
  %98 = getelementptr inbounds i8, ptr %3, i32 %93
  %99 = load i8, ptr %98, align 1, !tbaa !5
  %100 = icmp eq i8 %97, %99
  br i1 %100, label %91, label %101, !llvm.loop !31

101:                                              ; preds = %95, %91
  %102 = icmp ult i32 %93, %4
  br i1 %102, label %111, label %103

103:                                              ; preds = %82, %101
  %104 = call fastcc i32 @at_boundary(ptr noundef nonnull %7, i32 noundef %84) #42, !range !39
  %105 = icmp eq i32 %104, 0
  br i1 %105, label %111, label %106

106:                                              ; preds = %103
  %107 = call fastcc i32 @at_boundary(ptr noundef nonnull %8, i32 noundef %83) #42, !range !39
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
  br i1 %115, label %116, label %82, !llvm.loop !40

116:                                              ; preds = %111, %109
  %117 = phi i64 [ %112, %111 ], [ %110, %109 ]
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %8) #44
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %7) #44
  br label %120

118:                                              ; preds = %70
  %119 = zext i32 %53 to i64
  br label %120

120:                                              ; preds = %40, %72, %16, %118, %46, %38, %6, %116
  %121 = phi i64 [ %117, %116 ], [ -1, %6 ], [ %39, %38 ], [ %119, %118 ], [ -1, %46 ], [ -1, %16 ], [ -1, %72 ], [ -1, %40 ]
  ret i64 %121
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define hidden i32 @fa_string_starts_with(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3, i32 noundef %4) local_unnamed_addr #11 {
  %6 = icmp ugt i32 %4, %1
  br i1 %6, label %37, label %7

7:                                                ; preds = %5
  %8 = icmp eq i32 %4, 0
  br i1 %8, label %27, label %9

9:                                                ; preds = %7
  %10 = load i8, ptr %0, align 1, !tbaa !5
  %11 = load i8, ptr %3, align 1, !tbaa !5
  %12 = icmp eq i8 %10, %11
  br i1 %12, label %13, label %37

13:                                               ; preds = %9, %17
  %14 = phi i32 [ %15, %17 ], [ 0, %9 ]
  %15 = add nuw i32 %14, 1
  %16 = icmp eq i32 %15, %4
  br i1 %16, label %23, label %17, !llvm.loop !31

17:                                               ; preds = %13
  %18 = getelementptr i8, ptr %0, i32 %15
  %19 = load i8, ptr %18, align 1, !tbaa !5
  %20 = getelementptr inbounds i8, ptr %3, i32 %15
  %21 = load i8, ptr %20, align 1, !tbaa !5
  %22 = icmp eq i8 %19, %21
  br i1 %22, label %13, label %23, !llvm.loop !31

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
  %31 = tail call fastcc i32 @next_boundary(ptr noundef nonnull %0, i32 noundef %1, i32 noundef %30) #42
  %32 = icmp ult i32 %31, %4
  br i1 %32, label %29, label %33, !llvm.loop !41

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
define hidden i32 @fa_string_ends_with(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3, i32 noundef %4) local_unnamed_addr #11 {
  %6 = icmp ugt i32 %4, %1
  br i1 %6, label %39, label %7

7:                                                ; preds = %5
  %8 = sub i32 %1, %4
  %9 = getelementptr i8, ptr %0, i32 %8
  %10 = icmp eq i32 %4, 0
  br i1 %10, label %27, label %11

11:                                               ; preds = %7
  %12 = load i8, ptr %9, align 1, !tbaa !5
  %13 = load i8, ptr %3, align 1, !tbaa !5
  %14 = icmp eq i8 %12, %13
  br i1 %14, label %15, label %39

15:                                               ; preds = %11, %19
  %16 = phi i32 [ %17, %19 ], [ 0, %11 ]
  %17 = add nuw i32 %16, 1
  %18 = icmp eq i32 %17, %4
  br i1 %18, label %25, label %19, !llvm.loop !31

19:                                               ; preds = %15
  %20 = getelementptr i8, ptr %9, i32 %17
  %21 = load i8, ptr %20, align 1, !tbaa !5
  %22 = getelementptr inbounds i8, ptr %3, i32 %17
  %23 = load i8, ptr %22, align 1, !tbaa !5
  %24 = icmp eq i8 %21, %23
  br i1 %24, label %15, label %25, !llvm.loop !31

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
  %33 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %32) #42
  %34 = icmp ult i32 %33, %8
  br i1 %34, label %31, label %35, !llvm.loop !41

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
define hidden i64 @fa_string_index_of(ptr noundef %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3, i32 noundef %4, i32 noundef %5) local_unnamed_addr #12 {
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
  %17 = load i8, ptr %3, align 1, !tbaa !5
  br label %18

18:                                               ; preds = %16, %25
  %19 = phi i32 [ 0, %16 ], [ %26, %25 ]
  %20 = getelementptr inbounds i8, ptr %0, i32 %19
  %21 = load i8, ptr %20, align 1, !tbaa !5
  %22 = icmp eq i8 %21, %17
  br i1 %22, label %23, label %25

23:                                               ; preds = %18
  %24 = zext nneg i32 %19 to i64
  br label %30

25:                                               ; preds = %18
  %26 = add nuw i32 %19, 1
  %27 = icmp eq i32 %26, %1
  br i1 %27, label %30, label %18, !llvm.loop !42

28:                                               ; preds = %6
  %29 = tail call fastcc i64 @string_index_of(ptr noundef %0, i32 noundef %1, i32 noundef %2, ptr noundef %3, i32 noundef %4, i32 noundef %5) #42
  br label %30

30:                                               ; preds = %25, %14, %23, %28
  %31 = phi i64 [ %29, %28 ], [ %24, %23 ], [ -1, %14 ], [ -1, %25 ]
  ret i64 %31
}

; Function Attrs: nofree noinline norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc i64 @string_index_of(ptr noundef %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3, i32 noundef %4, i32 noundef %5) unnamed_addr #13 {
  %7 = and i32 %2, -2147483648
  %8 = tail call fastcc i64 @find(ptr noundef %0, i32 noundef %1, i32 noundef %7, ptr noundef %3, i32 noundef %4, i32 noundef %5) #42
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
  %19 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %16) #42
  %20 = icmp ult i32 %19, %13
  br i1 %20, label %15, label %21, !llvm.loop !43

21:                                               ; preds = %15
  %22 = zext i32 %18 to i64
  br label %23

23:                                               ; preds = %12, %21, %6
  %24 = phi i64 [ %8, %6 ], [ 0, %12 ], [ %22, %21 ]
  ret i64 %24
}

; Function Attrs: nounwind optsize
define hidden void @fa_string_replace(ptr noundef %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3, i32 noundef %4, ptr noundef %5, i32 noundef %6, i32 noundef %7, ptr nocapture noundef writeonly %8) local_unnamed_addr #1 {
  %10 = alloca %struct.FaString, align 4
  %11 = alloca %struct.Replacing, align 4
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  call void @llvm.lifetime.start.p0(i64 28, ptr nonnull %11) #44
  store ptr %0, ptr %11, align 4, !tbaa !44
  %12 = getelementptr inbounds %struct.Replacing, ptr %11, i32 0, i32 1
  store i32 0, ptr %12, align 4, !tbaa !46
  %13 = getelementptr inbounds %struct.Replacing, ptr %11, i32 0, i32 2
  store i32 %4, ptr %13, align 4, !tbaa !47
  %14 = getelementptr inbounds %struct.Replacing, ptr %11, i32 0, i32 3
  store ptr %5, ptr %14, align 4, !tbaa !48
  %15 = getelementptr inbounds %struct.Replacing, ptr %11, i32 0, i32 4
  store i32 %6, ptr %15, align 4, !tbaa !49
  %16 = getelementptr inbounds %struct.Replacing, ptr %11, i32 0, i32 5
  store i32 %7, ptr %16, align 4, !tbaa !50
  %17 = getelementptr inbounds %struct.Replacing, ptr %11, i32 0, i32 6
  store i32 0, ptr %17, align 4, !tbaa !51
  %18 = and i32 %2, -2147483648
  call fastcc void @each_match(ptr noundef %0, i32 noundef %1, i32 noundef %18, ptr noundef %3, i32 noundef %4, ptr noundef nonnull @replace_one, ptr noundef nonnull %11) #42
  %19 = load i32, ptr %12, align 4, !tbaa !46
  %20 = getelementptr inbounds i8, ptr %0, i32 %19
  %21 = sub i32 %1, %19
  call fastcc void @reserve(i32 noundef %21) #42
  %22 = load ptr, ptr @builder, align 4, !tbaa !6
  %23 = load i32, ptr @builder_length, align 4, !tbaa !1
  %24 = getelementptr inbounds i8, ptr %22, i32 %23
  call fastcc void @copy_bytes(ptr noundef %24, ptr noundef %20, i32 noundef %21) #42
  %25 = add i32 %23, %21
  store i32 %25, ptr @builder_length, align 4, !tbaa !1
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %10) #44
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %10, ptr noundef %22, i32 noundef %25, i32 noundef 0) #42
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %8, ptr noundef nonnull align 4 dereferenceable(12) %10, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %10) #44
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  call void @llvm.lifetime.end.p0(i64 28, ptr nonnull %11) #44
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @each_match(ptr noundef %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3, i32 noundef %4, ptr nocapture noundef readonly %5, ptr noundef %6) unnamed_addr #1 {
  %8 = alloca %struct.Boundaries, align 4
  %9 = alloca %struct.Boundaries, align 4
  %10 = icmp ne i32 %2, 0
  %11 = icmp ne i32 %4, 0
  %12 = and i1 %10, %11
  br i1 %12, label %13, label %47

13:                                               ; preds = %7
  %14 = load i8, ptr %3, align 1, !tbaa !5
  %15 = tail call fastcc i32 @next_byte(ptr noundef %0, i32 noundef %1, i32 noundef 0, i8 noundef zeroext %14) #42
  %16 = add i32 %15, %4
  %17 = icmp ugt i32 %16, %1
  br i1 %17, label %91, label %18

18:                                               ; preds = %13, %41
  %19 = phi i8 [ %42, %41 ], [ %14, %13 ]
  %20 = phi i32 [ %45, %41 ], [ %16, %13 ]
  %21 = phi i32 [ %44, %41 ], [ %15, %13 ]
  %22 = getelementptr i8, ptr %0, i32 %21
  %23 = load i8, ptr %22, align 1, !tbaa !5
  %24 = icmp eq i8 %23, %19
  br i1 %24, label %25, label %39

25:                                               ; preds = %18, %29
  %26 = phi i32 [ %27, %29 ], [ 0, %18 ]
  %27 = add nuw i32 %26, 1
  %28 = icmp eq i32 %27, %4
  br i1 %28, label %35, label %29, !llvm.loop !31

29:                                               ; preds = %25
  %30 = getelementptr i8, ptr %22, i32 %27
  %31 = load i8, ptr %30, align 1, !tbaa !5
  %32 = getelementptr inbounds i8, ptr %3, i32 %27
  %33 = load i8, ptr %32, align 1, !tbaa !5
  %34 = icmp eq i8 %31, %33
  br i1 %34, label %25, label %35, !llvm.loop !31

35:                                               ; preds = %29, %25
  %36 = icmp ult i32 %27, %4
  br i1 %36, label %39, label %37

37:                                               ; preds = %35
  tail call void %5(ptr noundef %6, i32 noundef %21) #41, !callees !52
  %38 = load i8, ptr %3, align 1, !tbaa !5
  br label %41

39:                                               ; preds = %18, %35
  %40 = add i32 %21, 1
  br label %41

41:                                               ; preds = %39, %37
  %42 = phi i8 [ %38, %37 ], [ %19, %39 ]
  %43 = phi i32 [ %20, %37 ], [ %40, %39 ]
  %44 = tail call fastcc i32 @next_byte(ptr noundef nonnull %0, i32 noundef %1, i32 noundef %43, i8 noundef zeroext %42) #42
  %45 = add i32 %44, %4
  %46 = icmp ugt i32 %45, %1
  br i1 %46, label %91, label %18, !llvm.loop !53

47:                                               ; preds = %7
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %8) #44
  store ptr %0, ptr %8, align 4, !tbaa !34
  %48 = getelementptr inbounds %struct.Boundaries, ptr %8, i32 0, i32 1
  store i32 %1, ptr %48, align 4, !tbaa !36
  %49 = getelementptr inbounds %struct.Boundaries, ptr %8, i32 0, i32 2
  store i32 0, ptr %49, align 4, !tbaa !37
  %50 = getelementptr inbounds %struct.Boundaries, ptr %8, i32 0, i32 3
  store i32 %2, ptr %50, align 4, !tbaa !38
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %9) #44
  store ptr %0, ptr %9, align 4, !tbaa !34
  %51 = getelementptr inbounds %struct.Boundaries, ptr %9, i32 0, i32 1
  store i32 %1, ptr %51, align 4, !tbaa !36
  %52 = getelementptr inbounds %struct.Boundaries, ptr %9, i32 0, i32 2
  store i32 0, ptr %52, align 4, !tbaa !37
  %53 = getelementptr inbounds %struct.Boundaries, ptr %9, i32 0, i32 3
  store i32 %2, ptr %53, align 4, !tbaa !38
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
  %63 = load i8, ptr %61, align 1, !tbaa !5
  %64 = load i8, ptr %3, align 1, !tbaa !5
  %65 = icmp eq i8 %63, %64
  br i1 %65, label %66, label %85

66:                                               ; preds = %62, %70
  %67 = phi i32 [ %68, %70 ], [ 0, %62 ]
  %68 = add nuw i32 %67, 1
  %69 = icmp eq i32 %68, %4
  br i1 %69, label %76, label %70, !llvm.loop !31

70:                                               ; preds = %66
  %71 = getelementptr i8, ptr %61, i32 %68
  %72 = load i8, ptr %71, align 1, !tbaa !5
  %73 = getelementptr inbounds i8, ptr %3, i32 %68
  %74 = load i8, ptr %73, align 1, !tbaa !5
  %75 = icmp eq i8 %72, %74
  br i1 %75, label %66, label %76, !llvm.loop !31

76:                                               ; preds = %70, %66
  %77 = icmp ult i32 %68, %4
  br i1 %77, label %85, label %78

78:                                               ; preds = %58, %76
  %79 = call fastcc i32 @at_boundary(ptr noundef nonnull %8, i32 noundef %60) #42, !range !39
  %80 = icmp eq i32 %79, 0
  br i1 %80, label %85, label %81

81:                                               ; preds = %78
  %82 = call fastcc i32 @at_boundary(ptr noundef nonnull %9, i32 noundef %59) #42, !range !39
  %83 = icmp eq i32 %82, 0
  br i1 %83, label %85, label %84

84:                                               ; preds = %81
  tail call void %5(ptr noundef %6, i32 noundef %60) #41, !callees !52
  br label %85

85:                                               ; preds = %62, %76, %78, %81, %84
  %86 = phi i32 [ %57, %84 ], [ 1, %81 ], [ 1, %78 ], [ 1, %76 ], [ 1, %62 ]
  %87 = add i32 %86, %60
  %88 = add i32 %87, %4
  %89 = icmp ugt i32 %88, %1
  br i1 %89, label %90, label %58, !llvm.loop !54

90:                                               ; preds = %85, %47
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %9) #44
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %8) #44
  br label %91

91:                                               ; preds = %41, %13, %90
  ret void
}

; Function Attrs: nounwind optsize
define internal void @replace_one(ptr nocapture noundef %0, i32 noundef %1) #1 {
  %3 = getelementptr inbounds %struct.Replacing, ptr %0, i32 0, i32 6
  %4 = load i32, ptr %3, align 4, !tbaa !51
  %5 = icmp eq i32 %4, 0
  br i1 %5, label %6, label %31

6:                                                ; preds = %2
  %7 = load ptr, ptr %0, align 4, !tbaa !44
  %8 = getelementptr inbounds %struct.Replacing, ptr %0, i32 0, i32 1
  %9 = load i32, ptr %8, align 4, !tbaa !46
  %10 = getelementptr inbounds i8, ptr %7, i32 %9
  %11 = sub i32 %1, %9
  tail call fastcc void @reserve(i32 noundef %11) #42
  %12 = load ptr, ptr @builder, align 4, !tbaa !6
  %13 = load i32, ptr @builder_length, align 4, !tbaa !1
  %14 = getelementptr inbounds i8, ptr %12, i32 %13
  tail call fastcc void @copy_bytes(ptr noundef %14, ptr noundef %10, i32 noundef %11) #42
  %15 = add i32 %13, %11
  store i32 %15, ptr @builder_length, align 4, !tbaa !1
  %16 = getelementptr inbounds %struct.Replacing, ptr %0, i32 0, i32 3
  %17 = load ptr, ptr %16, align 4, !tbaa !48
  %18 = getelementptr inbounds %struct.Replacing, ptr %0, i32 0, i32 4
  %19 = load i32, ptr %18, align 4, !tbaa !49
  tail call fastcc void @reserve(i32 noundef %19) #42
  %20 = load ptr, ptr @builder, align 4, !tbaa !6
  %21 = load i32, ptr @builder_length, align 4, !tbaa !1
  %22 = getelementptr inbounds i8, ptr %20, i32 %21
  tail call fastcc void @copy_bytes(ptr noundef %22, ptr noundef %17, i32 noundef %19) #42
  %23 = add i32 %21, %19
  store i32 %23, ptr @builder_length, align 4, !tbaa !1
  %24 = getelementptr inbounds %struct.Replacing, ptr %0, i32 0, i32 2
  %25 = load i32, ptr %24, align 4, !tbaa !47
  %26 = add i32 %25, %1
  store i32 %26, ptr %8, align 4, !tbaa !46
  %27 = getelementptr inbounds %struct.Replacing, ptr %0, i32 0, i32 5
  %28 = load i32, ptr %27, align 4, !tbaa !50
  %29 = icmp eq i32 %28, 0
  %30 = zext i1 %29 to i32
  store i32 %30, ptr %3, align 4, !tbaa !51
  br label %31

31:                                               ; preds = %2, %6
  ret void
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_string_characters(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2) local_unnamed_addr #1 {
  %4 = tail call fastcc ptr @characters_of(ptr noundef %0, i32 noundef %1, i32 noundef %2) #42
  ret ptr %4
}

; Function Attrs: nounwind optsize
define internal fastcc ptr @characters_of(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2) unnamed_addr #1 {
  %4 = alloca %struct.FaString, align 4
  %5 = icmp sgt i32 %2, -1
  %6 = icmp eq i32 %1, 0
  br i1 %5, label %24, label %7

7:                                                ; preds = %3
  %8 = zext i32 %1 to i64
  %9 = tail call ptr @fa_array_new(i64 noundef %8, i64 noundef %8, i64 noundef 12) #42
  %10 = load ptr, ptr %9, align 8, !tbaa !55
  br i1 %6, label %35, label %11

11:                                               ; preds = %7, %11
  %12 = phi i32 [ %22, %11 ], [ 0, %7 ]
  %13 = getelementptr inbounds %struct.FaString, ptr %10, i32 %12
  %14 = getelementptr inbounds i8, ptr %0, i32 %12
  %15 = load i8, ptr %14, align 1, !tbaa !5
  %16 = zext i8 %15 to i32
  %17 = getelementptr inbounds i8, ptr @ascii_table, i32 %16
  %18 = icmp eq i8 %15, 13
  %19 = select i1 %18, i32 0, i32 -2147483648
  store ptr %17, ptr %13, align 4, !tbaa.struct !10
  %20 = getelementptr inbounds i8, ptr %13, i32 4
  store i32 1, ptr %20, align 4, !tbaa.struct !58
  %21 = getelementptr inbounds i8, ptr %13, i32 8
  store i32 %19, ptr %21, align 4, !tbaa.struct !59
  %22 = add nuw i32 %12, 1
  %23 = icmp eq i32 %22, %1
  br i1 %23, label %35, label %11, !llvm.loop !60

24:                                               ; preds = %3
  %25 = tail call i32 @fa_graphemes(ptr noundef %0, i32 noundef %1) #42
  %26 = zext i32 %25 to i64
  %27 = tail call ptr @fa_array_new(i64 noundef 0, i64 noundef %26, i64 noundef 12) #42
  br i1 %6, label %35, label %28

28:                                               ; preds = %24, %28
  %29 = phi i32 [ %30, %28 ], [ 0, %24 ]
  %30 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %29) #42
  %31 = tail call ptr @fa_array_push_slot(ptr noundef %27, i64 noundef 12) #42
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %4) #44
  %32 = getelementptr inbounds i8, ptr %0, i32 %29
  %33 = sub i32 %30, %29
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %4, ptr noundef %32, i32 noundef %33, i32 noundef 0) #42
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %31, ptr noundef nonnull align 4 dereferenceable(12) %4, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %4) #44
  %34 = icmp ult i32 %30, %1
  br i1 %34, label %28, label %35, !llvm.loop !61

35:                                               ; preds = %11, %28, %7, %24
  %36 = phi ptr [ %27, %24 ], [ %9, %7 ], [ %27, %28 ], [ %9, %11 ]
  ret ptr %36
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_string_split(ptr noundef %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3, i32 noundef %4) local_unnamed_addr #1 {
  %6 = alloca %struct.Splitting, align 4
  %7 = alloca %struct.FaString, align 4
  %8 = icmp eq i32 %4, 0
  br i1 %8, label %9, label %11

9:                                                ; preds = %5
  %10 = tail call fastcc ptr @characters_of(ptr noundef %0, i32 noundef %1, i32 noundef %2) #42
  br label %28

11:                                               ; preds = %5
  call void @llvm.lifetime.start.p0(i64 20, ptr nonnull %6) #44
  store ptr %0, ptr %6, align 4, !tbaa !62
  %12 = getelementptr inbounds %struct.Splitting, ptr %6, i32 0, i32 1
  store i32 0, ptr %12, align 4, !tbaa !64
  %13 = getelementptr inbounds %struct.Splitting, ptr %6, i32 0, i32 2
  store i32 %4, ptr %13, align 4, !tbaa !65
  %14 = getelementptr inbounds %struct.Splitting, ptr %6, i32 0, i32 3
  %15 = tail call ptr @fa_alloc(i32 noundef 80, i32 noundef 16) #41
  %16 = getelementptr inbounds i8, ptr %15, i32 24
  store i64 1, ptr %16, align 1
  %17 = getelementptr inbounds i8, ptr %15, i32 32
  store ptr %17, ptr %15, align 8, !tbaa !55
  %18 = getelementptr inbounds %struct.FaArrayHeader, ptr %15, i32 0, i32 1
  store i64 0, ptr %18, align 8, !tbaa !66
  %19 = getelementptr inbounds %struct.FaArrayHeader, ptr %15, i32 0, i32 2
  store i64 4, ptr %19, align 8, !tbaa !67
  store ptr %15, ptr %14, align 4, !tbaa !68
  %20 = getelementptr inbounds %struct.Splitting, ptr %6, i32 0, i32 4
  %21 = and i32 %2, -2147483648
  store i32 %21, ptr %20, align 4, !tbaa !69
  call fastcc void @each_match(ptr noundef %0, i32 noundef %1, i32 noundef %21, ptr noundef %3, i32 noundef %4, ptr noundef nonnull @split_one, ptr noundef nonnull %6) #42
  %22 = load ptr, ptr %14, align 4, !tbaa !68
  %23 = call ptr @fa_array_push_slot(ptr noundef %22, i64 noundef 12) #42
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %7) #44
  %24 = load i32, ptr %12, align 4, !tbaa !64
  %25 = getelementptr inbounds i8, ptr %0, i32 %24
  %26 = sub i32 %1, %24
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %7, ptr noundef %25, i32 noundef %26, i32 noundef %21) #42
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %23, ptr noundef nonnull align 4 dereferenceable(12) %7, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %7) #44
  %27 = load ptr, ptr %14, align 4, !tbaa !68
  call void @llvm.lifetime.end.p0(i64 20, ptr nonnull %6) #44
  br label %28

28:                                               ; preds = %11, %9
  %29 = phi ptr [ %10, %9 ], [ %27, %11 ]
  ret ptr %29
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_array_new(i64 noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #1 {
  %4 = tail call i64 @llvm.smax.i64(i64 %1, i64 %0)
  %5 = mul nsw i64 %4, %2
  %6 = trunc i64 %5 to i32
  %7 = icmp ult i32 %6, 97
  br i1 %7, label %8, label %13

8:                                                ; preds = %3
  %9 = add nuw nsw i32 %6, 32
  %10 = tail call ptr @fa_alloc(i32 noundef %9, i32 noundef 16) #41
  %11 = getelementptr inbounds i8, ptr %10, i32 24
  store i64 1, ptr %11, align 1
  %12 = getelementptr inbounds i8, ptr %10, i32 32
  store ptr %12, ptr %10, align 8, !tbaa !55
  br label %17

13:                                               ; preds = %3
  %14 = tail call ptr @fa_alloc(i32 noundef 32, i32 noundef 8) #41
  %15 = getelementptr inbounds i8, ptr %14, i32 24
  store i64 0, ptr %15, align 1
  %16 = tail call ptr @fa_alloc(i32 noundef %6, i32 noundef 16) #41
  store ptr %16, ptr %14, align 8, !tbaa !55
  br label %17

17:                                               ; preds = %13, %8
  %18 = phi ptr [ %14, %13 ], [ %10, %8 ]
  %19 = getelementptr inbounds %struct.FaArrayHeader, ptr %18, i32 0, i32 1
  store i64 %0, ptr %19, align 8, !tbaa !66
  %20 = getelementptr inbounds %struct.FaArrayHeader, ptr %18, i32 0, i32 2
  store i64 %4, ptr %20, align 8, !tbaa !67
  ret ptr %18
}

; Function Attrs: nounwind optsize
define internal void @split_one(ptr nocapture noundef %0, i32 noundef %1) #1 {
  %3 = alloca %struct.FaString, align 4
  %4 = getelementptr inbounds %struct.Splitting, ptr %0, i32 0, i32 3
  %5 = load ptr, ptr %4, align 4, !tbaa !68
  %6 = tail call ptr @fa_array_push_slot(ptr noundef %5, i64 noundef 12) #42
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %3) #44
  %7 = load ptr, ptr %0, align 4, !tbaa !62
  %8 = getelementptr inbounds %struct.Splitting, ptr %0, i32 0, i32 1
  %9 = load i32, ptr %8, align 4, !tbaa !64
  %10 = getelementptr inbounds i8, ptr %7, i32 %9
  %11 = sub i32 %1, %9
  %12 = getelementptr inbounds %struct.Splitting, ptr %0, i32 0, i32 4
  %13 = load i32, ptr %12, align 4, !tbaa !69
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %3, ptr noundef %10, i32 noundef %11, i32 noundef %13) #42
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %6, ptr noundef nonnull align 4 dereferenceable(12) %3, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %3) #44
  %14 = getelementptr inbounds %struct.Splitting, ptr %0, i32 0, i32 2
  %15 = load i32, ptr %14, align 4, !tbaa !65
  %16 = add i32 %15, %1
  store i32 %16, ptr %8, align 4, !tbaa !64
  ret void
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_array_push_slot(ptr noundef %0, i64 noundef %1) local_unnamed_addr #1 {
  %3 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %4 = load i64, ptr %3, align 8, !tbaa !66
  %5 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %6 = load i64, ptr %5, align 8, !tbaa !67
  %7 = icmp eq i64 %4, %6
  br i1 %7, label %8, label %10, !prof !70

8:                                                ; preds = %2
  tail call fastcc void @array_grow(ptr noundef nonnull %0, i64 noundef %1) #42
  %9 = load i64, ptr %3, align 8, !tbaa !66
  br label %10

10:                                               ; preds = %8, %2
  %11 = phi i64 [ %9, %8 ], [ %4, %2 ]
  %12 = load ptr, ptr %0, align 8, !tbaa !55
  %13 = mul nsw i64 %11, %1
  %14 = trunc i64 %13 to i32
  %15 = getelementptr inbounds i8, ptr %12, i32 %14
  %16 = add nsw i64 %11, 1
  store i64 %16, ptr %3, align 8, !tbaa !66
  ret ptr %15
}

; Function Attrs: nounwind optsize
define internal fastcc void @owned_copy_with(ptr dead_on_unwind noalias nocapture writable writeonly sret(%struct.FaString) align 4 %0, ptr nocapture noundef readonly %1, i32 noundef %2, i32 noundef %3) unnamed_addr #1 {
  switch i32 %2, label %18 [
    i32 0, label %5
    i32 1, label %8
  ]

5:                                                ; preds = %4
  store ptr @.str.33, ptr %0, align 4, !tbaa.struct !10
  %6 = getelementptr inbounds i8, ptr %0, i32 4
  store i32 0, ptr %6, align 4, !tbaa.struct !58
  %7 = getelementptr inbounds i8, ptr %0, i32 8
  store i32 -2147483648, ptr %7, align 4, !tbaa.struct !59
  br label %28

8:                                                ; preds = %4
  %9 = load i8, ptr %1, align 1, !tbaa !5
  %10 = icmp sgt i8 %9, -1
  br i1 %10, label %11, label %18

11:                                               ; preds = %8
  %12 = zext nneg i8 %9 to i32
  %13 = getelementptr inbounds i8, ptr @ascii_table, i32 %12
  store ptr %13, ptr %0, align 4, !tbaa !18, !alias.scope !71
  %14 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 1
  store i32 1, ptr %14, align 4, !tbaa !20, !alias.scope !71
  %15 = icmp eq i8 %9, 13
  %16 = select i1 %15, i32 0, i32 -2147483648
  %17 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 2
  store i32 %16, ptr %17, align 4, !tbaa !23, !alias.scope !71
  br label %28

18:                                               ; preds = %4, %8
  %19 = tail call ptr @fa_alloc(i32 noundef %2, i32 noundef 1) #41
  tail call fastcc void @copy_bytes(ptr noundef %19, ptr noundef %1, i32 noundef %2) #42
  %20 = icmp eq i32 %3, 0
  br i1 %20, label %21, label %23

21:                                               ; preds = %18
  %22 = tail call fastcc i32 @simple_flag(ptr noundef %19, i32 noundef %2) #42, !range !22
  br label %23

23:                                               ; preds = %18, %21
  %24 = phi i32 [ %22, %21 ], [ %3, %18 ]
  %25 = or i32 %24, %2
  store ptr %19, ptr %0, align 4, !tbaa.struct !10
  %26 = getelementptr inbounds i8, ptr %0, i32 4
  store i32 %2, ptr %26, align 4, !tbaa.struct !58
  %27 = getelementptr inbounds i8, ptr %0, i32 8
  store i32 %25, ptr %27, align 4, !tbaa.struct !59
  br label %28

28:                                               ; preds = %23, %11, %5
  ret void
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_string_code_points(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2) local_unnamed_addr #1 {
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
  %12 = load i8, ptr %11, align 1, !tbaa !5
  %13 = and i8 %12, -64
  %14 = icmp ne i8 %13, -128
  %15 = zext i1 %14 to i32
  %16 = add i32 %9, %15
  %17 = add nuw i32 %10, 1
  %18 = icmp eq i32 %17, %1
  br i1 %18, label %19, label %8, !llvm.loop !74

19:                                               ; preds = %8, %6, %3
  %20 = phi i32 [ %1, %3 ], [ 0, %6 ], [ %16, %8 ]
  %21 = zext i32 %20 to i64
  %22 = tail call ptr @fa_array_new(i64 noundef %21, i64 noundef %21, i64 noundef 4) #42
  %23 = load ptr, ptr %22, align 8, !tbaa !55
  %24 = icmp eq i32 %20, %1
  br i1 %24, label %25, label %35

25:                                               ; preds = %19
  %26 = icmp eq i32 %1, 0
  br i1 %26, label %45, label %27

27:                                               ; preds = %25, %27
  %28 = phi i32 [ %33, %27 ], [ 0, %25 ]
  %29 = getelementptr inbounds i8, ptr %0, i32 %28
  %30 = load i8, ptr %29, align 1, !tbaa !5
  %31 = zext i8 %30 to i32
  %32 = getelementptr inbounds i32, ptr %23, i32 %28
  store i32 %31, ptr %32, align 4, !tbaa !12
  %33 = add nuw i32 %28, 1
  %34 = icmp eq i32 %33, %1
  br i1 %34, label %45, label %27, !llvm.loop !75

35:                                               ; preds = %19
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %4) #44
  store i32 0, ptr %4, align 4, !tbaa !1
  %36 = icmp eq i32 %1, 0
  br i1 %36, label %37, label %38

37:                                               ; preds = %38, %35
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %4) #44
  br label %45

38:                                               ; preds = %35, %38
  %39 = phi i32 [ %42, %38 ], [ 0, %35 ]
  %40 = call fastcc i32 @decode(ptr noundef %0, i32 noundef %1, ptr noundef nonnull %4) #42
  %41 = getelementptr inbounds i32, ptr %23, i32 %39
  store i32 %40, ptr %41, align 4, !tbaa !12
  %42 = add i32 %39, 1
  %43 = load i32, ptr %4, align 4, !tbaa !1
  %44 = icmp ult i32 %43, %1
  br i1 %44, label %38, label %37, !llvm.loop !76

45:                                               ; preds = %27, %25, %37
  ret ptr %22
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define internal fastcc i32 @decode(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef %2) unnamed_addr #11 {
  %4 = load i32, ptr %2, align 4, !tbaa !1
  %5 = getelementptr inbounds i8, ptr %0, i32 %4
  %6 = load i8, ptr %5, align 1, !tbaa !5
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
  store i32 %24, ptr %2, align 4, !tbaa !1
  ret i32 %23

25:                                               ; preds = %16
  %26 = shl i32 %18, 6
  %27 = getelementptr inbounds i8, ptr %0, i32 %19
  %28 = load i8, ptr %27, align 1, !tbaa !5
  %29 = and i8 %28, 63
  %30 = zext nneg i8 %29 to i32
  %31 = or disjoint i32 %26, %30
  %32 = add nuw nsw i32 %17, 1
  %33 = icmp eq i32 %32, %13
  br i1 %33, label %21, label %16, !llvm.loop !77
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define hidden i64 @fa_string_code_point_at(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2, i64 noundef %3) local_unnamed_addr #11 {
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
  %16 = load i8, ptr %15, align 1, !tbaa !5
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
  %25 = load i8, ptr %24, align 1, !tbaa !5
  %26 = and i8 %25, -64
  %27 = icmp ne i8 %26, -128
  %28 = zext i1 %27 to i64
  %29 = add nuw nsw i64 %23, %28
  %30 = add nuw i32 %22, 1
  %31 = icmp eq i32 %30, %1
  br i1 %31, label %32, label %21, !llvm.loop !78

32:                                               ; preds = %21, %19
  %33 = phi i64 [ 0, %19 ], [ %29, %21 ]
  %34 = add nsw i64 %33, %3
  %35 = icmp slt i64 %34, 0
  br i1 %35, label %51, label %36

36:                                               ; preds = %32, %18
  %37 = phi i64 [ %34, %32 ], [ %3, %18 ]
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %5) #44
  store i32 0, ptr %5, align 4, !tbaa !1
  %38 = icmp eq i32 %1, 0
  br i1 %38, label %49, label %43

39:                                               ; preds = %43
  %40 = add nsw i64 %44, -1
  %41 = load i32, ptr %5, align 4, !tbaa !1
  %42 = icmp ult i32 %41, %1
  br i1 %42, label %43, label %49, !llvm.loop !79

43:                                               ; preds = %36, %39
  %44 = phi i64 [ %40, %39 ], [ %37, %36 ]
  %45 = call fastcc i32 @decode(ptr noundef %0, i32 noundef %1, ptr noundef nonnull %5) #42
  %46 = icmp eq i64 %44, 0
  br i1 %46, label %47, label %39

47:                                               ; preds = %43
  %48 = zext i32 %45 to i64
  br label %49

49:                                               ; preds = %39, %36, %47
  %50 = phi i64 [ %48, %47 ], [ -1, %36 ], [ -1, %39 ]
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %5) #44
  br label %51

51:                                               ; preds = %49, %32, %13, %8
  %52 = phi i64 [ %17, %13 ], [ -1, %8 ], [ -1, %32 ], [ %50, %49 ]
  ret i64 %52
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: read)
define hidden i64 @fa_string_code_point_count(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2) local_unnamed_addr #9 {
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
  %13 = load i8, ptr %12, align 1, !tbaa !5
  %14 = and i8 %13, -64
  %15 = icmp ne i8 %14, -128
  %16 = zext i1 %15 to i64
  %17 = add nuw nsw i64 %11, %16
  %18 = add nuw i32 %10, 1
  %19 = icmp eq i32 %18, %1
  br i1 %19, label %20, label %9, !llvm.loop !78

20:                                               ; preds = %9, %5, %7
  %21 = phi i64 [ %8, %7 ], [ 0, %5 ], [ %17, %9 ]
  ret i64 %21
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define hidden i32 @fa_next_code_point(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef %2) local_unnamed_addr #11 {
  %4 = alloca i32, align 4
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %4) #44
  %5 = load i64, ptr %2, align 8, !tbaa !80
  %6 = trunc i64 %5 to i32
  store i32 %6, ptr %4, align 4, !tbaa !1
  %7 = getelementptr inbounds i8, ptr %0, i32 %6
  %8 = load i8, ptr %7, align 1, !tbaa !5
  %9 = icmp sgt i8 %8, -1
  br i1 %9, label %10, label %14

10:                                               ; preds = %3
  %11 = zext nneg i8 %8 to i32
  %12 = and i64 %5, 4294967295
  %13 = add nuw nsw i64 %12, 1
  br label %18

14:                                               ; preds = %3
  %15 = call fastcc i32 @decode(ptr noundef nonnull %0, i32 noundef %1, ptr noundef nonnull %4) #42
  %16 = load i32, ptr %4, align 4, !tbaa !1
  %17 = zext i32 %16 to i64
  br label %18

18:                                               ; preds = %14, %10
  %19 = phi i64 [ %17, %14 ], [ %13, %10 ]
  %20 = phi i32 [ %15, %14 ], [ %11, %10 ]
  store i64 %19, ptr %2, align 8, !tbaa !80
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %4) #44
  ret i32 %20
}

; Function Attrs: nounwind optsize
define hidden noundef i32 @fa_string_get(ptr noundef %0, i32 noundef %1, i32 noundef %2, i64 noundef %3, ptr nocapture noundef writeonly %4) local_unnamed_addr #1 {
  %6 = icmp sgt i32 %2, -1
  br i1 %6, label %23, label %7

7:                                                ; preds = %5
  %8 = icmp slt i64 %3, 0
  %9 = zext i32 %1 to i64
  %10 = select i1 %8, i64 %9, i64 0
  %11 = add nsw i64 %10, %3
  %12 = icmp ult i64 %11, %9
  br i1 %12, label %13, label %25

13:                                               ; preds = %7
  %14 = trunc i64 %11 to i32
  %15 = getelementptr inbounds i8, ptr %0, i32 %14
  %16 = load i8, ptr %15, align 1, !tbaa !5
  %17 = zext i8 %16 to i32
  %18 = getelementptr inbounds i8, ptr @ascii_table, i32 %17
  %19 = icmp eq i8 %16, 13
  %20 = select i1 %19, i32 0, i32 -2147483648
  store ptr %18, ptr %4, align 4, !tbaa.struct !10
  %21 = getelementptr inbounds i8, ptr %4, i32 4
  store i32 1, ptr %21, align 4, !tbaa.struct !58
  %22 = getelementptr inbounds i8, ptr %4, i32 8
  store i32 %20, ptr %22, align 4, !tbaa.struct !59
  br label %25

23:                                               ; preds = %5
  %24 = tail call fastcc i32 @string_get_slow(ptr noundef %0, i32 noundef %1, i32 noundef %2, i64 noundef %3, ptr noundef %4) #42
  br label %25

25:                                               ; preds = %7, %23, %13
  %26 = phi i32 [ 1, %13 ], [ %24, %23 ], [ 0, %7 ]
  ret i32 %26
}

; Function Attrs: noinline nounwind optsize
define internal fastcc noundef i32 @string_get_slow(ptr noundef %0, i32 noundef %1, i32 noundef %2, i64 noundef %3, ptr nocapture noundef writeonly %4) unnamed_addr #10 {
  %6 = alloca %struct.FaString, align 4
  %7 = icmp slt i64 %3, 0
  br i1 %7, label %8, label %13

8:                                                ; preds = %5
  %9 = tail call i32 @fa_graphemes(ptr noundef %0, i32 noundef %1) #42
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
  br i1 %18, label %19, label %36, !llvm.loop !81

19:                                               ; preds = %13, %16
  %20 = phi i64 [ %17, %16 ], [ 0, %13 ]
  %21 = phi i32 [ %22, %16 ], [ 0, %13 ]
  %22 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %21) #42
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
  store ptr %31, ptr %4, align 4, !tbaa !18
  %32 = getelementptr inbounds %struct.FaString, ptr %4, i32 0, i32 1
  store i32 %27, ptr %32, align 4, !tbaa !20
  %33 = getelementptr inbounds %struct.FaString, ptr %4, i32 0, i32 2
  store i32 0, ptr %33, align 4, !tbaa !23
  br label %36

34:                                               ; preds = %24
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %6) #44
  %35 = getelementptr inbounds i8, ptr %0, i32 %21
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %6, ptr noundef %35, i32 noundef %27, i32 noundef 0) #42
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %4, ptr noundef nonnull align 4 dereferenceable(12) %6, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %6) #44
  br label %36

36:                                               ; preds = %16, %30, %34, %13, %8
  %37 = phi i32 [ 0, %8 ], [ 1, %34 ], [ 1, %30 ], [ 0, %13 ], [ 0, %16 ]
  ret i32 %37
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define hidden i32 @fa_string_length(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2) local_unnamed_addr #11 {
  %4 = icmp sgt i32 %2, -1
  br i1 %4, label %5, label %7

5:                                                ; preds = %3
  %6 = tail call i32 @fa_graphemes(ptr noundef %0, i32 noundef %1) #42
  br label %7

7:                                                ; preds = %3, %5
  %8 = phi i32 [ %6, %5 ], [ %1, %3 ]
  ret i32 %8
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_string_slice(ptr noundef %0, i32 noundef %1, i32 noundef %2, i64 noundef %3, i64 noundef %4, i8 noundef signext %5, ptr nocapture noundef writeonly %6) local_unnamed_addr #1 {
  %8 = alloca %struct.FaString, align 4
  %9 = and i32 %2, -2147483648
  %10 = icmp eq i32 %9, 0
  br i1 %10, label %11, label %13

11:                                               ; preds = %7
  %12 = tail call i32 @fa_graphemes(ptr noundef %0, i32 noundef %1) #42
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
  %25 = tail call ptr @fa_error_index(i64 noundef %3, i64 noundef %15) #42
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
  %52 = tail call ptr @fa_error_index(i64 noundef %4, i64 noundef %15) #42
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
  %66 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %65) #42
  %67 = add nuw i32 %64, 1
  %68 = icmp eq i32 %67, %56
  br i1 %68, label %60, label %63, !llvm.loop !82

69:                                               ; preds = %60, %69
  %70 = phi i32 [ %73, %69 ], [ %56, %60 ]
  %71 = phi i32 [ %72, %69 ], [ %61, %60 ]
  %72 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %71) #42
  %73 = add i32 %70, 1
  %74 = icmp eq i32 %73, %57
  br i1 %74, label %75, label %69, !llvm.loop !83

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
  store ptr %84, ptr %6, align 4, !tbaa !18
  %85 = getelementptr inbounds %struct.FaString, ptr %6, i32 0, i32 1
  store i32 %80, ptr %85, align 4, !tbaa !20
  %86 = getelementptr inbounds %struct.FaString, ptr %6, i32 0, i32 2
  store i32 %9, ptr %86, align 4, !tbaa !23
  br label %89

87:                                               ; preds = %75
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %8) #44
  %88 = getelementptr inbounds i8, ptr %0, i32 %76
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %8, ptr noundef %88, i32 noundef %80, i32 noundef 0) #42
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %6, ptr noundef nonnull align 4 dereferenceable(12) %8, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %8) #44
  br label %89

89:                                               ; preds = %51, %83, %87, %24
  %90 = phi ptr [ %25, %24 ], [ %52, %51 ], [ null, %87 ], [ null, %83 ]
  ret ptr %90
}

; Function Attrs: cold noinline nounwind optsize
define hidden noundef ptr @fa_error_index(i64 noundef %0, i64 noundef %1) local_unnamed_addr #14 {
  %3 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #41
  store i64 1, ptr %3, align 8, !tbaa !84
  %4 = getelementptr inbounds %struct.IndexError, ptr %3, i32 0, i32 1
  store i64 %0, ptr %4, align 8, !tbaa !86
  %5 = getelementptr inbounds %struct.IndexError, ptr %3, i32 0, i32 2
  store i64 %1, ptr %5, align 8, !tbaa !87
  ret ptr %3
}

; Function Attrs: nounwind optsize
define hidden void @fa_string_case(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef writeonly %3) local_unnamed_addr #1 {
  %5 = alloca %struct.FaString, align 4
  %6 = alloca i32, align 4
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %6) #44
  store i32 0, ptr %6, align 4, !tbaa !1
  %7 = icmp eq i32 %1, 0
  br i1 %7, label %147, label %8

8:                                                ; preds = %4
  %9 = icmp eq i32 %2, 0
  br label %10

10:                                               ; preds = %8, %142
  %11 = call fastcc i32 @decode(ptr noundef %0, i32 noundef %1, ptr noundef nonnull %6) #42
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
  tail call fastcc void @append_c(ptr noundef nonnull @.str.12) #42
  br label %142, !llvm.loop !88

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
  tail call fastcc void @reserve(i32 noundef 1) #42
  %69 = load ptr, ptr @builder, align 4, !tbaa !6
  %70 = load i32, ptr @builder_length, align 4, !tbaa !1
  %71 = getelementptr inbounds i8, ptr %69, i32 %70
  store i8 %68, ptr %71, align 1, !tbaa !5
  br label %139

72:                                               ; preds = %64
  %73 = icmp ult i32 %65, 2048
  br i1 %73, label %74, label %89

74:                                               ; preds = %33, %36, %37, %60, %35, %62, %72
  %75 = phi i32 [ %65, %72 ], [ 376, %35 ], [ %61, %60 ], [ 931, %37 ], [ 924, %36 ], [ %34, %33 ], [ 255, %62 ]
  %76 = lshr i32 %75, 6
  %77 = trunc i32 %76 to i8
  %78 = or disjoint i8 %77, -64
  tail call fastcc void @reserve(i32 noundef 1) #42
  %79 = load ptr, ptr @builder, align 4, !tbaa !6
  %80 = load i32, ptr @builder_length, align 4, !tbaa !1
  %81 = getelementptr inbounds i8, ptr %79, i32 %80
  store i8 %78, ptr %81, align 1, !tbaa !5
  %82 = add i32 %80, 1
  store i32 %82, ptr @builder_length, align 4, !tbaa !1
  %83 = trunc i32 %75 to i8
  %84 = and i8 %83, 63
  %85 = or disjoint i8 %84, -128
  tail call fastcc void @reserve(i32 noundef 1) #42
  %86 = load ptr, ptr @builder, align 4, !tbaa !6
  %87 = load i32, ptr @builder_length, align 4, !tbaa !1
  %88 = getelementptr inbounds i8, ptr %86, i32 %87
  store i8 %85, ptr %88, align 1, !tbaa !5
  br label %139

89:                                               ; preds = %72
  %90 = icmp ult i32 %65, 65536
  tail call fastcc void @reserve(i32 noundef 1) #42
  %91 = load ptr, ptr @builder, align 4, !tbaa !6
  %92 = load i32, ptr @builder_length, align 4, !tbaa !1
  %93 = getelementptr inbounds i8, ptr %91, i32 %92
  %94 = add i32 %92, 1
  br i1 %90, label %95, label %113

95:                                               ; preds = %89
  %96 = lshr i32 %65, 12
  %97 = trunc i32 %96 to i8
  %98 = or disjoint i8 %97, -32
  store i8 %98, ptr %93, align 1, !tbaa !5
  store i32 %94, ptr @builder_length, align 4, !tbaa !1
  %99 = lshr i32 %65, 6
  %100 = trunc i32 %99 to i8
  %101 = and i8 %100, 63
  %102 = or disjoint i8 %101, -128
  tail call fastcc void @reserve(i32 noundef 1) #42
  %103 = load ptr, ptr @builder, align 4, !tbaa !6
  %104 = load i32, ptr @builder_length, align 4, !tbaa !1
  %105 = getelementptr inbounds i8, ptr %103, i32 %104
  store i8 %102, ptr %105, align 1, !tbaa !5
  %106 = add i32 %104, 1
  store i32 %106, ptr @builder_length, align 4, !tbaa !1
  %107 = trunc i32 %65 to i8
  %108 = and i8 %107, 63
  %109 = or disjoint i8 %108, -128
  tail call fastcc void @reserve(i32 noundef 1) #42
  %110 = load ptr, ptr @builder, align 4, !tbaa !6
  %111 = load i32, ptr @builder_length, align 4, !tbaa !1
  %112 = getelementptr inbounds i8, ptr %110, i32 %111
  store i8 %109, ptr %112, align 1, !tbaa !5
  br label %139

113:                                              ; preds = %89
  %114 = lshr i32 %65, 18
  %115 = trunc i32 %114 to i8
  %116 = or i8 %115, -16
  store i8 %116, ptr %93, align 1, !tbaa !5
  store i32 %94, ptr @builder_length, align 4, !tbaa !1
  %117 = lshr i32 %65, 12
  %118 = trunc i32 %117 to i8
  %119 = and i8 %118, 63
  %120 = or disjoint i8 %119, -128
  tail call fastcc void @reserve(i32 noundef 1) #42
  %121 = load ptr, ptr @builder, align 4, !tbaa !6
  %122 = load i32, ptr @builder_length, align 4, !tbaa !1
  %123 = getelementptr inbounds i8, ptr %121, i32 %122
  store i8 %120, ptr %123, align 1, !tbaa !5
  %124 = add i32 %122, 1
  store i32 %124, ptr @builder_length, align 4, !tbaa !1
  %125 = lshr i32 %65, 6
  %126 = trunc i32 %125 to i8
  %127 = and i8 %126, 63
  %128 = or disjoint i8 %127, -128
  tail call fastcc void @reserve(i32 noundef 1) #42
  %129 = load ptr, ptr @builder, align 4, !tbaa !6
  %130 = load i32, ptr @builder_length, align 4, !tbaa !1
  %131 = getelementptr inbounds i8, ptr %129, i32 %130
  store i8 %128, ptr %131, align 1, !tbaa !5
  %132 = add i32 %130, 1
  store i32 %132, ptr @builder_length, align 4, !tbaa !1
  %133 = trunc i32 %65 to i8
  %134 = and i8 %133, 63
  %135 = or disjoint i8 %134, -128
  tail call fastcc void @reserve(i32 noundef 1) #42
  %136 = load ptr, ptr @builder, align 4, !tbaa !6
  %137 = load i32, ptr @builder_length, align 4, !tbaa !1
  %138 = getelementptr inbounds i8, ptr %136, i32 %137
  store i8 %135, ptr %138, align 1, !tbaa !5
  br label %139

139:                                              ; preds = %67, %74, %95, %113
  %140 = phi i32 [ %87, %74 ], [ %137, %113 ], [ %111, %95 ], [ %70, %67 ]
  %141 = add i32 %140, 1
  store i32 %141, ptr @builder_length, align 4, !tbaa !1
  br label %142

142:                                              ; preds = %139, %38
  %143 = load i32, ptr %6, align 4, !tbaa !1
  %144 = icmp ult i32 %143, %1
  br i1 %144, label %10, label %145

145:                                              ; preds = %142
  %146 = load i32, ptr @builder_length, align 4, !tbaa !1
  br label %147

147:                                              ; preds = %145, %4
  %148 = phi i32 [ %146, %145 ], [ 0, %4 ]
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %5) #44
  %149 = load ptr, ptr @builder, align 4, !tbaa !6
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %5, ptr noundef %149, i32 noundef %148, i32 noundef 0) #42
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %3, ptr noundef nonnull align 4 dereferenceable(12) %5, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %5) #44
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %6) #44
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_string_trim(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef writeonly %3) local_unnamed_addr #1 {
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
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %5) #44
  store i32 %11, ptr %5, align 4, !tbaa !1
  %14 = call fastcc i32 @decode(ptr noundef %0, i32 noundef %1, ptr noundef nonnull %5) #42
  %15 = tail call fastcc i32 @is_space(i32 noundef %14) #42, !range !39
  %16 = icmp eq i32 %15, 0
  %17 = load i32, ptr %5, align 4
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %5) #44
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
  %31 = load i8, ptr %30, align 1, !tbaa !5
  %32 = and i8 %31, -64
  %33 = icmp eq i8 %32, -128
  br i1 %33, label %25, label %34, !llvm.loop !89

34:                                               ; preds = %25, %29
  %35 = phi i32 [ %19, %25 ], [ %27, %29 ]
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %6) #44
  store i32 %35, ptr %6, align 4, !tbaa !1
  %36 = call fastcc i32 @decode(ptr noundef %0, i32 noundef %23, ptr noundef nonnull %6) #42
  %37 = tail call fastcc i32 @is_space(i32 noundef %36) #42, !range !39
  %38 = icmp eq i32 %37, 0
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %6) #44
  br i1 %38, label %39, label %22

39:                                               ; preds = %34, %22, %18
  %40 = phi i32 [ %1, %18 ], [ %23, %22 ], [ %23, %34 ]
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %7) #44
  %41 = getelementptr inbounds i8, ptr %0, i32 %19
  %42 = sub i32 %40, %19
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %7, ptr noundef %41, i32 noundef %42, i32 noundef 0) #42
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %3, ptr noundef nonnull align 4 dereferenceable(12) %7, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %7) #44
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(none)
define internal fastcc noundef i32 @is_space(i32 noundef %0) unnamed_addr #15 {
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
define hidden void @fa_string_repeat(ptr nocapture noundef readonly %0, i32 noundef %1, i64 noundef %2, ptr nocapture noundef writeonly %3) local_unnamed_addr #1 {
  %5 = alloca %struct.FaString, align 4
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  %6 = icmp sgt i64 %2, 0
  br i1 %6, label %12, label %7

7:                                                ; preds = %4
  %8 = load ptr, ptr @builder, align 4, !tbaa !6
  br label %9

9:                                                ; preds = %12, %7
  %10 = phi i32 [ 0, %7 ], [ %17, %12 ]
  %11 = phi ptr [ %8, %7 ], [ %14, %12 ]
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %5) #44
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %5, ptr noundef %11, i32 noundef %10, i32 noundef 0) #42
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %3, ptr noundef nonnull align 4 dereferenceable(12) %5, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %5) #44
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  ret void

12:                                               ; preds = %4, %12
  %13 = phi i64 [ %18, %12 ], [ 0, %4 ]
  tail call fastcc void @reserve(i32 noundef %1) #42
  %14 = load ptr, ptr @builder, align 4, !tbaa !6
  %15 = load i32, ptr @builder_length, align 4, !tbaa !1
  %16 = getelementptr inbounds i8, ptr %14, i32 %15
  tail call fastcc void @copy_bytes(ptr noundef %16, ptr noundef %0, i32 noundef %1) #42
  %17 = add i32 %15, %1
  store i32 %17, ptr @builder_length, align 4, !tbaa !1
  %18 = add nuw nsw i64 %13, 1
  %19 = icmp eq i64 %18, %2
  br i1 %19, label %9, label %12, !llvm.loop !90
}

; Function Attrs: nounwind optsize
define hidden void @fa_array_join(ptr nocapture noundef readonly %0, ptr nocapture noundef readonly %1, i32 noundef %2, ptr nocapture noundef writeonly %3) local_unnamed_addr #1 {
  %5 = alloca %struct.FaString, align 4
  %6 = load ptr, ptr %0, align 8, !tbaa !55
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  %7 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %8 = load i64, ptr %7, align 8, !tbaa !66
  %9 = icmp sgt i64 %8, 0
  br i1 %9, label %15, label %10

10:                                               ; preds = %4
  %11 = load ptr, ptr @builder, align 4, !tbaa !6
  br label %12

12:                                               ; preds = %23, %10
  %13 = phi i32 [ 0, %10 ], [ %32, %23 ]
  %14 = phi ptr [ %11, %10 ], [ %29, %23 ]
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %5) #44
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %5, ptr noundef %14, i32 noundef %13, i32 noundef 0) #42
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %3, ptr noundef nonnull align 4 dereferenceable(12) %5, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %5) #44
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  ret void

15:                                               ; preds = %4, %23
  %16 = phi i64 [ %33, %23 ], [ 0, %4 ]
  %17 = icmp eq i64 %16, 0
  br i1 %17, label %23, label %18

18:                                               ; preds = %15
  tail call fastcc void @reserve(i32 noundef %2) #42
  %19 = load ptr, ptr @builder, align 4, !tbaa !6
  %20 = load i32, ptr @builder_length, align 4, !tbaa !1
  %21 = getelementptr inbounds i8, ptr %19, i32 %20
  tail call fastcc void @copy_bytes(ptr noundef %21, ptr noundef %1, i32 noundef %2) #42
  %22 = add i32 %20, %2
  store i32 %22, ptr @builder_length, align 4, !tbaa !1
  br label %23

23:                                               ; preds = %18, %15
  %24 = trunc i64 %16 to i32
  %25 = getelementptr inbounds %struct.FaString, ptr %6, i32 %24
  %26 = load ptr, ptr %25, align 4, !tbaa !18
  %27 = getelementptr inbounds %struct.FaString, ptr %6, i32 %24, i32 1
  %28 = load i32, ptr %27, align 4, !tbaa !20
  tail call fastcc void @reserve(i32 noundef %28) #42
  %29 = load ptr, ptr @builder, align 4, !tbaa !6
  %30 = load i32, ptr @builder_length, align 4, !tbaa !1
  %31 = getelementptr inbounds i8, ptr %29, i32 %30
  tail call fastcc void @copy_bytes(ptr noundef %31, ptr noundef %26, i32 noundef %28) #42
  %32 = add i32 %30, %28
  store i32 %32, ptr @builder_length, align 4, !tbaa !1
  %33 = add nuw nsw i64 %16, 1
  %34 = load i64, ptr %7, align 8, !tbaa !66
  %35 = icmp slt i64 %33, %34
  br i1 %35, label %15, label %12, !llvm.loop !91
}

; Function Attrs: nounwind optsize
define hidden void @fa_string_pad(ptr nocapture noundef readonly %0, i32 noundef %1, i64 noundef %2, ptr nocapture noundef readonly %3, i32 noundef %4, i32 noundef %5, ptr nocapture noundef writeonly %6) local_unnamed_addr #1 {
  %8 = alloca %struct.FaString, align 4
  %9 = tail call i32 @fa_graphemes(ptr noundef %0, i32 noundef %1) #42
  %10 = zext i32 %9 to i64
  %11 = sub nsw i64 %2, %10
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  %12 = icmp eq i32 %5, 0
  br i1 %12, label %13, label %18

13:                                               ; preds = %7
  tail call fastcc void @reserve(i32 noundef %1) #42
  %14 = load ptr, ptr @builder, align 4, !tbaa !6
  %15 = load i32, ptr @builder_length, align 4, !tbaa !1
  %16 = getelementptr inbounds i8, ptr %14, i32 %15
  tail call fastcc void @copy_bytes(ptr noundef %16, ptr noundef %0, i32 noundef %1) #42
  %17 = add i32 %15, %1
  store i32 %17, ptr @builder_length, align 4, !tbaa !1
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
  %30 = tail call fastcc i32 @next_boundary(ptr noundef %3, i32 noundef %4, i32 noundef %28) #42
  %31 = getelementptr inbounds i8, ptr %3, i32 %28
  %32 = sub i32 %30, %28
  tail call fastcc void @reserve(i32 noundef %32) #42
  %33 = load ptr, ptr @builder, align 4, !tbaa !6
  %34 = load i32, ptr @builder_length, align 4, !tbaa !1
  %35 = getelementptr inbounds i8, ptr %33, i32 %34
  tail call fastcc void @copy_bytes(ptr noundef %35, ptr noundef %31, i32 noundef %32) #42
  %36 = add i32 %34, %32
  store i32 %36, ptr @builder_length, align 4, !tbaa !1
  %37 = icmp eq i32 %30, %4
  %38 = select i1 %37, i32 0, i32 %30
  %39 = add nuw nsw i64 %29, 1
  %40 = icmp slt i64 %39, %11
  br i1 %40, label %27, label %23, !llvm.loop !92

41:                                               ; preds = %23
  tail call fastcc void @reserve(i32 noundef %1) #42
  %42 = load ptr, ptr @builder, align 4, !tbaa !6
  %43 = load i32, ptr @builder_length, align 4, !tbaa !1
  %44 = getelementptr inbounds i8, ptr %42, i32 %43
  tail call fastcc void @copy_bytes(ptr noundef %44, ptr noundef %0, i32 noundef %1) #42
  %45 = add i32 %43, %1
  store i32 %45, ptr @builder_length, align 4, !tbaa !1
  br label %46

46:                                               ; preds = %25, %41
  %47 = phi i32 [ %24, %25 ], [ %45, %41 ]
  %48 = phi ptr [ %26, %25 ], [ %42, %41 ]
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %8) #44
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %8, ptr noundef %48, i32 noundef %47, i32 noundef 0) #42
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %6, ptr noundef nonnull align 4 dereferenceable(12) %8, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %8) #44
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_arguments_init(i32 noundef %0, ptr nocapture noundef readnone %1) local_unnamed_addr #1 {
  %3 = tail call i32 @fa_host_arguments_size() #41
  %4 = tail call i32 @llvm.umax.i32(i32 %3, i32 1)
  %5 = tail call ptr @fa_alloc(i32 noundef %4, i32 noundef 1) #41
  store ptr %5, ptr @argument_bytes, align 4, !tbaa !6
  tail call void @fa_host_arguments_copy(ptr noundef %5) #41
  store i32 0, ptr @argument_count, align 4, !tbaa !1
  %6 = icmp eq i32 %3, 0
  br i1 %6, label %7, label %13

7:                                                ; preds = %2
  %8 = tail call ptr @fa_alloc(i32 noundef 12, i32 noundef 4) #41
  store ptr %8, ptr @argument_items, align 4, !tbaa !6
  br label %23

9:                                                ; preds = %13
  store i32 %20, ptr @argument_count, align 4, !tbaa !1
  %10 = mul i32 %20, 12
  %11 = add i32 %10, 12
  %12 = tail call ptr @fa_alloc(i32 noundef %11, i32 noundef 4) #41
  store ptr %12, ptr @argument_items, align 4, !tbaa !6
  br label %24

13:                                               ; preds = %2, %13
  %14 = phi i32 [ %21, %13 ], [ 0, %2 ]
  %15 = phi i32 [ %20, %13 ], [ 0, %2 ]
  %16 = getelementptr inbounds i8, ptr %5, i32 %14
  %17 = load i8, ptr %16, align 1, !tbaa !5
  %18 = icmp eq i8 %17, 0
  %19 = zext i1 %18 to i32
  %20 = add i32 %15, %19
  %21 = add nuw i32 %14, 1
  %22 = icmp eq i32 %21, %3
  br i1 %22, label %9, label %13, !llvm.loop !93

23:                                               ; preds = %42, %7
  ret void

24:                                               ; preds = %9, %42
  %25 = phi i32 [ %43, %42 ], [ 0, %9 ]
  %26 = phi i32 [ %45, %42 ], [ 0, %9 ]
  %27 = phi i32 [ %44, %42 ], [ 0, %9 ]
  %28 = getelementptr inbounds i8, ptr %5, i32 %25
  %29 = load i8, ptr %28, align 1, !tbaa !5
  %30 = icmp eq i8 %29, 0
  br i1 %30, label %33, label %31

31:                                               ; preds = %24
  %32 = add nuw i32 %25, 1
  br label %42

33:                                               ; preds = %24
  %34 = getelementptr inbounds i8, ptr %5, i32 %27
  %35 = getelementptr inbounds %struct.FaString, ptr %12, i32 %26
  store ptr %34, ptr %35, align 4, !tbaa !18
  %36 = sub i32 %25, %27
  %37 = getelementptr inbounds %struct.FaString, ptr %12, i32 %26, i32 1
  store i32 %36, ptr %37, align 4, !tbaa !20
  %38 = tail call fastcc i32 @simple_flag(ptr noundef %34, i32 noundef %36) #42, !range !22
  %39 = getelementptr inbounds %struct.FaString, ptr %12, i32 %26, i32 2
  store i32 %38, ptr %39, align 4, !tbaa !23
  %40 = add i32 %26, 1
  %41 = add nuw i32 %25, 1
  br label %42

42:                                               ; preds = %31, %33
  %43 = phi i32 [ %32, %31 ], [ %41, %33 ]
  %44 = phi i32 [ %27, %31 ], [ %41, %33 ]
  %45 = phi i32 [ %26, %31 ], [ %40, %33 ]
  %46 = icmp eq i32 %43, %3
  br i1 %46, label %23, label %24, !llvm.loop !94
}

; Function Attrs: optsize
declare i32 @fa_host_arguments_size() local_unnamed_addr #16

; Function Attrs: optsize
declare void @fa_host_arguments_copy(ptr noundef) local_unnamed_addr #17

; Function Attrs: noinline nounwind optsize
define internal fastcc void @array_grow(ptr noundef %0, i64 noundef %1) unnamed_addr #10 {
  %3 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %4 = load i64, ptr %3, align 8, !tbaa !67
  %5 = icmp slt i64 %4, 4
  %6 = shl nuw nsw i64 %4, 1
  %7 = select i1 %5, i64 4, i64 %6
  %8 = mul nsw i64 %7, %1
  %9 = trunc i64 %8 to i32
  %10 = tail call ptr @fa_alloc(i32 noundef %9, i32 noundef 16) #41
  %11 = load ptr, ptr %0, align 8, !tbaa !55
  %12 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %13 = load i64, ptr %12, align 8, !tbaa !66
  %14 = mul nsw i64 %13, %1
  %15 = trunc i64 %14 to i32
  tail call fastcc void @copy_bytes(ptr noundef %10, ptr noundef %11, i32 noundef %15) #42
  tail call fastcc void @free_array_items(ptr noundef nonnull %0, i64 noundef %1) #42
  store ptr %10, ptr %0, align 8, !tbaa !55
  store i64 %7, ptr %3, align 8, !tbaa !67
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_array_append(ptr noundef %0, ptr nocapture noundef readonly %1, i64 noundef %2) local_unnamed_addr #1 {
  %4 = getelementptr inbounds %struct.FaArrayHeader, ptr %1, i32 0, i32 1
  %5 = load i64, ptr %4, align 8, !tbaa !66
  %6 = icmp sgt i64 %5, 0
  br i1 %6, label %7, label %9

7:                                                ; preds = %3
  %8 = trunc i64 %2 to i32
  br label %10

9:                                                ; preds = %10, %3
  ret void

10:                                               ; preds = %7, %10
  %11 = phi i64 [ 0, %7 ], [ %17, %10 ]
  %12 = tail call ptr @fa_array_push_slot(ptr noundef %0, i64 noundef %2) #42
  %13 = load ptr, ptr %1, align 8, !tbaa !55
  %14 = mul nsw i64 %11, %2
  %15 = trunc i64 %14 to i32
  %16 = getelementptr inbounds i8, ptr %13, i32 %15
  tail call fastcc void @copy_bytes(ptr noundef %12, ptr noundef %16, i32 noundef %8) #42
  %17 = add nuw nsw i64 %11, 1
  %18 = icmp eq i64 %17, %5
  br i1 %18, label %9, label %10, !llvm.loop !95
}

; Function Attrs: mustprogress nofree noinline norecurse nosync nounwind optsize willreturn memory(none)
define hidden i64 @fa_index_from_end(i64 noundef %0, i64 noundef %1) local_unnamed_addr #18 {
  %3 = add nsw i64 %1, %0
  %4 = tail call i64 @llvm.smax.i64(i64 %3, i64 -1)
  %5 = icmp sgt i64 %0, -1
  %6 = select i1 %5, i64 -1, i64 %4
  ret i64 %6
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_array_slice(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2, i8 noundef signext %3, i64 noundef %4, ptr nocapture noundef writeonly %5) local_unnamed_addr #1 {
  %7 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %8 = load i64, ptr %7, align 8, !tbaa !66
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
  %18 = tail call ptr @fa_error_index(i64 noundef %1, i64 noundef %8) #42
  br label %58

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
  %45 = tail call ptr @fa_error_index(i64 noundef %2, i64 noundef %8) #42
  br label %58

46:                                               ; preds = %38, %23
  %47 = phi i64 [ %8, %23 ], [ %43, %38 ]
  %48 = tail call i64 @llvm.smax.i64(i64 %47, i64 %24)
  %49 = sub nsw i64 %48, %24
  %50 = tail call ptr @fa_array_new(i64 noundef %49, i64 noundef %49, i64 noundef %4) #42
  %51 = load ptr, ptr %50, align 8, !tbaa !55
  %52 = load ptr, ptr %0, align 8, !tbaa !55
  %53 = mul nsw i64 %24, %4
  %54 = trunc i64 %53 to i32
  %55 = getelementptr inbounds i8, ptr %52, i32 %54
  %56 = mul nsw i64 %49, %4
  %57 = trunc i64 %56 to i32
  tail call fastcc void @copy_bytes(ptr noundef %51, ptr noundef %55, i32 noundef %57) #42
  store ptr %50, ptr %5, align 4, !tbaa !6
  br label %58

58:                                               ; preds = %44, %46, %17
  %59 = phi ptr [ %18, %17 ], [ null, %46 ], [ %45, %44 ]
  ret ptr %59
}

; Function Attrs: nounwind optsize
define hidden void @fa_array_reserve(ptr noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #1 {
  %4 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %5 = load i64, ptr %4, align 8, !tbaa !67
  %6 = icmp slt i64 %5, %1
  br i1 %6, label %7, label %22

7:                                                ; preds = %3
  %8 = icmp sgt i64 %2, 0
  br i1 %8, label %9, label %13

9:                                                ; preds = %7
  %10 = udiv i64 2147483647, %2
  %11 = icmp slt i64 %10, %1
  br i1 %11, label %12, label %13

12:                                               ; preds = %9
  tail call fastcc void @out_of_memory() #45
  unreachable

13:                                               ; preds = %9, %7
  %14 = mul nsw i64 %2, %1
  %15 = trunc i64 %14 to i32
  %16 = tail call ptr @fa_alloc(i32 noundef %15, i32 noundef 16) #41
  %17 = load ptr, ptr %0, align 8, !tbaa !55
  %18 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %19 = load i64, ptr %18, align 8, !tbaa !66
  %20 = mul nsw i64 %19, %2
  %21 = trunc i64 %20 to i32
  tail call fastcc void @copy_bytes(ptr noundef %16, ptr noundef %17, i32 noundef %21) #42
  tail call fastcc void @free_array_items(ptr noundef nonnull %0, i64 noundef %2) #42
  store ptr %16, ptr %0, align 8, !tbaa !55
  store i64 %1, ptr %4, align 8, !tbaa !67
  br label %22

22:                                               ; preds = %3, %13
  ret void
}

; Function Attrs: noreturn nounwind optsize
define internal fastcc void @out_of_memory() unnamed_addr #4 {
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @out_of_memory.message, i32 noundef 20) #41
  tail call void @fa_exit(i32 noundef 101) #43
  unreachable
}

; Function Attrs: nounwind optsize
define internal fastcc void @free_array_items(ptr noundef %0, i64 noundef %1) unnamed_addr #1 {
  %3 = getelementptr inbounds i8, ptr %0, i32 24
  %4 = load i64, ptr %3, align 1
  %5 = icmp ne i64 %4, 0
  %6 = load ptr, ptr %0, align 8, !tbaa !55
  %7 = getelementptr inbounds i8, ptr %0, i32 32
  %8 = icmp eq ptr %6, %7
  %9 = select i1 %5, i1 %8, i1 false
  br i1 %9, label %10, label %11

10:                                               ; preds = %2
  store i64 0, ptr %3, align 1
  br label %16

11:                                               ; preds = %2
  %12 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %13 = load i64, ptr %12, align 8, !tbaa !67
  %14 = mul nsw i64 %13, %1
  %15 = trunc i64 %14 to i32
  tail call void @fa_free(ptr noundef %6, i32 noundef %15, i32 noundef 16) #41
  br label %16

16:                                               ; preds = %11, %10
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_array_assign(ptr noundef %0, ptr noundef readonly %1, i64 noundef %2) local_unnamed_addr #1 {
  %4 = icmp eq ptr %0, %1
  br i1 %4, label %26, label %5

5:                                                ; preds = %3
  %6 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %7 = load i64, ptr %6, align 8, !tbaa !67
  %8 = getelementptr inbounds %struct.FaArrayHeader, ptr %1, i32 0, i32 1
  %9 = load i64, ptr %8, align 8, !tbaa !66
  %10 = icmp slt i64 %7, %9
  br i1 %10, label %13, label %11

11:                                               ; preds = %5
  %12 = load ptr, ptr %0, align 8, !tbaa !55
  br label %18

13:                                               ; preds = %5
  %14 = mul nsw i64 %9, %2
  %15 = trunc i64 %14 to i32
  %16 = tail call ptr @fa_alloc(i32 noundef %15, i32 noundef 16) #41
  tail call fastcc void @free_array_items(ptr noundef nonnull %0, i64 noundef %2) #42
  store ptr %16, ptr %0, align 8, !tbaa !55
  %17 = load i64, ptr %8, align 8, !tbaa !66
  store i64 %17, ptr %6, align 8, !tbaa !67
  br label %18

18:                                               ; preds = %11, %13
  %19 = phi i64 [ %9, %11 ], [ %17, %13 ]
  %20 = phi ptr [ %12, %11 ], [ %16, %13 ]
  %21 = load ptr, ptr %1, align 8, !tbaa !55
  %22 = mul nsw i64 %19, %2
  %23 = trunc i64 %22 to i32
  tail call fastcc void @copy_bytes(ptr noundef %20, ptr noundef %21, i32 noundef %23) #42
  %24 = load i64, ptr %8, align 8, !tbaa !66
  %25 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  store i64 %24, ptr %25, align 8, !tbaa !66
  br label %26

26:                                               ; preds = %3, %18
  ret void
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_array_copy(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #1 {
  %3 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %4 = load i64, ptr %3, align 8, !tbaa !66
  %5 = tail call ptr @fa_array_new(i64 noundef %4, i64 noundef %4, i64 noundef %1) #42
  %6 = load ptr, ptr %5, align 8, !tbaa !55
  %7 = load ptr, ptr %0, align 8, !tbaa !55
  %8 = load i64, ptr %3, align 8, !tbaa !66
  %9 = mul nsw i64 %8, %1
  %10 = trunc i64 %9 to i32
  tail call fastcc void @copy_bytes(ptr noundef %6, ptr noundef %7, i32 noundef %10) #42
  ret ptr %5
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden void @fa_array_reverse(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #12 {
  %3 = alloca [64 x i8], align 16
  %4 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %5 = load i64, ptr %4, align 8, !tbaa !66
  %6 = icmp slt i64 %5, 2
  br i1 %6, label %36, label %7

7:                                                ; preds = %2
  call void @llvm.lifetime.start.p0(i64 64, ptr nonnull %3) #44
  %8 = add nuw i64 %5, 4294967295
  %9 = mul i64 %8, %1
  %10 = trunc i64 %9 to i32
  %11 = icmp sgt i32 %10, 0
  br i1 %11, label %12, label %35

12:                                               ; preds = %7
  %13 = load ptr, ptr %0, align 8, !tbaa !55
  %14 = getelementptr inbounds i8, ptr %13, i32 %10
  %15 = icmp sgt i64 %1, 0
  %16 = trunc i64 %1 to i32
  %17 = sub i32 0, %16
  br label %18

18:                                               ; preds = %12, %21
  %19 = phi ptr [ %13, %12 ], [ %22, %21 ]
  %20 = phi ptr [ %14, %12 ], [ %23, %21 ]
  br i1 %15, label %25, label %21

21:                                               ; preds = %25, %18
  %22 = getelementptr inbounds i8, ptr %19, i32 %16
  %23 = getelementptr inbounds i8, ptr %20, i32 %17
  %24 = icmp ult ptr %22, %23
  br i1 %24, label %18, label %35, !llvm.loop !96

25:                                               ; preds = %18, %25
  %26 = phi i64 [ %33, %25 ], [ 0, %18 ]
  %27 = sub nsw i64 %1, %26
  %28 = trunc i64 %27 to i32
  %29 = tail call i32 @llvm.umin.i32(i32 %28, i32 64)
  %30 = trunc i64 %26 to i32
  %31 = getelementptr inbounds i8, ptr %19, i32 %30
  call fastcc void @copy_bytes(ptr noundef nonnull %3, ptr noundef %31, i32 noundef %29) #42
  %32 = getelementptr inbounds i8, ptr %20, i32 %30
  tail call fastcc void @copy_bytes(ptr noundef %31, ptr noundef nonnull %32, i32 noundef %29) #42
  call fastcc void @copy_bytes(ptr noundef nonnull %32, ptr noundef nonnull %3, i32 noundef %29) #42
  %33 = add nuw nsw i64 %26, 64
  %34 = icmp slt i64 %33, %1
  br i1 %34, label %25, label %21, !llvm.loop !97

35:                                               ; preds = %21, %7
  call void @llvm.lifetime.end.p0(i64 64, ptr nonnull %3) #44
  br label %36

36:                                               ; preds = %2, %35
  ret void
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_array_insert_slot(ptr noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #1 {
  %4 = tail call ptr @fa_array_push_slot(ptr noundef %0, i64 noundef %2) #42
  %5 = load ptr, ptr %0, align 8, !tbaa !55
  %6 = mul nsw i64 %2, %1
  %7 = trunc i64 %6 to i32
  %8 = getelementptr inbounds i8, ptr %5, i32 %7
  %9 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %10 = load i64, ptr %9, align 8, !tbaa !66
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
declare void @llvm.memmove.p0.p0.i32(ptr nocapture writeonly, ptr nocapture readonly, i32, i1 immarg) #8

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define hidden void @fa_array_remove(ptr nocapture noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #19 {
  %4 = load ptr, ptr %0, align 8, !tbaa !55
  %5 = mul nsw i64 %2, %1
  %6 = trunc i64 %5 to i32
  %7 = getelementptr inbounds i8, ptr %4, i32 %6
  %8 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %9 = load i64, ptr %8, align 8, !tbaa !66
  %10 = xor i64 %1, -1
  %11 = add i64 %9, %10
  %12 = mul nsw i64 %11, %2
  %13 = trunc i64 %12 to i32
  %14 = trunc i64 %2 to i32
  %15 = getelementptr inbounds i8, ptr %7, i32 %14
  tail call void @llvm.memmove.p0.p0.i32(ptr align 1 %7, ptr align 1 %15, i32 %13, i1 false)
  %16 = load i64, ptr %8, align 8, !tbaa !66
  %17 = add nsw i64 %16, -1
  store i64 %17, ptr %8, align 8, !tbaa !66
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_array_sort(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef readonly %2, ptr noundef %3) local_unnamed_addr #1 {
  %5 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %6 = load i64, ptr %5, align 8, !tbaa !66
  %7 = icmp slt i64 %6, 2
  br i1 %7, label %86, label %8

8:                                                ; preds = %4
  %9 = mul nsw i64 %6, %1
  %10 = trunc i64 %9 to i32
  %11 = load ptr, ptr %0, align 8, !tbaa !55
  %12 = tail call ptr @fa_alloc(i32 noundef %10, i32 noundef 16) #41
  %13 = trunc i64 %1 to i32
  br label %14

14:                                               ; preds = %22, %8
  %15 = phi ptr [ %11, %8 ], [ %16, %22 ]
  %16 = phi ptr [ %12, %8 ], [ %15, %22 ]
  %17 = phi i64 [ 1, %8 ], [ %18, %22 ]
  %18 = shl nsw i64 %17, 1
  br label %24

19:                                               ; preds = %22
  %20 = load ptr, ptr %0, align 8, !tbaa !55
  %21 = icmp eq ptr %16, %20
  br i1 %21, label %84, label %83

22:                                               ; preds = %59
  %23 = icmp slt i64 %18, %6
  br i1 %23, label %14, label %19, !llvm.loop !98

24:                                               ; preds = %14, %59
  %25 = phi i64 [ 0, %14 ], [ %28, %59 ]
  %26 = add nsw i64 %25, %17
  %27 = tail call i64 @llvm.smin.i64(i64 %26, i64 %6)
  %28 = add nsw i64 %25, %18
  %29 = tail call i64 @llvm.smin.i64(i64 %28, i64 %6)
  %30 = icmp slt i64 %25, %27
  %31 = icmp slt i64 %26, %29
  %32 = select i1 %30, i1 %31, i1 false
  br i1 %32, label %33, label %59

33:                                               ; preds = %24, %52
  %34 = phi i64 [ %55, %52 ], [ %25, %24 ]
  %35 = phi i64 [ %54, %52 ], [ %27, %24 ]
  %36 = phi i64 [ %53, %52 ], [ %25, %24 ]
  %37 = mul nsw i64 %35, %1
  %38 = trunc i64 %37 to i32
  %39 = getelementptr inbounds i8, ptr %15, i32 %38
  %40 = mul nsw i64 %36, %1
  %41 = trunc i64 %40 to i32
  %42 = getelementptr inbounds i8, ptr %15, i32 %41
  %43 = tail call i32 %2(ptr noundef %3, ptr noundef %39, ptr noundef %42) #41
  %44 = icmp slt i32 %43, 0
  %45 = mul nsw i64 %34, %1
  %46 = trunc i64 %45 to i32
  %47 = getelementptr inbounds i8, ptr %16, i32 %46
  br i1 %44, label %48, label %50

48:                                               ; preds = %33
  tail call fastcc void @copy_bytes(ptr noundef %47, ptr noundef %39, i32 noundef %13) #42
  %49 = add nsw i64 %35, 1
  br label %52

50:                                               ; preds = %33
  tail call fastcc void @copy_bytes(ptr noundef %47, ptr noundef %42, i32 noundef %13) #42
  %51 = add nsw i64 %36, 1
  br label %52

52:                                               ; preds = %50, %48
  %53 = phi i64 [ %36, %48 ], [ %51, %50 ]
  %54 = phi i64 [ %49, %48 ], [ %35, %50 ]
  %55 = add nsw i64 %34, 1
  %56 = icmp slt i64 %53, %27
  %57 = icmp slt i64 %54, %29
  %58 = select i1 %56, i1 %57, i1 false
  br i1 %58, label %33, label %59, !llvm.loop !99

59:                                               ; preds = %52, %24
  %60 = phi i64 [ %25, %24 ], [ %53, %52 ]
  %61 = phi i64 [ %27, %24 ], [ %54, %52 ]
  %62 = phi i64 [ %25, %24 ], [ %55, %52 ]
  %63 = mul nsw i64 %62, %1
  %64 = trunc i64 %63 to i32
  %65 = getelementptr inbounds i8, ptr %16, i32 %64
  %66 = mul nsw i64 %60, %1
  %67 = trunc i64 %66 to i32
  %68 = getelementptr inbounds i8, ptr %15, i32 %67
  %69 = sub nsw i64 %27, %60
  %70 = mul nsw i64 %69, %1
  %71 = trunc i64 %70 to i32
  tail call fastcc void @copy_bytes(ptr noundef %65, ptr noundef %68, i32 noundef %71) #42
  %72 = add nsw i64 %62, %69
  %73 = mul nsw i64 %72, %1
  %74 = trunc i64 %73 to i32
  %75 = getelementptr inbounds i8, ptr %16, i32 %74
  %76 = mul nsw i64 %61, %1
  %77 = trunc i64 %76 to i32
  %78 = getelementptr inbounds i8, ptr %15, i32 %77
  %79 = sub nsw i64 %29, %61
  %80 = mul nsw i64 %79, %1
  %81 = trunc i64 %80 to i32
  tail call fastcc void @copy_bytes(ptr noundef %75, ptr noundef %78, i32 noundef %81) #42
  %82 = icmp slt i64 %28, %6
  br i1 %82, label %24, label %22, !llvm.loop !100

83:                                               ; preds = %19
  tail call fastcc void @copy_bytes(ptr noundef %20, ptr noundef %16, i32 noundef %10) #42
  br label %84

84:                                               ; preds = %19, %83
  %85 = phi ptr [ %16, %83 ], [ %15, %19 ]
  tail call void @fa_free(ptr noundef %85, i32 noundef %10, i32 noundef 16) #41
  br label %86

86:                                               ; preds = %84, %4
  ret void
}

; Function Attrs: optsize
declare void @fa_free(ptr noundef, i32 noundef, i32 noundef) local_unnamed_addr #2

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_arguments() local_unnamed_addr #1 {
  %1 = load i32, ptr @argument_count, align 4, !tbaa !1
  %2 = zext i32 %1 to i64
  %3 = tail call ptr @fa_array_new(i64 noundef %2, i64 noundef %2, i64 noundef 12) #42
  %4 = load ptr, ptr %3, align 8, !tbaa !55
  %5 = load ptr, ptr @argument_items, align 4, !tbaa !6
  %6 = load i32, ptr @argument_count, align 4, !tbaa !1
  %7 = mul i32 %6, 12
  tail call fastcc void @copy_bytes(ptr noundef %4, ptr noundef %5, i32 noundef %7) #42
  ret ptr %3
}

; Function Attrs: nounwind optsize
define hidden void @fa_array_free(ptr noundef %0, i64 noundef %1) local_unnamed_addr #1 {
  %3 = getelementptr inbounds i8, ptr %0, i32 24
  %4 = load i64, ptr %3, align 1
  %5 = icmp ne i64 %4, 0
  %6 = load ptr, ptr %0, align 8, !tbaa !55
  %7 = getelementptr inbounds i8, ptr %0, i32 32
  %8 = icmp eq ptr %6, %7
  %9 = select i1 %5, i1 %8, i1 false
  br i1 %9, label %15, label %10

10:                                               ; preds = %2
  %11 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %12 = load i64, ptr %11, align 8, !tbaa !67
  %13 = mul nsw i64 %12, %1
  %14 = trunc i64 %13 to i32
  tail call void @fa_free(ptr noundef %6, i32 noundef %14, i32 noundef 16) #41
  br label %15

15:                                               ; preds = %2, %10
  tail call void @fa_free(ptr noundef nonnull %0, i32 noundef 32, i32 noundef 8) #41
  ret void
}

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read)
define hidden ptr @fa_segmented_at(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2, i32 noundef %3) local_unnamed_addr #20 {
  %5 = zext nneg i32 %3 to i64
  %6 = lshr i64 %1, %5
  %7 = icmp eq i64 %6, 0
  br i1 %7, label %16, label %8

8:                                                ; preds = %4
  %9 = tail call i64 @llvm.ctlz.i64(i64 %6, i1 true), !range !101
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
define hidden noundef ptr @fa_segmented_new() local_unnamed_addr #1 {
  %1 = tail call ptr @fa_alloc(i32 noundef 208, i32 noundef 8) #41
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(208) %1, i8 0, i32 208, i1 false)
  ret ptr %1
}

; Function Attrs: mustprogress nocallback nofree nounwind willreturn memory(argmem: write)
declare void @llvm.memset.p0.i32(ptr nocapture writeonly, i8, i32, i1 immarg) #21

; Function Attrs: nounwind optsize
define hidden ptr @fa_segmented_push_slot(ptr nocapture noundef %0, i64 noundef %1, i32 noundef %2) local_unnamed_addr #1 {
  %4 = load i64, ptr %0, align 8, !tbaa !102
  %5 = zext nneg i32 %2 to i64
  %6 = lshr i64 %4, %5
  %7 = icmp eq i64 %6, 0
  br i1 %7, label %15, label %8

8:                                                ; preds = %3
  %9 = tail call i64 @llvm.ctlz.i64(i64 %6, i1 true), !range !101
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
  %19 = load i64, ptr %18, align 8, !tbaa !104
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
  %29 = tail call ptr @fa_alloc(i32 noundef %28, i32 noundef 16) #41
  %30 = trunc i64 %17 to i32
  %31 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 2, i32 %30
  store ptr %29, ptr %31, align 4, !tbaa !6
  %32 = add nuw nsw i64 %17, 1
  store i64 %32, ptr %18, align 8, !tbaa !104
  br label %33

33:                                               ; preds = %21, %23
  %34 = phi i32 [ %22, %21 ], [ %30, %23 ]
  %35 = add nsw i64 %4, 1
  store i64 %35, ptr %0, align 8, !tbaa !102
  %36 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 2, i32 %34
  %37 = load ptr, ptr %36, align 4, !tbaa !6
  %38 = add i64 %16, %4
  %39 = mul nsw i64 %38, %1
  %40 = trunc i64 %39 to i32
  %41 = getelementptr inbounds i8, ptr %37, i32 %40
  ret ptr %41
}

; Function Attrs: nounwind optsize
define hidden void @fa_segmented_free(ptr noundef %0, i64 noundef %1, i32 noundef %2) local_unnamed_addr #1 {
  %4 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 1
  %5 = load i64, ptr %4, align 8, !tbaa !104
  %6 = icmp sgt i64 %5, 0
  br i1 %6, label %7, label %9

7:                                                ; preds = %3
  %8 = sext i32 %2 to i64
  br label %10

9:                                                ; preds = %10, %3
  tail call void @fa_free(ptr noundef nonnull %0, i32 noundef 208, i32 noundef 8) #41
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
  tail call void @fa_free(ptr noundef %14, i32 noundef %18, i32 noundef 16) #41
  %19 = add nuw nsw i64 %11, 1
  %20 = load i64, ptr %4, align 8, !tbaa !104
  %21 = icmp slt i64 %19, %20
  br i1 %21, label %10, label %9, !llvm.loop !105
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_segmented_flatten(ptr nocapture noundef readonly %0, i64 noundef %1, i32 noundef %2) local_unnamed_addr #1 {
  %4 = load i64, ptr %0, align 8, !tbaa !102
  %5 = tail call ptr @fa_array_new(i64 noundef %4, i64 noundef %4, i64 noundef %1) #42
  %6 = load i64, ptr %0, align 8, !tbaa !102
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
  %20 = load ptr, ptr %5, align 8, !tbaa !55
  %21 = mul nsw i64 %14, %1
  %22 = trunc i64 %21 to i32
  %23 = getelementptr inbounds i8, ptr %20, i32 %22
  %24 = trunc i64 %13 to i32
  %25 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 2, i32 %24
  %26 = load ptr, ptr %25, align 4, !tbaa !6
  %27 = mul nsw i64 %19, %1
  %28 = trunc i64 %27 to i32
  tail call fastcc void @copy_bytes(ptr noundef %23, ptr noundef %26, i32 noundef %28) #42
  %29 = add nsw i64 %19, %14
  %30 = add nuw nsw i64 %13, 1
  %31 = load i64, ptr %0, align 8, !tbaa !102
  %32 = icmp slt i64 %29, %31
  br i1 %32, label %11, label %10, !llvm.loop !106
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_segmented_from_array(ptr noundef %0, i64 noundef %1, i32 noundef %2) local_unnamed_addr #1 {
  %4 = tail call noundef ptr @fa_alloc(i32 noundef 208, i32 noundef 8) #41
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(208) %4, i8 0, i32 208, i1 false)
  %5 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %6 = load i64, ptr %5, align 8, !tbaa !66
  %7 = icmp sgt i64 %6, 0
  br i1 %7, label %8, label %10

8:                                                ; preds = %3
  %9 = trunc i64 %1 to i32
  br label %11

10:                                               ; preds = %11, %3
  tail call void @fa_array_free(ptr noundef nonnull %0, i64 noundef %1) #42
  ret ptr %4

11:                                               ; preds = %8, %11
  %12 = phi i64 [ 0, %8 ], [ %18, %11 ]
  %13 = tail call ptr @fa_segmented_push_slot(ptr noundef %4, i64 noundef %1, i32 noundef %2) #42
  %14 = load ptr, ptr %0, align 8, !tbaa !55
  %15 = mul nsw i64 %12, %1
  %16 = trunc i64 %15 to i32
  %17 = getelementptr inbounds i8, ptr %14, i32 %16
  tail call fastcc void @copy_bytes(ptr noundef %13, ptr noundef %17, i32 noundef %9) #42
  %18 = add nuw nsw i64 %12, 1
  %19 = load i64, ptr %5, align 8, !tbaa !66
  %20 = icmp slt i64 %18, %19
  br i1 %20, label %11, label %10, !llvm.loop !107
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_heap_alloc(i32 noundef %0, i32 noundef %1) local_unnamed_addr #1 {
  %3 = icmp ugt i32 %1, 8
  %4 = add i32 %0, 15
  %5 = and i32 %4, -16
  %6 = select i1 %3, i32 %5, i32 %0
  %7 = icmp ult i32 %6, 129
  br i1 %7, label %8, label %19

8:                                                ; preds = %2
  %9 = icmp ult i32 %6, 9
  %10 = add nuw nsw i32 %6, 7
  %11 = lshr i32 %10, 3
  %12 = add nsw i32 %11, -1
  %13 = select i1 %9, i32 0, i32 %12
  %14 = getelementptr inbounds [40 x ptr], ptr @free_blocks, i32 0, i32 %13
  %15 = load ptr, ptr %14, align 4, !tbaa !6
  %16 = icmp eq ptr %15, null
  br i1 %16, label %19, label %17, !prof !70

17:                                               ; preds = %8
  %18 = load ptr, ptr %15, align 4, !tbaa !108
  store ptr %18, ptr %14, align 4, !tbaa !6
  br label %21

19:                                               ; preds = %8, %2
  %20 = tail call fastcc ptr @heap_alloc_slow(i32 noundef %6) #42
  br label %21

21:                                               ; preds = %17, %19
  %22 = phi ptr [ %15, %17 ], [ %20, %19 ]
  ret ptr %22
}

; Function Attrs: noinline nounwind optsize
define internal fastcc ptr @heap_alloc_slow(i32 noundef %0) unnamed_addr #10 {
  %2 = icmp ugt i32 %0, 8192
  br i1 %2, label %3, label %13

3:                                                ; preds = %1
  %4 = add i32 %0, 65551
  %5 = lshr i32 %4, 16
  %6 = icmp ult i32 %4, 65536
  br i1 %6, label %66, label %7

7:                                                ; preds = %3
  %8 = tail call fastcc ptr @take_pages(i32 noundef %5) #42
  %9 = icmp eq ptr %8, null
  br i1 %9, label %66, label %10

10:                                               ; preds = %7
  store i32 65535, ptr %8, align 4, !tbaa !110
  %11 = getelementptr inbounds %struct.PageHeader, ptr %8, i32 0, i32 1
  store i32 %5, ptr %11, align 4, !tbaa !112
  %12 = getelementptr inbounds i8, ptr %8, i32 16
  br label %66

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
  br i1 %36, label %37, label %27, !llvm.loop !113

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
  %46 = load ptr, ptr %43, align 4, !tbaa !108
  store ptr %46, ptr %42, align 4, !tbaa !6
  br label %66

47:                                               ; preds = %39
  %48 = getelementptr inbounds [40 x ptr], ptr @class_next, i32 0, i32 %41
  %49 = load ptr, ptr %48, align 4, !tbaa !6
  %50 = getelementptr inbounds i8, ptr %49, i32 %40
  %51 = getelementptr inbounds [40 x ptr], ptr @class_end, i32 0, i32 %41
  %52 = load ptr, ptr %51, align 4, !tbaa !6
  %53 = icmp ugt ptr %50, %52
  %54 = icmp eq ptr %49, null
  %55 = or i1 %54, %53
  br i1 %55, label %56, label %63

56:                                               ; preds = %47
  %57 = tail call fastcc ptr @take_pages(i32 noundef 1) #42
  %58 = icmp eq ptr %57, null
  br i1 %58, label %66, label %59

59:                                               ; preds = %56
  store i32 %41, ptr %57, align 4, !tbaa !110
  %60 = getelementptr inbounds %struct.PageHeader, ptr %57, i32 0, i32 1
  store i32 1, ptr %60, align 4, !tbaa !112
  %61 = getelementptr inbounds i8, ptr %57, i32 16
  %62 = getelementptr inbounds i8, ptr %57, i32 65536
  store ptr %62, ptr %51, align 4, !tbaa !6
  br label %63

63:                                               ; preds = %59, %47
  %64 = phi ptr [ %61, %59 ], [ %49, %47 ]
  %65 = getelementptr inbounds i8, ptr %64, i32 %40
  store ptr %65, ptr %48, align 4, !tbaa !6
  br label %66

66:                                               ; preds = %45, %63, %56, %3, %7, %10
  %67 = phi ptr [ null, %3 ], [ %12, %10 ], [ null, %7 ], [ %43, %45 ], [ %64, %63 ], [ null, %56 ]
  ret ptr %67
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden void @fa_heap_free(ptr noundef %0) local_unnamed_addr #12 {
  %2 = ptrtoint ptr %0 to i32
  %3 = and i32 %2, -65536
  %4 = inttoptr i32 %3 to ptr
  %5 = load i32, ptr %4, align 65536, !tbaa !110
  %6 = icmp eq i32 %5, 65535
  br i1 %6, label %7, label %10

7:                                                ; preds = %1
  %8 = getelementptr inbounds %struct.PageHeader, ptr %4, i32 0, i32 1
  %9 = load i32, ptr %8, align 4, !tbaa !112
  tail call fastcc void @give_pages(ptr noundef nonnull %4, i32 noundef %9) #42
  br label %13

10:                                               ; preds = %1
  %11 = getelementptr inbounds [40 x ptr], ptr @free_blocks, i32 0, i32 %5
  %12 = load ptr, ptr %11, align 4, !tbaa !6
  store ptr %12, ptr %0, align 4, !tbaa !108
  store ptr %0, ptr %11, align 4, !tbaa !6
  br label %13

13:                                               ; preds = %10, %7
  ret void
}

; Function Attrs: nofree noinline norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc void @give_pages(ptr noundef %0, i32 noundef %1) unnamed_addr #13 {
  %3 = getelementptr inbounds %struct.FreeSpan, ptr %0, i32 0, i32 1
  store i32 %1, ptr %3, align 4, !tbaa !114
  br label %4

4:                                                ; preds = %4, %2
  %5 = phi ptr [ null, %2 ], [ %7, %4 ]
  %6 = phi ptr [ @free_spans, %2 ], [ %7, %4 ]
  %7 = load ptr, ptr %6, align 4, !tbaa !6
  %8 = icmp ne ptr %7, null
  %9 = icmp ult ptr %7, %0
  %10 = and i1 %8, %9
  br i1 %10, label %4, label %11, !llvm.loop !116

11:                                               ; preds = %4
  store ptr %7, ptr %0, align 4, !tbaa !117
  br i1 %8, label %12, label %21

12:                                               ; preds = %11
  %13 = shl i32 %1, 16
  %14 = getelementptr inbounds i8, ptr %0, i32 %13
  %15 = icmp eq ptr %14, %7
  br i1 %15, label %16, label %21

16:                                               ; preds = %12
  %17 = getelementptr inbounds %struct.FreeSpan, ptr %7, i32 0, i32 1
  %18 = load i32, ptr %17, align 4, !tbaa !114
  %19 = add i32 %18, %1
  store i32 %19, ptr %3, align 4, !tbaa !114
  %20 = load ptr, ptr %7, align 4, !tbaa !117
  store ptr %20, ptr %0, align 4, !tbaa !117
  br label %21

21:                                               ; preds = %16, %12, %11
  %22 = phi ptr [ %20, %16 ], [ %7, %12 ], [ null, %11 ]
  %23 = phi i32 [ %19, %16 ], [ %1, %12 ], [ %1, %11 ]
  %24 = icmp eq ptr %5, null
  br i1 %24, label %34, label %25

25:                                               ; preds = %21
  %26 = getelementptr inbounds %struct.FreeSpan, ptr %5, i32 0, i32 1
  %27 = load i32, ptr %26, align 4, !tbaa !114
  %28 = shl i32 %27, 16
  %29 = getelementptr inbounds i8, ptr %5, i32 %28
  %30 = icmp eq ptr %29, %0
  br i1 %30, label %31, label %33

31:                                               ; preds = %25
  %32 = add i32 %27, %23
  store i32 %32, ptr %26, align 4, !tbaa !114
  store ptr %22, ptr %5, align 4, !tbaa !117
  br label %35

33:                                               ; preds = %25
  store ptr %0, ptr %5, align 4, !tbaa !117
  br label %35

34:                                               ; preds = %21
  store ptr %0, ptr @free_spans, align 4, !tbaa !6
  br label %35

35:                                               ; preds = %31, %33, %34
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_release_builder() local_unnamed_addr #1 {
  %1 = load ptr, ptr @builder, align 4, !tbaa !6
  %2 = icmp eq ptr %1, null
  br i1 %2, label %5, label %3

3:                                                ; preds = %0
  %4 = load i32, ptr @builder_capacity, align 4, !tbaa !1
  tail call void @fa_free(ptr noundef nonnull %1, i32 noundef %4, i32 noundef 1) #41
  store ptr null, ptr @builder, align 4, !tbaa !6
  store i32 0, ptr @builder_capacity, align 4, !tbaa !1
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  br label %5

5:                                                ; preds = %3, %0
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_leak_check() local_unnamed_addr #1 {
  %1 = load i32, ptr @output_length, align 4, !tbaa !1
  %2 = icmp eq i32 %1, 0
  br i1 %2, label %4, label %3

3:                                                ; preds = %0
  tail call void @fa_write(i32 noundef 1, ptr noundef nonnull @output, i32 noundef %1) #41
  store i32 0, ptr @output_length, align 4, !tbaa !1
  br label %4

4:                                                ; preds = %0, %3
  %5 = load ptr, ptr @builder, align 4, !tbaa !6
  %6 = icmp eq ptr %5, null
  br i1 %6, label %9, label %7

7:                                                ; preds = %4
  %8 = load i32, ptr @builder_capacity, align 4, !tbaa !1
  tail call void @fa_free(ptr noundef nonnull %5, i32 noundef %8, i32 noundef 1) #41
  store ptr null, ptr @builder, align 4, !tbaa !6
  store i32 0, ptr @builder_capacity, align 4, !tbaa !1
  br label %9

9:                                                ; preds = %7, %4
  %10 = load ptr, ptr @argument_items, align 4, !tbaa !6
  %11 = icmp eq ptr %10, null
  br i1 %11, label %16, label %12

12:                                               ; preds = %9
  %13 = load i32, ptr @argument_count, align 4, !tbaa !1
  %14 = mul i32 %13, 12
  %15 = add i32 %14, 12
  tail call void @fa_free(ptr noundef nonnull %10, i32 noundef %15, i32 noundef 4) #41
  store ptr null, ptr @argument_items, align 4, !tbaa !6
  br label %16

16:                                               ; preds = %12, %9
  %17 = load ptr, ptr @argument_bytes, align 4, !tbaa !6
  %18 = icmp eq ptr %17, null
  br i1 %18, label %20, label %19

19:                                               ; preds = %16
  tail call void @fa_free(ptr noundef nonnull %17, i32 noundef 1, i32 noundef 1) #41
  store ptr null, ptr @argument_bytes, align 4, !tbaa !6
  br label %20

20:                                               ; preds = %19, %16
  %21 = load i32, ptr @fa_live_blocks, align 4, !tbaa !1
  %22 = icmp eq i32 %21, 0
  br i1 %22, label %26, label %23

23:                                               ; preds = %20
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  tail call fastcc void @append_c(ptr noundef nonnull @.str.13) #42
  %24 = load i32, ptr @fa_live_blocks, align 4, !tbaa !1
  %25 = zext i32 %24 to i64
  tail call void @fa_b_u64(i64 noundef %25) #42
  tail call fastcc void @append_c(ptr noundef nonnull @.str.14) #42
  tail call void @fa_b_line(i32 noundef 2) #42
  tail call void @fa_exit(i32 noundef 102) #43
  unreachable

26:                                               ; preds = %20
  ret void
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_pool_new(i64 noundef %0) local_unnamed_addr #1 {
  %2 = tail call ptr @fa_alloc(i32 noundef 48, i32 noundef 8) #41
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(48) %2, i8 0, i32 48, i1 false)
  %3 = tail call i64 @llvm.smax.i64(i64 %0, i64 4)
  %4 = trunc i64 %3 to i32
  %5 = getelementptr inbounds %struct.FaPool, ptr %2, i32 0, i32 6
  store i32 %4, ptr %5, align 4, !tbaa !118
  %6 = shl i32 %4, 6
  %7 = add i32 %6, 104
  br label %8

8:                                                ; preds = %8, %1
  %9 = phi i32 [ 1, %1 ], [ %12, %8 ]
  %10 = shl i32 %9, 16
  %11 = icmp ult i32 %10, %7
  %12 = shl i32 %9, 1
  br i1 %11, label %8, label %13, !llvm.loop !120

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
  store i32 %20, ptr %31, align 8, !tbaa !121
  %32 = getelementptr inbounds %struct.FaPool, ptr %2, i32 0, i32 8
  store i32 %9, ptr %32, align 4, !tbaa !122
  %33 = getelementptr inbounds %struct.FaPool, ptr %2, i32 0, i32 9
  store i32 %25, ptr %33, align 8, !tbaa !123
  ret ptr %2
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_columnar_new(i64 noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #1 {
  %4 = tail call ptr @fa_alloc(i32 noundef 48, i32 noundef 8) #41
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(48) %4, i8 0, i32 48, i1 false)
  %5 = getelementptr inbounds %struct.FaPool, ptr %4, i32 0, i32 6
  store i32 4, ptr %5, align 4, !tbaa !118
  %6 = trunc i64 %0 to i32
  %7 = getelementptr inbounds %struct.FaPool, ptr %4, i32 0, i32 7
  store i32 %6, ptr %7, align 8, !tbaa !121
  %8 = trunc i64 %1 to i32
  %9 = getelementptr inbounds %struct.FaPool, ptr %4, i32 0, i32 8
  store i32 %8, ptr %9, align 4, !tbaa !122
  %10 = trunc i64 %2 to i32
  %11 = getelementptr inbounds %struct.FaPool, ptr %4, i32 0, i32 9
  store i32 %10, ptr %11, align 8, !tbaa !123
  ret ptr %4
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_pool_add_slot(ptr nocapture noundef %0) local_unnamed_addr #1 {
  %2 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 4
  %3 = load i32, ptr %2, align 4, !tbaa !124
  %4 = add i32 %3, 63
  %5 = lshr i32 %4, 6
  %6 = icmp ult i32 %4, 64
  br i1 %6, label %28, label %7

7:                                                ; preds = %1
  %8 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 2
  %9 = load ptr, ptr %8, align 4, !tbaa !125
  br label %10

10:                                               ; preds = %7, %15
  %11 = phi i32 [ 0, %7 ], [ %16, %15 ]
  %12 = getelementptr inbounds i64, ptr %9, i32 %11
  %13 = load i64, ptr %12, align 8, !tbaa !80
  %14 = icmp eq i64 %13, 0
  br i1 %14, label %15, label %18

15:                                               ; preds = %10
  %16 = add nuw nsw i32 %11, 1
  %17 = icmp eq i32 %16, %5
  br i1 %17, label %28, label %10, !llvm.loop !126

18:                                               ; preds = %10
  %19 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 1
  %20 = load ptr, ptr %19, align 8, !tbaa !127
  %21 = shl nuw i32 %11, 6
  %22 = tail call i64 @llvm.cttz.i64(i64 %13, i1 true), !range !101
  %23 = trunc i64 %22 to i32
  %24 = or disjoint i32 %21, %23
  %25 = getelementptr inbounds ptr, ptr %20, i32 %24
  %26 = load ptr, ptr %25, align 4, !tbaa !6
  %27 = icmp eq ptr %26, null
  br i1 %27, label %28, label %30

28:                                               ; preds = %15, %1, %18
  %29 = tail call fastcc ptr @pool_new_chunk(ptr noundef nonnull %0) #46
  br label %30

30:                                               ; preds = %28, %18
  %31 = phi ptr [ %29, %28 ], [ %26, %18 ]
  %32 = getelementptr inbounds %struct.PoolChunk, ptr %31, i32 0, i32 4
  %33 = load i32, ptr %32, align 8, !tbaa !12
  %34 = icmp eq i32 %33, 0
  br i1 %34, label %44, label %35

35:                                               ; preds = %30
  %36 = add i32 %33, -1
  %37 = load ptr, ptr %31, align 8, !tbaa !6
  %38 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 6
  %39 = load i32, ptr %38, align 4, !tbaa !118
  %40 = mul i32 %39, %36
  %41 = getelementptr inbounds i8, ptr %37, i32 %40
  %42 = load i32, ptr %41, align 1
  store i32 %42, ptr %32, align 8, !tbaa !12
  %43 = icmp eq i32 %42, 0
  br label %48

44:                                               ; preds = %30
  %45 = getelementptr inbounds %struct.PoolChunk, ptr %31, i32 0, i32 3
  %46 = load i32, ptr %45, align 4, !tbaa !12
  %47 = add i32 %46, 1
  store i32 %47, ptr %45, align 4, !tbaa !12
  br label %48

48:                                               ; preds = %44, %35
  %49 = phi i1 [ %43, %35 ], [ true, %44 ]
  %50 = phi i32 [ %36, %35 ], [ %46, %44 ]
  %51 = and i32 %50, 63
  %52 = zext nneg i32 %51 to i64
  %53 = shl nuw i64 1, %52
  %54 = lshr i32 %50, 6
  %55 = getelementptr inbounds %struct.PoolChunk, ptr %31, i32 0, i32 5, i32 %54
  %56 = load i64, ptr %55, align 8, !tbaa !80
  %57 = or i64 %53, %56
  store i64 %57, ptr %55, align 8, !tbaa !80
  %58 = getelementptr inbounds %struct.PoolChunk, ptr %31, i32 0, i32 2
  %59 = load i32, ptr %58, align 8, !tbaa !12
  %60 = add i32 %59, 1
  store i32 %60, ptr %58, align 8, !tbaa !12
  %61 = load i64, ptr %0, align 8, !tbaa !128
  %62 = add nsw i64 %61, 1
  store i64 %62, ptr %0, align 8, !tbaa !128
  br i1 %49, label %63, label %82

63:                                               ; preds = %48
  %64 = getelementptr inbounds %struct.PoolChunk, ptr %31, i32 0, i32 3
  %65 = load i32, ptr %64, align 4, !tbaa !12
  %66 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 7
  %67 = load i32, ptr %66, align 8, !tbaa !121
  %68 = icmp eq i32 %65, %67
  br i1 %68, label %69, label %82

69:                                               ; preds = %63
  %70 = getelementptr inbounds %struct.PoolChunk, ptr %31, i32 0, i32 1
  %71 = load i32, ptr %70, align 4, !tbaa !12
  %72 = and i32 %71, 63
  %73 = zext nneg i32 %72 to i64
  %74 = shl nuw i64 1, %73
  %75 = xor i64 %74, -1
  %76 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 2
  %77 = load ptr, ptr %76, align 4, !tbaa !125
  %78 = lshr i32 %71, 6
  %79 = getelementptr inbounds i64, ptr %77, i32 %78
  %80 = load i64, ptr %79, align 8, !tbaa !80
  %81 = and i64 %80, %75
  store i64 %81, ptr %79, align 8, !tbaa !80
  br label %82

82:                                               ; preds = %69, %63, %48
  %83 = load ptr, ptr %31, align 8, !tbaa !6
  %84 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 6
  %85 = load i32, ptr %84, align 4, !tbaa !118
  %86 = mul i32 %85, %50
  %87 = getelementptr inbounds i8, ptr %83, i32 %86
  ret ptr %87
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.cttz.i64(i64, i1 immarg) #22

; Function Attrs: cold noinline nounwind optsize
define internal fastcc noundef ptr @pool_new_chunk(ptr nocapture noundef %0) unnamed_addr #14 {
  %2 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 3
  %3 = load ptr, ptr %2, align 8, !tbaa !129
  store ptr null, ptr %2, align 8, !tbaa !129
  %4 = icmp eq ptr %3, null
  br i1 %4, label %5, label %40

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 8
  %7 = load i32, ptr %6, align 4, !tbaa !122
  %8 = icmp eq i32 %7, 1
  br i1 %8, label %9, label %11

9:                                                ; preds = %5
  %10 = tail call fastcc ptr @take_pages(i32 noundef 1) #42
  br label %34

11:                                               ; preds = %5
  %12 = shl i32 %7, 1
  %13 = add i32 %12, -1
  %14 = tail call fastcc ptr @take_pages(i32 noundef %13) #42
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
  tail call fastcc void @give_pages(ptr noundef nonnull %14, i32 noundef %25) #42
  br label %28

28:                                               ; preds = %27, %16
  %29 = xor i32 %25, -1
  %30 = add i32 %7, %29
  %31 = icmp eq i32 %30, 0
  br i1 %31, label %34, label %32

32:                                               ; preds = %28
  %33 = getelementptr inbounds i8, ptr %23, i32 %17
  tail call fastcc void @give_pages(ptr noundef %33, i32 noundef %30) #42
  br label %34

34:                                               ; preds = %9, %28, %32
  %35 = phi ptr [ %10, %9 ], [ %23, %32 ], [ %23, %28 ]
  %36 = icmp eq ptr %35, null
  br i1 %36, label %37, label %38

37:                                               ; preds = %11, %34
  tail call fastcc void @out_of_memory() #45
  unreachable

38:                                               ; preds = %34
  %39 = atomicrmw add ptr @fa_live_blocks, i32 1 monotonic, align 4
  br label %40

40:                                               ; preds = %38, %1
  %41 = phi ptr [ %35, %38 ], [ %3, %1 ]
  %42 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 7
  %43 = load i32, ptr %42, align 8, !tbaa !121
  %44 = add i32 %43, 63
  %45 = lshr i32 %44, 3
  %46 = and i32 %45, 536870904
  %47 = add nuw nsw i32 %46, 24
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(1) %41, i8 0, i32 %47, i1 false)
  %48 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 9
  %49 = load i32, ptr %48, align 8, !tbaa !123
  %50 = getelementptr inbounds i8, ptr %41, i32 %49
  store ptr %50, ptr %41, align 8, !tbaa !6
  %51 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 4
  %52 = load i32, ptr %51, align 4, !tbaa !124
  %53 = icmp eq i32 %52, 0
  br i1 %53, label %65, label %54

54:                                               ; preds = %40
  %55 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 1
  %56 = load ptr, ptr %55, align 8, !tbaa !127
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
  br i1 %64, label %68, label %57, !llvm.loop !130

65:                                               ; preds = %57, %40
  %66 = phi i32 [ 0, %40 ], [ %58, %57 ]
  %67 = icmp eq i32 %66, %52
  br i1 %67, label %68, label %110

68:                                               ; preds = %62, %65
  %69 = phi i32 [ %66, %65 ], [ %52, %62 ]
  %70 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 5
  %71 = load i32, ptr %70, align 8, !tbaa !131
  %72 = icmp eq i32 %52, %71
  br i1 %72, label %73, label %107

73:                                               ; preds = %68
  %74 = shl i32 %52, 1
  %75 = select i1 %53, i32 4, i32 %74
  %76 = shl i32 %75, 2
  %77 = tail call ptr @fa_alloc(i32 noundef %76, i32 noundef 4) #41
  %78 = add i32 %75, 63
  %79 = lshr i32 %78, 3
  %80 = and i32 %79, 536870904
  %81 = tail call ptr @fa_alloc(i32 noundef %80, i32 noundef 8) #41
  tail call void @llvm.memset.p0.i32(ptr align 4 %77, i8 0, i32 %76, i1 false)
  tail call void @llvm.memset.p0.i32(ptr align 8 %81, i8 0, i32 %80, i1 false)
  %82 = load i32, ptr %70, align 8, !tbaa !131
  %83 = icmp eq i32 %82, 0
  br i1 %83, label %103, label %84

84:                                               ; preds = %73
  %85 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 1
  %86 = load ptr, ptr %85, align 8, !tbaa !127
  %87 = load i32, ptr %51, align 4, !tbaa !124
  %88 = shl i32 %87, 2
  tail call fastcc void @copy_bytes(ptr noundef %77, ptr noundef %86, i32 noundef %88) #42
  %89 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 2
  %90 = load ptr, ptr %89, align 4, !tbaa !125
  %91 = load i32, ptr %70, align 8, !tbaa !131
  %92 = add i32 %91, 63
  %93 = lshr i32 %92, 3
  %94 = and i32 %93, 536870904
  tail call fastcc void @copy_bytes(ptr noundef %81, ptr noundef %90, i32 noundef %94) #42
  %95 = load ptr, ptr %85, align 8, !tbaa !127
  %96 = load i32, ptr %70, align 8, !tbaa !131
  %97 = shl i32 %96, 2
  tail call void @fa_free(ptr noundef %95, i32 noundef %97, i32 noundef 4) #41
  %98 = load ptr, ptr %89, align 4, !tbaa !125
  %99 = load i32, ptr %70, align 8, !tbaa !131
  %100 = add i32 %99, 63
  %101 = lshr i32 %100, 3
  %102 = and i32 %101, 536870904
  tail call void @fa_free(ptr noundef %98, i32 noundef %102, i32 noundef 8) #41
  br label %103

103:                                              ; preds = %84, %73
  %104 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 1
  store ptr %77, ptr %104, align 8, !tbaa !127
  %105 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 2
  store ptr %81, ptr %105, align 4, !tbaa !125
  store i32 %75, ptr %70, align 8, !tbaa !131
  %106 = load i32, ptr %51, align 4, !tbaa !124
  br label %107

107:                                              ; preds = %103, %68
  %108 = phi i32 [ %106, %103 ], [ %52, %68 ]
  %109 = add i32 %108, 1
  store i32 %109, ptr %51, align 4, !tbaa !124
  br label %110

110:                                              ; preds = %107, %65
  %111 = phi i32 [ %69, %107 ], [ %66, %65 ]
  %112 = getelementptr inbounds %struct.PoolChunk, ptr %41, i32 0, i32 1
  store i32 %111, ptr %112, align 4, !tbaa !12
  %113 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 1
  %114 = load ptr, ptr %113, align 8, !tbaa !127
  %115 = getelementptr inbounds ptr, ptr %114, i32 %111
  store ptr %41, ptr %115, align 4, !tbaa !6
  %116 = and i32 %111, 63
  %117 = zext nneg i32 %116 to i64
  %118 = shl nuw i64 1, %117
  %119 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 2
  %120 = load ptr, ptr %119, align 4, !tbaa !125
  %121 = lshr i32 %111, 6
  %122 = getelementptr inbounds i64, ptr %120, i32 %121
  %123 = load i64, ptr %122, align 8, !tbaa !80
  %124 = or i64 %123, %118
  store i64 %124, ptr %122, align 8, !tbaa !80
  ret ptr %41
}

; Function Attrs: nofree norecurse nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden void @fa_pool_remove(ptr nocapture noundef %0, ptr noundef %1) local_unnamed_addr #23 {
  %3 = ptrtoint ptr %1 to i32
  %4 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 8
  %5 = load i32, ptr %4, align 4, !tbaa !122
  %6 = mul i32 %5, -65536
  %7 = and i32 %6, %3
  %8 = inttoptr i32 %7 to ptr
  %9 = load ptr, ptr %8, align 65536, !tbaa !6
  %10 = ptrtoint ptr %9 to i32
  %11 = sub i32 %3, %10
  %12 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 6
  %13 = load i32, ptr %12, align 4, !tbaa !118
  %14 = udiv i32 %11, %13
  %15 = and i32 %14, 63
  %16 = zext nneg i32 %15 to i64
  %17 = shl nuw i64 1, %16
  %18 = xor i64 %17, -1
  %19 = lshr i32 %14, 6
  %20 = getelementptr inbounds %struct.PoolChunk, ptr %8, i32 0, i32 5, i32 %19
  %21 = load i64, ptr %20, align 8, !tbaa !80
  %22 = and i64 %21, %18
  store i64 %22, ptr %20, align 8, !tbaa !80
  %23 = getelementptr inbounds %struct.PoolChunk, ptr %8, i32 0, i32 4
  %24 = load i32, ptr %23, align 16
  store i32 %24, ptr %1, align 1
  %25 = add i32 %14, 1
  store i32 %25, ptr %23, align 16, !tbaa !12
  %26 = getelementptr inbounds %struct.PoolChunk, ptr %8, i32 0, i32 2
  %27 = load i32, ptr %26, align 8, !tbaa !12
  %28 = add i32 %27, -1
  store i32 %28, ptr %26, align 8, !tbaa !12
  %29 = load i64, ptr %0, align 8, !tbaa !128
  %30 = add nsw i64 %29, -1
  store i64 %30, ptr %0, align 8, !tbaa !128
  %31 = getelementptr inbounds %struct.PoolChunk, ptr %8, i32 0, i32 1
  %32 = load i32, ptr %31, align 4, !tbaa !12
  %33 = icmp eq i32 %28, 0
  br i1 %33, label %34, label %69

34:                                               ; preds = %2
  %35 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 1
  %36 = load ptr, ptr %35, align 8, !tbaa !127
  %37 = getelementptr inbounds ptr, ptr %36, i32 %32
  store ptr null, ptr %37, align 4, !tbaa !6
  %38 = and i32 %32, 63
  %39 = zext nneg i32 %38 to i64
  %40 = shl nuw i64 1, %39
  %41 = xor i64 %40, -1
  %42 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 2
  %43 = load ptr, ptr %42, align 4, !tbaa !125
  %44 = lshr i32 %32, 6
  %45 = getelementptr inbounds i64, ptr %43, i32 %44
  %46 = load i64, ptr %45, align 8, !tbaa !80
  %47 = and i64 %46, %41
  store i64 %47, ptr %45, align 8, !tbaa !80
  %48 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 4
  %49 = load i32, ptr %48, align 4, !tbaa !124
  %50 = icmp eq i32 %49, 0
  br i1 %50, label %61, label %51

51:                                               ; preds = %34
  %52 = load ptr, ptr %35, align 8, !tbaa !127
  br label %53

53:                                               ; preds = %51, %59
  %54 = phi i32 [ %49, %51 ], [ %55, %59 ]
  %55 = add i32 %54, -1
  %56 = getelementptr inbounds ptr, ptr %52, i32 %55
  %57 = load ptr, ptr %56, align 4, !tbaa !6
  %58 = icmp eq ptr %57, null
  br i1 %58, label %59, label %61

59:                                               ; preds = %53
  store i32 %55, ptr %48, align 4, !tbaa !124
  %60 = icmp eq i32 %55, 0
  br i1 %60, label %61, label %53, !llvm.loop !132

61:                                               ; preds = %53, %59, %34
  %62 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 3
  %63 = load ptr, ptr %62, align 8, !tbaa !129
  %64 = icmp eq ptr %63, null
  br i1 %64, label %65, label %66

65:                                               ; preds = %61
  store ptr %8, ptr %62, align 8, !tbaa !129
  br label %79

66:                                               ; preds = %61
  %67 = load i32, ptr %4, align 4, !tbaa !122
  tail call fastcc void @give_pages(ptr noundef nonnull %8, i32 noundef %67) #42
  %68 = atomicrmw sub ptr @fa_live_blocks, i32 1 monotonic, align 4
  br label %79

69:                                               ; preds = %2
  %70 = and i32 %32, 63
  %71 = zext nneg i32 %70 to i64
  %72 = shl nuw i64 1, %71
  %73 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 2
  %74 = load ptr, ptr %73, align 4, !tbaa !125
  %75 = lshr i32 %32, 6
  %76 = getelementptr inbounds i64, ptr %74, i32 %75
  %77 = load i64, ptr %76, align 8, !tbaa !80
  %78 = or i64 %77, %72
  store i64 %78, ptr %76, align 8, !tbaa !80
  br label %79

79:                                               ; preds = %65, %66, %69
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(read, inaccessiblemem: none)
define hidden i64 @fa_pool_seek(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #24 {
  %3 = lshr i64 %1, 32
  %4 = trunc i64 %3 to i32
  %5 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 4
  %6 = load i32, ptr %5, align 4, !tbaa !124
  %7 = icmp ugt i32 %6, %4
  br i1 %7, label %8, label %48

8:                                                ; preds = %2
  %9 = trunc i64 %1 to i32
  %10 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 1
  %11 = load ptr, ptr %10, align 8, !tbaa !127
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
  %20 = load i32, ptr %19, align 4, !tbaa !12
  %21 = icmp ult i32 %14, %20
  br i1 %21, label %22, label %45

22:                                               ; preds = %18, %36
  %23 = phi i32 [ %38, %36 ], [ %14, %18 ]
  %24 = lshr i32 %23, 6
  %25 = getelementptr inbounds %struct.PoolChunk, ptr %16, i32 0, i32 5, i32 %24
  %26 = load i64, ptr %25, align 8, !tbaa !80
  %27 = and i32 %23, 63
  %28 = zext nneg i32 %27 to i64
  %29 = lshr i64 %26, %28
  %30 = icmp eq i64 %29, 0
  br i1 %30, label %36, label %31

31:                                               ; preds = %22
  %32 = tail call i64 @llvm.cttz.i64(i64 %29, i1 true), !range !101
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
  br i1 %47, label %48, label %12, !llvm.loop !133

48:                                               ; preds = %45, %2, %40
  %49 = phi i64 [ %44, %40 ], [ -1, %2 ], [ -1, %45 ]
  ret i64 %49
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(read, inaccessiblemem: none)
define hidden ptr @fa_pool_at(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #25 {
  %3 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 1
  %4 = load ptr, ptr %3, align 8, !tbaa !127
  %5 = lshr i64 %1, 32
  %6 = trunc i64 %5 to i32
  %7 = getelementptr inbounds ptr, ptr %4, i32 %6
  %8 = load ptr, ptr %7, align 4, !tbaa !6
  %9 = load ptr, ptr %8, align 8, !tbaa !6
  %10 = trunc i64 %1 to i32
  %11 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 6
  %12 = load i32, ptr %11, align 4, !tbaa !118
  %13 = mul i32 %12, %10
  %14 = getelementptr inbounds i8, ptr %9, i32 %13
  ret ptr %14
}

; Function Attrs: nofree norecurse nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden void @fa_pool_clear(ptr nocapture noundef %0) local_unnamed_addr #23 {
  tail call fastcc void @pool_release_chunks(ptr noundef %0) #42
  ret void
}

; Function Attrs: nofree norecurse nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc void @pool_release_chunks(ptr nocapture noundef %0) unnamed_addr #23 {
  %2 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 4
  %3 = load i32, ptr %2, align 4, !tbaa !124
  %4 = icmp eq i32 %3, 0
  br i1 %4, label %8, label %5

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 1
  %7 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 8
  br label %12

8:                                                ; preds = %23, %1
  %9 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 5
  %10 = load i32, ptr %9, align 8, !tbaa !131
  %11 = icmp eq i32 %10, 0
  br i1 %11, label %37, label %27

12:                                               ; preds = %5, %23
  %13 = phi i32 [ %3, %5 ], [ %24, %23 ]
  %14 = phi i32 [ 0, %5 ], [ %25, %23 ]
  %15 = load ptr, ptr %6, align 8, !tbaa !127
  %16 = getelementptr inbounds ptr, ptr %15, i32 %14
  %17 = load ptr, ptr %16, align 4, !tbaa !6
  %18 = icmp eq ptr %17, null
  br i1 %18, label %23, label %19

19:                                               ; preds = %12
  %20 = load i32, ptr %7, align 4, !tbaa !122
  tail call fastcc void @give_pages(ptr noundef nonnull %17, i32 noundef %20) #42
  %21 = atomicrmw sub ptr @fa_live_blocks, i32 1 monotonic, align 4
  %22 = load i32, ptr %2, align 4, !tbaa !124
  br label %23

23:                                               ; preds = %12, %19
  %24 = phi i32 [ %13, %12 ], [ %22, %19 ]
  %25 = add nuw i32 %14, 1
  %26 = icmp ult i32 %25, %24
  br i1 %26, label %12, label %8, !llvm.loop !134

27:                                               ; preds = %8
  %28 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 1
  %29 = load ptr, ptr %28, align 8, !tbaa !127
  %30 = shl i32 %10, 2
  tail call void @llvm.memset.p0.i32(ptr align 4 %29, i8 0, i32 %30, i1 false)
  %31 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 2
  %32 = load ptr, ptr %31, align 4, !tbaa !125
  %33 = load i32, ptr %9, align 8, !tbaa !131
  %34 = add i32 %33, 63
  %35 = lshr i32 %34, 3
  %36 = and i32 %35, 536870904
  tail call void @llvm.memset.p0.i32(ptr align 8 %32, i8 0, i32 %36, i1 false)
  br label %37

37:                                               ; preds = %27, %8
  store i32 0, ptr %2, align 4, !tbaa !124
  store i64 0, ptr %0, align 8, !tbaa !128
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_pool_free(ptr noundef %0) local_unnamed_addr #1 {
  tail call fastcc void @pool_release_chunks(ptr noundef %0) #42
  %2 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 3
  %3 = load ptr, ptr %2, align 8, !tbaa !129
  %4 = icmp eq ptr %3, null
  br i1 %4, label %9, label %5

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 8
  %7 = load i32, ptr %6, align 4, !tbaa !122
  tail call fastcc void @give_pages(ptr noundef nonnull %3, i32 noundef %7) #42
  %8 = atomicrmw sub ptr @fa_live_blocks, i32 1 monotonic, align 4
  br label %9

9:                                                ; preds = %5, %1
  %10 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 5
  %11 = load i32, ptr %10, align 8, !tbaa !131
  %12 = icmp eq i32 %11, 0
  br i1 %12, label %23, label %13

13:                                               ; preds = %9
  %14 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 1
  %15 = load ptr, ptr %14, align 8, !tbaa !127
  %16 = shl i32 %11, 2
  tail call void @fa_free(ptr noundef %15, i32 noundef %16, i32 noundef 4) #41
  %17 = getelementptr inbounds %struct.FaPool, ptr %0, i32 0, i32 2
  %18 = load ptr, ptr %17, align 4, !tbaa !125
  %19 = load i32, ptr %10, align 8, !tbaa !131
  %20 = add i32 %19, 63
  %21 = lshr i32 %20, 3
  %22 = and i32 %21, 536870904
  tail call void @fa_free(ptr noundef %18, i32 noundef %22, i32 noundef 8) #41
  br label %23

23:                                               ; preds = %13, %9
  tail call void @fa_free(ptr noundef nonnull %0, i32 noundef 48, i32 noundef 8) #41
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(read, inaccessiblemem: none)
define hidden i32 @fa_links_has(ptr noundef readonly %0, ptr noundef readonly %1) local_unnamed_addr #24 {
  %3 = tail call fastcc i64 @links_find(ptr noundef %0, ptr noundef %1) #42
  %4 = icmp sgt i64 %3, -1
  %5 = zext i1 %4 to i32
  ret i32 %5
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(read, inaccessiblemem: none)
define internal fastcc i64 @links_find(ptr noundef readonly %0, ptr noundef %1) unnamed_addr #24 {
  %3 = icmp eq ptr %0, null
  br i1 %3, label %57, label %4

4:                                                ; preds = %2
  %5 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 4
  %6 = load ptr, ptr %5, align 8, !tbaa !135
  %7 = icmp eq ptr %6, null
  br i1 %7, label %8, label %24

8:                                                ; preds = %4
  %9 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 2
  %10 = load i64, ptr %9, align 8, !tbaa !137
  %11 = icmp sgt i64 %10, 0
  br i1 %11, label %12, label %57

12:                                               ; preds = %8
  %13 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 1
  %14 = load ptr, ptr %13, align 8, !tbaa !138
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
  br i1 %23, label %57, label %15, !llvm.loop !139

24:                                               ; preds = %4
  %25 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 5
  %26 = load i64, ptr %25, align 8, !tbaa !140
  %27 = ptrtoint ptr %1 to i32
  %28 = zext i32 %27 to i64
  %29 = mul i64 %28, -49064778989728563
  %30 = lshr i64 %29, 33
  %31 = xor i64 %30, %29
  %32 = and i64 %31, %26
  %33 = trunc i64 %32 to i32
  %34 = getelementptr inbounds i32, ptr %6, i32 %33
  %35 = load i32, ptr %34, align 4, !tbaa !12
  %36 = icmp eq i32 %35, 0
  br i1 %36, label %57, label %37

37:                                               ; preds = %24
  %38 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 1
  %39 = load ptr, ptr %38, align 8, !tbaa !138
  %40 = getelementptr ptr, ptr %39, i32 -1
  br label %48

41:                                               ; preds = %48
  %42 = add i64 %50, 1
  %43 = and i64 %42, %26
  %44 = trunc i64 %43 to i32
  %45 = getelementptr inbounds i32, ptr %6, i32 %44
  %46 = load i32, ptr %45, align 4, !tbaa !12
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
define hidden noundef i32 @fa_links_add(ptr nocapture noundef %0, ptr noundef %1) local_unnamed_addr #1 {
  %3 = load ptr, ptr %0, align 4, !tbaa !6
  %4 = icmp eq ptr %3, null
  br i1 %4, label %5, label %7

5:                                                ; preds = %2
  %6 = tail call ptr @fa_alloc(i32 noundef 48, i32 noundef 8) #41
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(48) %6, i8 0, i32 48, i1 false)
  store ptr %6, ptr %0, align 4, !tbaa !6
  br label %10

7:                                                ; preds = %2
  %8 = tail call fastcc i64 @links_find(ptr noundef nonnull %3, ptr noundef %1) #42
  %9 = icmp sgt i64 %8, -1
  br i1 %9, label %157, label %10

10:                                               ; preds = %7, %5
  %11 = phi ptr [ %6, %5 ], [ %3, %7 ]
  %12 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 2
  %13 = load i64, ptr %12, align 8, !tbaa !137
  %14 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 3
  %15 = load i64, ptr %14, align 8, !tbaa !141
  %16 = icmp eq i64 %13, %15
  br i1 %16, label %17, label %124

17:                                               ; preds = %10
  %18 = icmp sgt i64 %13, 0
  br i1 %18, label %19, label %42

19:                                               ; preds = %17
  %20 = load i64, ptr %11, align 8, !tbaa !142
  %21 = shl nsw i64 %20, 1
  %22 = icmp sgt i64 %21, %13
  br i1 %22, label %42, label %23

23:                                               ; preds = %19
  %24 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 1
  br label %26

25:                                               ; preds = %38
  store i64 %39, ptr %12, align 8, !tbaa !137
  br label %63

26:                                               ; preds = %23, %38
  %27 = phi i64 [ 0, %23 ], [ %40, %38 ]
  %28 = phi i64 [ 0, %23 ], [ %39, %38 ]
  %29 = load ptr, ptr %24, align 8, !tbaa !138
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
  br i1 %41, label %25, label %26, !llvm.loop !143

42:                                               ; preds = %19, %17
  %43 = icmp eq i64 %13, 0
  %44 = shl nsw i64 %13, 1
  %45 = select i1 %43, i64 4, i64 %44
  %46 = trunc i64 %45 to i32
  %47 = shl i32 %46, 2
  %48 = tail call ptr @fa_alloc(i32 noundef %47, i32 noundef 4) #41
  %49 = load i64, ptr %14, align 8, !tbaa !141
  %50 = icmp sgt i64 %49, 0
  br i1 %50, label %51, label %61

51:                                               ; preds = %42
  %52 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 1
  %53 = load ptr, ptr %52, align 8, !tbaa !138
  %54 = load i64, ptr %12, align 8, !tbaa !137
  %55 = trunc i64 %54 to i32
  %56 = shl i32 %55, 2
  tail call fastcc void @copy_bytes(ptr noundef %48, ptr noundef %53, i32 noundef %56) #42
  %57 = load ptr, ptr %52, align 8, !tbaa !138
  %58 = load i64, ptr %14, align 8, !tbaa !141
  %59 = trunc i64 %58 to i32
  %60 = shl i32 %59, 2
  tail call void @fa_free(ptr noundef %57, i32 noundef %60, i32 noundef 4) #41
  br label %61

61:                                               ; preds = %51, %42
  %62 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 1
  store ptr %48, ptr %62, align 8, !tbaa !138
  store i64 %45, ptr %14, align 8, !tbaa !141
  br label %63

63:                                               ; preds = %61, %25
  %64 = phi i64 [ %45, %61 ], [ %13, %25 ]
  %65 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 4
  %66 = load ptr, ptr %65, align 8, !tbaa !135
  %67 = icmp eq ptr %66, null
  br i1 %67, label %75, label %68

68:                                               ; preds = %63
  %69 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 5
  %70 = load i64, ptr %69, align 8, !tbaa !140
  %71 = trunc i64 %70 to i32
  %72 = shl i32 %71, 2
  %73 = add i32 %72, 4
  tail call void @fa_free(ptr noundef nonnull %66, i32 noundef %73, i32 noundef 4) #41
  store ptr null, ptr %65, align 8, !tbaa !135
  %74 = load i64, ptr %14, align 8, !tbaa !141
  br label %75

75:                                               ; preds = %68, %63
  %76 = phi i64 [ %74, %68 ], [ %64, %63 ]
  %77 = icmp slt i64 %76, 9
  br i1 %77, label %78, label %80

78:                                               ; preds = %75
  %79 = load i64, ptr %12, align 8, !tbaa !137
  br label %124

80:                                               ; preds = %75
  %81 = shl nuw nsw i64 %76, 1
  br label %82

82:                                               ; preds = %82, %80
  %83 = phi i64 [ %85, %82 ], [ 16, %80 ]
  %84 = icmp slt i64 %83, %81
  %85 = shl nsw i64 %83, 1
  br i1 %84, label %82, label %86, !llvm.loop !144

86:                                               ; preds = %82
  %87 = add nsw i64 %83, -1
  %88 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 5
  store i64 %87, ptr %88, align 8, !tbaa !140
  %89 = trunc i64 %83 to i32
  %90 = shl i32 %89, 2
  %91 = tail call ptr @fa_alloc(i32 noundef %90, i32 noundef 4) #41
  store ptr %91, ptr %65, align 8, !tbaa !135
  tail call void @llvm.memset.p0.i32(ptr align 4 %91, i8 0, i32 %90, i1 false)
  %92 = load i64, ptr %12, align 8, !tbaa !137
  %93 = icmp sgt i64 %92, 0
  br i1 %93, label %94, label %124

94:                                               ; preds = %86
  %95 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 1
  %96 = load ptr, ptr %95, align 8, !tbaa !138
  br label %97

97:                                               ; preds = %121, %94
  %98 = phi i64 [ 0, %94 ], [ %122, %121 ]
  %99 = trunc i64 %98 to i32
  %100 = getelementptr inbounds ptr, ptr %96, i32 %99
  %101 = load ptr, ptr %100, align 4, !tbaa !6
  %102 = icmp eq ptr %101, null
  br i1 %102, label %121, label %103

103:                                              ; preds = %97
  %104 = load i64, ptr %88, align 8, !tbaa !140
  %105 = ptrtoint ptr %101 to i32
  %106 = zext i32 %105 to i64
  %107 = mul i64 %106, -49064778989728563
  %108 = lshr i64 %107, 33
  %109 = xor i64 %108, %107
  %110 = load ptr, ptr %65, align 8, !tbaa !135
  br label %111

111:                                              ; preds = %111, %103
  %112 = phi i64 [ %109, %103 ], [ %118, %111 ]
  %113 = and i64 %112, %104
  %114 = trunc i64 %113 to i32
  %115 = getelementptr inbounds i32, ptr %110, i32 %114
  %116 = load i32, ptr %115, align 4, !tbaa !12
  %117 = icmp eq i32 %116, 0
  %118 = add i64 %113, 1
  br i1 %117, label %119, label %111, !llvm.loop !145

119:                                              ; preds = %111
  %120 = add i32 %99, 1
  store i32 %120, ptr %115, align 4, !tbaa !12
  br label %121

121:                                              ; preds = %119, %97
  %122 = add nuw nsw i64 %98, 1
  %123 = icmp eq i64 %122, %92
  br i1 %123, label %124, label %97, !llvm.loop !146

124:                                              ; preds = %121, %78, %86, %10
  %125 = phi i64 [ %79, %78 ], [ %92, %86 ], [ %13, %10 ], [ %92, %121 ]
  %126 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 1
  %127 = load ptr, ptr %126, align 8, !tbaa !138
  %128 = trunc i64 %125 to i32
  %129 = getelementptr inbounds ptr, ptr %127, i32 %128
  store ptr %1, ptr %129, align 4, !tbaa !6
  %130 = add nsw i64 %125, 1
  store i64 %130, ptr %12, align 8, !tbaa !137
  %131 = load i64, ptr %11, align 8, !tbaa !142
  %132 = add nsw i64 %131, 1
  store i64 %132, ptr %11, align 8, !tbaa !142
  %133 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 4
  %134 = load ptr, ptr %133, align 8, !tbaa !135
  %135 = icmp eq ptr %134, null
  br i1 %135, label %157, label %136

136:                                              ; preds = %124
  %137 = getelementptr inbounds %struct.FaLinks, ptr %11, i32 0, i32 5
  %138 = load i64, ptr %137, align 8, !tbaa !140
  %139 = load ptr, ptr %126, align 8, !tbaa !138
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
  %152 = load i32, ptr %151, align 4, !tbaa !12
  %153 = icmp eq i32 %152, 0
  %154 = add i64 %149, 1
  br i1 %153, label %155, label %147, !llvm.loop !145

155:                                              ; preds = %147
  %156 = add i32 %128, 1
  store i32 %156, ptr %151, align 4, !tbaa !12
  br label %157

157:                                              ; preds = %124, %155, %7
  %158 = phi i32 [ 0, %7 ], [ 1, %155 ], [ 1, %124 ]
  ret i32 %158
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden noundef i32 @fa_links_delete(ptr noundef %0, ptr noundef %1) local_unnamed_addr #12 {
  %3 = tail call fastcc i64 @links_find(ptr noundef %0, ptr noundef %1) #42
  %4 = icmp slt i64 %3, 0
  br i1 %4, label %78, label %5

5:                                                ; preds = %2
  %6 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 1
  %7 = load ptr, ptr %6, align 8, !tbaa !138
  %8 = trunc i64 %3 to i32
  %9 = getelementptr inbounds ptr, ptr %7, i32 %8
  store ptr null, ptr %9, align 4, !tbaa !6
  %10 = load i64, ptr %0, align 8, !tbaa !142
  %11 = add nsw i64 %10, -1
  store i64 %11, ptr %0, align 8, !tbaa !142
  %12 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 4
  %13 = load ptr, ptr %12, align 8, !tbaa !135
  %14 = icmp eq ptr %13, null
  br i1 %14, label %74, label %15

15:                                               ; preds = %5
  %16 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 5
  %17 = load i64, ptr %16, align 8, !tbaa !140
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
  %29 = load i32, ptr %28, align 4, !tbaa !12
  %30 = icmp eq i32 %29, %23
  %31 = add i64 %26, 1
  br i1 %30, label %32, label %24, !llvm.loop !147

32:                                               ; preds = %24
  %33 = and i64 %31, %17
  %34 = trunc i64 %33 to i32
  %35 = getelementptr inbounds i32, ptr %13, i32 %34
  %36 = load i32, ptr %35, align 4, !tbaa !12
  %37 = icmp eq i32 %36, 0
  br i1 %37, label %71, label %38

38:                                               ; preds = %32
  %39 = load ptr, ptr %6, align 8, !tbaa !138
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
  store i32 %42, ptr %60, align 4, !tbaa !12
  br label %61

61:                                               ; preds = %41, %58
  %62 = phi i64 [ %43, %58 ], [ %44, %41 ]
  %63 = add i64 %43, 1
  %64 = and i64 %63, %17
  %65 = trunc i64 %64 to i32
  %66 = getelementptr inbounds i32, ptr %13, i32 %65
  %67 = load i32, ptr %66, align 4, !tbaa !12
  %68 = icmp eq i32 %67, 0
  br i1 %68, label %69, label %41

69:                                               ; preds = %61
  %70 = trunc i64 %62 to i32
  br label %71

71:                                               ; preds = %69, %32
  %72 = phi i32 [ %70, %69 ], [ %27, %32 ]
  %73 = getelementptr inbounds i32, ptr %13, i32 %72
  store i32 0, ptr %73, align 4, !tbaa !12
  br label %74

74:                                               ; preds = %71, %5
  %75 = icmp eq i64 %11, 0
  br i1 %75, label %76, label %78

76:                                               ; preds = %74
  %77 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 2
  store i64 0, ptr %77, align 8, !tbaa !137
  br label %78

78:                                               ; preds = %74, %76, %2
  %79 = phi i32 [ 0, %2 ], [ 1, %76 ], [ 1, %74 ]
  ret i32 %79
}

; Function Attrs: nounwind optsize
define hidden void @fa_links_free(ptr noundef %0) local_unnamed_addr #1 {
  %2 = icmp eq ptr %0, null
  br i1 %2, label %23, label %3

3:                                                ; preds = %1
  %4 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 4
  %5 = load ptr, ptr %4, align 8, !tbaa !135
  %6 = icmp eq ptr %5, null
  br i1 %6, label %13, label %7

7:                                                ; preds = %3
  %8 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 5
  %9 = load i64, ptr %8, align 8, !tbaa !140
  %10 = trunc i64 %9 to i32
  %11 = shl i32 %10, 2
  %12 = add i32 %11, 4
  tail call void @fa_free(ptr noundef nonnull %5, i32 noundef %12, i32 noundef 4) #41
  br label %13

13:                                               ; preds = %7, %3
  %14 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 3
  %15 = load i64, ptr %14, align 8, !tbaa !141
  %16 = icmp sgt i64 %15, 0
  br i1 %16, label %17, label %22

17:                                               ; preds = %13
  %18 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 1
  %19 = load ptr, ptr %18, align 8, !tbaa !138
  %20 = trunc i64 %15 to i32
  %21 = shl i32 %20, 2
  tail call void @fa_free(ptr noundef %19, i32 noundef %21, i32 noundef 4) #41
  br label %22

22:                                               ; preds = %17, %13
  tail call void @fa_free(ptr noundef nonnull %0, i32 noundef 48, i32 noundef 8) #41
  br label %23

23:                                               ; preds = %1, %22
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(read, inaccessiblemem: none)
define hidden i64 @fa_links_seek(ptr noundef readonly %0, i64 noundef %1) local_unnamed_addr #24 {
  %3 = icmp eq ptr %0, null
  br i1 %3, label %20, label %4

4:                                                ; preds = %2
  %5 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 2
  %6 = load i64, ptr %5, align 8, !tbaa !137
  %7 = icmp ugt i64 %6, %1
  br i1 %7, label %8, label %20

8:                                                ; preds = %4
  %9 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 1
  %10 = load ptr, ptr %9, align 8, !tbaa !138
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
  br i1 %19, label %20, label %11, !llvm.loop !148

20:                                               ; preds = %11, %17, %4, %2
  %21 = phi i64 [ -1, %2 ], [ -1, %4 ], [ %12, %11 ], [ -1, %17 ]
  ret i64 %21
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(read, inaccessiblemem: none)
define hidden ptr @fa_links_at(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #25 {
  %3 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 1
  %4 = load ptr, ptr %3, align 8, !tbaa !138
  %5 = trunc i64 %1 to i32
  %6 = getelementptr inbounds ptr, ptr %4, i32 %5
  %7 = load ptr, ptr %6, align 4, !tbaa !6
  ret ptr %7
}

; Function Attrs: nounwind optsize
define hidden void @fa_relate_one(ptr noundef %0, i64 noundef %1, i64 noundef %2, ptr noundef %3, i32 noundef %4) local_unnamed_addr #1 {
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
  %18 = tail call i32 @fa_links_delete(ptr noundef %17, ptr noundef nonnull %0) #42, !range !39
  br label %19

19:                                               ; preds = %14, %13
  store ptr %3, ptr %7, align 4, !tbaa !6
  %20 = icmp eq ptr %3, null
  br i1 %20, label %40, label %21

21:                                               ; preds = %19
  %22 = trunc i64 %2 to i32
  %23 = getelementptr inbounds i8, ptr %3, i32 %22
  %24 = tail call i32 @fa_links_add(ptr noundef nonnull %23, ptr noundef nonnull %0) #42, !range !39
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
define hidden noundef i32 @fa_relate_add(ptr noundef %0, i64 noundef %1, i64 noundef %2, ptr noundef %3) local_unnamed_addr #1 {
  %5 = trunc i64 %1 to i32
  %6 = getelementptr inbounds i8, ptr %0, i32 %5
  %7 = tail call i32 @fa_links_add(ptr noundef %6, ptr noundef %3) #42, !range !39
  %8 = icmp eq i32 %7, 0
  br i1 %8, label %13, label %9

9:                                                ; preds = %4
  %10 = trunc i64 %2 to i32
  %11 = getelementptr inbounds i8, ptr %3, i32 %10
  %12 = tail call i32 @fa_links_add(ptr noundef %11, ptr noundef %0) #42, !range !39
  br label %13

13:                                               ; preds = %4, %9
  %14 = phi i32 [ 1, %9 ], [ 0, %4 ]
  ret i32 %14
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden noundef i32 @fa_relate_delete(ptr noundef %0, i64 noundef %1, i64 noundef %2, ptr noundef %3) local_unnamed_addr #12 {
  %5 = trunc i64 %1 to i32
  %6 = getelementptr inbounds i8, ptr %0, i32 %5
  %7 = load ptr, ptr %6, align 4, !tbaa !6
  %8 = tail call i32 @fa_links_delete(ptr noundef %7, ptr noundef %3) #42, !range !39
  %9 = icmp eq i32 %8, 0
  br i1 %9, label %15, label %10

10:                                               ; preds = %4
  %11 = trunc i64 %2 to i32
  %12 = getelementptr inbounds i8, ptr %3, i32 %11
  %13 = load ptr, ptr %12, align 4, !tbaa !6
  %14 = tail call i32 @fa_links_delete(ptr noundef %13, ptr noundef nonnull %0) #42, !range !39
  br label %15

15:                                               ; preds = %4, %10
  %16 = phi i32 [ 1, %10 ], [ 0, %4 ]
  ret i32 %16
}

; Function Attrs: nounwind optsize
define hidden void @fa_relate_clear(ptr noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #1 {
  %4 = trunc i64 %1 to i32
  %5 = getelementptr inbounds i8, ptr %0, i32 %4
  %6 = load ptr, ptr %5, align 4, !tbaa !6
  %7 = icmp eq ptr %6, null
  br i1 %7, label %33, label %8

8:                                                ; preds = %3
  %9 = getelementptr inbounds %struct.FaLinks, ptr %6, i32 0, i32 2
  %10 = load i64, ptr %9, align 8, !tbaa !137
  %11 = icmp sgt i64 %10, 0
  br i1 %11, label %12, label %15

12:                                               ; preds = %8
  %13 = getelementptr inbounds %struct.FaLinks, ptr %6, i32 0, i32 1
  %14 = trunc i64 %2 to i32
  br label %16

15:                                               ; preds = %29, %8
  tail call void @fa_links_free(ptr noundef nonnull %6) #42
  store ptr null, ptr %5, align 4, !tbaa !6
  br label %33

16:                                               ; preds = %12, %29
  %17 = phi i64 [ %10, %12 ], [ %30, %29 ]
  %18 = phi i64 [ 0, %12 ], [ %31, %29 ]
  %19 = load ptr, ptr %13, align 8, !tbaa !138
  %20 = trunc i64 %18 to i32
  %21 = getelementptr inbounds ptr, ptr %19, i32 %20
  %22 = load ptr, ptr %21, align 4, !tbaa !6
  %23 = icmp eq ptr %22, null
  br i1 %23, label %29, label %24

24:                                               ; preds = %16
  %25 = getelementptr inbounds i8, ptr %22, i32 %14
  %26 = load ptr, ptr %25, align 4, !tbaa !6
  %27 = tail call i32 @fa_links_delete(ptr noundef %26, ptr noundef %0) #42, !range !39
  %28 = load i64, ptr %9, align 8, !tbaa !137
  br label %29

29:                                               ; preds = %24, %16
  %30 = phi i64 [ %28, %24 ], [ %17, %16 ]
  %31 = add nuw nsw i64 %18, 1
  %32 = icmp slt i64 %31, %30
  br i1 %32, label %16, label %15, !llvm.loop !149

33:                                               ; preds = %3, %15
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden void @fa_unlink(ptr noundef %0, i32 noundef %1, i32 noundef %2, i64 noundef %3, i64 noundef %4, ptr noundef %5) local_unnamed_addr #12 {
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
  %19 = load i64, ptr %14, align 8, !tbaa !137
  %20 = icmp sgt i64 %19, 0
  br i1 %20, label %21, label %99

21:                                               ; preds = %16, %32
  %22 = phi i64 [ %33, %32 ], [ 0, %16 ]
  %23 = load ptr, ptr %17, align 8, !tbaa !138
  %24 = trunc i64 %22 to i32
  %25 = getelementptr inbounds ptr, ptr %23, i32 %24
  %26 = load ptr, ptr %25, align 4, !tbaa !6
  %27 = icmp eq ptr %26, null
  br i1 %27, label %32, label %28

28:                                               ; preds = %21
  %29 = getelementptr inbounds i8, ptr %26, i32 %18
  %30 = load ptr, ptr %29, align 4, !tbaa !6
  %31 = tail call i32 @fa_links_delete(ptr noundef %30, ptr noundef %0) #42, !range !39
  br label %32

32:                                               ; preds = %28, %21
  %33 = add nuw nsw i64 %22, 1
  %34 = load i64, ptr %14, align 8, !tbaa !137
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
  %44 = tail call i32 @fa_links_delete(ptr noundef %43, ptr noundef nonnull %0) #42, !range !39
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
  %57 = load i64, ptr %51, align 8, !tbaa !137
  %58 = icmp sgt i64 %57, 0
  br i1 %58, label %59, label %99

59:                                               ; preds = %53, %72
  %60 = phi i64 [ %73, %72 ], [ 0, %53 ]
  %61 = load ptr, ptr %54, align 8, !tbaa !138
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
  %71 = tail call i32 @fa_links_delete(ptr noundef %70, ptr noundef %0) #42, !range !39
  br label %72

72:                                               ; preds = %68, %69, %59
  %73 = add nuw nsw i64 %60, 1
  %74 = load i64, ptr %51, align 8, !tbaa !137
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
define hidden ptr @fa_address_map_new(i64 noundef %0) local_unnamed_addr #1 {
  %2 = tail call ptr @fa_alloc(i32 noundef 16, i32 noundef 8) #41
  %3 = shl nsw i64 %0, 1
  br label %4

4:                                                ; preds = %4, %1
  %5 = phi i64 [ 16, %1 ], [ %7, %4 ]
  %6 = icmp slt i64 %5, %3
  %7 = shl nsw i64 %5, 1
  br i1 %6, label %4, label %8, !llvm.loop !150

8:                                                ; preds = %4
  %9 = add nsw i64 %5, -1
  %10 = getelementptr inbounds %struct.FaAddressMap, ptr %2, i32 0, i32 2
  store i64 %9, ptr %10, align 8, !tbaa !151
  %11 = trunc i64 %5 to i32
  %12 = shl i32 %11, 2
  %13 = tail call ptr @fa_alloc(i32 noundef %12, i32 noundef 4) #41
  store ptr %13, ptr %2, align 8, !tbaa !153
  %14 = tail call ptr @fa_alloc(i32 noundef %12, i32 noundef 4) #41
  %15 = getelementptr inbounds %struct.FaAddressMap, ptr %2, i32 0, i32 1
  store ptr %14, ptr %15, align 4, !tbaa !154
  %16 = load ptr, ptr %2, align 8, !tbaa !153
  tail call void @llvm.memset.p0.i32(ptr align 4 %16, i8 0, i32 %12, i1 false)
  ret ptr %2
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden void @fa_address_map_put(ptr nocapture noundef readonly %0, ptr noundef %1, ptr noundef %2) local_unnamed_addr #12 {
  %4 = getelementptr inbounds %struct.FaAddressMap, ptr %0, i32 0, i32 2
  %5 = load i64, ptr %4, align 8, !tbaa !151
  %6 = ptrtoint ptr %1 to i32
  %7 = zext i32 %6 to i64
  %8 = mul i64 %7, -49064778989728563
  %9 = lshr i64 %8, 33
  %10 = xor i64 %9, %8
  %11 = load ptr, ptr %0, align 8, !tbaa !153
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
  br i1 %20, label %22, label %12, !llvm.loop !155

22:                                               ; preds = %12
  store ptr %1, ptr %16, align 4, !tbaa !6
  %23 = getelementptr inbounds %struct.FaAddressMap, ptr %0, i32 0, i32 1
  %24 = load ptr, ptr %23, align 4, !tbaa !154
  %25 = getelementptr inbounds ptr, ptr %24, i32 %15
  store ptr %2, ptr %25, align 4, !tbaa !6
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(read, inaccessiblemem: none)
define hidden ptr @fa_address_map_get(ptr nocapture noundef readonly %0, ptr noundef %1) local_unnamed_addr #24 {
  %3 = icmp eq ptr %1, null
  br i1 %3, label %36, label %4

4:                                                ; preds = %2
  %5 = getelementptr inbounds %struct.FaAddressMap, ptr %0, i32 0, i32 2
  %6 = load i64, ptr %5, align 8, !tbaa !151
  %7 = ptrtoint ptr %1 to i32
  %8 = zext i32 %7 to i64
  %9 = mul i64 %8, -49064778989728563
  %10 = lshr i64 %9, 33
  %11 = xor i64 %10, %9
  %12 = load ptr, ptr %0, align 8, !tbaa !153
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
  br i1 %27, label %36, label %28, !llvm.loop !156

28:                                               ; preds = %20
  %29 = icmp eq ptr %26, %1
  br i1 %29, label %30, label %20, !llvm.loop !156

30:                                               ; preds = %28, %18
  %31 = phi i32 [ %14, %18 ], [ %24, %28 ]
  %32 = getelementptr inbounds %struct.FaAddressMap, ptr %0, i32 0, i32 1
  %33 = load ptr, ptr %32, align 4, !tbaa !154
  %34 = getelementptr inbounds ptr, ptr %33, i32 %31
  %35 = load ptr, ptr %34, align 4, !tbaa !6
  br label %36

36:                                               ; preds = %20, %4, %30, %2
  %37 = phi ptr [ null, %2 ], [ %35, %30 ], [ null, %4 ], [ null, %20 ]
  ret ptr %37
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_links_relink(ptr noundef readonly %0, ptr nocapture noundef readonly %1) local_unnamed_addr #1 {
  %3 = alloca ptr, align 4
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %3) #44
  store ptr null, ptr %3, align 4, !tbaa !6
  %4 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 2
  %5 = icmp eq ptr %0, null
  br i1 %5, label %12, label %6

6:                                                ; preds = %2
  %7 = getelementptr inbounds %struct.FaLinks, ptr %0, i32 0, i32 1
  %8 = load i64, ptr %4, align 8, !tbaa !137
  %9 = icmp sgt i64 %8, 0
  br i1 %9, label %14, label %10

10:                                               ; preds = %24, %6
  %11 = load ptr, ptr %3, align 4, !tbaa !6
  br label %12

12:                                               ; preds = %10, %2
  %13 = phi ptr [ %11, %10 ], [ null, %2 ]
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %3) #44
  ret ptr %13

14:                                               ; preds = %6, %24
  %15 = phi i64 [ %25, %24 ], [ 0, %6 ]
  %16 = load ptr, ptr %7, align 8, !tbaa !138
  %17 = trunc i64 %15 to i32
  %18 = getelementptr inbounds ptr, ptr %16, i32 %17
  %19 = load ptr, ptr %18, align 4, !tbaa !6
  %20 = tail call ptr @fa_address_map_get(ptr noundef %1, ptr noundef %19) #42
  %21 = icmp eq ptr %20, null
  br i1 %21, label %24, label %22

22:                                               ; preds = %14
  %23 = call i32 @fa_links_add(ptr noundef nonnull %3, ptr noundef nonnull %20) #42, !range !39
  br label %24

24:                                               ; preds = %22, %14
  %25 = add nuw nsw i64 %15, 1
  %26 = load i64, ptr %4, align 8, !tbaa !137
  %27 = icmp slt i64 %25, %26
  br i1 %27, label %14, label %10
}

; Function Attrs: nounwind optsize
define hidden void @fa_address_map_free(ptr noundef %0) local_unnamed_addr #1 {
  %2 = load ptr, ptr %0, align 8, !tbaa !153
  %3 = getelementptr inbounds %struct.FaAddressMap, ptr %0, i32 0, i32 2
  %4 = load i64, ptr %3, align 8, !tbaa !151
  %5 = trunc i64 %4 to i32
  %6 = shl i32 %5, 2
  %7 = add i32 %6, 4
  tail call void @fa_free(ptr noundef %2, i32 noundef %7, i32 noundef 4) #41
  %8 = getelementptr inbounds %struct.FaAddressMap, ptr %0, i32 0, i32 1
  %9 = load ptr, ptr %8, align 4, !tbaa !154
  %10 = load i64, ptr %3, align 8, !tbaa !151
  %11 = trunc i64 %10 to i32
  %12 = shl i32 %11, 2
  %13 = add i32 %12, 4
  tail call void @fa_free(ptr noundef %9, i32 noundef %13, i32 noundef 4) #41
  tail call void @fa_free(ptr noundef nonnull %0, i32 noundef 16, i32 noundef 8) #41
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, argmem: none, inaccessiblemem: none)
define hidden void @fa_argument_source_set(i32 noundef %0, ptr noundef %1) local_unnamed_addr #6 {
  %3 = and i32 %0, 15
  %4 = getelementptr inbounds [16 x ptr], ptr @argument_sources, i32 0, i32 %3
  store ptr %1, ptr %4, align 4, !tbaa !6
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(read, argmem: none, inaccessiblemem: none)
define hidden ptr @fa_argument_source_get(i32 noundef %0) local_unnamed_addr #26 {
  %2 = and i32 %0, 15
  %3 = getelementptr inbounds [16 x ptr], ptr @argument_sources, i32 0, i32 %2
  %4 = load ptr, ptr %3, align 4, !tbaa !6
  ret ptr %4
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_source_new() local_unnamed_addr #1 {
  %1 = tail call ptr @fa_alloc(i32 noundef 64, i32 noundef 8) #41
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(64) %1, i8 0, i64 64, i1 false)
  %2 = load ptr, ptr @reactive_made, align 4, !tbaa !6
  %3 = load i64, ptr @reactive_made_count, align 8, !tbaa !80
  %4 = tail call fastcc ptr @grow_items(ptr noundef %2, i64 noundef %3, ptr noundef nonnull @reactive_made_capacity, i32 noundef 4) #42
  store ptr %4, ptr @reactive_made, align 4, !tbaa !6
  %5 = load i64, ptr @reactive_made_count, align 8, !tbaa !80
  %6 = trunc i64 %5 to i32
  %7 = getelementptr inbounds ptr, ptr %4, i32 %6
  store ptr %1, ptr %7, align 4, !tbaa !6
  %8 = add nsw i64 %5, 1
  store i64 %8, ptr @reactive_made_count, align 8, !tbaa !80
  ret ptr %1
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_source_child(ptr noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #1 {
  %4 = icmp eq ptr %0, null
  br i1 %4, label %50, label %5

5:                                                ; preds = %3
  %6 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 7
  %7 = load i64, ptr %6, align 8, !tbaa !157
  %8 = icmp sgt i64 %7, 0
  br i1 %8, label %9, label %28

9:                                                ; preds = %5
  %10 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 6
  %11 = load ptr, ptr %10, align 4, !tbaa !159
  br label %12

12:                                               ; preds = %9, %22
  %13 = phi i64 [ 0, %9 ], [ %23, %22 ]
  %14 = trunc i64 %13 to i32
  %15 = getelementptr inbounds %struct.FaChild, ptr %11, i32 %14
  %16 = load i64, ptr %15, align 8, !tbaa !160
  %17 = icmp eq i64 %16, %1
  br i1 %17, label %18, label %22

18:                                               ; preds = %12
  %19 = getelementptr inbounds %struct.FaChild, ptr %11, i32 %14, i32 1
  %20 = load i64, ptr %19, align 8, !tbaa !162
  %21 = icmp eq i64 %20, %2
  br i1 %21, label %25, label %22

22:                                               ; preds = %18, %12
  %23 = add nuw nsw i64 %13, 1
  %24 = icmp eq i64 %23, %7
  br i1 %24, label %28, label %12, !llvm.loop !163

25:                                               ; preds = %18
  %26 = getelementptr inbounds %struct.FaChild, ptr %11, i32 %14, i32 2
  %27 = load ptr, ptr %26, align 8, !tbaa !164
  br label %50

28:                                               ; preds = %22, %5
  %29 = tail call ptr @fa_alloc(i32 noundef 64, i32 noundef 8) #41
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(64) %29, i8 0, i32 64, i1 false)
  %30 = getelementptr inbounds %struct.FaSource, ptr %29, i32 0, i32 5
  store ptr %0, ptr %30, align 8, !tbaa !165
  %31 = load ptr, ptr @reactive_made, align 4, !tbaa !6
  %32 = load i64, ptr @reactive_made_count, align 8, !tbaa !80
  %33 = tail call fastcc ptr @grow_items(ptr noundef %31, i64 noundef %32, ptr noundef nonnull @reactive_made_capacity, i32 noundef 4) #42
  store ptr %33, ptr @reactive_made, align 4, !tbaa !6
  %34 = load i64, ptr @reactive_made_count, align 8, !tbaa !80
  %35 = trunc i64 %34 to i32
  %36 = getelementptr inbounds ptr, ptr %33, i32 %35
  store ptr %29, ptr %36, align 4, !tbaa !6
  %37 = add nsw i64 %34, 1
  store i64 %37, ptr @reactive_made_count, align 8, !tbaa !80
  %38 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 6
  %39 = load ptr, ptr %38, align 4, !tbaa !159
  %40 = load i64, ptr %6, align 8, !tbaa !157
  %41 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 8
  %42 = tail call fastcc ptr @grow_items(ptr noundef %39, i64 noundef %40, ptr noundef nonnull %41, i32 noundef 24) #42
  store ptr %42, ptr %38, align 4, !tbaa !159
  %43 = load i64, ptr %6, align 8, !tbaa !157
  %44 = trunc i64 %43 to i32
  %45 = getelementptr inbounds %struct.FaChild, ptr %42, i32 %44
  store i64 %1, ptr %45, align 8, !tbaa.struct !166
  %46 = getelementptr inbounds i8, ptr %45, i32 8
  store i64 %2, ptr %46, align 8, !tbaa.struct !167
  %47 = getelementptr inbounds i8, ptr %45, i32 16
  store ptr %29, ptr %47, align 8, !tbaa.struct !168
  %48 = load i64, ptr %6, align 8, !tbaa !157
  %49 = add nsw i64 %48, 1
  store i64 %49, ptr %6, align 8, !tbaa !157
  br label %50

50:                                               ; preds = %25, %3, %28
  %51 = phi ptr [ %27, %25 ], [ %29, %28 ], [ null, %3 ]
  ret ptr %51
}

; Function Attrs: nounwind optsize
define internal fastcc ptr @grow_items(ptr noundef %0, i64 noundef %1, ptr nocapture noundef %2, i32 noundef %3) unnamed_addr #1 {
  %5 = load i64, ptr %2, align 8, !tbaa !80
  %6 = icmp sgt i64 %5, %1
  br i1 %6, label %23, label %7

7:                                                ; preds = %4
  %8 = icmp eq i64 %5, 0
  %9 = shl nsw i64 %5, 1
  %10 = select i1 %8, i64 4, i64 %9
  %11 = trunc i64 %10 to i32
  %12 = mul i32 %11, %3
  %13 = tail call ptr @fa_alloc(i32 noundef %12, i32 noundef 4) #41
  %14 = load i64, ptr %2, align 8, !tbaa !80
  %15 = icmp sgt i64 %14, 0
  br i1 %15, label %16, label %22

16:                                               ; preds = %7
  %17 = trunc i64 %1 to i32
  %18 = mul i32 %17, %3
  tail call fastcc void @copy_bytes(ptr noundef %13, ptr noundef %0, i32 noundef %18) #42
  %19 = load i64, ptr %2, align 8, !tbaa !80
  %20 = trunc i64 %19 to i32
  %21 = mul i32 %20, %3
  tail call void @fa_free(ptr noundef %0, i32 noundef %21, i32 noundef 4) #41
  br label %22

22:                                               ; preds = %16, %7
  store i64 %10, ptr %2, align 8, !tbaa !80
  br label %23

23:                                               ; preds = %4, %22
  %24 = phi ptr [ %13, %22 ], [ %0, %4 ]
  ret ptr %24
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_field_source(ptr noundef %0, i64 noundef %1) local_unnamed_addr #1 {
  %3 = ptrtoint ptr %0 to i32
  %4 = zext i32 %3 to i64
  %5 = load i64, ptr @field_source_count, align 8, !tbaa !80
  %6 = icmp sgt i64 %5, 0
  br i1 %6, label %7, label %25

7:                                                ; preds = %2
  %8 = load ptr, ptr @field_sources, align 4, !tbaa !6
  br label %9

9:                                                ; preds = %7, %22
  %10 = phi i64 [ 0, %7 ], [ %23, %22 ]
  %11 = trunc i64 %10 to i32
  %12 = getelementptr inbounds %struct.FaChild, ptr %8, i32 %11
  %13 = load i64, ptr %12, align 8, !tbaa !160
  %14 = icmp eq i64 %13, %4
  br i1 %14, label %15, label %22

15:                                               ; preds = %9
  %16 = getelementptr inbounds %struct.FaChild, ptr %8, i32 %11, i32 1
  %17 = load i64, ptr %16, align 8, !tbaa !162
  %18 = icmp eq i64 %17, %1
  br i1 %18, label %19, label %22

19:                                               ; preds = %15
  %20 = getelementptr inbounds %struct.FaChild, ptr %8, i32 %11, i32 2
  %21 = load ptr, ptr %20, align 8, !tbaa !164
  br label %36

22:                                               ; preds = %15, %9
  %23 = add nuw nsw i64 %10, 1
  %24 = icmp eq i64 %23, %5
  br i1 %24, label %25, label %9, !llvm.loop !169

25:                                               ; preds = %22, %2
  %26 = tail call ptr @fa_source_new() #42
  %27 = load ptr, ptr @field_sources, align 4, !tbaa !6
  %28 = load i64, ptr @field_source_count, align 8, !tbaa !80
  %29 = tail call fastcc ptr @grow_items(ptr noundef %27, i64 noundef %28, ptr noundef nonnull @field_source_capacity, i32 noundef 24) #42
  store ptr %29, ptr @field_sources, align 4, !tbaa !6
  %30 = load i64, ptr @field_source_count, align 8, !tbaa !80
  %31 = trunc i64 %30 to i32
  %32 = getelementptr inbounds %struct.FaChild, ptr %29, i32 %31
  store i64 %4, ptr %32, align 8, !tbaa.struct !166
  %33 = getelementptr inbounds i8, ptr %32, i32 8
  store i64 %1, ptr %33, align 8, !tbaa.struct !167
  %34 = getelementptr inbounds i8, ptr %32, i32 16
  store ptr %26, ptr %34, align 8, !tbaa.struct !168
  %35 = add nsw i64 %30, 1
  store i64 %35, ptr @field_source_count, align 8, !tbaa !80
  br label %36

36:                                               ; preds = %19, %25
  %37 = phi ptr [ %26, %25 ], [ %21, %19 ]
  ret ptr %37
}

; Function Attrs: nounwind optsize
define hidden void @fa_track(ptr noundef %0) local_unnamed_addr #1 {
  %2 = alloca i64, align 8
  %3 = load ptr, ptr @reactive_observer, align 4, !tbaa !6
  %4 = icmp eq ptr %3, null
  %5 = icmp eq ptr %0, null
  %6 = or i1 %5, %4
  br i1 %6, label %66, label %7

7:                                                ; preds = %1
  %8 = getelementptr inbounds %struct.FaObserver, ptr %3, i32 0, i32 8
  %9 = load i64, ptr %8, align 8, !tbaa !170
  %10 = icmp sgt i64 %9, 0
  %11 = getelementptr inbounds %struct.FaObserver, ptr %3, i32 0, i32 6
  %12 = load ptr, ptr %11, align 4, !tbaa !172
  br i1 %10, label %16, label %22

13:                                               ; preds = %16
  %14 = add nuw nsw i64 %17, 1
  %15 = icmp eq i64 %14, %9
  br i1 %15, label %22, label %16, !llvm.loop !173

16:                                               ; preds = %7, %13
  %17 = phi i64 [ %14, %13 ], [ 0, %7 ]
  %18 = trunc i64 %17 to i32
  %19 = getelementptr inbounds ptr, ptr %12, i32 %18
  %20 = load ptr, ptr %19, align 4, !tbaa !6
  %21 = icmp eq ptr %20, %0
  br i1 %21, label %66, label %13

22:                                               ; preds = %13, %7
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %2) #44
  %23 = getelementptr inbounds %struct.FaObserver, ptr %3, i32 0, i32 9
  %24 = load i64, ptr %23, align 8, !tbaa !174
  store i64 %24, ptr %2, align 8, !tbaa !80
  %25 = getelementptr inbounds %struct.FaObserver, ptr %3, i32 0, i32 6
  %26 = call fastcc ptr @grow_items(ptr noundef %12, i64 noundef %9, ptr noundef nonnull %2, i32 noundef 4) #42
  store ptr %26, ptr %25, align 4, !tbaa !172
  %27 = load i64, ptr %23, align 8, !tbaa !174
  store i64 %27, ptr %2, align 8, !tbaa !80
  %28 = getelementptr inbounds %struct.FaObserver, ptr %3, i32 0, i32 7
  %29 = load ptr, ptr %28, align 8, !tbaa !175
  %30 = load i64, ptr %8, align 8, !tbaa !170
  %31 = call fastcc ptr @grow_items(ptr noundef %29, i64 noundef %30, ptr noundef nonnull %2, i32 noundef 8) #42
  store ptr %31, ptr %28, align 8, !tbaa !175
  %32 = load i64, ptr %2, align 8, !tbaa !80
  store i64 %32, ptr %23, align 8, !tbaa !174
  %33 = load ptr, ptr %25, align 4, !tbaa !172
  %34 = load i64, ptr %8, align 8, !tbaa !170
  %35 = trunc i64 %34 to i32
  %36 = getelementptr inbounds ptr, ptr %33, i32 %35
  store ptr %0, ptr %36, align 4, !tbaa !6
  %37 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 1
  %38 = load i64, ptr %37, align 8, !tbaa !176
  %39 = load ptr, ptr %28, align 8, !tbaa !175
  %40 = getelementptr inbounds i64, ptr %39, i32 %35
  store i64 %38, ptr %40, align 8, !tbaa !80
  %41 = load i64, ptr %8, align 8, !tbaa !170
  %42 = add nsw i64 %41, 1
  store i64 %42, ptr %8, align 8, !tbaa !170
  %43 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 3
  %44 = load i64, ptr %43, align 8, !tbaa !177
  %45 = icmp sgt i64 %44, 0
  %46 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 2
  %47 = load ptr, ptr %46, align 8, !tbaa !178
  br i1 %45, label %51, label %57

48:                                               ; preds = %51
  %49 = add nuw nsw i64 %52, 1
  %50 = icmp eq i64 %49, %44
  br i1 %50, label %57, label %51, !llvm.loop !179

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
  %60 = tail call fastcc ptr @grow_items(ptr noundef %47, i64 noundef %44, ptr noundef nonnull %59, i32 noundef 4) #42
  store ptr %60, ptr %58, align 8, !tbaa !178
  %61 = load i64, ptr %43, align 8, !tbaa !177
  %62 = trunc i64 %61 to i32
  %63 = getelementptr inbounds ptr, ptr %60, i32 %62
  store ptr %3, ptr %63, align 4, !tbaa !6
  %64 = add nsw i64 %61, 1
  store i64 %64, ptr %43, align 8, !tbaa !177
  br label %65

65:                                               ; preds = %51, %57
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %2) #44
  br label %66

66:                                               ; preds = %16, %1, %65
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_notify(ptr noundef %0) local_unnamed_addr #1 {
  %2 = icmp eq ptr %0, null
  br i1 %2, label %15, label %3

3:                                                ; preds = %1
  tail call fastcc void @bump(ptr noundef nonnull %0) #42
  %4 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 5
  %5 = load ptr, ptr %4, align 8, !tbaa !165
  %6 = icmp eq ptr %5, null
  br i1 %6, label %15, label %7

7:                                                ; preds = %3, %7
  %8 = phi ptr [ %13, %7 ], [ %5, %3 ]
  %9 = getelementptr inbounds %struct.FaSource, ptr %8, i32 0, i32 1
  %10 = load i64, ptr %9, align 8, !tbaa !176
  %11 = add nsw i64 %10, 1
  store i64 %11, ptr %9, align 8, !tbaa !176
  tail call fastcc void @mark_observers(ptr noundef nonnull %8) #42
  %12 = getelementptr inbounds %struct.FaSource, ptr %8, i32 0, i32 5
  %13 = load ptr, ptr %12, align 8, !tbaa !165
  %14 = icmp eq ptr %13, null
  br i1 %14, label %15, label %7, !llvm.loop !180

15:                                               ; preds = %7, %3, %1
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @bump(ptr nocapture noundef %0) unnamed_addr #1 {
  %2 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 1
  %3 = load i64, ptr %2, align 8, !tbaa !176
  %4 = add nsw i64 %3, 1
  store i64 %4, ptr %2, align 8, !tbaa !176
  tail call fastcc void @mark_observers(ptr noundef %0) #42
  %5 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 7
  %6 = load i64, ptr %5, align 8, !tbaa !157
  %7 = icmp sgt i64 %6, 0
  br i1 %7, label %8, label %10

8:                                                ; preds = %1
  %9 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 6
  br label %11

10:                                               ; preds = %11, %1
  ret void

11:                                               ; preds = %8, %11
  %12 = phi i64 [ 0, %8 ], [ %17, %11 ]
  %13 = load ptr, ptr %9, align 4, !tbaa !159
  %14 = trunc i64 %12 to i32
  %15 = getelementptr inbounds %struct.FaChild, ptr %13, i32 %14, i32 2
  %16 = load ptr, ptr %15, align 8, !tbaa !164
  tail call fastcc void @bump(ptr noundef %16) #42
  %17 = add nuw nsw i64 %12, 1
  %18 = load i64, ptr %5, align 8, !tbaa !157
  %19 = icmp slt i64 %17, %18
  br i1 %19, label %11, label %10, !llvm.loop !181
}

; Function Attrs: nounwind optsize
define internal fastcc void @mark_observers(ptr nocapture noundef readonly %0) unnamed_addr #1 {
  %2 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 3
  %3 = load i64, ptr %2, align 8, !tbaa !177
  %4 = icmp sgt i64 %3, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 2
  br label %8

7:                                                ; preds = %8, %1
  ret void

8:                                                ; preds = %5, %8
  %9 = phi i64 [ 0, %5 ], [ %14, %8 ]
  %10 = load ptr, ptr %6, align 8, !tbaa !178
  %11 = trunc i64 %9 to i32
  %12 = getelementptr inbounds ptr, ptr %10, i32 %11
  %13 = load ptr, ptr %12, align 4, !tbaa !6
  tail call fastcc void @mark(ptr noundef %13) #42
  %14 = add nuw nsw i64 %9, 1
  %15 = load i64, ptr %2, align 8, !tbaa !177
  %16 = icmp slt i64 %14, %15
  br i1 %16, label %8, label %7, !llvm.loop !182
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_derived_new(ptr noundef %0, ptr noundef %1, ptr noundef %2, i64 noundef %3) local_unnamed_addr #1 {
  %5 = tail call fastcc ptr @observer_new(ptr noundef %0, ptr noundef null, ptr noundef %1, ptr noundef %2, i64 noundef %3) #42
  store i64 1, ptr %5, align 8, !tbaa !183
  %6 = getelementptr inbounds %struct.FaObserver, ptr %5, i32 0, i32 11
  store i32 1, ptr %6, align 8, !tbaa !184
  ret ptr %5
}

; Function Attrs: nounwind optsize
define internal fastcc noundef ptr @observer_new(ptr noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef %3, i64 noundef %4) unnamed_addr #1 {
  %6 = tail call ptr @fa_alloc(i32 noundef 168, i32 noundef 8) #41
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(168) %6, i8 0, i32 168, i1 false)
  %7 = getelementptr inbounds %struct.FaObserver, ptr %6, i32 0, i32 3
  store ptr %0, ptr %7, align 8, !tbaa !185
  %8 = getelementptr inbounds %struct.FaObserver, ptr %6, i32 0, i32 4
  store ptr %1, ptr %8, align 4, !tbaa !186
  %9 = getelementptr inbounds %struct.FaObserver, ptr %6, i32 0, i32 5
  store ptr %2, ptr %9, align 8, !tbaa !187
  %10 = getelementptr inbounds %struct.FaObserver, ptr %6, i32 0, i32 2
  store ptr %3, ptr %10, align 4, !tbaa !188
  %11 = icmp sgt i64 %4, 0
  br i1 %11, label %12, label %17

12:                                               ; preds = %5
  %13 = trunc i64 %4 to i32
  %14 = tail call ptr @fa_alloc(i32 noundef %13, i32 noundef 16) #41
  %15 = getelementptr inbounds %struct.FaObserver, ptr %6, i32 0, i32 1
  store ptr %14, ptr %15, align 8, !tbaa !189
  tail call void @llvm.memset.p0.i32(ptr align 1 %14, i8 0, i32 %13, i1 false)
  %16 = getelementptr inbounds %struct.FaObserver, ptr %6, i32 0, i32 19
  store i64 %4, ptr %16, align 8, !tbaa !190
  br label %17

17:                                               ; preds = %12, %5
  %18 = load ptr, ptr @reactive_made, align 4, !tbaa !6
  %19 = load i64, ptr @reactive_made_count, align 8, !tbaa !80
  %20 = tail call fastcc ptr @grow_items(ptr noundef %18, i64 noundef %19, ptr noundef nonnull @reactive_made_capacity, i32 noundef 4) #42
  store ptr %20, ptr @reactive_made, align 4, !tbaa !6
  %21 = load i64, ptr @reactive_made_count, align 8, !tbaa !80
  %22 = trunc i64 %21 to i32
  %23 = getelementptr inbounds ptr, ptr %20, i32 %22
  store ptr %6, ptr %23, align 4, !tbaa !6
  %24 = add nsw i64 %21, 1
  store i64 %24, ptr @reactive_made_count, align 8, !tbaa !80
  ret ptr %6
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_derived_read(ptr noundef %0) local_unnamed_addr #1 {
  tail call fastcc void @refresh(ptr noundef %0) #42
  tail call void @fa_track(ptr noundef %0) #42
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 1
  %3 = load ptr, ptr %2, align 8, !tbaa !189
  ret ptr %3
}

; Function Attrs: nounwind optsize
define internal fastcc void @refresh(ptr noundef %0) unnamed_addr #1 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 11
  %3 = load i32, ptr %2, align 8, !tbaa !184
  %4 = icmp eq i32 %3, 0
  br i1 %4, label %29, label %5

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 12
  %7 = load i32, ptr %6, align 4, !tbaa !191
  %8 = icmp eq i32 %7, 0
  br i1 %8, label %13, label %9

9:                                                ; preds = %5
  %10 = tail call fastcc i32 @outdated(ptr noundef nonnull %0) #42
  %11 = icmp eq i32 %10, 0
  br i1 %11, label %12, label %13

12:                                               ; preds = %9
  store i32 0, ptr %2, align 8, !tbaa !184
  br label %29

13:                                               ; preds = %9, %5
  %14 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 13
  %15 = load i32, ptr %14, align 8, !tbaa !192
  %16 = icmp eq i32 %15, 0
  br i1 %16, label %18, label %17

17:                                               ; preds = %13
  tail call fastcc void @reactive_stop(ptr noundef nonnull @.str.34) #45
  unreachable

18:                                               ; preds = %13
  tail call fastcc void @unsubscribe(ptr noundef nonnull %0) #42
  %19 = load ptr, ptr @reactive_observer, align 4, !tbaa !6
  store ptr %0, ptr @reactive_observer, align 4, !tbaa !6
  store i32 1, ptr %14, align 8, !tbaa !192
  %20 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 16
  store i32 1, ptr %20, align 4, !tbaa !193
  %21 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 3
  %22 = load ptr, ptr %21, align 8, !tbaa !185
  tail call void %22(ptr noundef nonnull %0) #41
  store i32 0, ptr %14, align 8, !tbaa !192
  store ptr %19, ptr @reactive_observer, align 4, !tbaa !6
  store i32 0, ptr %2, align 8, !tbaa !184
  store i32 1, ptr %6, align 4, !tbaa !191
  %23 = load i32, ptr %20, align 4, !tbaa !193
  %24 = icmp eq i32 %23, 0
  br i1 %24, label %29, label %25

25:                                               ; preds = %18
  %26 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 1
  %27 = load i64, ptr %26, align 8, !tbaa !194
  %28 = add nsw i64 %27, 1
  store i64 %28, ptr %26, align 8, !tbaa !194
  br label %29

29:                                               ; preds = %18, %25, %1, %12
  ret void
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_watch_new(ptr noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef %3) local_unnamed_addr #1 {
  %5 = tail call fastcc ptr @observer_new(ptr noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef %3, i64 noundef 0) #42
  store i64 2, ptr %5, align 8, !tbaa !183
  %6 = getelementptr inbounds %struct.FaObserver, ptr %5, i32 0, i32 17
  store i32 1, ptr %6, align 8, !tbaa !195
  %7 = load i64, ptr @reactive_sequence, align 8, !tbaa !80
  %8 = add nsw i64 %7, 1
  store i64 %8, ptr @reactive_sequence, align 8, !tbaa !80
  %9 = getelementptr inbounds %struct.FaObserver, ptr %5, i32 0, i32 10
  store i64 %8, ptr %9, align 8, !tbaa !196
  tail call fastcc void @unsubscribe(ptr noundef nonnull %5) #42
  %10 = load ptr, ptr @reactive_observer, align 4, !tbaa !6
  store ptr null, ptr @reactive_observer, align 4, !tbaa !6
  %11 = getelementptr inbounds %struct.FaObserver, ptr %5, i32 0, i32 4
  %12 = load ptr, ptr %11, align 4, !tbaa !186
  tail call void %12(ptr noundef nonnull %5) #41
  store ptr %5, ptr @reactive_observer, align 4, !tbaa !6
  %13 = getelementptr inbounds %struct.FaObserver, ptr %5, i32 0, i32 3
  %14 = load ptr, ptr %13, align 8, !tbaa !185
  tail call void %14(ptr noundef nonnull %5) #41
  store ptr %10, ptr @reactive_observer, align 4, !tbaa !6
  ret ptr %5
}

; Function Attrs: nounwind optsize
define hidden void @fa_dispose(ptr noundef %0) local_unnamed_addr #1 {
  %2 = icmp eq ptr %0, null
  br i1 %2, label %11, label %3

3:                                                ; preds = %1
  %4 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 15
  %5 = load i32, ptr %4, align 8, !tbaa !197
  %6 = icmp eq i32 %5, 0
  br i1 %6, label %7, label %11

7:                                                ; preds = %3
  store i32 1, ptr %4, align 8, !tbaa !197
  tail call fastcc void @unsubscribe(ptr noundef nonnull %0) #42
  %8 = load ptr, ptr @reactive_observer, align 4, !tbaa !6
  store ptr null, ptr @reactive_observer, align 4, !tbaa !6
  %9 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 4
  %10 = load ptr, ptr %9, align 4, !tbaa !186
  tail call void %10(ptr noundef nonnull %0) #41
  store ptr %8, ptr @reactive_observer, align 4, !tbaa !6
  br label %11

11:                                               ; preds = %1, %3, %7
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc void @unsubscribe(ptr noundef %0) unnamed_addr #12 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 8
  %3 = load i64, ptr %2, align 8, !tbaa !170
  %4 = icmp sgt i64 %3, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 6
  br label %8

7:                                                ; preds = %35, %1
  store i64 0, ptr %2, align 8, !tbaa !170
  ret void

8:                                                ; preds = %5, %35
  %9 = phi i64 [ 0, %5 ], [ %36, %35 ]
  %10 = load ptr, ptr %6, align 4, !tbaa !172
  %11 = trunc i64 %9 to i32
  %12 = getelementptr inbounds ptr, ptr %10, i32 %11
  %13 = load ptr, ptr %12, align 4, !tbaa !6
  %14 = getelementptr inbounds %struct.FaSource, ptr %13, i32 0, i32 3
  %15 = load i64, ptr %14, align 8, !tbaa !177
  %16 = icmp sgt i64 %15, 0
  br i1 %16, label %17, label %35

17:                                               ; preds = %8
  %18 = getelementptr inbounds %struct.FaSource, ptr %13, i32 0, i32 2
  %19 = load ptr, ptr %18, align 8, !tbaa !178
  br label %23

20:                                               ; preds = %23
  %21 = add nuw nsw i64 %24, 1
  %22 = icmp eq i64 %21, %15
  br i1 %22, label %35, label %23, !llvm.loop !198

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
  store i64 %34, ptr %14, align 8, !tbaa !177
  br label %35

35:                                               ; preds = %20, %8, %29
  %36 = add nuw nsw i64 %9, 1
  %37 = icmp eq i64 %36, %3
  br i1 %37, label %7, label %8, !llvm.loop !199
}

; Function Attrs: nounwind optsize
define hidden void @fa_reactive_flush() local_unnamed_addr #1 {
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
  tail call fastcc void @reactive_stop(ptr noundef nonnull @.str.15) #45
  unreachable

13:                                               ; preds = %7
  %14 = trunc i64 %9 to i32
  %15 = shl i32 %14, 2
  %16 = tail call ptr @fa_alloc(i32 noundef %15, i32 noundef 4) #41
  %17 = load ptr, ptr @reactive_pending, align 4, !tbaa !6
  tail call fastcc void @copy_bytes(ptr noundef %16, ptr noundef %17, i32 noundef %15) #42
  store i64 0, ptr @reactive_pending_count, align 8, !tbaa !80
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
  %26 = load i64, ptr %25, align 8, !tbaa !196
  br label %27

27:                                               ; preds = %20, %36
  %28 = phi i64 [ %21, %20 ], [ %29, %36 ]
  %29 = add nsw i64 %28, -1
  %30 = trunc i64 %29 to i32
  %31 = getelementptr inbounds ptr, ptr %16, i32 %30
  %32 = load ptr, ptr %31, align 4, !tbaa !6
  %33 = getelementptr inbounds %struct.FaObserver, ptr %32, i32 0, i32 10
  %34 = load i64, ptr %33, align 8, !tbaa !196
  %35 = icmp sgt i64 %34, %26
  br i1 %35, label %36, label %40

36:                                               ; preds = %27
  %37 = trunc i64 %28 to i32
  %38 = getelementptr inbounds ptr, ptr %16, i32 %37
  store ptr %32, ptr %38, align 4, !tbaa !6
  %39 = icmp sgt i64 %28, 1
  br i1 %39, label %27, label %40, !llvm.loop !200

40:                                               ; preds = %36, %27
  %41 = phi i64 [ 0, %36 ], [ %28, %27 ]
  %42 = trunc i64 %41 to i32
  %43 = getelementptr inbounds ptr, ptr %16, i32 %42
  store ptr %24, ptr %43, align 4, !tbaa !6
  %44 = add nuw nsw i64 %21, 1
  %45 = icmp eq i64 %44, %9
  br i1 %45, label %19, label %20, !llvm.loop !201

46:                                               ; preds = %67
  tail call void @fa_free(ptr noundef nonnull %16, i32 noundef %15, i32 noundef 4) #41
  %47 = load i64, ptr @reactive_pending_count, align 8, !tbaa !80
  %48 = icmp sgt i64 %47, 0
  br i1 %48, label %7, label %70, !llvm.loop !202

49:                                               ; preds = %19, %67
  %50 = phi i64 [ %68, %67 ], [ 0, %19 ]
  %51 = trunc i64 %50 to i32
  %52 = getelementptr inbounds ptr, ptr %16, i32 %51
  %53 = load ptr, ptr %52, align 4, !tbaa !6
  %54 = getelementptr inbounds %struct.FaObserver, ptr %53, i32 0, i32 14
  store i32 0, ptr %54, align 4, !tbaa !203
  %55 = getelementptr inbounds %struct.FaObserver, ptr %53, i32 0, i32 15
  %56 = load i32, ptr %55, align 8, !tbaa !197
  %57 = icmp eq i32 %56, 0
  br i1 %57, label %58, label %67

58:                                               ; preds = %49
  %59 = tail call fastcc i32 @outdated(ptr noundef nonnull %53) #42
  %60 = icmp eq i32 %59, 0
  br i1 %60, label %67, label %61

61:                                               ; preds = %58
  tail call fastcc void @unsubscribe(ptr noundef nonnull %53) #42
  %62 = load ptr, ptr @reactive_observer, align 4, !tbaa !6
  store ptr null, ptr @reactive_observer, align 4, !tbaa !6
  %63 = getelementptr inbounds %struct.FaObserver, ptr %53, i32 0, i32 4
  %64 = load ptr, ptr %63, align 4, !tbaa !186
  tail call void %64(ptr noundef nonnull %53) #41
  store ptr %53, ptr @reactive_observer, align 4, !tbaa !6
  %65 = getelementptr inbounds %struct.FaObserver, ptr %53, i32 0, i32 3
  %66 = load ptr, ptr %65, align 8, !tbaa !185
  tail call void %66(ptr noundef nonnull %53) #41
  store ptr %62, ptr @reactive_observer, align 4, !tbaa !6
  br label %67

67:                                               ; preds = %61, %58, %49
  %68 = add nuw i64 %50, 1
  %69 = icmp eq i64 %68, %9
  br i1 %69, label %46, label %49, !llvm.loop !204

70:                                               ; preds = %46, %5
  store i1 false, ptr @reactive_flushing, align 4
  br label %71

71:                                               ; preds = %0, %70
  ret void
}

; Function Attrs: noreturn nounwind optsize
define internal fastcc void @reactive_stop(ptr noundef %0) unnamed_addr #4 {
  tail call void @fa_flush() #42
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.35, i32 noundef 6) #41
  br label %2

2:                                                ; preds = %2, %1
  %3 = phi i32 [ 0, %1 ], [ %7, %2 ]
  %4 = getelementptr inbounds i8, ptr %0, i32 %3
  %5 = load i8, ptr %4, align 1, !tbaa !5
  %6 = icmp eq i8 %5, 0
  %7 = add i32 %3, 1
  br i1 %6, label %8, label %2, !llvm.loop !16

8:                                                ; preds = %2
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull %0, i32 noundef %3) #41
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.22, i32 noundef 1) #41
  tail call void @fa_exit(i32 noundef 101) #43
  unreachable
}

; Function Attrs: nounwind optsize
define internal fastcc noundef i32 @outdated(ptr nocapture noundef readonly %0) unnamed_addr #1 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 8
  %3 = load i64, ptr %2, align 8, !tbaa !170
  %4 = icmp sgt i64 %3, 0
  br i1 %4, label %5, label %30

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 6
  %7 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 7
  br label %12

8:                                                ; preds = %21
  %9 = add nuw nsw i64 %13, 1
  %10 = load i64, ptr %2, align 8, !tbaa !170
  %11 = icmp slt i64 %9, %10
  br i1 %11, label %12, label %28, !llvm.loop !205

12:                                               ; preds = %5, %8
  %13 = phi i64 [ 0, %5 ], [ %9, %8 ]
  %14 = load ptr, ptr %6, align 4, !tbaa !172
  %15 = trunc i64 %13 to i32
  %16 = getelementptr inbounds ptr, ptr %14, i32 %15
  %17 = load ptr, ptr %16, align 4, !tbaa !6
  %18 = load i64, ptr %17, align 8, !tbaa !206
  %19 = icmp eq i64 %18, 1
  br i1 %19, label %20, label %21

20:                                               ; preds = %12
  tail call fastcc void @refresh(ptr noundef nonnull %17) #42
  br label %21

21:                                               ; preds = %20, %12
  %22 = getelementptr inbounds %struct.FaSource, ptr %17, i32 0, i32 1
  %23 = load i64, ptr %22, align 8, !tbaa !176
  %24 = load ptr, ptr %7, align 8, !tbaa !175
  %25 = getelementptr inbounds i64, ptr %24, i32 %15
  %26 = load i64, ptr %25, align 8, !tbaa !80
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
define hidden ptr @fa_observer_value(ptr nocapture noundef readonly %0) local_unnamed_addr #27 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 1
  %3 = load ptr, ptr %2, align 8, !tbaa !189
  ret ptr %3
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read)
define hidden ptr @fa_observer_env(ptr nocapture noundef readonly %0) local_unnamed_addr #27 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 2
  %3 = load ptr, ptr %2, align 4, !tbaa !188
  ret ptr %3
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(none)
define hidden nonnull ptr @fa_observer_cleanup(ptr noundef readnone %0) local_unnamed_addr #15 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 20
  ret ptr %2
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read)
define hidden i32 @fa_observer_computed(ptr nocapture noundef readonly %0) local_unnamed_addr #27 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 12
  %3 = load i32, ptr %2, align 4, !tbaa !191
  ret i32 %3
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: write)
define hidden void @fa_observer_changed(ptr nocapture noundef writeonly %0, i32 noundef %1) local_unnamed_addr #28 {
  %3 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 16
  store i32 %1, ptr %3, align 4, !tbaa !193
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_reactive_release() local_unnamed_addr #1 {
  %1 = load i64, ptr @reactive_made_count, align 8, !tbaa !80
  %2 = icmp sgt i64 %1, 0
  br i1 %2, label %25, label %3

3:                                                ; preds = %78, %0
  %4 = load i64, ptr @reactive_made_capacity, align 8, !tbaa !80
  %5 = icmp sgt i64 %4, 0
  br i1 %5, label %6, label %10

6:                                                ; preds = %3
  %7 = load ptr, ptr @reactive_made, align 4, !tbaa !6
  %8 = trunc i64 %4 to i32
  %9 = shl i32 %8, 2
  tail call void @fa_free(ptr noundef %7, i32 noundef %9, i32 noundef 4) #41
  br label %10

10:                                               ; preds = %3, %6
  %11 = load i64, ptr @reactive_pending_capacity, align 8, !tbaa !80
  %12 = icmp sgt i64 %11, 0
  br i1 %12, label %13, label %17

13:                                               ; preds = %10
  %14 = load ptr, ptr @reactive_pending, align 4, !tbaa !6
  %15 = trunc i64 %11 to i32
  %16 = shl i32 %15, 2
  tail call void @fa_free(ptr noundef %14, i32 noundef %16, i32 noundef 4) #41
  br label %17

17:                                               ; preds = %10, %13
  %18 = load i64, ptr @field_source_capacity, align 8, !tbaa !80
  %19 = icmp sgt i64 %18, 0
  br i1 %19, label %20, label %24

20:                                               ; preds = %17
  %21 = load ptr, ptr @field_sources, align 4, !tbaa !6
  %22 = trunc i64 %18 to i32
  %23 = mul i32 %22, 24
  tail call void @fa_free(ptr noundef %21, i32 noundef %23, i32 noundef 4) #41
  br label %24

24:                                               ; preds = %17, %20
  store i64 0, ptr @reactive_made_count, align 8, !tbaa !80
  ret void

25:                                               ; preds = %0, %78
  %26 = phi i64 [ %27, %78 ], [ %1, %0 ]
  %27 = add nsw i64 %26, -1
  %28 = load ptr, ptr @reactive_made, align 4, !tbaa !6
  %29 = trunc i64 %27 to i32
  %30 = getelementptr inbounds ptr, ptr %28, i32 %29
  %31 = load ptr, ptr %30, align 4, !tbaa !6
  %32 = getelementptr inbounds %struct.FaSource, ptr %31, i32 0, i32 4
  %33 = load i64, ptr %32, align 8, !tbaa !207
  %34 = icmp sgt i64 %33, 0
  br i1 %34, label %35, label %40

35:                                               ; preds = %25
  %36 = getelementptr inbounds %struct.FaSource, ptr %31, i32 0, i32 2
  %37 = load ptr, ptr %36, align 8, !tbaa !178
  %38 = trunc i64 %33 to i32
  %39 = shl i32 %38, 2
  tail call void @fa_free(ptr noundef %37, i32 noundef %39, i32 noundef 4) #41
  br label %40

40:                                               ; preds = %25, %35
  %41 = getelementptr inbounds %struct.FaSource, ptr %31, i32 0, i32 8
  %42 = load i64, ptr %41, align 8, !tbaa !208
  %43 = icmp sgt i64 %42, 0
  br i1 %43, label %44, label %49

44:                                               ; preds = %40
  %45 = getelementptr inbounds %struct.FaSource, ptr %31, i32 0, i32 6
  %46 = load ptr, ptr %45, align 4, !tbaa !159
  %47 = trunc i64 %42 to i32
  %48 = mul i32 %47, 24
  tail call void @fa_free(ptr noundef %46, i32 noundef %48, i32 noundef 4) #41
  br label %49

49:                                               ; preds = %40, %44
  %50 = load i64, ptr %31, align 8, !tbaa !206
  %51 = icmp eq i64 %50, 0
  br i1 %51, label %78, label %52

52:                                               ; preds = %49
  %53 = getelementptr inbounds %struct.FaObserver, ptr %31, i32 0, i32 5
  %54 = load ptr, ptr %53, align 8, !tbaa !187
  tail call void %54(ptr noundef nonnull %31) #41
  %55 = getelementptr inbounds %struct.FaObserver, ptr %31, i32 0, i32 9
  %56 = load i64, ptr %55, align 8, !tbaa !174
  %57 = icmp sgt i64 %56, 0
  br i1 %57, label %58, label %70

58:                                               ; preds = %52
  %59 = getelementptr inbounds %struct.FaObserver, ptr %31, i32 0, i32 6
  %60 = load ptr, ptr %59, align 4, !tbaa !172
  %61 = trunc i64 %56 to i32
  %62 = shl i32 %61, 2
  tail call void @fa_free(ptr noundef %60, i32 noundef %62, i32 noundef 4) #41
  %63 = load i64, ptr %55, align 8, !tbaa !174
  %64 = icmp sgt i64 %63, 0
  br i1 %64, label %65, label %70

65:                                               ; preds = %58
  %66 = getelementptr inbounds %struct.FaObserver, ptr %31, i32 0, i32 7
  %67 = load ptr, ptr %66, align 8, !tbaa !175
  %68 = trunc i64 %63 to i32
  %69 = shl i32 %68, 3
  tail call void @fa_free(ptr noundef %67, i32 noundef %69, i32 noundef 4) #41
  br label %70

70:                                               ; preds = %52, %58, %65
  %71 = getelementptr inbounds %struct.FaObserver, ptr %31, i32 0, i32 19
  %72 = load i64, ptr %71, align 8, !tbaa !190
  %73 = icmp sgt i64 %72, 0
  br i1 %73, label %74, label %78

74:                                               ; preds = %70
  %75 = getelementptr inbounds %struct.FaObserver, ptr %31, i32 0, i32 1
  %76 = load ptr, ptr %75, align 8, !tbaa !189
  %77 = trunc i64 %72 to i32
  tail call void @fa_free(ptr noundef %76, i32 noundef %77, i32 noundef 16) #41
  br label %78

78:                                               ; preds = %70, %74, %49
  %79 = phi i32 [ 64, %49 ], [ 168, %74 ], [ 168, %70 ]
  tail call void @fa_free(ptr noundef nonnull %31, i32 noundef %79, i32 noundef 8) #41
  %80 = icmp ugt i64 %26, 1
  br i1 %80, label %25, label %3, !llvm.loop !209
}

; Function Attrs: cold noinline noreturn nounwind optsize
define hidden void @fa_overflow(i64 noundef %0, i64 noundef %1, i32 noundef %2, ptr noundef %3) local_unnamed_addr #29 {
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  tail call fastcc void @append_c(ptr noundef nonnull @.str.16) #42
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
  tail call fastcc void @append_u128(i128 noundef %13, i32 noundef %15) #42
  br label %17

16:                                               ; preds = %4
  tail call fastcc void @append_u128(i128 noundef %9, i32 noundef 0) #42
  br label %17

17:                                               ; preds = %16, %10
  tail call fastcc void @append_c(ptr noundef nonnull @.str.17) #42
  tail call fastcc void @append_c(ptr noundef %3) #42
  br label %18

18:                                               ; preds = %18, %17
  %19 = phi i32 [ 0, %17 ], [ %23, %18 ]
  %20 = getelementptr inbounds i8, ptr %3, i32 %19
  %21 = load i8, ptr %20, align 1, !tbaa !5
  %22 = icmp eq i8 %21, 0
  %23 = add i32 %19, 1
  br i1 %22, label %24, label %18, !llvm.loop !16

24:                                               ; preds = %18
  %25 = getelementptr inbounds i8, ptr %20, i32 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %25) #45
  unreachable
}

; Function Attrs: noreturn nounwind optsize
define internal fastcc void @stop_with_builder(ptr noundef %0) unnamed_addr #4 {
  tail call void @fa_flush() #42
  %2 = load ptr, ptr @builder, align 4, !tbaa !6
  %3 = load i32, ptr @builder_length, align 4, !tbaa !1
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.35, i32 noundef 6) #41
  tail call void @fa_write(i32 noundef 2, ptr noundef %2, i32 noundef %3) #41
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.36, i32 noundef 6) #41
  br label %4

4:                                                ; preds = %4, %1
  %5 = phi i32 [ 0, %1 ], [ %9, %4 ]
  %6 = getelementptr inbounds i8, ptr %0, i32 %5
  %7 = load i8, ptr %6, align 1, !tbaa !5
  %8 = icmp eq i8 %7, 0
  %9 = add i32 %5, 1
  br i1 %8, label %10, label %4, !llvm.loop !16

10:                                               ; preds = %4
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull %0, i32 noundef %5) #41
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.22, i32 noundef 1) #41
  tail call void @fa_exit(i32 noundef 101) #43
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define hidden void @fa_overflow_unknown(ptr noundef %0) local_unnamed_addr #29 {
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  tail call fastcc void @append_c(ptr noundef nonnull @.str.18) #42
  tail call fastcc void @append_c(ptr noundef %0) #42
  br label %2

2:                                                ; preds = %2, %1
  %3 = phi i32 [ 0, %1 ], [ %7, %2 ]
  %4 = getelementptr inbounds i8, ptr %0, i32 %3
  %5 = load i8, ptr %4, align 1, !tbaa !5
  %6 = icmp eq i8 %5, 0
  %7 = add i32 %3, 1
  br i1 %6, label %8, label %2, !llvm.loop !16

8:                                                ; preds = %2
  %9 = getelementptr inbounds i8, ptr %4, i32 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %9) #45
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define hidden void @fa_float_overflow(double noundef %0, ptr noundef %1) local_unnamed_addr #29 {
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  tail call fastcc void @append_c(ptr noundef nonnull @.str.16) #42
  tail call void @fa_b_f64(double noundef %0) #42
  tail call fastcc void @append_c(ptr noundef nonnull @.str.17) #42
  tail call fastcc void @append_c(ptr noundef %1) #42
  br label %3

3:                                                ; preds = %3, %2
  %4 = phi i32 [ 0, %2 ], [ %8, %3 ]
  %5 = getelementptr inbounds i8, ptr %1, i32 %4
  %6 = load i8, ptr %5, align 1, !tbaa !5
  %7 = icmp eq i8 %6, 0
  %8 = add i32 %4, 1
  br i1 %7, label %9, label %3, !llvm.loop !16

9:                                                ; preds = %3
  %10 = getelementptr inbounds i8, ptr %5, i32 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %10) #45
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define hidden void @fa_assert_failed(ptr noundef %0) local_unnamed_addr #29 {
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  tail call fastcc void @append_c(ptr noundef nonnull @.str.19) #42
  tail call fastcc void @append_c(ptr noundef %0) #42
  br label %2

2:                                                ; preds = %2, %1
  %3 = phi i32 [ 0, %1 ], [ %7, %2 ]
  %4 = getelementptr inbounds i8, ptr %0, i32 %3
  %5 = load i8, ptr %4, align 1, !tbaa !5
  %6 = icmp eq i8 %5, 0
  %7 = add i32 %3, 1
  br i1 %6, label %8, label %2, !llvm.loop !16

8:                                                ; preds = %2
  %9 = getelementptr inbounds i8, ptr %4, i32 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %9) #45
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define hidden void @fa_assert_failed_operands(ptr noundef %0) local_unnamed_addr #29 {
  %2 = alloca %struct.FaString, align 4
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %2) #44
  call void @fa_b_take(ptr noundef nonnull %2) #42
  tail call fastcc void @append_c(ptr noundef nonnull @.str.19) #42
  tail call fastcc void @append_c(ptr noundef %0) #42
  tail call fastcc void @append_c(ptr noundef nonnull @.str.20) #42
  %3 = load ptr, ptr %2, align 4, !tbaa !18
  %4 = getelementptr inbounds %struct.FaString, ptr %2, i32 0, i32 1
  %5 = load i32, ptr %4, align 4, !tbaa !20
  tail call void @fa_b_bytes(ptr noundef %3, i32 noundef %5) #42
  br label %6

6:                                                ; preds = %6, %1
  %7 = phi i32 [ 0, %1 ], [ %11, %6 ]
  %8 = getelementptr inbounds i8, ptr %0, i32 %7
  %9 = load i8, ptr %8, align 1, !tbaa !5
  %10 = icmp eq i8 %9, 0
  %11 = add i32 %7, 1
  br i1 %10, label %12, label %6, !llvm.loop !16

12:                                               ; preds = %6
  %13 = getelementptr inbounds i8, ptr %8, i32 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %13) #45
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define hidden void @fa_uncaught(ptr noundef %0) local_unnamed_addr #29 {
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  tail call void @fa_format_union(ptr noundef %0, i32 noundef 1) #41
  tail call void @fa_flush() #42
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.21, i32 noundef 7) #41
  %2 = load ptr, ptr @builder, align 4, !tbaa !6
  %3 = load i32, ptr @builder_length, align 4, !tbaa !1
  tail call void @fa_write(i32 noundef 2, ptr noundef %2, i32 noundef %3) #41
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.22, i32 noundef 1) #41
  tail call void @fa_exit(i32 noundef 1) #43
  unreachable
}

; Function Attrs: optsize
declare void @fa_format_union(ptr noundef, i32 noundef) local_unnamed_addr #2

; Function Attrs: cold mustprogress nofree noinline norecurse nosync nounwind optsize willreturn memory(none)
define hidden noundef nonnull ptr @fa_error_division_by_zero() local_unnamed_addr #30 {
  ret ptr @division_by_zero
}

; Function Attrs: cold noinline nounwind optsize
define hidden noundef ptr @fa_error_conversion(i64 noundef %0, i64 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3) local_unnamed_addr #14 {
  store i32 0, ptr @builder_length, align 4, !tbaa !1
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
  tail call fastcc void @append_u128(i128 noundef %13, i32 noundef %15) #42
  br label %17

16:                                               ; preds = %4
  tail call fastcc void @append_u128(i128 noundef %9, i32 noundef 0) #42
  br label %17

17:                                               ; preds = %16, %10
  tail call fastcc void @append_c(ptr noundef nonnull @.str.17) #42
  tail call fastcc void @append_c(ptr noundef %3) #42
  %18 = tail call fastcc ptr @arithmetic_error_from_builder() #42
  ret ptr %18
}

; Function Attrs: nounwind optsize
define internal fastcc noundef ptr @arithmetic_error_from_builder() unnamed_addr #1 {
  %1 = alloca %struct.FaString, align 4
  %2 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #41
  store i64 4, ptr %2, align 8, !tbaa !210
  %3 = getelementptr inbounds %struct.MessageError, ptr %2, i32 0, i32 1
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %1) #44
  %4 = load ptr, ptr @builder, align 4, !tbaa !6
  %5 = load i32, ptr @builder_length, align 4, !tbaa !1
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %1, ptr noundef %4, i32 noundef %5, i32 noundef 0) #42
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %3, ptr noundef nonnull align 4 dereferenceable(12) %1, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %1) #44
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  ret ptr %2
}

; Function Attrs: cold noinline nounwind optsize
define hidden noundef ptr @fa_error_negative_exponent(i64 noundef %0) local_unnamed_addr #14 {
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  tail call fastcc void @append_c(ptr noundef nonnull @.str.23) #42
  %2 = tail call i64 @llvm.abs.i64(i64 %0, i1 false)
  %3 = lshr i64 %0, 63
  %4 = trunc i64 %3 to i32
  tail call fastcc void @append_u64(i64 noundef %2, i32 noundef %4) #42
  %5 = tail call fastcc ptr @arithmetic_error_from_builder() #42
  ret ptr %5
}

; Function Attrs: cold noinline nounwind optsize
define hidden noundef ptr @fa_error_float(double noundef %0, i32 noundef %1) local_unnamed_addr #14 {
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  tail call void @fa_b_f64(double noundef %0) #42
  %3 = icmp eq i32 %1, 0
  %4 = select i1 %3, ptr @.str.25, ptr @.str.24
  tail call fastcc void @append_c(ptr noundef nonnull %4) #42
  %5 = tail call fastcc ptr @arithmetic_error_from_builder() #42
  ret ptr %5
}

; Function Attrs: cold noinline nounwind optsize
define hidden noundef ptr @fa_error_shift(i64 noundef %0, i64 noundef %1) local_unnamed_addr #14 {
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  tail call fastcc void @append_c(ptr noundef nonnull @.str.26) #42
  %3 = tail call i64 @llvm.abs.i64(i64 %0, i1 false)
  %4 = lshr i64 %0, 63
  %5 = trunc i64 %4 to i32
  tail call fastcc void @append_u64(i64 noundef %3, i32 noundef %5) #42
  tail call fastcc void @append_c(ptr noundef nonnull @.str.27) #42
  %6 = tail call i64 @llvm.abs.i64(i64 %1, i1 false)
  %7 = lshr i64 %1, 63
  %8 = trunc i64 %7 to i32
  tail call fastcc void @append_u64(i64 noundef %6, i32 noundef %8) #42
  %9 = tail call fastcc ptr @arithmetic_error_from_builder() #42
  ret ptr %9
}

; Function Attrs: cold noinline nounwind optsize
define hidden noundef ptr @fa_error_checked(i64 noundef %0, i64 noundef %1, ptr nocapture noundef readonly %2) local_unnamed_addr #14 {
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  tail call fastcc void @append_c(ptr noundef nonnull @.str.16) #42
  %4 = zext i64 %1 to i128
  %5 = shl nuw i128 %4, 64
  %6 = zext i64 %0 to i128
  %7 = or disjoint i128 %5, %6
  %8 = icmp slt i64 %1, 0
  %9 = sub i128 0, %7
  %10 = select i1 %8, i128 %9, i128 %7
  %11 = lshr i64 %1, 63
  %12 = trunc i64 %11 to i32
  tail call fastcc void @append_u128(i128 noundef %10, i32 noundef %12) #42
  tail call fastcc void @append_c(ptr noundef nonnull @.str.17) #42
  tail call fastcc void @append_c(ptr noundef %2) #42
  %13 = tail call fastcc ptr @arithmetic_error_from_builder() #42
  ret ptr %13
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_parse_integer(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef writeonly %2) local_unnamed_addr #1 {
  %4 = icmp eq i32 %1, 0
  br i1 %4, label %13, label %5

5:                                                ; preds = %3
  %6 = load i8, ptr %0, align 1, !tbaa !5
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
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  br label %54

14:                                               ; preds = %22
  %15 = icmp eq i32 %29, 0
  br i1 %15, label %16, label %17

16:                                               ; preds = %14
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  br label %33

17:                                               ; preds = %14
  br i1 %12, label %18, label %75

18:                                               ; preds = %17
  %19 = getelementptr inbounds i8, ptr %0, i32 %11
  %20 = load i8, ptr %19, align 1, !tbaa !5
  %21 = zext i8 %20 to i64
  br label %64

22:                                               ; preds = %9, %22
  %23 = phi i32 [ %30, %22 ], [ %11, %9 ]
  %24 = phi i32 [ %29, %22 ], [ 1, %9 ]
  %25 = getelementptr inbounds i8, ptr %0, i32 %23
  %26 = load i8, ptr %25, align 1, !tbaa !5
  %27 = add i8 %26, -48
  %28 = icmp ult i8 %27, 10
  %29 = select i1 %28, i32 %24, i32 0
  %30 = add nuw i32 %23, 1
  %31 = icmp eq i32 %30, %1
  br i1 %31, label %14, label %22, !llvm.loop !212

32:                                               ; preds = %9
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  br i1 %4, label %54, label %33

33:                                               ; preds = %16, %32
  tail call fastcc void @reserve(i32 noundef 1) #42
  %34 = load ptr, ptr @builder, align 4, !tbaa !6
  %35 = load i32, ptr @builder_length, align 4, !tbaa !1
  %36 = getelementptr inbounds i8, ptr %34, i32 %35
  store i8 34, ptr %36, align 1, !tbaa !5
  %37 = add i32 %35, 1
  store i32 %37, ptr @builder_length, align 4, !tbaa !1
  br label %38

38:                                               ; preds = %33, %47
  %39 = phi i32 [ 0, %33 ], [ %52, %47 ]
  %40 = getelementptr inbounds i8, ptr %0, i32 %39
  %41 = load i8, ptr %40, align 1, !tbaa !5
  switch i8 %41, label %47 [
    i8 92, label %42
    i8 34, label %42
  ]

42:                                               ; preds = %38, %38
  tail call fastcc void @reserve(i32 noundef 1) #42
  %43 = load ptr, ptr @builder, align 4, !tbaa !6
  %44 = load i32, ptr @builder_length, align 4, !tbaa !1
  %45 = getelementptr inbounds i8, ptr %43, i32 %44
  store i8 92, ptr %45, align 1, !tbaa !5
  %46 = add i32 %44, 1
  store i32 %46, ptr @builder_length, align 4, !tbaa !1
  br label %47

47:                                               ; preds = %38, %42
  tail call fastcc void @reserve(i32 noundef 1) #42
  %48 = load ptr, ptr @builder, align 4, !tbaa !6
  %49 = load i32, ptr @builder_length, align 4, !tbaa !1
  %50 = getelementptr inbounds i8, ptr %48, i32 %49
  store i8 %41, ptr %50, align 1, !tbaa !5
  %51 = add i32 %49, 1
  store i32 %51, ptr @builder_length, align 4, !tbaa !1
  %52 = add nuw i32 %39, 1
  %53 = icmp eq i32 %52, %1
  br i1 %53, label %54, label %38, !llvm.loop !213

54:                                               ; preds = %47, %32, %13
  %55 = phi ptr [ @.str.28, %13 ], [ @.str.28, %32 ], [ @.str.29, %47 ]
  tail call fastcc void @append_c(ptr noundef nonnull %55) #42
  %56 = tail call fastcc ptr @parse_error(ptr noundef %0, i32 noundef %1) #42
  br label %90

57:                                               ; preds = %64
  %58 = getelementptr inbounds i8, ptr %0, i32 %71
  %59 = load i8, ptr %58, align 1, !tbaa !5
  %60 = zext i8 %59 to i64
  %61 = sub nsw i64 47, %60
  %62 = udiv i64 %61, 10
  %63 = icmp ugt i64 %70, %62
  br i1 %63, label %73, label %64, !llvm.loop !214

64:                                               ; preds = %18, %57
  %65 = phi i64 [ %21, %18 ], [ %60, %57 ]
  %66 = phi i64 [ 0, %18 ], [ %70, %57 ]
  %67 = phi i32 [ %11, %18 ], [ %71, %57 ]
  %68 = mul nuw i64 %66, 10
  %69 = add i64 %68, -48
  %70 = add i64 %69, %65
  %71 = add nuw i32 %67, 1
  %72 = icmp eq i32 %71, %1
  br i1 %72, label %73, label %57, !llvm.loop !214

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
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  tail call fastcc void @reserve(i32 noundef %1) #42
  %82 = load ptr, ptr @builder, align 4, !tbaa !6
  %83 = load i32, ptr @builder_length, align 4, !tbaa !1
  %84 = getelementptr inbounds i8, ptr %82, i32 %83
  tail call fastcc void @copy_bytes(ptr noundef %84, ptr noundef nonnull %0, i32 noundef %1) #42
  %85 = add i32 %83, %1
  store i32 %85, ptr @builder_length, align 4, !tbaa !1
  tail call fastcc void @append_c(ptr noundef nonnull @.str.24) #42
  %86 = tail call fastcc ptr @parse_error(ptr noundef nonnull %0, i32 noundef %1) #42
  br label %90

87:                                               ; preds = %75
  %88 = sub i64 0, %76
  %89 = select i1 %10, i64 %76, i64 %88
  store i64 %89, ptr %2, align 8, !tbaa !80
  br label %90

90:                                               ; preds = %81, %87, %54
  %91 = phi ptr [ %56, %54 ], [ %86, %81 ], [ null, %87 ]
  ret ptr %91
}

; Function Attrs: nounwind optsize
define internal fastcc noundef ptr @parse_error(ptr nocapture noundef readonly %0, i32 noundef %1) unnamed_addr #1 {
  %3 = alloca %struct.FaString, align 4
  %4 = alloca %struct.FaString, align 4
  %5 = tail call ptr @fa_alloc(i32 noundef 32, i32 noundef 8) #41
  store i64 5, ptr %5, align 8, !tbaa !215
  %6 = getelementptr inbounds %struct.ParseError, ptr %5, i32 0, i32 1
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %4) #44
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %4, ptr noundef %0, i32 noundef %1, i32 noundef 0) #42
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 8 dereferenceable(12) %6, ptr noundef nonnull align 4 dereferenceable(12) %4, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %4) #44
  %7 = getelementptr inbounds %struct.ParseError, ptr %5, i32 0, i32 2
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %3) #44
  %8 = load ptr, ptr @builder, align 4, !tbaa !6
  %9 = load i32, ptr @builder_length, align 4, !tbaa !1
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %3, ptr noundef %8, i32 noundef %9, i32 noundef 0) #42
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %7, ptr noundef nonnull align 4 dereferenceable(12) %3, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %3) #44
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  ret ptr %5
}

; Function Attrs: nounwind optsize
define hidden i64 @fa_pow_i64(i64 noundef %0, i64 noundef %1, i32 noundef %2, ptr noundef %3) local_unnamed_addr #1 {
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
  tail call void @fa_overflow_unknown(ptr noundef %3) #47
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
  tail call void @fa_overflow_unknown(ptr noundef %3) #47
  unreachable
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare { i64, i1 } @llvm.smul.with.overflow.i64(i64, i64) #22

; Function Attrs: nounwind optsize
define weak hidden double @pow(double noundef %0, double noundef %1) local_unnamed_addr #1 {
  %3 = tail call double @fa_host_pow(double noundef %0, double noundef %1) #41
  ret double %3
}

; Function Attrs: optsize
declare double @fa_host_pow(double noundef, double noundef) local_unnamed_addr #31

; Function Attrs: nounwind optsize
define weak hidden double @round(double noundef %0) local_unnamed_addr #1 {
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
declare double @llvm.trunc.f64(double) #22

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(read, inaccessiblemem: none)
define hidden ptr @fa_table_entry(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #32 {
  %4 = load ptr, ptr %0, align 8, !tbaa !217
  %5 = icmp ult i64 %1, 4
  br i1 %5, label %13, label %6

6:                                                ; preds = %3
  %7 = lshr i64 %1, 2
  %8 = tail call i64 @llvm.ctlz.i64(i64 %7, i1 true), !range !101
  %9 = sub nuw nsw i64 65, %8
  %10 = shl nsw i64 -1, %9
  %11 = trunc i64 %8 to i32
  %12 = sub nuw nsw i32 64, %11
  br label %13

13:                                               ; preds = %3, %6
  %14 = phi i64 [ %10, %6 ], [ 0, %3 ]
  %15 = phi i32 [ %12, %6 ], [ 0, %3 ]
  %16 = getelementptr inbounds %struct.FaSegmented, ptr %4, i32 0, i32 2, i32 %15
  %17 = load ptr, ptr %16, align 4, !tbaa !6
  %18 = add i64 %14, %1
  %19 = mul nsw i64 %18, %2
  %20 = trunc i64 %19 to i32
  %21 = getelementptr inbounds i8, ptr %17, i32 %20
  ret ptr %21
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_table_new(i64 noundef %0, i64 noundef %1) local_unnamed_addr #1 {
  %3 = tail call ptr @fa_alloc(i32 noundef 56, i32 noundef 8) #41
  %4 = tail call i64 @llvm.smax.i64(i64 %0, i64 4)
  %5 = mul nsw i64 %4, %1
  %6 = trunc i64 %5 to i32
  %7 = tail call ptr @fa_alloc(i32 noundef %6, i32 noundef 16) #41
  store ptr %7, ptr %3, align 8, !tbaa !217
  %8 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 1
  store i64 0, ptr %8, align 8, !tbaa !219
  %9 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 2
  store i64 0, ptr %9, align 8, !tbaa !220
  %10 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 3
  store i64 %4, ptr %10, align 8, !tbaa !221
  %11 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 4
  store ptr null, ptr %11, align 8, !tbaa !222
  %12 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 6
  store i64 0, ptr %12, align 8, !tbaa !223
  %13 = shl nuw nsw i64 %4, 1
  br label %14

14:                                               ; preds = %14, %2
  %15 = phi i64 [ 8, %2 ], [ %17, %14 ]
  %16 = icmp slt i64 %15, %13
  %17 = shl nsw i64 %15, 1
  br i1 %16, label %14, label %18, !llvm.loop !224

18:                                               ; preds = %14
  tail call fastcc void @table_index(ptr noundef nonnull %3, i64 noundef %15, i64 noundef %1) #42
  ret ptr %3
}

; Function Attrs: nounwind optsize
define internal fastcc void @table_index(ptr nocapture noundef %0, i64 noundef %1, i64 noundef %2) unnamed_addr #1 {
  %4 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %5 = load ptr, ptr %4, align 8, !tbaa !222
  %6 = icmp eq ptr %5, null
  br i1 %6, label %13, label %7

7:                                                ; preds = %3
  %8 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %9 = load i64, ptr %8, align 8, !tbaa !225
  %10 = trunc i64 %9 to i32
  %11 = shl i32 %10, 3
  %12 = add i32 %11, 8
  tail call void @fa_free(ptr noundef nonnull %5, i32 noundef %12, i32 noundef 8) #41
  br label %13

13:                                               ; preds = %7, %3
  %14 = trunc i64 %1 to i32
  %15 = shl i32 %14, 3
  %16 = tail call ptr @fa_alloc(i32 noundef %15, i32 noundef 8) #41
  store ptr %16, ptr %4, align 8, !tbaa !222
  tail call void @llvm.memset.p0.i32(ptr align 8 %16, i8 0, i32 %15, i1 false)
  %17 = add nsw i64 %1, -1
  %18 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  store i64 %17, ptr %18, align 8, !tbaa !225
  %19 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 2
  %20 = load i64, ptr %19, align 8, !tbaa !220
  %21 = icmp sgt i64 %20, 0
  br i1 %21, label %22, label %25

22:                                               ; preds = %13
  %23 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 6
  %24 = load ptr, ptr %0, align 8, !tbaa !217
  br label %26

25:                                               ; preds = %72, %13
  ret void

26:                                               ; preds = %22, %72
  %27 = phi i64 [ %20, %22 ], [ %74, %72 ]
  %28 = phi i64 [ 0, %22 ], [ %73, %72 ]
  %29 = load i64, ptr %23, align 8, !tbaa !223
  %30 = icmp eq i64 %29, 0
  br i1 %30, label %46, label %31

31:                                               ; preds = %26
  %32 = icmp ult i64 %28, 4
  br i1 %32, label %39, label %33

33:                                               ; preds = %31
  %34 = lshr i64 %28, 2
  %35 = tail call i64 @llvm.ctlz.i64(i64 %34, i1 true), !range !101
  %36 = sub nuw nsw i64 64, %35
  %37 = sub nuw nsw i64 65, %35
  %38 = shl nsw i64 -1, %37
  br label %39

39:                                               ; preds = %33, %31
  %40 = phi i64 [ %38, %33 ], [ 0, %31 ]
  %41 = phi i64 [ %36, %33 ], [ 0, %31 ]
  %42 = trunc i64 %41 to i32
  %43 = getelementptr inbounds %struct.FaSegmented, ptr %24, i32 0, i32 2, i32 %42
  %44 = load ptr, ptr %43, align 4, !tbaa !6
  %45 = add i64 %40, %28
  br label %46

46:                                               ; preds = %26, %39
  %47 = phi i64 [ %45, %39 ], [ %28, %26 ]
  %48 = phi ptr [ %44, %39 ], [ %24, %26 ]
  %49 = mul nsw i64 %47, %2
  %50 = trunc i64 %49 to i32
  %51 = getelementptr inbounds i8, ptr %48, i32 %50
  %52 = getelementptr inbounds i8, ptr %51, i32 8
  %53 = load i64, ptr %52, align 8, !tbaa !80
  %54 = icmp eq i64 %53, 0
  br i1 %54, label %55, label %57

55:                                               ; preds = %46
  %56 = add nuw nsw i64 %28, 1
  br label %72

57:                                               ; preds = %46
  %58 = load i64, ptr %51, align 8, !tbaa !80
  %59 = load i64, ptr %18, align 8, !tbaa !225
  %60 = load ptr, ptr %4, align 8, !tbaa !222
  br label %61

61:                                               ; preds = %61, %57
  %62 = phi i64 [ %58, %57 ], [ %68, %61 ]
  %63 = and i64 %62, %59
  %64 = trunc i64 %63 to i32
  %65 = getelementptr inbounds i64, ptr %60, i32 %64
  %66 = load i64, ptr %65, align 8, !tbaa !80
  %67 = icmp eq i64 %66, 0
  %68 = add nsw i64 %63, 1
  br i1 %67, label %69, label %61, !llvm.loop !226

69:                                               ; preds = %61
  %70 = add nuw nsw i64 %28, 1
  store i64 %70, ptr %65, align 8, !tbaa !80
  %71 = load i64, ptr %19, align 8, !tbaa !220
  br label %72

72:                                               ; preds = %55, %69
  %73 = phi i64 [ %56, %55 ], [ %70, %69 ]
  %74 = phi i64 [ %27, %55 ], [ %71, %69 ]
  %75 = icmp slt i64 %73, %74
  br i1 %75, label %26, label %25, !llvm.loop !227
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_table_new_growing(i64 noundef %0, i64 noundef %1) local_unnamed_addr #1 {
  %3 = tail call ptr @fa_alloc(i32 noundef 56, i32 noundef 8) #41
  %4 = tail call i64 @llvm.smax.i64(i64 %0, i64 4)
  %5 = tail call noundef ptr @fa_alloc(i32 noundef 208, i32 noundef 8) #41
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(208) %5, i8 0, i32 208, i1 false)
  store ptr %5, ptr %3, align 8, !tbaa !217
  %6 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 1
  store i64 0, ptr %6, align 8, !tbaa !219
  %7 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 2
  store i64 0, ptr %7, align 8, !tbaa !220
  %8 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 3
  store i64 %4, ptr %8, align 8, !tbaa !221
  %9 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 4
  store ptr null, ptr %9, align 8, !tbaa !222
  %10 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 6
  store i64 1, ptr %10, align 8, !tbaa !223
  %11 = shl nuw nsw i64 %4, 1
  br label %12

12:                                               ; preds = %12, %2
  %13 = phi i64 [ 8, %2 ], [ %15, %12 ]
  %14 = icmp slt i64 %13, %11
  %15 = shl nsw i64 %13, 1
  br i1 %14, label %12, label %16, !llvm.loop !224

16:                                               ; preds = %12
  tail call fastcc void @table_index(ptr noundef nonnull %3, i64 noundef %13, i64 noundef %1) #42
  ret ptr %3
}

; Function Attrs: alwaysinline nounwind optsize
define hidden i64 @fa_table_find(ptr nocapture noundef readonly %0, i64 noundef %1, ptr noundef %2, ptr nocapture noundef readonly %3, i64 noundef %4) local_unnamed_addr #33 {
  %6 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %7 = load i64, ptr %6, align 8, !tbaa !225
  %8 = and i64 %7, %1
  %9 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %10 = load ptr, ptr %9, align 8, !tbaa !222
  %11 = trunc i64 %8 to i32
  %12 = getelementptr inbounds i64, ptr %10, i32 %11
  %13 = load i64, ptr %12, align 8, !tbaa !80
  %14 = icmp eq i64 %13, 0
  br i1 %14, label %45, label %15

15:                                               ; preds = %5, %36
  %16 = phi ptr [ %37, %36 ], [ %10, %5 ]
  %17 = phi i64 [ %38, %36 ], [ %7, %5 ]
  %18 = phi i64 [ %43, %36 ], [ %13, %5 ]
  %19 = phi i64 [ %40, %36 ], [ %8, %5 ]
  %20 = icmp sgt i64 %18, 0
  br i1 %20, label %21, label %36

21:                                               ; preds = %15
  %22 = add nsw i64 %18, -1
  %23 = load ptr, ptr %0, align 8, !tbaa !217
  %24 = mul nsw i64 %22, %4
  %25 = trunc i64 %24 to i32
  %26 = getelementptr inbounds i8, ptr %23, i32 %25
  %27 = load i64, ptr %26, align 8, !tbaa !80
  %28 = icmp eq i64 %27, %1
  br i1 %28, label %29, label %36

29:                                               ; preds = %21
  %30 = getelementptr inbounds i8, ptr %26, i32 16
  %31 = tail call i32 %3(ptr noundef nonnull %30, ptr noundef %2) #41
  %32 = icmp eq i32 %31, 0
  br i1 %32, label %33, label %45

33:                                               ; preds = %29
  %34 = load i64, ptr %6, align 8, !tbaa !225
  %35 = load ptr, ptr %9, align 8, !tbaa !222
  br label %36

36:                                               ; preds = %33, %21, %15
  %37 = phi ptr [ %35, %33 ], [ %16, %21 ], [ %16, %15 ]
  %38 = phi i64 [ %34, %33 ], [ %17, %21 ], [ %17, %15 ]
  %39 = add nsw i64 %19, 1
  %40 = and i64 %38, %39
  %41 = trunc i64 %40 to i32
  %42 = getelementptr inbounds i64, ptr %37, i32 %41
  %43 = load i64, ptr %42, align 8, !tbaa !80
  %44 = icmp eq i64 %43, 0
  br i1 %44, label %45, label %15

45:                                               ; preds = %29, %36, %5
  %46 = phi i64 [ -1, %5 ], [ -1, %36 ], [ %22, %29 ]
  ret i64 %46
}

; Function Attrs: alwaysinline nounwind optsize
define hidden i64 @fa_table_find_growing(ptr nocapture noundef readonly %0, i64 noundef %1, ptr noundef %2, ptr nocapture noundef readonly %3, i64 noundef %4) local_unnamed_addr #33 {
  %6 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %7 = load i64, ptr %6, align 8, !tbaa !225
  %8 = and i64 %7, %1
  %9 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %10 = load ptr, ptr %9, align 8, !tbaa !222
  %11 = trunc i64 %8 to i32
  %12 = getelementptr inbounds i64, ptr %10, i32 %11
  %13 = load i64, ptr %12, align 8, !tbaa !80
  %14 = icmp eq i64 %13, 0
  br i1 %14, label %59, label %15

15:                                               ; preds = %5, %50
  %16 = phi ptr [ %51, %50 ], [ %10, %5 ]
  %17 = phi i64 [ %52, %50 ], [ %7, %5 ]
  %18 = phi i64 [ %57, %50 ], [ %13, %5 ]
  %19 = phi i64 [ %54, %50 ], [ %8, %5 ]
  %20 = icmp sgt i64 %18, 0
  br i1 %20, label %21, label %50

21:                                               ; preds = %15
  %22 = add nsw i64 %18, -1
  %23 = load ptr, ptr %0, align 8, !tbaa !217
  %24 = icmp ult i64 %18, 5
  br i1 %24, label %31, label %25

25:                                               ; preds = %21
  %26 = lshr i64 %22, 2
  %27 = tail call i64 @llvm.ctlz.i64(i64 %26, i1 true), !range !101
  %28 = sub nuw nsw i64 64, %27
  %29 = sub nuw nsw i64 65, %27
  %30 = shl nsw i64 -1, %29
  br label %31

31:                                               ; preds = %25, %21
  %32 = phi i64 [ %30, %25 ], [ 0, %21 ]
  %33 = phi i64 [ %28, %25 ], [ 0, %21 ]
  %34 = trunc i64 %33 to i32
  %35 = getelementptr inbounds %struct.FaSegmented, ptr %23, i32 0, i32 2, i32 %34
  %36 = load ptr, ptr %35, align 4, !tbaa !6
  %37 = add nsw i64 %32, %22
  %38 = mul nsw i64 %37, %4
  %39 = trunc i64 %38 to i32
  %40 = getelementptr inbounds i8, ptr %36, i32 %39
  %41 = load i64, ptr %40, align 8, !tbaa !80
  %42 = icmp eq i64 %41, %1
  br i1 %42, label %43, label %50

43:                                               ; preds = %31
  %44 = getelementptr inbounds i8, ptr %40, i32 16
  %45 = tail call i32 %3(ptr noundef nonnull %44, ptr noundef %2) #41
  %46 = icmp eq i32 %45, 0
  br i1 %46, label %47, label %59

47:                                               ; preds = %43
  %48 = load i64, ptr %6, align 8, !tbaa !225
  %49 = load ptr, ptr %9, align 8, !tbaa !222
  br label %50

50:                                               ; preds = %47, %31, %15
  %51 = phi ptr [ %49, %47 ], [ %16, %31 ], [ %16, %15 ]
  %52 = phi i64 [ %48, %47 ], [ %17, %31 ], [ %17, %15 ]
  %53 = add nsw i64 %19, 1
  %54 = and i64 %52, %53
  %55 = trunc i64 %54 to i32
  %56 = getelementptr inbounds i64, ptr %51, i32 %55
  %57 = load i64, ptr %56, align 8, !tbaa !80
  %58 = icmp eq i64 %57, 0
  br i1 %58, label %59, label %15

59:                                               ; preds = %43, %50, %5
  %60 = phi i64 [ -1, %5 ], [ -1, %50 ], [ %22, %43 ]
  ret i64 %60
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_table_append(ptr nocapture noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #1 {
  %4 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 2
  %5 = load i64, ptr %4, align 8, !tbaa !220
  %6 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 3
  %7 = load i64, ptr %6, align 8, !tbaa !221
  %8 = icmp slt i64 %5, %7
  br i1 %8, label %72, label %9

9:                                                ; preds = %3
  %10 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 6
  %11 = load i64, ptr %10, align 8, !tbaa !223
  %12 = icmp eq i64 %11, 0
  br i1 %12, label %21, label %13

13:                                               ; preds = %9
  %14 = shl nsw i64 %7, 1
  store i64 %14, ptr %6, align 8, !tbaa !221
  %15 = shl nsw i64 %7, 2
  br label %16

16:                                               ; preds = %16, %13
  %17 = phi i64 [ 8, %13 ], [ %19, %16 ]
  %18 = icmp slt i64 %17, %15
  %19 = shl nsw i64 %17, 1
  br i1 %18, label %16, label %20, !llvm.loop !224

20:                                               ; preds = %16
  tail call fastcc void @table_index(ptr noundef %0, i64 noundef %17, i64 noundef %2) #42
  br label %72

21:                                               ; preds = %9
  %22 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 1
  %23 = load i64, ptr %22, align 8, !tbaa !219
  %24 = shl nsw i64 %23, 1
  %25 = icmp sgt i64 %24, %5
  %26 = zext i1 %25 to i64
  %27 = shl nsw i64 %7, %26
  %28 = mul nsw i64 %27, %2
  %29 = trunc i64 %28 to i32
  %30 = tail call ptr @fa_alloc(i32 noundef %29, i32 noundef 16) #41
  %31 = load i64, ptr %4, align 8, !tbaa !220
  %32 = icmp sgt i64 %31, 0
  br i1 %32, label %33, label %35

33:                                               ; preds = %21
  %34 = trunc i64 %2 to i32
  br label %45

35:                                               ; preds = %62, %21
  %36 = phi i64 [ 0, %21 ], [ %64, %62 ]
  %37 = load ptr, ptr %0, align 8, !tbaa !217
  %38 = load i64, ptr %6, align 8, !tbaa !221
  %39 = mul nsw i64 %38, %2
  %40 = trunc i64 %39 to i32
  tail call void @fa_free(ptr noundef %37, i32 noundef %40, i32 noundef 16) #41
  store ptr %30, ptr %0, align 8, !tbaa !217
  store i64 %36, ptr %4, align 8, !tbaa !220
  store i64 %27, ptr %6, align 8, !tbaa !221
  %41 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %42 = load i64, ptr %41, align 8, !tbaa !225
  %43 = add nsw i64 %42, 1
  %44 = shl nsw i64 %27, 1
  br label %67

45:                                               ; preds = %62, %33
  %46 = phi i64 [ %31, %33 ], [ %63, %62 ]
  %47 = phi i64 [ 0, %33 ], [ %65, %62 ]
  %48 = phi i64 [ 0, %33 ], [ %64, %62 ]
  %49 = load ptr, ptr %0, align 8, !tbaa !217
  %50 = mul nsw i64 %47, %2
  %51 = trunc i64 %50 to i32
  %52 = getelementptr inbounds i8, ptr %49, i32 %51
  %53 = getelementptr inbounds i8, ptr %52, i32 8
  %54 = load i64, ptr %53, align 8, !tbaa !80
  %55 = icmp eq i64 %54, 0
  br i1 %55, label %62, label %56

56:                                               ; preds = %45
  %57 = mul nsw i64 %48, %2
  %58 = trunc i64 %57 to i32
  %59 = getelementptr inbounds i8, ptr %30, i32 %58
  tail call fastcc void @copy_bytes(ptr noundef %59, ptr noundef nonnull %52, i32 noundef %34) #42
  %60 = add nsw i64 %48, 1
  %61 = load i64, ptr %4, align 8, !tbaa !220
  br label %62

62:                                               ; preds = %56, %45
  %63 = phi i64 [ %61, %56 ], [ %46, %45 ]
  %64 = phi i64 [ %60, %56 ], [ %48, %45 ]
  %65 = add nuw nsw i64 %47, 1
  %66 = icmp slt i64 %65, %63
  br i1 %66, label %45, label %35, !llvm.loop !228

67:                                               ; preds = %67, %35
  %68 = phi i64 [ %43, %35 ], [ %70, %67 ]
  %69 = icmp slt i64 %68, %44
  %70 = shl nsw i64 %68, 1
  br i1 %69, label %67, label %71, !llvm.loop !229

71:                                               ; preds = %67
  tail call fastcc void @table_index(ptr noundef nonnull %0, i64 noundef %68, i64 noundef %2) #42
  br label %72

72:                                               ; preds = %3, %20, %71
  %73 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 6
  %74 = load i64, ptr %73, align 8, !tbaa !223
  %75 = icmp eq i64 %74, 0
  %76 = load ptr, ptr %0, align 8, !tbaa !217
  br i1 %75, label %79, label %77

77:                                               ; preds = %72
  %78 = tail call ptr @fa_segmented_push_slot(ptr noundef %76, i64 noundef %2, i32 noundef 2) #42
  br label %84

79:                                               ; preds = %72
  %80 = load i64, ptr %4, align 8, !tbaa !220
  %81 = mul nsw i64 %80, %2
  %82 = trunc i64 %81 to i32
  %83 = getelementptr inbounds i8, ptr %76, i32 %82
  br label %84

84:                                               ; preds = %79, %77
  %85 = phi ptr [ %78, %77 ], [ %83, %79 ]
  store i64 %1, ptr %85, align 8, !tbaa !80
  %86 = getelementptr inbounds i8, ptr %85, i32 8
  store i64 1, ptr %86, align 8, !tbaa !80
  %87 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %88 = load i64, ptr %87, align 8, !tbaa !225
  %89 = and i64 %88, %1
  %90 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %91 = load ptr, ptr %90, align 8, !tbaa !222
  %92 = trunc i64 %89 to i32
  %93 = getelementptr inbounds i64, ptr %91, i32 %92
  %94 = load i64, ptr %93, align 8, !tbaa !80
  %95 = icmp sgt i64 %94, 0
  br i1 %95, label %96, label %104

96:                                               ; preds = %84, %96
  %97 = phi i64 [ %99, %96 ], [ %89, %84 ]
  %98 = add nsw i64 %97, 1
  %99 = and i64 %98, %88
  %100 = trunc i64 %99 to i32
  %101 = getelementptr inbounds i64, ptr %91, i32 %100
  %102 = load i64, ptr %101, align 8, !tbaa !80
  %103 = icmp sgt i64 %102, 0
  br i1 %103, label %96, label %104, !llvm.loop !230

104:                                              ; preds = %96, %84
  %105 = phi ptr [ %93, %84 ], [ %101, %96 ]
  %106 = load i64, ptr %4, align 8, !tbaa !220
  %107 = add nsw i64 %106, 1
  store i64 %107, ptr %105, align 8, !tbaa !80
  %108 = load i64, ptr %4, align 8, !tbaa !220
  %109 = add nsw i64 %108, 1
  store i64 %109, ptr %4, align 8, !tbaa !220
  %110 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 1
  %111 = load i64, ptr %110, align 8, !tbaa !219
  %112 = add nsw i64 %111, 1
  store i64 %112, ptr %110, align 8, !tbaa !219
  ret ptr %85
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden void @fa_table_kill(ptr nocapture noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #12 {
  %4 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 6
  %5 = load i64, ptr %4, align 8, !tbaa !223
  %6 = icmp eq i64 %5, 0
  %7 = load ptr, ptr %0, align 8, !tbaa !217
  br i1 %6, label %24, label %8

8:                                                ; preds = %3
  %9 = icmp ult i64 %1, 4
  br i1 %9, label %17, label %10

10:                                               ; preds = %8
  %11 = lshr i64 %1, 2
  %12 = tail call i64 @llvm.ctlz.i64(i64 %11, i1 true), !range !101
  %13 = sub nuw nsw i64 65, %12
  %14 = shl nsw i64 -1, %13
  %15 = trunc i64 %12 to i32
  %16 = sub nuw nsw i32 64, %15
  br label %17

17:                                               ; preds = %10, %8
  %18 = phi i64 [ %14, %10 ], [ 0, %8 ]
  %19 = phi i32 [ %16, %10 ], [ 0, %8 ]
  %20 = getelementptr inbounds %struct.FaSegmented, ptr %7, i32 0, i32 2, i32 %19
  %21 = load ptr, ptr %20, align 4, !tbaa !6
  %22 = add i64 %18, %1
  %23 = mul nsw i64 %22, %2
  br label %26

24:                                               ; preds = %3
  %25 = mul nsw i64 %2, %1
  br label %26

26:                                               ; preds = %17, %24
  %27 = phi i64 [ %23, %17 ], [ %25, %24 ]
  %28 = phi ptr [ %21, %17 ], [ %7, %24 ]
  %29 = trunc i64 %27 to i32
  %30 = getelementptr inbounds i8, ptr %28, i32 %29
  %31 = load i64, ptr %30, align 8, !tbaa !80
  %32 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %33 = load i64, ptr %32, align 8, !tbaa !225
  %34 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %35 = load ptr, ptr %34, align 8, !tbaa !222
  %36 = add nsw i64 %1, 1
  br label %37

37:                                               ; preds = %37, %26
  %38 = phi i64 [ %31, %26 ], [ %44, %37 ]
  %39 = and i64 %38, %33
  %40 = trunc i64 %39 to i32
  %41 = getelementptr inbounds i64, ptr %35, i32 %40
  %42 = load i64, ptr %41, align 8, !tbaa !80
  %43 = icmp eq i64 %42, %36
  %44 = add nsw i64 %39, 1
  br i1 %43, label %45, label %37, !llvm.loop !231

45:                                               ; preds = %37
  store i64 -1, ptr %41, align 8, !tbaa !80
  %46 = getelementptr inbounds i8, ptr %30, i32 8
  store i64 0, ptr %46, align 8, !tbaa !80
  %47 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 1
  %48 = load i64, ptr %47, align 8, !tbaa !219
  %49 = add nsw i64 %48, -1
  store i64 %49, ptr %47, align 8, !tbaa !219
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, argmem: readwrite, inaccessiblemem: none)
define hidden void @fa_table_clear(ptr nocapture noundef %0) local_unnamed_addr #34 {
  %2 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 1
  store i64 0, ptr %2, align 8, !tbaa !219
  %3 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 2
  store i64 0, ptr %3, align 8, !tbaa !220
  %4 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 6
  %5 = load i64, ptr %4, align 8, !tbaa !223
  %6 = icmp eq i64 %5, 0
  br i1 %6, label %9, label %7

7:                                                ; preds = %1
  %8 = load ptr, ptr %0, align 8, !tbaa !217
  store i64 0, ptr %8, align 8, !tbaa !102
  br label %9

9:                                                ; preds = %7, %1
  %10 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %11 = load ptr, ptr %10, align 8, !tbaa !222
  %12 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %13 = load i64, ptr %12, align 8, !tbaa !225
  %14 = trunc i64 %13 to i32
  %15 = shl i32 %14, 3
  %16 = add i32 %15, 8
  tail call void @llvm.memset.p0.i32(ptr align 8 %11, i8 0, i32 %16, i1 false)
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_table_free(ptr noundef %0, i64 noundef %1) local_unnamed_addr #1 {
  %3 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 6
  %4 = load i64, ptr %3, align 8, !tbaa !223
  %5 = icmp eq i64 %4, 0
  %6 = load ptr, ptr %0, align 8, !tbaa !217
  br i1 %5, label %8, label %7

7:                                                ; preds = %2
  tail call void @fa_segmented_free(ptr noundef %6, i64 noundef %1, i32 noundef 2) #42
  br label %13

8:                                                ; preds = %2
  %9 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 3
  %10 = load i64, ptr %9, align 8, !tbaa !221
  %11 = mul nsw i64 %10, %1
  %12 = trunc i64 %11 to i32
  tail call void @fa_free(ptr noundef %6, i32 noundef %12, i32 noundef 16) #41
  br label %13

13:                                               ; preds = %8, %7
  %14 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %15 = load ptr, ptr %14, align 8, !tbaa !222
  %16 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %17 = load i64, ptr %16, align 8, !tbaa !225
  %18 = trunc i64 %17 to i32
  %19 = shl i32 %18, 3
  %20 = add i32 %19, 8
  tail call void @fa_free(ptr noundef %15, i32 noundef %20, i32 noundef 8) #41
  tail call void @fa_free(ptr noundef nonnull %0, i32 noundef 56, i32 noundef 8) #41
  ret void
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_table_copy(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #1 {
  %3 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 6
  %4 = load i64, ptr %3, align 8, !tbaa !223
  %5 = icmp eq i64 %4, 0
  %6 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 1
  %7 = load i64, ptr %6, align 8, !tbaa !219
  %8 = tail call i64 @llvm.smax.i64(i64 %7, i64 4)
  br i1 %5, label %11, label %9

9:                                                ; preds = %2
  %10 = tail call ptr @fa_table_new_growing(i64 noundef %8, i64 noundef %1) #42
  br label %13

11:                                               ; preds = %2
  %12 = tail call ptr @fa_table_new(i64 noundef %8, i64 noundef %1) #42
  br label %13

13:                                               ; preds = %11, %9
  %14 = phi ptr [ %10, %9 ], [ %12, %11 ]
  %15 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 2
  %16 = load i64, ptr %15, align 8, !tbaa !220
  %17 = icmp sgt i64 %16, 0
  br i1 %17, label %18, label %20

18:                                               ; preds = %13
  %19 = trunc i64 %1 to i32
  br label %27

20:                                               ; preds = %73, %13
  %21 = phi i64 [ 0, %13 ], [ %75, %73 ]
  %22 = getelementptr inbounds %struct.FaTable, ptr %14, i32 0, i32 2
  store i64 %21, ptr %22, align 8, !tbaa !220
  %23 = getelementptr inbounds %struct.FaTable, ptr %14, i32 0, i32 1
  store i64 %21, ptr %23, align 8, !tbaa !219
  %24 = getelementptr inbounds %struct.FaTable, ptr %14, i32 0, i32 5
  %25 = load i64, ptr %24, align 8, !tbaa !225
  %26 = add nsw i64 %25, 1
  tail call fastcc void @table_index(ptr noundef %14, i64 noundef %26, i64 noundef %1) #42
  ret ptr %14

27:                                               ; preds = %18, %73
  %28 = phi i64 [ %16, %18 ], [ %74, %73 ]
  %29 = phi i64 [ 0, %18 ], [ %75, %73 ]
  %30 = phi i64 [ 0, %18 ], [ %76, %73 ]
  %31 = load ptr, ptr %0, align 8, !tbaa !217
  br i1 %5, label %40, label %32

32:                                               ; preds = %27
  %33 = icmp ult i64 %30, 4
  br i1 %33, label %47, label %34

34:                                               ; preds = %32
  %35 = lshr i64 %30, 2
  %36 = tail call i64 @llvm.ctlz.i64(i64 %35, i1 true), !range !101
  %37 = sub nuw nsw i64 64, %36
  %38 = sub nuw nsw i64 65, %36
  %39 = shl nsw i64 -1, %38
  br label %47

40:                                               ; preds = %27
  %41 = mul nsw i64 %30, %1
  %42 = trunc i64 %41 to i32
  %43 = getelementptr inbounds i8, ptr %31, i32 %42
  %44 = getelementptr inbounds i8, ptr %43, i32 8
  %45 = load i64, ptr %44, align 8, !tbaa !80
  %46 = icmp eq i64 %45, 0
  br i1 %46, label %73, label %63

47:                                               ; preds = %32, %34
  %48 = phi i64 [ %39, %34 ], [ 0, %32 ]
  %49 = phi i64 [ %37, %34 ], [ 0, %32 ]
  %50 = trunc i64 %49 to i32
  %51 = getelementptr inbounds %struct.FaSegmented, ptr %31, i32 0, i32 2, i32 %50
  %52 = load ptr, ptr %51, align 4, !tbaa !6
  %53 = add i64 %48, %30
  %54 = mul nsw i64 %53, %1
  %55 = trunc i64 %54 to i32
  %56 = getelementptr inbounds i8, ptr %52, i32 %55
  %57 = getelementptr inbounds i8, ptr %56, i32 8
  %58 = load i64, ptr %57, align 8, !tbaa !80
  %59 = icmp eq i64 %58, 0
  br i1 %59, label %73, label %60

60:                                               ; preds = %47
  %61 = load ptr, ptr %14, align 8, !tbaa !217
  %62 = tail call ptr @fa_segmented_push_slot(ptr noundef %61, i64 noundef %1, i32 noundef 2) #42
  br label %68

63:                                               ; preds = %40
  %64 = load ptr, ptr %14, align 8, !tbaa !217
  %65 = mul nsw i64 %29, %1
  %66 = trunc i64 %65 to i32
  %67 = getelementptr inbounds i8, ptr %64, i32 %66
  br label %68

68:                                               ; preds = %63, %60
  %69 = phi ptr [ %56, %60 ], [ %43, %63 ]
  %70 = phi ptr [ %62, %60 ], [ %67, %63 ]
  tail call fastcc void @copy_bytes(ptr noundef %70, ptr noundef nonnull %69, i32 noundef %19) #42
  %71 = add nsw i64 %29, 1
  %72 = load i64, ptr %15, align 8, !tbaa !220
  br label %73

73:                                               ; preds = %47, %68, %40
  %74 = phi i64 [ %72, %68 ], [ %28, %40 ], [ %28, %47 ]
  %75 = phi i64 [ %71, %68 ], [ %29, %40 ], [ %29, %47 ]
  %76 = add nuw nsw i64 %30, 1
  %77 = icmp slt i64 %76, %74
  br i1 %77, label %27, label %20, !llvm.loop !232
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: read)
define hidden i64 @fa_hash_bytes(ptr nocapture noundef readonly %0, i32 noundef %1) local_unnamed_addr #9 {
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
  br i1 %18, label %19, label %7, !llvm.loop !233

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
  %37 = load i8, ptr %26, align 1, !tbaa !5
  %38 = zext i8 %37 to i64
  %39 = lshr i32 %24, 1
  %40 = or disjoint i32 %39, %20
  %41 = getelementptr inbounds i8, ptr %0, i32 %40
  %42 = load i8, ptr %41, align 1, !tbaa !5
  %43 = zext i8 %42 to i64
  %44 = shl nuw nsw i64 %43, 8
  %45 = or disjoint i64 %44, %38
  %46 = getelementptr i8, ptr %0, i32 %1
  %47 = getelementptr i8, ptr %46, i32 -1
  %48 = load i8, ptr %47, align 1, !tbaa !5
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

; Function Attrs: cold noinline nounwind optsize
define hidden noundef ptr @fa_error_key_not_found() local_unnamed_addr #14 {
  %1 = alloca %struct.FaString, align 4
  %2 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #41
  store i64 2, ptr %2, align 8, !tbaa !210
  %3 = getelementptr inbounds %struct.MessageError, ptr %2, i32 0, i32 1
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %1) #44
  %4 = load ptr, ptr @builder, align 4, !tbaa !6
  %5 = load i32, ptr @builder_length, align 4, !tbaa !1
  call fastcc void @owned_copy_with(ptr dead_on_unwind nonnull writable sret(%struct.FaString) align 4 %1, ptr noundef %4, i32 noundef %5, i32 noundef 0) #42
  call void @llvm.memcpy.p0.p0.i32(ptr noundef nonnull align 4 dereferenceable(12) %3, ptr noundef nonnull align 4 dereferenceable(12) %1, i32 12, i1 false), !tbaa.struct !10
  call void @llvm.lifetime.end.p0(i64 12, ptr nonnull %1) #44
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  ret ptr %2
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(argmem: readwrite)
define hidden noundef ptr @fa_shared_retain(ptr noundef returned %0) local_unnamed_addr #35 {
  %2 = icmp eq ptr %0, null
  br i1 %2, label %5, label %3

3:                                                ; preds = %1
  %4 = atomicrmw add ptr %0, i64 1 monotonic, align 8
  br label %5

5:                                                ; preds = %3, %1
  ret ptr %0
}

; Function Attrs: nounwind optsize
define hidden void @fa_shared_release(ptr noundef %0) local_unnamed_addr #1 {
  %2 = icmp eq ptr %0, null
  br i1 %2, label %15, label %3

3:                                                ; preds = %1
  %4 = atomicrmw sub ptr %0, i64 1 acq_rel, align 8
  %5 = icmp eq i64 %4, 1
  br i1 %5, label %6, label %15

6:                                                ; preds = %3
  %7 = getelementptr inbounds %struct.FaShared, ptr %0, i32 0, i32 2
  %8 = load ptr, ptr %7, align 8, !tbaa !234
  %9 = icmp eq ptr %8, null
  br i1 %9, label %11, label %10

10:                                               ; preds = %6
  tail call void %8(ptr noundef nonnull %0) #41
  br label %11

11:                                               ; preds = %10, %6
  %12 = getelementptr inbounds %struct.FaShared, ptr %0, i32 0, i32 1
  %13 = load i64, ptr %12, align 8, !tbaa !236
  %14 = trunc i64 %13 to i32
  tail call void @fa_free(ptr noundef nonnull %0, i32 noundef %14, i32 noundef 16) #41
  br label %15

15:                                               ; preds = %11, %3, %1
  ret void
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_atomic_new(i64 noundef %0) local_unnamed_addr #1 {
  %2 = tail call fastcc ptr @shared_new(i32 noundef 40, ptr noundef null) #42
  %3 = getelementptr inbounds %struct.FaAtomicBlock, ptr %2, i32 0, i32 1
  store i64 %0, ptr %3, align 8, !tbaa !237
  ret ptr %2
}

; Function Attrs: nounwind optsize
define internal fastcc ptr @shared_new(i32 noundef %0, ptr noundef %1) unnamed_addr #1 {
  %3 = tail call ptr @fa_alloc(i32 noundef %0, i32 noundef 16) #41
  %4 = icmp eq ptr %3, null
  br i1 %4, label %5, label %6

5:                                                ; preds = %2
  tail call fastcc void @out_of_memory() #45
  unreachable

6:                                                ; preds = %2
  tail call void @llvm.memset.p0.i32(ptr nonnull align 8 %3, i8 0, i32 %0, i1 false)
  store i64 1, ptr %3, align 8, !tbaa !239
  %7 = zext i32 %0 to i64
  %8 = getelementptr inbounds %struct.FaShared, ptr %3, i32 0, i32 1
  store i64 %7, ptr %8, align 8, !tbaa !236
  %9 = getelementptr inbounds %struct.FaShared, ptr %3, i32 0, i32 2
  store ptr %1, ptr %9, align 8, !tbaa !234
  ret ptr %3
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_mutex_new(i64 noundef %0, ptr noundef %1) local_unnamed_addr #1 {
  %3 = trunc i64 %0 to i32
  %4 = add i32 %3, 176
  %5 = tail call fastcc ptr @shared_new(i32 noundef %4, ptr noundef %1) #42
  ret ptr %5
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_mutex_lock(ptr noundef returned %0) local_unnamed_addr #1 {
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
  tail call fastcc void @deadlock(ptr noundef nonnull @.str.30) #47
  unreachable

10:                                               ; preds = %5
  ret ptr %0
}

; Function Attrs: cold noinline noreturn nounwind optsize
define internal fastcc void @deadlock(ptr noundef %0) unnamed_addr #29 {
  tail call void @fa_flush() #42
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.37, i32 noundef 16) #41
  br label %2

2:                                                ; preds = %2, %1
  %3 = phi i32 [ 0, %1 ], [ %7, %2 ]
  %4 = getelementptr inbounds i8, ptr %0, i32 %3
  %5 = load i8, ptr %4, align 1, !tbaa !5
  %6 = icmp eq i8 %5, 0
  %7 = add i32 %3, 1
  br i1 %6, label %8, label %2, !llvm.loop !16

8:                                                ; preds = %2
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull %0, i32 noundef %3) #41
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.22, i32 noundef 1) #41
  tail call void @fa_exit(i32 noundef 101) #43
  unreachable
}

; Function Attrs: mustprogress nofree norecurse nounwind optsize willreturn memory(argmem: readwrite)
define hidden noundef ptr @fa_mutex_try_lock(ptr noundef %0) local_unnamed_addr #35 {
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
define hidden void @fa_mutex_unlock(ptr noundef %0) local_unnamed_addr #1 {
  %2 = icmp eq ptr %0, null
  br i1 %2, label %5, label %3

3:                                                ; preds = %1
  %4 = getelementptr inbounds %struct.FaMutex, ptr %0, i32 0, i32 1
  store atomic i32 0, ptr %4 release, align 16
  tail call void @fa_shared_release(ptr noundef nonnull %0) #42
  br label %5

5:                                                ; preds = %1, %3
  ret void
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_channel_new(i64 noundef %0, i64 noundef %1, ptr noundef %2) local_unnamed_addr #1 {
  %4 = tail call fastcc ptr @shared_new(i32 noundef 224, ptr noundef nonnull @channel_drop) #42
  %5 = tail call i64 @llvm.umin.i64(i64 %0, i64 9223372036854775807)
  %6 = getelementptr inbounds %struct.FaChannel, ptr %4, i32 0, i32 3
  store i64 %5, ptr %6, align 16, !tbaa !240
  %7 = tail call i64 @llvm.smax.i64(i64 %1, i64 1)
  %8 = getelementptr inbounds %struct.FaChannel, ptr %4, i32 0, i32 4
  store i64 %7, ptr %8, align 8, !tbaa !243
  %9 = getelementptr inbounds %struct.FaChannel, ptr %4, i32 0, i32 11
  store ptr %2, ptr %9, align 8, !tbaa !244
  ret ptr %4
}

; Function Attrs: nounwind optsize
define internal void @channel_drop(ptr nocapture noundef readonly %0) #1 {
  %2 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 7
  %3 = load i64, ptr %2, align 16, !tbaa !245
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
  %13 = load i64, ptr %12, align 8, !tbaa !246
  %14 = icmp sgt i64 %13, 0
  br i1 %14, label %35, label %42

15:                                               ; preds = %5, %31
  %16 = phi i64 [ %3, %5 ], [ %32, %31 ]
  %17 = phi i64 [ 0, %5 ], [ %33, %31 ]
  %18 = load ptr, ptr %6, align 8, !tbaa !244
  %19 = icmp eq ptr %18, null
  br i1 %19, label %31, label %20

20:                                               ; preds = %15
  %21 = load i64, ptr %7, align 8, !tbaa !247
  %22 = add nsw i64 %21, %17
  %23 = load i64, ptr %8, align 8, !tbaa !246
  %24 = srem i64 %22, %23
  %25 = load ptr, ptr %9, align 16, !tbaa !248
  %26 = load i64, ptr %10, align 8, !tbaa !243
  %27 = mul nsw i64 %26, %24
  %28 = trunc i64 %27 to i32
  %29 = getelementptr inbounds i8, ptr %25, i32 %28
  tail call void %18(ptr noundef %29) #41
  %30 = load i64, ptr %2, align 16, !tbaa !245
  br label %31

31:                                               ; preds = %20, %15
  %32 = phi i64 [ %30, %20 ], [ %16, %15 ]
  %33 = add nuw nsw i64 %17, 1
  %34 = icmp slt i64 %33, %32
  br i1 %34, label %15, label %11, !llvm.loop !249

35:                                               ; preds = %11
  %36 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 5
  %37 = load ptr, ptr %36, align 16, !tbaa !248
  %38 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 4
  %39 = load i64, ptr %38, align 8, !tbaa !243
  %40 = mul nsw i64 %39, %13
  %41 = trunc i64 %40 to i32
  tail call void @fa_free(ptr noundef %37, i32 noundef %41, i32 noundef 16) #41
  br label %42

42:                                               ; preds = %35, %11
  ret void
}

; Function Attrs: nounwind optsize
define hidden noundef i32 @fa_channel_send(ptr nocapture noundef %0, ptr nocapture noundef readonly %1, i32 noundef %2) local_unnamed_addr #1 {
  %4 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 9
  %5 = load i32, ptr %4, align 16, !tbaa !250
  %6 = icmp eq i32 %5, 0
  br i1 %6, label %7, label %90

7:                                                ; preds = %3
  %8 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 7
  %9 = load i64, ptr %8, align 16, !tbaa !245
  %10 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 3
  %11 = load i64, ptr %10, align 16, !tbaa !240
  %12 = icmp slt i64 %9, %11
  br i1 %12, label %16, label %13

13:                                               ; preds = %7
  %14 = icmp eq i32 %2, 0
  br i1 %14, label %90, label %15

15:                                               ; preds = %13
  tail call fastcc void @deadlock(ptr noundef nonnull @.str.31) #47
  unreachable

16:                                               ; preds = %7
  %17 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 8
  %18 = load i64, ptr %17, align 8, !tbaa !246
  %19 = icmp eq i64 %9, %18
  br i1 %19, label %25, label %20

20:                                               ; preds = %16
  %21 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 6
  %22 = load i64, ptr %21, align 8, !tbaa !247
  %23 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 5
  %24 = load ptr, ptr %23, align 16, !tbaa !248
  br label %75

25:                                               ; preds = %16
  %26 = icmp slt i64 %9, 4
  %27 = shl nuw nsw i64 %9, 1
  %28 = select i1 %26, i64 4, i64 %27
  %29 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 4
  %30 = load i64, ptr %29, align 8, !tbaa !243
  %31 = mul nsw i64 %30, %28
  %32 = trunc i64 %31 to i32
  %33 = tail call ptr @fa_alloc(i32 noundef %32, i32 noundef 16) #41
  %34 = icmp eq ptr %33, null
  br i1 %34, label %41, label %35

35:                                               ; preds = %25
  %36 = load i64, ptr %8, align 16, !tbaa !245
  %37 = icmp sgt i64 %36, 0
  br i1 %37, label %38, label %42

38:                                               ; preds = %35
  %39 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 6
  %40 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 5
  br label %46

41:                                               ; preds = %25
  tail call fastcc void @out_of_memory() #45
  unreachable

42:                                               ; preds = %46, %35
  %43 = phi i64 [ %36, %35 ], [ %62, %46 ]
  %44 = load i64, ptr %17, align 8, !tbaa !246
  %45 = icmp sgt i64 %44, 0
  br i1 %45, label %64, label %71

46:                                               ; preds = %46, %38
  %47 = phi i64 [ 0, %38 ], [ %61, %46 ]
  %48 = load i64, ptr %39, align 8, !tbaa !247
  %49 = add nsw i64 %48, %47
  %50 = load i64, ptr %17, align 8, !tbaa !246
  %51 = srem i64 %49, %50
  %52 = load i64, ptr %29, align 8, !tbaa !243
  %53 = mul nsw i64 %52, %47
  %54 = trunc i64 %53 to i32
  %55 = getelementptr inbounds i8, ptr %33, i32 %54
  %56 = load ptr, ptr %40, align 16, !tbaa !248
  %57 = mul nsw i64 %52, %51
  %58 = trunc i64 %57 to i32
  %59 = getelementptr inbounds i8, ptr %56, i32 %58
  %60 = trunc i64 %52 to i32
  tail call fastcc void @copy_bytes(ptr noundef nonnull %55, ptr noundef %59, i32 noundef %60) #42
  %61 = add nuw nsw i64 %47, 1
  %62 = load i64, ptr %8, align 16, !tbaa !245
  %63 = icmp slt i64 %61, %62
  br i1 %63, label %46, label %42, !llvm.loop !251

64:                                               ; preds = %42
  %65 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 5
  %66 = load ptr, ptr %65, align 16, !tbaa !248
  %67 = load i64, ptr %29, align 8, !tbaa !243
  %68 = mul nsw i64 %67, %44
  %69 = trunc i64 %68 to i32
  tail call void @fa_free(ptr noundef %66, i32 noundef %69, i32 noundef 16) #41
  %70 = load i64, ptr %8, align 16, !tbaa !245
  br label %71

71:                                               ; preds = %64, %42
  %72 = phi i64 [ %70, %64 ], [ %43, %42 ]
  %73 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 5
  store ptr %33, ptr %73, align 16, !tbaa !248
  %74 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 6
  store i64 0, ptr %74, align 8, !tbaa !247
  store i64 %28, ptr %17, align 8, !tbaa !246
  br label %75

75:                                               ; preds = %20, %71
  %76 = phi ptr [ %24, %20 ], [ %33, %71 ]
  %77 = phi i64 [ %18, %20 ], [ %28, %71 ]
  %78 = phi i64 [ %9, %20 ], [ %72, %71 ]
  %79 = phi i64 [ %22, %20 ], [ 0, %71 ]
  %80 = add nsw i64 %79, %78
  %81 = srem i64 %80, %77
  %82 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 4
  %83 = load i64, ptr %82, align 8, !tbaa !243
  %84 = mul nsw i64 %83, %81
  %85 = trunc i64 %84 to i32
  %86 = getelementptr inbounds i8, ptr %76, i32 %85
  %87 = trunc i64 %83 to i32
  tail call fastcc void @copy_bytes(ptr noundef %86, ptr noundef %1, i32 noundef %87) #42
  %88 = load i64, ptr %8, align 16, !tbaa !245
  %89 = add nsw i64 %88, 1
  store i64 %89, ptr %8, align 16, !tbaa !245
  br label %90

90:                                               ; preds = %3, %13, %75
  %91 = phi i32 [ 1, %75 ], [ 0, %13 ], [ 0, %3 ]
  ret i32 %91
}

; Function Attrs: nounwind optsize
define hidden noundef i32 @fa_channel_receive(ptr nocapture noundef %0, ptr nocapture noundef writeonly %1, i32 noundef %2) local_unnamed_addr #1 {
  %4 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 7
  %5 = load i64, ptr %4, align 16, !tbaa !245
  %6 = icmp eq i64 %5, 0
  br i1 %6, label %7, label %14

7:                                                ; preds = %3
  %8 = icmp eq i32 %2, 0
  br i1 %8, label %32, label %9

9:                                                ; preds = %7
  %10 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 9
  %11 = load i32, ptr %10, align 16, !tbaa !250
  %12 = icmp eq i32 %11, 0
  br i1 %12, label %13, label %32

13:                                               ; preds = %9
  tail call fastcc void @deadlock(ptr noundef nonnull @.str.32) #47
  unreachable

14:                                               ; preds = %3
  %15 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 5
  %16 = load ptr, ptr %15, align 16, !tbaa !248
  %17 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 6
  %18 = load i64, ptr %17, align 8, !tbaa !247
  %19 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 4
  %20 = load i64, ptr %19, align 8, !tbaa !243
  %21 = mul nsw i64 %20, %18
  %22 = trunc i64 %21 to i32
  %23 = getelementptr inbounds i8, ptr %16, i32 %22
  %24 = trunc i64 %20 to i32
  tail call fastcc void @copy_bytes(ptr noundef %1, ptr noundef %23, i32 noundef %24) #42
  %25 = load i64, ptr %17, align 8, !tbaa !247
  %26 = add nsw i64 %25, 1
  %27 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 8
  %28 = load i64, ptr %27, align 8, !tbaa !246
  %29 = srem i64 %26, %28
  store i64 %29, ptr %17, align 8, !tbaa !247
  %30 = load i64, ptr %4, align 16, !tbaa !245
  %31 = add nsw i64 %30, -1
  store i64 %31, ptr %4, align 16, !tbaa !245
  br label %32

32:                                               ; preds = %7, %9, %14
  %33 = phi i32 [ 1, %14 ], [ 0, %9 ], [ 0, %7 ]
  ret i32 %33
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: write)
define hidden void @fa_channel_close(ptr nocapture noundef writeonly %0) local_unnamed_addr #28 {
  %2 = getelementptr inbounds %struct.FaChannel, ptr %0, i32 0, i32 9
  store i32 1, ptr %2, align 16, !tbaa !250
  ret void
}

; Function Attrs: nounwind optsize
define weak hidden i128 @__multi3(i128 noundef %0, i128 noundef %1) local_unnamed_addr #1 {
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
define weak hidden i128 @__udivti3(i128 noundef %0, i128 noundef %1) local_unnamed_addr #1 {
  %3 = alloca i128, align 16
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %3) #44
  %4 = call fastcc i128 @divide_u128(i128 noundef %0, i128 noundef %1, ptr noundef nonnull %3) #42
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %3) #44
  ret i128 %4
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: write)
define internal fastcc i128 @divide_u128(i128 noundef %0, i128 noundef %1, ptr nocapture noundef writeonly %2) unnamed_addr #36 {
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
  store i128 %14, ptr %2, align 16, !tbaa !252
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
  br i1 %58, label %10, label %19, !llvm.loop !254
}

; Function Attrs: nounwind optsize
define weak hidden i128 @__umodti3(i128 noundef %0, i128 noundef %1) local_unnamed_addr #1 {
  %3 = alloca i128, align 16
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %3) #44
  %4 = call fastcc i128 @divide_u128(i128 noundef %0, i128 noundef %1, ptr noundef nonnull %3) #42
  %5 = load i128, ptr %3, align 16, !tbaa !252
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %3) #44
  ret i128 %5
}

; Function Attrs: nounwind optsize
define weak hidden i128 @__divti3(i128 noundef %0, i128 noundef %1) local_unnamed_addr #1 {
  %3 = alloca i128, align 16
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %3) #44
  %4 = tail call noundef i128 @llvm.abs.i128(i128 %0, i1 false)
  %5 = tail call noundef i128 @llvm.abs.i128(i128 %1, i1 false)
  %6 = call fastcc i128 @divide_u128(i128 noundef %4, i128 noundef %5, ptr noundef nonnull %3) #42
  %7 = lshr i128 %0, 127
  %8 = trunc i128 %7 to i32
  %9 = lshr i128 %1, 127
  %10 = trunc i128 %9 to i32
  %11 = icmp eq i32 %8, %10
  %12 = sub i128 0, %6
  %13 = select i1 %11, i128 %6, i128 %12
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %3) #44
  ret i128 %13
}

; Function Attrs: nounwind optsize
define weak hidden i128 @__modti3(i128 noundef %0, i128 noundef %1) local_unnamed_addr #1 {
  %3 = alloca i128, align 16
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %3) #44
  %4 = tail call noundef i128 @llvm.abs.i128(i128 %0, i1 false)
  %5 = tail call noundef i128 @llvm.abs.i128(i128 %1, i1 false)
  %6 = call fastcc i128 @divide_u128(i128 noundef %4, i128 noundef %5, ptr noundef nonnull %3) #42
  %7 = icmp slt i128 %0, 0
  %8 = load i128, ptr %3, align 16, !tbaa !252
  %9 = sub i128 0, %8
  %10 = select i1 %7, i128 %9, i128 %8
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %3) #44
  ret i128 %10
}

; Function Attrs: optsize
declare i32 @fa_host_format_float(double noundef, i32 noundef, ptr noundef) local_unnamed_addr #37

; Function Attrs: noinline nounwind optsize
define internal fastcc void @string_grow(ptr nocapture noundef %0, ptr nocapture noundef readonly %1, i32 noundef %2, i32 noundef %3) unnamed_addr #10 {
  %5 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 1
  %6 = load i32, ptr %5, align 4, !tbaa !20
  %7 = add i32 %6, %2
  %8 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 2
  %9 = load i32, ptr %8, align 4, !tbaa !23
  %10 = and i32 %9, 2147483647
  %11 = icmp ne i32 %10, 0
  %12 = icmp ne i32 %10, 2147483647
  %13 = and i1 %11, %12
  %14 = shl nuw i32 %10, 1
  %15 = tail call i32 @llvm.umax.i32(i32 %14, i32 %7)
  %16 = select i1 %13, i32 %15, i32 %7
  %17 = tail call i32 @llvm.umax.i32(i32 %16, i32 16)
  %18 = tail call ptr @fa_alloc(i32 noundef %17, i32 noundef 1) #41
  %19 = load ptr, ptr %0, align 4, !tbaa !18
  tail call fastcc void @copy_bytes(ptr noundef %18, ptr noundef %19, i32 noundef %6) #42
  %20 = getelementptr inbounds i8, ptr %18, i32 %6
  tail call fastcc void @copy_bytes(ptr noundef %20, ptr noundef %1, i32 noundef %2) #42
  switch i32 %9, label %21 [
    i32 -1, label %23
    i32 -2147483648, label %23
    i32 2147483647, label %23
    i32 0, label %23
  ]

21:                                               ; preds = %4
  %22 = load ptr, ptr %0, align 4, !tbaa !18
  tail call void @fa_free(ptr noundef %22, i32 noundef %10, i32 noundef 1) #41
  br label %23

23:                                               ; preds = %4, %4, %4, %4, %21
  store ptr %18, ptr %0, align 4, !tbaa !18
  store i32 %7, ptr %5, align 4, !tbaa !20
  %24 = or i32 %17, %3
  store i32 %24, ptr %8, align 4, !tbaa !23
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(none)
define internal fastcc i32 @break_kind(i32 noundef %0) unnamed_addr #38 {
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
  %59 = load i32, ptr %58, align 8, !tbaa !110
  %60 = icmp ugt i32 %59, %0
  br i1 %60, label %52, label %61, !llvm.loop !255

61:                                               ; preds = %55
  %62 = getelementptr inbounds %struct.Range, ptr @EXTEND, i32 %57, i32 1
  %63 = load i32, ptr %62, align 4, !tbaa !112
  %64 = icmp ult i32 %63, %0
  %65 = add nuw i32 %57, 1
  br i1 %64, label %49, label %83, !llvm.loop !255

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
  %76 = load i32, ptr %75, align 8, !tbaa !110
  %77 = icmp ugt i32 %76, %0
  br i1 %77, label %69, label %78, !llvm.loop !255

78:                                               ; preds = %72
  %79 = getelementptr inbounds %struct.Range, ptr @PICTOGRAPHIC, i32 %74, i32 1
  %80 = load i32, ptr %79, align 4, !tbaa !112
  %81 = icmp ult i32 %80, %0
  %82 = add nuw i32 %74, 1
  br i1 %81, label %66, label %83, !llvm.loop !255

83:                                               ; preds = %61, %78, %69, %35, %29, %23, %20, %18, %3, %5, %1, %44, %13, %2
  %84 = phi i32 [ 2, %2 ], [ %17, %13 ], [ %48, %44 ], [ 1, %1 ], [ 3, %5 ], [ 3, %3 ], [ 5, %18 ], [ 6, %20 ], [ 8, %23 ], [ 9, %29 ], [ 10, %35 ], [ 0, %69 ], [ 7, %78 ], [ 4, %61 ]
  ret i32 %84
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: read)
define internal fastcc i32 @next_byte(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2, i8 noundef zeroext %3) unnamed_addr #9 {
  %5 = zext i8 %3 to i64
  %6 = mul nuw i64 %5, 72340172838076673
  br label %7

7:                                                ; preds = %13, %4
  %8 = phi i32 [ %2, %4 ], [ %9, %13 ]
  %9 = add i32 %8, 8
  %10 = icmp ugt i32 %9, %1
  br i1 %10, label %11, label %13

11:                                               ; preds = %7
  %12 = icmp ult i32 %8, %1
  br i1 %12, label %22, label %35

13:                                               ; preds = %7
  %14 = getelementptr inbounds i8, ptr %0, i32 %8
  %15 = load i64, ptr %14, align 1
  %16 = xor i64 %15, %6
  %17 = add i64 %16, -72340172838076673
  %18 = and i64 %16, -9187201950435737472
  %19 = xor i64 %18, -9187201950435737472
  %20 = and i64 %19, %17
  %21 = icmp eq i64 %20, 0
  br i1 %21, label %7, label %30, !llvm.loop !256

22:                                               ; preds = %11, %27
  %23 = phi i32 [ %28, %27 ], [ %8, %11 ]
  %24 = getelementptr inbounds i8, ptr %0, i32 %23
  %25 = load i8, ptr %24, align 1, !tbaa !5
  %26 = icmp eq i8 %25, %3
  br i1 %26, label %35, label %27

27:                                               ; preds = %22
  %28 = add nuw i32 %23, 1
  %29 = icmp ult i32 %28, %1
  br i1 %29, label %22, label %35, !llvm.loop !257

30:                                               ; preds = %13
  %31 = tail call i64 @llvm.cttz.i64(i64 %20, i1 true), !range !258
  %32 = trunc i64 %31 to i32
  %33 = lshr i32 %32, 3
  %34 = add i32 %33, %8
  br label %35

35:                                               ; preds = %22, %27, %11, %30
  %36 = phi i32 [ %34, %30 ], [ %1, %11 ], [ %23, %22 ], [ %1, %27 ]
  ret i32 %36
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define internal fastcc i32 @at_boundary(ptr nocapture noundef %0, i32 noundef %1) unnamed_addr #12 {
  %3 = load ptr, ptr %0, align 4, !tbaa !34
  %4 = getelementptr inbounds %struct.Boundaries, ptr %0, i32 0, i32 3
  %5 = load i32, ptr %4, align 4, !tbaa !38
  %6 = icmp ne i32 %5, 0
  %7 = icmp eq i32 %1, 0
  %8 = or i1 %7, %6
  br i1 %8, label %37, label %9

9:                                                ; preds = %2
  %10 = getelementptr inbounds %struct.Boundaries, ptr %0, i32 0, i32 1
  %11 = load i32, ptr %10, align 4, !tbaa !36
  %12 = icmp ugt i32 %11, %1
  br i1 %12, label %13, label %37

13:                                               ; preds = %9
  %14 = getelementptr inbounds i8, ptr %3, i32 %1
  %15 = load i8, ptr %14, align 1, !tbaa !5
  %16 = icmp sgt i8 %15, -1
  br i1 %16, label %17, label %25

17:                                               ; preds = %13
  %18 = getelementptr i8, ptr %14, i32 -1
  %19 = load i8, ptr %18, align 1, !tbaa !5
  %20 = icmp sgt i8 %19, -1
  br i1 %20, label %21, label %25

21:                                               ; preds = %17
  %22 = icmp eq i8 %19, 13
  %23 = icmp eq i8 %15, 10
  %24 = and i1 %23, %22
  br i1 %24, label %25, label %37

25:                                               ; preds = %21, %17, %13
  %26 = getelementptr inbounds %struct.Boundaries, ptr %0, i32 0, i32 2
  %27 = load i32, ptr %26, align 4, !tbaa !37
  %28 = icmp ult i32 %27, %1
  br i1 %28, label %29, label %33

29:                                               ; preds = %25, %29
  %30 = phi i32 [ %31, %29 ], [ %27, %25 ]
  %31 = tail call fastcc i32 @next_boundary(ptr noundef %3, i32 noundef %11, i32 noundef %30) #42
  store i32 %31, ptr %26, align 4, !tbaa !37
  %32 = icmp ult i32 %31, %1
  br i1 %32, label %29, label %33, !llvm.loop !259

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
declare i64 @llvm.ctlz.i64(i64, i1 immarg) #22

; Function Attrs: noinline nounwind optsize
define internal fastcc ptr @take_pages(i32 noundef %0) unnamed_addr #10 {
  br label %2

2:                                                ; preds = %6, %1
  %3 = phi ptr [ @free_spans, %1 ], [ %4, %6 ]
  %4 = load ptr, ptr %3, align 4, !tbaa !6
  %5 = icmp eq ptr %4, null
  br i1 %5, label %22, label %6

6:                                                ; preds = %2
  %7 = getelementptr inbounds %struct.FreeSpan, ptr %4, i32 0, i32 1
  %8 = load i32, ptr %7, align 4, !tbaa !114
  %9 = icmp ult i32 %8, %0
  br i1 %9, label %2, label %10

10:                                               ; preds = %6
  %11 = icmp eq i32 %8, %0
  br i1 %11, label %12, label %14

12:                                               ; preds = %10
  %13 = load ptr, ptr %4, align 4, !tbaa !117
  br label %20

14:                                               ; preds = %10
  %15 = shl i32 %0, 16
  %16 = getelementptr inbounds i8, ptr %4, i32 %15
  %17 = load ptr, ptr %4, align 4, !tbaa !117
  store ptr %17, ptr %16, align 4, !tbaa !117
  %18 = sub i32 %8, %0
  %19 = getelementptr inbounds %struct.FreeSpan, ptr %16, i32 0, i32 1
  store i32 %18, ptr %19, align 4, !tbaa !114
  br label %20

20:                                               ; preds = %14, %12
  %21 = phi ptr [ %16, %14 ], [ %13, %12 ]
  store ptr %21, ptr %3, align 4, !tbaa !6
  br label %40

22:                                               ; preds = %2
  %23 = tail call i32 @llvm.umax.i32(i32 %0, i32 16)
  %24 = tail call ptr @fa_pages(i32 noundef %23) #41
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
  store i32 %31, ptr %32, align 4, !tbaa !114
  store ptr null, ptr %30, align 4, !tbaa !117
  br label %33

33:                                               ; preds = %33, %28
  %34 = phi ptr [ @free_spans, %28 ], [ %35, %33 ]
  %35 = load ptr, ptr %34, align 4, !tbaa !6
  %36 = icmp ne ptr %35, null
  %37 = icmp ult ptr %35, %30
  %38 = and i1 %36, %37
  br i1 %38, label %33, label %39, !llvm.loop !260

39:                                               ; preds = %33
  store ptr %35, ptr %30, align 4, !tbaa !117
  store ptr %30, ptr %34, align 4, !tbaa !6
  br label %40

40:                                               ; preds = %20, %22, %26, %39
  %41 = phi ptr [ null, %22 ], [ %24, %39 ], [ %24, %26 ], [ %4, %20 ]
  ret ptr %41
}

; Function Attrs: optsize
declare ptr @fa_pages(i32 noundef) local_unnamed_addr #2

; Function Attrs: nounwind optsize
define internal fastcc void @mark(ptr noundef %0) unnamed_addr #1 {
  %2 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 17
  %3 = load i32, ptr %2, align 8, !tbaa !195
  %4 = icmp eq i32 %3, 0
  br i1 %4, label %21, label %5

5:                                                ; preds = %1
  %6 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 14
  %7 = load i32, ptr %6, align 4, !tbaa !203
  %8 = icmp eq i32 %7, 0
  br i1 %8, label %9, label %40

9:                                                ; preds = %5
  %10 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 15
  %11 = load i32, ptr %10, align 8, !tbaa !197
  %12 = icmp eq i32 %11, 0
  br i1 %12, label %13, label %40

13:                                               ; preds = %9
  store i32 1, ptr %6, align 4, !tbaa !203
  %14 = load ptr, ptr @reactive_pending, align 4, !tbaa !6
  %15 = load i64, ptr @reactive_pending_count, align 8, !tbaa !80
  %16 = tail call fastcc ptr @grow_items(ptr noundef %14, i64 noundef %15, ptr noundef nonnull @reactive_pending_capacity, i32 noundef 4) #42
  store ptr %16, ptr @reactive_pending, align 4, !tbaa !6
  %17 = load i64, ptr @reactive_pending_count, align 8, !tbaa !80
  %18 = trunc i64 %17 to i32
  %19 = getelementptr inbounds ptr, ptr %16, i32 %18
  store ptr %0, ptr %19, align 4, !tbaa !6
  %20 = add nsw i64 %17, 1
  store i64 %20, ptr @reactive_pending_count, align 8, !tbaa !80
  br label %40

21:                                               ; preds = %1
  %22 = getelementptr inbounds %struct.FaObserver, ptr %0, i32 0, i32 11
  %23 = load i32, ptr %22, align 8, !tbaa !184
  %24 = icmp eq i32 %23, 0
  br i1 %24, label %25, label %40

25:                                               ; preds = %21
  store i32 1, ptr %22, align 8, !tbaa !184
  %26 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 3
  %27 = load i64, ptr %26, align 8, !tbaa !261
  %28 = icmp sgt i64 %27, 0
  br i1 %28, label %29, label %40

29:                                               ; preds = %25
  %30 = getelementptr inbounds %struct.FaSource, ptr %0, i32 0, i32 2
  br label %31

31:                                               ; preds = %29, %31
  %32 = phi i64 [ 0, %29 ], [ %37, %31 ]
  %33 = load ptr, ptr %30, align 8, !tbaa !262
  %34 = trunc i64 %32 to i32
  %35 = getelementptr inbounds ptr, ptr %33, i32 %34
  %36 = load ptr, ptr %35, align 4, !tbaa !6
  tail call fastcc void @mark(ptr noundef %36) #42
  %37 = add nuw nsw i64 %32, 1
  %38 = load i64, ptr %26, align 8, !tbaa !261
  %39 = icmp slt i64 %37, %38
  br i1 %39, label %31, label %40, !llvm.loop !263

40:                                               ; preds = %31, %25, %21, %5, %9, %13
  ret void
}

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.abs.i64(i64, i1 immarg) #39

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.umin.i32(i32, i32) #39

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.umax.i32(i32, i32) #39

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.smin.i64(i64, i64) #39

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.smax.i64(i64, i64) #39

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.umin.i64(i64, i64) #39

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.fshl.i64(i64, i64, i64) #39

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.usub.sat.i64(i64, i64) #39

; Function Attrs: nocallback nofree nounwind willreturn memory(argmem: write)
declare void @llvm.memset.p0.i64(ptr nocapture writeonly, i8, i64, i1 immarg) #40

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i128 @llvm.abs.i128(i128, i1 immarg) #39

attributes #0 = { nounwind optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" "wasm-export-name"="flush" }
attributes #1 = { nounwind optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #2 = { optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #3 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: readwrite) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #4 = { noreturn nounwind optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #5 = { noreturn optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #6 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, argmem: none, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #7 = { mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite) }
attributes #8 = { mustprogress nocallback nofree nounwind willreturn memory(argmem: readwrite) }
attributes #9 = { nofree norecurse nosync nounwind optsize memory(argmem: read) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #10 = { noinline nounwind optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #11 = { nofree norecurse nosync nounwind optsize memory(argmem: readwrite) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #12 = { nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #13 = { nofree noinline norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #14 = { cold noinline nounwind optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #15 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #16 = { optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" "wasm-import-module"="fa" "wasm-import-name"="argumentsSize" }
attributes #17 = { optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" "wasm-import-module"="fa" "wasm-import-name"="argumentsCopy" }
attributes #18 = { mustprogress nofree noinline norecurse nosync nounwind optsize willreturn memory(none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #19 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(readwrite, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #20 = { alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #21 = { mustprogress nocallback nofree nounwind willreturn memory(argmem: write) }
attributes #22 = { mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #23 = { nofree norecurse nounwind optsize memory(readwrite, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #24 = { nofree norecurse nosync nounwind optsize memory(read, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #25 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(read, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #26 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(read, argmem: none, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #27 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #28 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: write) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #29 = { cold noinline noreturn nounwind optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #30 = { cold mustprogress nofree noinline norecurse nosync nounwind optsize willreturn memory(none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #31 = { optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" "wasm-import-module"="fa" "wasm-import-name"="pow" }
attributes #32 = { alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(read, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #33 = { alwaysinline nounwind optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #34 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, argmem: readwrite, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #35 = { mustprogress nofree norecurse nounwind optsize willreturn memory(argmem: readwrite) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #36 = { nofree norecurse nosync nounwind optsize memory(argmem: write) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #37 = { optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" "wasm-import-module"="fa" "wasm-import-name"="formatFloat" }
attributes #38 = { nofree norecurse nosync nounwind optsize memory(none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #39 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #40 = { nocallback nofree nounwind willreturn memory(argmem: write) }
attributes #41 = { nounwind optsize }
attributes #42 = { optsize }
attributes #43 = { noreturn nounwind optsize }
attributes #44 = { nounwind }
attributes #45 = { noreturn optsize }
attributes #46 = { cold optsize }
attributes #47 = { cold noreturn optsize }

!llvm.module.flags = !{!0}

!0 = !{i32 1, !"wchar_size", i32 4}
!1 = !{!2, !2, i64 0}
!2 = !{!"long", !3, i64 0}
!3 = !{!"omnipotent char", !4, i64 0}
!4 = !{!"Simple C/C++ TBAA"}
!5 = !{!3, !3, i64 0}
!6 = !{!7, !7, i64 0}
!7 = !{!"any pointer", !3, i64 0}
!8 = distinct !{!8, !9}
!9 = !{!"llvm.loop.mustprogress"}
!10 = !{i64 0, i64 4, !6, i64 4, i64 4, !1, i64 8, i64 4, !1}
!11 = distinct !{!11, !9}
!12 = !{!13, !13, i64 0}
!13 = !{!"int", !3, i64 0}
!14 = distinct !{!14, !9}
!15 = distinct !{!15, !9}
!16 = distinct !{!16, !9}
!17 = distinct !{!17, !9}
!18 = !{!19, !7, i64 0}
!19 = !{!"", !7, i64 0, !2, i64 4, !2, i64 8}
!20 = !{!19, !2, i64 4}
!21 = distinct !{!21, !9}
!22 = !{i32 0, i32 -2147483647}
!23 = !{!19, !2, i64 8}
!24 = distinct !{!24, !9}
!25 = distinct !{!25, !9}
!26 = !{!"branch_weights", i32 -294967296, i32 6003000}
!27 = distinct !{!27, !9}
!28 = distinct !{!28, !9}
!29 = !{i32 0, i32 13}
!30 = distinct !{!30, !9}
!31 = distinct !{!31, !9}
!32 = distinct !{!32, !9}
!33 = distinct !{!33, !9}
!34 = !{!35, !7, i64 0}
!35 = !{!"", !7, i64 0, !2, i64 4, !2, i64 8, !2, i64 12}
!36 = !{!35, !2, i64 4}
!37 = !{!35, !2, i64 8}
!38 = !{!35, !2, i64 12}
!39 = !{i32 0, i32 2}
!40 = distinct !{!40, !9}
!41 = distinct !{!41, !9}
!42 = distinct !{!42, !9}
!43 = distinct !{!43, !9}
!44 = !{!45, !7, i64 0}
!45 = !{!"", !7, i64 0, !2, i64 4, !2, i64 8, !7, i64 12, !2, i64 16, !13, i64 20, !13, i64 24}
!46 = !{!45, !2, i64 4}
!47 = !{!45, !2, i64 8}
!48 = !{!45, !7, i64 12}
!49 = !{!45, !2, i64 16}
!50 = !{!45, !13, i64 20}
!51 = !{!45, !13, i64 24}
!52 = !{ptr @replace_one, ptr @split_one}
!53 = distinct !{!53, !9}
!54 = distinct !{!54, !9}
!55 = !{!56, !7, i64 0}
!56 = !{!"", !7, i64 0, !57, i64 8, !57, i64 16}
!57 = !{!"long long", !3, i64 0}
!58 = !{i64 0, i64 4, !1, i64 4, i64 4, !1}
!59 = !{i64 0, i64 4, !1}
!60 = distinct !{!60, !9}
!61 = distinct !{!61, !9}
!62 = !{!63, !7, i64 0}
!63 = !{!"", !7, i64 0, !2, i64 4, !2, i64 8, !7, i64 12, !2, i64 16}
!64 = !{!63, !2, i64 4}
!65 = !{!63, !2, i64 8}
!66 = !{!56, !57, i64 8}
!67 = !{!56, !57, i64 16}
!68 = !{!63, !7, i64 12}
!69 = !{!63, !2, i64 16}
!70 = !{!"branch_weights", i32 1, i32 2000}
!71 = !{!72}
!72 = distinct !{!72, !73, !"ascii_string: argument 0"}
!73 = distinct !{!73, !"ascii_string"}
!74 = distinct !{!74, !9}
!75 = distinct !{!75, !9}
!76 = distinct !{!76, !9}
!77 = distinct !{!77, !9}
!78 = distinct !{!78, !9}
!79 = distinct !{!79, !9}
!80 = !{!57, !57, i64 0}
!81 = distinct !{!81, !9}
!82 = distinct !{!82, !9}
!83 = distinct !{!83, !9}
!84 = !{!85, !57, i64 0}
!85 = !{!"", !57, i64 0, !57, i64 8, !57, i64 16}
!86 = !{!85, !57, i64 8}
!87 = !{!85, !57, i64 16}
!88 = distinct !{!88, !9}
!89 = distinct !{!89, !9}
!90 = distinct !{!90, !9}
!91 = distinct !{!91, !9}
!92 = distinct !{!92, !9}
!93 = distinct !{!93, !9}
!94 = distinct !{!94, !9}
!95 = distinct !{!95, !9}
!96 = distinct !{!96, !9}
!97 = distinct !{!97, !9}
!98 = distinct !{!98, !9}
!99 = distinct !{!99, !9}
!100 = distinct !{!100, !9}
!101 = !{i64 0, i64 65}
!102 = !{!103, !57, i64 0}
!103 = !{!"", !57, i64 0, !57, i64 8, !3, i64 16}
!104 = !{!103, !57, i64 8}
!105 = distinct !{!105, !9}
!106 = distinct !{!106, !9}
!107 = distinct !{!107, !9}
!108 = !{!109, !7, i64 0}
!109 = !{!"FreeBlock", !7, i64 0}
!110 = !{!111, !13, i64 0}
!111 = !{!"", !13, i64 0, !13, i64 4}
!112 = !{!111, !13, i64 4}
!113 = distinct !{!113, !9}
!114 = !{!115, !2, i64 4}
!115 = !{!"FreeSpan", !7, i64 0, !2, i64 4}
!116 = distinct !{!116, !9}
!117 = !{!115, !7, i64 0}
!118 = !{!119, !13, i64 28}
!119 = !{!"", !57, i64 0, !7, i64 8, !7, i64 12, !7, i64 16, !13, i64 20, !13, i64 24, !13, i64 28, !13, i64 32, !13, i64 36, !13, i64 40}
!120 = distinct !{!120, !9}
!121 = !{!119, !13, i64 32}
!122 = !{!119, !13, i64 36}
!123 = !{!119, !13, i64 40}
!124 = !{!119, !13, i64 20}
!125 = !{!119, !7, i64 12}
!126 = distinct !{!126, !9}
!127 = !{!119, !7, i64 8}
!128 = !{!119, !57, i64 0}
!129 = !{!119, !7, i64 16}
!130 = distinct !{!130, !9}
!131 = !{!119, !13, i64 24}
!132 = distinct !{!132, !9}
!133 = distinct !{!133, !9}
!134 = distinct !{!134, !9}
!135 = !{!136, !7, i64 32}
!136 = !{!"", !57, i64 0, !7, i64 8, !57, i64 16, !57, i64 24, !7, i64 32, !57, i64 40}
!137 = !{!136, !57, i64 16}
!138 = !{!136, !7, i64 8}
!139 = distinct !{!139, !9}
!140 = !{!136, !57, i64 40}
!141 = !{!136, !57, i64 24}
!142 = !{!136, !57, i64 0}
!143 = distinct !{!143, !9}
!144 = distinct !{!144, !9}
!145 = distinct !{!145, !9}
!146 = distinct !{!146, !9}
!147 = distinct !{!147, !9}
!148 = distinct !{!148, !9}
!149 = distinct !{!149, !9}
!150 = distinct !{!150, !9}
!151 = !{!152, !57, i64 8}
!152 = !{!"", !7, i64 0, !7, i64 4, !57, i64 8}
!153 = !{!152, !7, i64 0}
!154 = !{!152, !7, i64 4}
!155 = distinct !{!155, !9}
!156 = distinct !{!156, !9}
!157 = !{!158, !57, i64 48}
!158 = !{!"FaSource", !57, i64 0, !57, i64 8, !7, i64 16, !57, i64 24, !57, i64 32, !7, i64 40, !7, i64 44, !57, i64 48, !57, i64 56}
!159 = !{!158, !7, i64 44}
!160 = !{!161, !57, i64 0}
!161 = !{!"", !57, i64 0, !57, i64 8, !7, i64 16}
!162 = !{!161, !57, i64 8}
!163 = distinct !{!163, !9}
!164 = !{!161, !7, i64 16}
!165 = !{!158, !7, i64 40}
!166 = !{i64 0, i64 8, !80, i64 8, i64 8, !80, i64 16, i64 4, !6}
!167 = !{i64 0, i64 8, !80, i64 8, i64 4, !6}
!168 = !{i64 0, i64 4, !6}
!169 = distinct !{!169, !9}
!170 = !{!171, !57, i64 96}
!171 = !{!"FaObserver", !158, i64 0, !7, i64 64, !7, i64 68, !7, i64 72, !7, i64 76, !7, i64 80, !7, i64 84, !7, i64 88, !57, i64 96, !57, i64 104, !57, i64 112, !13, i64 120, !13, i64 124, !13, i64 128, !13, i64 132, !13, i64 136, !13, i64 140, !13, i64 144, !13, i64 148, !57, i64 152, !3, i64 160}
!172 = !{!171, !7, i64 84}
!173 = distinct !{!173, !9}
!174 = !{!171, !57, i64 104}
!175 = !{!171, !7, i64 88}
!176 = !{!158, !57, i64 8}
!177 = !{!158, !57, i64 24}
!178 = !{!158, !7, i64 16}
!179 = distinct !{!179, !9}
!180 = distinct !{!180, !9}
!181 = distinct !{!181, !9}
!182 = distinct !{!182, !9}
!183 = !{!171, !57, i64 0}
!184 = !{!171, !13, i64 120}
!185 = !{!171, !7, i64 72}
!186 = !{!171, !7, i64 76}
!187 = !{!171, !7, i64 80}
!188 = !{!171, !7, i64 68}
!189 = !{!171, !7, i64 64}
!190 = !{!171, !57, i64 152}
!191 = !{!171, !13, i64 124}
!192 = !{!171, !13, i64 128}
!193 = !{!171, !13, i64 140}
!194 = !{!171, !57, i64 8}
!195 = !{!171, !13, i64 144}
!196 = !{!171, !57, i64 112}
!197 = !{!171, !13, i64 136}
!198 = distinct !{!198, !9}
!199 = distinct !{!199, !9}
!200 = distinct !{!200, !9}
!201 = distinct !{!201, !9}
!202 = distinct !{!202, !9}
!203 = !{!171, !13, i64 132}
!204 = distinct !{!204, !9}
!205 = distinct !{!205, !9}
!206 = !{!158, !57, i64 0}
!207 = !{!158, !57, i64 32}
!208 = !{!158, !57, i64 56}
!209 = distinct !{!209, !9}
!210 = !{!211, !57, i64 0}
!211 = !{!"", !57, i64 0, !19, i64 8}
!212 = distinct !{!212, !9}
!213 = distinct !{!213, !9}
!214 = distinct !{!214, !9}
!215 = !{!216, !57, i64 0}
!216 = !{!"", !57, i64 0, !19, i64 8, !19, i64 20}
!217 = !{!218, !7, i64 0}
!218 = !{!"", !7, i64 0, !57, i64 8, !57, i64 16, !57, i64 24, !7, i64 32, !57, i64 40, !57, i64 48}
!219 = !{!218, !57, i64 8}
!220 = !{!218, !57, i64 16}
!221 = !{!218, !57, i64 24}
!222 = !{!218, !7, i64 32}
!223 = !{!218, !57, i64 48}
!224 = distinct !{!224, !9}
!225 = !{!218, !57, i64 40}
!226 = distinct !{!226, !9}
!227 = distinct !{!227, !9}
!228 = distinct !{!228, !9}
!229 = distinct !{!229, !9}
!230 = distinct !{!230, !9}
!231 = distinct !{!231, !9}
!232 = distinct !{!232, !9}
!233 = distinct !{!233, !9}
!234 = !{!235, !7, i64 16}
!235 = !{!"", !57, i64 0, !57, i64 8, !7, i64 16, !57, i64 24}
!236 = !{!235, !57, i64 8}
!237 = !{!238, !57, i64 32}
!238 = !{!"", !235, i64 0, !57, i64 32}
!239 = !{!235, !57, i64 0}
!240 = !{!241, !57, i64 160}
!241 = !{!"", !235, i64 0, !242, i64 32, !242, i64 96, !57, i64 160, !57, i64 168, !7, i64 176, !57, i64 184, !57, i64 192, !57, i64 200, !13, i64 208, !13, i64 212, !7, i64 216}
!242 = !{!"", !3, i64 0}
!243 = !{!241, !57, i64 168}
!244 = !{!241, !7, i64 216}
!245 = !{!241, !57, i64 192}
!246 = !{!241, !57, i64 200}
!247 = !{!241, !57, i64 184}
!248 = !{!241, !7, i64 176}
!249 = distinct !{!249, !9}
!250 = !{!241, !13, i64 208}
!251 = distinct !{!251, !9}
!252 = !{!253, !253, i64 0}
!253 = !{!"__int128", !3, i64 0}
!254 = distinct !{!254, !9}
!255 = distinct !{!255, !9}
!256 = distinct !{!256, !9}
!257 = distinct !{!257, !9}
!258 = !{i64 7, i64 65}
!259 = distinct !{!259, !9}
!260 = distinct !{!260, !9}
!261 = !{!171, !57, i64 24}
!262 = !{!171, !7, i64 16}
!263 = distinct !{!263, !9}
