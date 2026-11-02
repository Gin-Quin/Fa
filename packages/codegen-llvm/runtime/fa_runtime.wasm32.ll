source_filename = "fa_runtime.c"

%struct.BareError = type { i64 }
%struct.Range = type { i32, i32 }
%struct.FaString = type { ptr, i32, i32 }
%struct.Replacing = type { ptr, i32, i32, ptr, i32, i32, i32 }
%struct.FaArrayHeader = type { ptr, i64, i64 }
%struct.Splitting = type { ptr, i32, i32, ptr }
%struct.IndexError = type { i64, i64, i64 }
%struct.FaSegmented = type { i64, i64, [48 x ptr] }
%struct.PageHeader = type { i32, i32 }
%struct.FreeSpan = type { ptr, i32 }
%struct.MessageError = type { i64, %struct.FaString }
%struct.ParseError = type { i64, %struct.FaString, %struct.FaString }
%struct.FaTable = type { ptr, i64, i64, i64, ptr, i64, i64 }

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
@class_next = internal unnamed_addr global [40 x ptr] zeroinitializer, align 16
@class_end = internal unnamed_addr global [40 x ptr] zeroinitializer, align 16
@builder_capacity = internal unnamed_addr global i32 0, align 4
@fa_live_blocks = hidden local_unnamed_addr global i32 0, align 4
@.str.13 = private unnamed_addr constant [7 x i8] c"leak: \00", align 1
@.str.14 = private unnamed_addr constant [8 x i8] c" blocks\00", align 1
@.str.15 = private unnamed_addr constant [19 x i8] c"integer overflow: \00", align 1
@.str.16 = private unnamed_addr constant [17 x i8] c" doesn't fit in \00", align 1
@.str.17 = private unnamed_addr constant [44 x i8] c"integer overflow: the value doesn't fit in \00", align 1
@.str.18 = private unnamed_addr constant [19 x i8] c"assertion failed: \00", align 1
@.str.19 = private unnamed_addr constant [13 x i8] c"\0A       \E2\86\B3 \00", align 1
@.str.20 = private unnamed_addr constant [8 x i8] c"error: \00", align 1
@.str.21 = private unnamed_addr constant [2 x i8] c"\0A\00", align 1
@division_by_zero = internal constant %struct.BareError { i64 3 }, align 8
@.str.22 = private unnamed_addr constant [20 x i8] c"negative exponent: \00", align 1
@.str.23 = private unnamed_addr constant [24 x i8] c" doesn't fit in Integer\00", align 1
@.str.24 = private unnamed_addr constant [24 x i8] c" is not a finite number\00", align 1
@.str.25 = private unnamed_addr constant [14 x i8] c"shift amount \00", align 1
@.str.26 = private unnamed_addr constant [16 x i8] c" is outside 0..\00", align 1
@.str.27 = private unnamed_addr constant [18 x i8] c"the text is empty\00", align 1
@.str.28 = private unnamed_addr constant [20 x i8] c"\22 is not an integer\00", align 1
@.str.29 = private unnamed_addr constant [1 x i8] zeroinitializer, align 1
@EXTEND = internal unnamed_addr constant [153 x %struct.Range] [%struct.Range { i32 768, i32 879 }, %struct.Range { i32 1155, i32 1161 }, %struct.Range { i32 1425, i32 1469 }, %struct.Range { i32 1471, i32 1471 }, %struct.Range { i32 1473, i32 1474 }, %struct.Range { i32 1476, i32 1477 }, %struct.Range { i32 1479, i32 1479 }, %struct.Range { i32 1552, i32 1562 }, %struct.Range { i32 1611, i32 1631 }, %struct.Range { i32 1648, i32 1648 }, %struct.Range { i32 1750, i32 1756 }, %struct.Range { i32 1759, i32 1764 }, %struct.Range { i32 1767, i32 1768 }, %struct.Range { i32 1770, i32 1773 }, %struct.Range { i32 1809, i32 1809 }, %struct.Range { i32 1840, i32 1866 }, %struct.Range { i32 1958, i32 1968 }, %struct.Range { i32 2027, i32 2035 }, %struct.Range { i32 2070, i32 2093 }, %struct.Range { i32 2137, i32 2139 }, %struct.Range { i32 2200, i32 2207 }, %struct.Range { i32 2250, i32 2273 }, %struct.Range { i32 2275, i32 2307 }, %struct.Range { i32 2362, i32 2364 }, %struct.Range { i32 2366, i32 2383 }, %struct.Range { i32 2385, i32 2391 }, %struct.Range { i32 2402, i32 2403 }, %struct.Range { i32 2433, i32 2435 }, %struct.Range { i32 2492, i32 2492 }, %struct.Range { i32 2494, i32 2509 }, %struct.Range { i32 2519, i32 2519 }, %struct.Range { i32 2530, i32 2531 }, %struct.Range { i32 2561, i32 2563 }, %struct.Range { i32 2620, i32 2641 }, %struct.Range { i32 2672, i32 2673 }, %struct.Range { i32 2677, i32 2677 }, %struct.Range { i32 2689, i32 2691 }, %struct.Range { i32 2748, i32 2748 }, %struct.Range { i32 2750, i32 2765 }, %struct.Range { i32 2786, i32 2787 }, %struct.Range { i32 2817, i32 2819 }, %struct.Range { i32 2876, i32 2876 }, %struct.Range { i32 2878, i32 2903 }, %struct.Range { i32 2914, i32 2915 }, %struct.Range { i32 2946, i32 2946 }, %struct.Range { i32 3006, i32 3021 }, %struct.Range { i32 3031, i32 3031 }, %struct.Range { i32 3072, i32 3076 }, %struct.Range { i32 3132, i32 3132 }, %struct.Range { i32 3134, i32 3158 }, %struct.Range { i32 3170, i32 3171 }, %struct.Range { i32 3201, i32 3203 }, %struct.Range { i32 3260, i32 3260 }, %struct.Range { i32 3262, i32 3286 }, %struct.Range { i32 3298, i32 3299 }, %struct.Range { i32 3328, i32 3331 }, %struct.Range { i32 3387, i32 3388 }, %struct.Range { i32 3390, i32 3405 }, %struct.Range { i32 3415, i32 3415 }, %struct.Range { i32 3426, i32 3427 }, %struct.Range { i32 3457, i32 3459 }, %struct.Range { i32 3530, i32 3551 }, %struct.Range { i32 3570, i32 3571 }, %struct.Range { i32 3633, i32 3633 }, %struct.Range { i32 3635, i32 3642 }, %struct.Range { i32 3655, i32 3662 }, %struct.Range { i32 3761, i32 3761 }, %struct.Range { i32 3763, i32 3772 }, %struct.Range { i32 3784, i32 3790 }, %struct.Range { i32 3864, i32 3865 }, %struct.Range { i32 3893, i32 3893 }, %struct.Range { i32 3895, i32 3895 }, %struct.Range { i32 3897, i32 3897 }, %struct.Range { i32 3902, i32 3903 }, %struct.Range { i32 3953, i32 3972 }, %struct.Range { i32 3974, i32 3975 }, %struct.Range { i32 3981, i32 4028 }, %struct.Range { i32 4038, i32 4038 }, %struct.Range { i32 4139, i32 4158 }, %struct.Range { i32 4182, i32 4185 }, %struct.Range { i32 4190, i32 4192 }, %struct.Range { i32 4194, i32 4196 }, %struct.Range { i32 4199, i32 4205 }, %struct.Range { i32 4209, i32 4212 }, %struct.Range { i32 4226, i32 4237 }, %struct.Range { i32 4239, i32 4239 }, %struct.Range { i32 4250, i32 4253 }, %struct.Range { i32 4957, i32 4959 }, %struct.Range { i32 5906, i32 5909 }, %struct.Range { i32 5938, i32 5940 }, %struct.Range { i32 5970, i32 5971 }, %struct.Range { i32 6002, i32 6003 }, %struct.Range { i32 6068, i32 6099 }, %struct.Range { i32 6109, i32 6109 }, %struct.Range { i32 6155, i32 6157 }, %struct.Range { i32 6159, i32 6159 }, %struct.Range { i32 6277, i32 6278 }, %struct.Range { i32 6313, i32 6313 }, %struct.Range { i32 6432, i32 6459 }, %struct.Range { i32 6679, i32 6683 }, %struct.Range { i32 6741, i32 6783 }, %struct.Range { i32 6832, i32 6911 }, %struct.Range { i32 6912, i32 6916 }, %struct.Range { i32 6964, i32 6980 }, %struct.Range { i32 7019, i32 7027 }, %struct.Range { i32 7040, i32 7042 }, %struct.Range { i32 7073, i32 7085 }, %struct.Range { i32 7142, i32 7155 }, %struct.Range { i32 7204, i32 7223 }, %struct.Range { i32 7376, i32 7378 }, %struct.Range { i32 7380, i32 7400 }, %struct.Range { i32 7405, i32 7405 }, %struct.Range { i32 7412, i32 7412 }, %struct.Range { i32 7415, i32 7417 }, %struct.Range { i32 7616, i32 7679 }, %struct.Range { i32 8204, i32 8204 }, %struct.Range { i32 8400, i32 8432 }, %struct.Range { i32 11503, i32 11505 }, %struct.Range { i32 11647, i32 11647 }, %struct.Range { i32 11744, i32 11775 }, %struct.Range { i32 12330, i32 12335 }, %struct.Range { i32 12441, i32 12442 }, %struct.Range { i32 42607, i32 42610 }, %struct.Range { i32 42612, i32 42621 }, %struct.Range { i32 42654, i32 42655 }, %struct.Range { i32 42736, i32 42737 }, %struct.Range { i32 43010, i32 43010 }, %struct.Range { i32 43014, i32 43014 }, %struct.Range { i32 43019, i32 43019 }, %struct.Range { i32 43043, i32 43047 }, %struct.Range { i32 43052, i32 43052 }, %struct.Range { i32 43136, i32 43137 }, %struct.Range { i32 43188, i32 43205 }, %struct.Range { i32 43232, i32 43249 }, %struct.Range { i32 43263, i32 43263 }, %struct.Range { i32 43302, i32 43309 }, %struct.Range { i32 43335, i32 43347 }, %struct.Range { i32 43392, i32 43395 }, %struct.Range { i32 43443, i32 43456 }, %struct.Range { i32 43561, i32 43574 }, %struct.Range { i32 43587, i32 43587 }, %struct.Range { i32 43596, i32 43597 }, %struct.Range { i32 43755, i32 43759 }, %struct.Range { i32 43765, i32 43766 }, %struct.Range { i32 44003, i32 44010 }, %struct.Range { i32 44012, i32 44013 }, %struct.Range { i32 64286, i32 64286 }, %struct.Range { i32 65024, i32 65039 }, %struct.Range { i32 65056, i32 65071 }, %struct.Range { i32 65438, i32 65439 }, %struct.Range { i32 127995, i32 127999 }, %struct.Range { i32 917536, i32 917631 }, %struct.Range { i32 917760, i32 917999 }], align 16
@PICTOGRAPHIC = internal unnamed_addr constant [22 x %struct.Range] [%struct.Range { i32 169, i32 169 }, %struct.Range { i32 174, i32 174 }, %struct.Range { i32 8252, i32 8252 }, %struct.Range { i32 8265, i32 8265 }, %struct.Range { i32 8482, i32 8482 }, %struct.Range { i32 8505, i32 8505 }, %struct.Range { i32 8596, i32 8618 }, %struct.Range { i32 8986, i32 9215 }, %struct.Range { i32 9410, i32 9410 }, %struct.Range { i32 9642, i32 9726 }, %struct.Range { i32 9728, i32 10175 }, %struct.Range { i32 10548, i32 10549 }, %struct.Range { i32 11013, i32 11093 }, %struct.Range { i32 12336, i32 12336 }, %struct.Range { i32 12349, i32 12349 }, %struct.Range { i32 12951, i32 12951 }, %struct.Range { i32 12953, i32 12953 }, %struct.Range { i32 126976, i32 127231 }, %struct.Range { i32 127245, i32 127405 }, %struct.Range { i32 127489, i32 127994 }, %struct.Range { i32 128000, i32 129791 }, %struct.Range { i32 130048, i32 131069 }], align 16
@free_spans = internal unnamed_addr global ptr null, align 4
@.str.30 = private unnamed_addr constant [7 x i8] c"stop: \00", align 1
@.str.31 = private unnamed_addr constant [7 x i8] c"\0A  at \00", align 1
@llvm.used = appending global [1 x ptr] [ptr @fa_flush], section "llvm.metadata"

; Function Attrs: nounwind optsize
define hidden void @fa_flush() #0 {
  %1 = load i32, ptr @output_length, align 4, !tbaa !1
  %2 = icmp eq i32 %1, 0
  br i1 %2, label %4, label %3

3:                                                ; preds = %0
  tail call void @fa_write(i32 noundef 1, ptr noundef nonnull @output, i32 noundef %1) #29
  store i32 0, ptr @output_length, align 4, !tbaa !1
  br label %4

4:                                                ; preds = %3, %0
  ret void
}

; Function Attrs: optsize
declare void @fa_write(i32 noundef, ptr noundef, i32 noundef) local_unnamed_addr #1

; Function Attrs: nounwind optsize
define hidden void @fa_out(i32 noundef %0, ptr noundef %1, i32 noundef %2) local_unnamed_addr #2 {
  %4 = icmp eq i32 %0, 1
  %5 = load i32, ptr @output_length, align 4, !tbaa !1
  br i1 %4, label %10, label %6

6:                                                ; preds = %3
  %7 = icmp eq i32 %5, 0
  br i1 %7, label %9, label %8

8:                                                ; preds = %6
  tail call void @fa_write(i32 noundef 1, ptr noundef nonnull @output, i32 noundef %5) #29
  store i32 0, ptr @output_length, align 4, !tbaa !1
  br label %9

9:                                                ; preds = %6, %8
  tail call void @fa_write(i32 noundef %0, ptr noundef %1, i32 noundef %2) #29
  br label %23

10:                                               ; preds = %3
  %11 = add i32 %5, %2
  %12 = icmp ugt i32 %11, 8192
  br i1 %12, label %13, label %19

13:                                               ; preds = %10
  %14 = icmp eq i32 %5, 0
  br i1 %14, label %16, label %15

15:                                               ; preds = %13
  tail call void @fa_write(i32 noundef 1, ptr noundef nonnull @output, i32 noundef %5) #29
  store i32 0, ptr @output_length, align 4, !tbaa !1
  br label %16

16:                                               ; preds = %13, %15
  %17 = icmp ugt i32 %2, 8192
  br i1 %17, label %18, label %19

18:                                               ; preds = %16
  tail call void @fa_write(i32 noundef 1, ptr noundef %1, i32 noundef %2) #29
  br label %23

19:                                               ; preds = %16, %10
  %20 = phi i32 [ %5, %10 ], [ 0, %16 ]
  %21 = getelementptr inbounds i8, ptr @output, i32 %20
  tail call void @llvm.memcpy.p0.p0.i32(ptr nonnull align 1 %21, ptr align 1 %1, i32 %2, i1 false)
  %22 = add i32 %20, %2
  store i32 %22, ptr @output_length, align 4, !tbaa !1
  br label %23

23:                                               ; preds = %19, %18, %9
  ret void
}

; Function Attrs: noreturn nounwind optsize
define hidden void @fa_terminate(i32 noundef %0) local_unnamed_addr #3 {
  tail call void @fa_flush() #30
  tail call void @fa_exit(i32 noundef %0) #31
  unreachable
}

; Function Attrs: noreturn optsize
declare void @fa_exit(i32 noundef) local_unnamed_addr #4

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, argmem: none, inaccessiblemem: none)
define hidden void @fa_b_reset() local_unnamed_addr #5 {
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_bytes(ptr nocapture noundef readonly %0, i32 noundef %1) local_unnamed_addr #2 {
  tail call fastcc void @reserve(i32 noundef %1) #30
  %3 = load ptr, ptr @builder, align 4, !tbaa !5
  %4 = load i32, ptr @builder_length, align 4, !tbaa !1
  %5 = getelementptr inbounds i8, ptr %3, i32 %4
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %5, ptr align 1 %0, i32 %1, i1 false)
  %6 = add i32 %4, %1
  store i32 %6, ptr @builder_length, align 4, !tbaa !1
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @reserve(i32 noundef %0) unnamed_addr #2 {
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
  br i1 %12, label %10, label %14, !llvm.loop !7

14:                                               ; preds = %10
  %15 = tail call ptr @fa_alloc(i32 noundef %11, i32 noundef 1) #29
  %16 = load ptr, ptr @builder, align 4, !tbaa !5
  %17 = load i32, ptr @builder_length, align 4, !tbaa !1
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %15, ptr align 1 %16, i32 %17, i1 false)
  %18 = icmp eq ptr %16, null
  br i1 %18, label %21, label %19

19:                                               ; preds = %14
  %20 = load i32, ptr @builder_capacity, align 4, !tbaa !1
  tail call void @fa_free(ptr noundef nonnull %16, i32 noundef %20, i32 noundef 1) #29
  br label %21

21:                                               ; preds = %19, %14
  store ptr %15, ptr @builder, align 4, !tbaa !5
  store i32 %11, ptr @builder_capacity, align 4, !tbaa !1
  br label %22

22:                                               ; preds = %1, %21
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_line(i32 noundef %0) local_unnamed_addr #2 {
  tail call fastcc void @reserve(i32 noundef 1) #30
  %2 = load ptr, ptr @builder, align 4, !tbaa !5
  %3 = load i32, ptr @builder_length, align 4, !tbaa !1
  %4 = getelementptr inbounds i8, ptr %2, i32 %3
  store i8 10, ptr %4, align 1, !tbaa !9
  %5 = add i32 %3, 1
  store i32 %5, ptr @builder_length, align 4, !tbaa !1
  tail call void @fa_out(i32 noundef %0, ptr noundef %2, i32 noundef %5) #30
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_take(ptr nocapture noundef writeonly %0) local_unnamed_addr #2 {
  %2 = load i32, ptr @builder_length, align 4, !tbaa !1
  %3 = icmp eq i32 %2, 0
  br i1 %3, label %7, label %4

4:                                                ; preds = %1
  %5 = load ptr, ptr @builder, align 4, !tbaa !5
  %6 = tail call ptr @fa_alloc(i32 noundef %2, i32 noundef 1) #29, !noalias !10
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %6, ptr align 1 %5, i32 %2, i1 false), !noalias !10
  br label %7

7:                                                ; preds = %1, %4
  %8 = phi ptr [ %6, %4 ], [ @.str.29, %1 ]
  store ptr %8, ptr %0, align 4, !tbaa.struct !13
  %9 = getelementptr inbounds i8, ptr %0, i32 4
  store i32 %2, ptr %9, align 4, !tbaa.struct !14
  %10 = getelementptr inbounds i8, ptr %0, i32 8
  store i32 %2, ptr %10, align 4, !tbaa.struct !15
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  ret void
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.start.p0(i64 immarg, ptr nocapture) #6

; Function Attrs: mustprogress nocallback nofree nounwind willreturn memory(argmem: readwrite)
declare void @llvm.memcpy.p0.p0.i32(ptr noalias nocapture writeonly, ptr noalias nocapture readonly, i32, i1 immarg) #7

; Function Attrs: mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.end.p0(i64 immarg, ptr nocapture) #6

; Function Attrs: nounwind optsize
define hidden void @fa_b_i64(i64 noundef %0) local_unnamed_addr #2 {
  %2 = tail call i64 @llvm.abs.i64(i64 %0, i1 false)
  %3 = lshr i64 %0, 63
  %4 = trunc i64 %3 to i32
  tail call fastcc void @append_u64(i64 noundef %2, i32 noundef %4) #30
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @append_u64(i64 noundef %0, i32 noundef %1) unnamed_addr #2 {
  %3 = alloca [24 x i8], align 16
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %3) #32
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
  store i8 %13, ptr %14, align 1, !tbaa !9
  %15 = icmp ult i64 %5, 10
  br i1 %15, label %16, label %4, !llvm.loop !16

16:                                               ; preds = %4
  %17 = icmp eq i32 %1, 0
  br i1 %17, label %21, label %18

18:                                               ; preds = %16
  %19 = add i32 %6, -2
  %20 = getelementptr inbounds [24 x i8], ptr %3, i32 0, i32 %19
  store i8 45, ptr %20, align 1, !tbaa !9
  br label %21

21:                                               ; preds = %18, %16
  %22 = phi i32 [ %19, %18 ], [ %7, %16 ]
  %23 = getelementptr inbounds i8, ptr %3, i32 %22
  %24 = sub i32 24, %22
  tail call fastcc void @reserve(i32 noundef %24) #30
  %25 = load ptr, ptr @builder, align 4, !tbaa !5
  %26 = load i32, ptr @builder_length, align 4, !tbaa !1
  %27 = getelementptr inbounds i8, ptr %25, i32 %26
  call void @llvm.memcpy.p0.p0.i32(ptr align 1 %27, ptr nonnull align 1 %23, i32 %24, i1 false)
  %28 = add i32 %26, %24
  store i32 %28, ptr @builder_length, align 4, !tbaa !1
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %3) #32
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_u64(i64 noundef %0) local_unnamed_addr #2 {
  tail call fastcc void @append_u64(i64 noundef %0, i32 noundef 0) #30
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_i128(i64 noundef %0, i64 noundef %1) local_unnamed_addr #2 {
  %3 = zext i64 %1 to i128
  %4 = shl nuw i128 %3, 64
  %5 = zext i64 %0 to i128
  %6 = or disjoint i128 %4, %5
  %7 = icmp slt i64 %1, 0
  %8 = sub i128 0, %6
  %9 = select i1 %7, i128 %8, i128 %6
  %10 = lshr i64 %1, 63
  %11 = trunc i64 %10 to i32
  tail call fastcc void @append_u128(i128 noundef %9, i32 noundef %11) #30
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @append_u128(i128 noundef %0, i32 noundef %1) unnamed_addr #2 {
  %3 = alloca [4 x i32], align 16
  %4 = alloca [48 x i8], align 16
  %5 = icmp ult i128 %0, 18446744073709551616
  br i1 %5, label %6, label %8

6:                                                ; preds = %2
  %7 = trunc i128 %0 to i64
  tail call fastcc void @append_u64(i64 noundef %7, i32 noundef %1) #30
  br label %80

8:                                                ; preds = %2
  %9 = lshr i128 %0, 64
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %3) #32
  %10 = lshr i128 %0, 96
  %11 = trunc i128 %10 to i32
  store i32 %11, ptr %3, align 16, !tbaa !17
  %12 = getelementptr inbounds i32, ptr %3, i32 1
  %13 = trunc i128 %9 to i32
  store i32 %13, ptr %12, align 4, !tbaa !17
  %14 = getelementptr inbounds i32, ptr %3, i32 2
  %15 = lshr i128 %0, 32
  %16 = trunc i128 %15 to i32
  store i32 %16, ptr %14, align 8, !tbaa !17
  %17 = getelementptr inbounds i32, ptr %3, i32 3
  %18 = trunc i128 %0 to i32
  store i32 %18, ptr %17, align 4, !tbaa !17
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %4) #32
  br label %19

19:                                               ; preds = %48, %8
  %20 = phi i32 [ 48, %8 ], [ %53, %48 ]
  br label %21

21:                                               ; preds = %21, %19
  %22 = phi i64 [ 0, %19 ], [ %33, %21 ]
  %23 = phi i32 [ 0, %19 ], [ %34, %21 ]
  %24 = shl nuw nsw i64 %22, 32
  %25 = getelementptr inbounds i32, ptr %3, i32 %23
  %26 = load i32, ptr %25, align 4, !tbaa !17
  %27 = zext i32 %26 to i64
  %28 = or disjoint i64 %24, %27
  %29 = freeze i64 %28
  %30 = udiv i64 %29, 1000000000
  %31 = trunc i64 %30 to i32
  store i32 %31, ptr %25, align 4, !tbaa !17
  %32 = mul i64 %30, 1000000000
  %33 = sub i64 %29, %32
  %34 = add nuw nsw i32 %23, 1
  %35 = icmp eq i32 %34, 4
  br i1 %35, label %36, label %21, !llvm.loop !19

36:                                               ; preds = %21
  %37 = trunc i64 %33 to i32
  %38 = load i32, ptr %3, align 16, !tbaa !17
  %39 = load i32, ptr %12, align 4, !tbaa !17
  %40 = or i32 %39, %38
  %41 = load i32, ptr %14, align 8, !tbaa !17
  %42 = or i32 %40, %41
  %43 = load i32, ptr %17, align 4, !tbaa !17
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
  store i8 %59, ptr %60, align 1, !tbaa !9
  %61 = add nuw nsw i32 %50, 1
  %62 = icmp ult i32 %50, 8
  %63 = icmp ugt i32 %51, 9
  %64 = select i1 %45, i1 true, i1 %63
  %65 = select i1 %62, i1 %64, i1 false
  br i1 %65, label %49, label %48, !llvm.loop !20

66:                                               ; preds = %36, %48
  %67 = phi i32 [ %53, %48 ], [ %20, %36 ]
  %68 = icmp eq i32 %1, 0
  br i1 %68, label %72, label %69

69:                                               ; preds = %66
  %70 = add i32 %67, -1
  %71 = getelementptr inbounds [48 x i8], ptr %4, i32 0, i32 %70
  store i8 45, ptr %71, align 1, !tbaa !9
  br label %72

72:                                               ; preds = %69, %66
  %73 = phi i32 [ %70, %69 ], [ %67, %66 ]
  %74 = getelementptr inbounds i8, ptr %4, i32 %73
  %75 = sub i32 48, %73
  tail call fastcc void @reserve(i32 noundef %75) #30
  %76 = load ptr, ptr @builder, align 4, !tbaa !5
  %77 = load i32, ptr @builder_length, align 4, !tbaa !1
  %78 = getelementptr inbounds i8, ptr %76, i32 %77
  call void @llvm.memcpy.p0.p0.i32(ptr align 1 %78, ptr nonnull align 1 %74, i32 %75, i1 false)
  %79 = add i32 %77, %75
  store i32 %79, ptr @builder_length, align 4, !tbaa !1
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %4) #32
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %3) #32
  br label %80

80:                                               ; preds = %72, %6
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_u128(i64 noundef %0, i64 noundef %1) local_unnamed_addr #2 {
  %3 = zext i64 %1 to i128
  %4 = shl nuw i128 %3, 64
  %5 = zext i64 %0 to i128
  %6 = or disjoint i128 %4, %5
  tail call fastcc void @append_u128(i128 noundef %6, i32 noundef 0) #30
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_bool(i32 noundef %0) local_unnamed_addr #2 {
  %2 = icmp eq i32 %0, 0
  %3 = select i1 %2, ptr @.str.1, ptr @.str
  tail call fastcc void @append_c(ptr noundef nonnull %3) #30
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @append_c(ptr nocapture noundef readonly %0) unnamed_addr #2 {
  br label %2

2:                                                ; preds = %2, %1
  %3 = phi i32 [ 0, %1 ], [ %7, %2 ]
  %4 = getelementptr inbounds i8, ptr %0, i32 %3
  %5 = load i8, ptr %4, align 1, !tbaa !9
  %6 = icmp eq i8 %5, 0
  %7 = add i32 %3, 1
  br i1 %6, label %8, label %2, !llvm.loop !21

8:                                                ; preds = %2
  tail call fastcc void @reserve(i32 noundef %3) #30
  %9 = load ptr, ptr @builder, align 4, !tbaa !5
  %10 = load i32, ptr @builder_length, align 4, !tbaa !1
  %11 = getelementptr inbounds i8, ptr %9, i32 %10
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %11, ptr nonnull align 1 %0, i32 %3, i1 false)
  %12 = add i32 %10, %3
  store i32 %12, ptr @builder_length, align 4, !tbaa !1
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_f64(double noundef %0) local_unnamed_addr #2 {
  tail call fastcc void @reserve(i32 noundef 40) #30
  %2 = load ptr, ptr @builder, align 4, !tbaa !5
  %3 = load i32, ptr @builder_length, align 4, !tbaa !1
  %4 = getelementptr inbounds i8, ptr %2, i32 %3
  %5 = tail call i32 @fa_host_format_float(double noundef %0, i32 noundef 0, ptr noundef %4) #29
  %6 = load i32, ptr @builder_length, align 4, !tbaa !1
  %7 = add i32 %6, %5
  store i32 %7, ptr @builder_length, align 4, !tbaa !1
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_f32(float noundef %0) local_unnamed_addr #2 {
  %2 = fpext float %0 to double
  tail call fastcc void @reserve(i32 noundef 40) #30
  %3 = load ptr, ptr @builder, align 4, !tbaa !5
  %4 = load i32, ptr @builder_length, align 4, !tbaa !1
  %5 = getelementptr inbounds i8, ptr %3, i32 %4
  %6 = tail call i32 @fa_host_format_float(double noundef %2, i32 noundef 1, ptr noundef %5) #29
  %7 = load i32, ptr @builder_length, align 4, !tbaa !1
  %8 = add i32 %7, %6
  store i32 %8, ptr @builder_length, align 4, !tbaa !1
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_percentage(double noundef %0) local_unnamed_addr #2 {
  tail call fastcc void @reserve(i32 noundef 40) #30
  %2 = load ptr, ptr @builder, align 4, !tbaa !5
  %3 = load i32, ptr @builder_length, align 4, !tbaa !1
  %4 = getelementptr inbounds i8, ptr %2, i32 %3
  %5 = tail call i32 @fa_host_format_float(double noundef %0, i32 noundef 2, ptr noundef %4) #29
  %6 = load i32, ptr @builder_length, align 4, !tbaa !1
  %7 = add i32 %6, %5
  store i32 %7, ptr @builder_length, align 4, !tbaa !1
  tail call fastcc void @reserve(i32 noundef 1) #30
  %8 = load ptr, ptr @builder, align 4, !tbaa !5
  %9 = load i32, ptr @builder_length, align 4, !tbaa !1
  %10 = getelementptr inbounds i8, ptr %8, i32 %9
  store i8 37, ptr %10, align 1, !tbaa !9
  %11 = add i32 %9, 1
  store i32 %11, ptr @builder_length, align 4, !tbaa !1
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_quoted(ptr nocapture noundef readonly %0, i32 noundef %1) local_unnamed_addr #2 {
  tail call fastcc void @reserve(i32 noundef 1) #30
  %3 = load ptr, ptr @builder, align 4, !tbaa !5
  %4 = load i32, ptr @builder_length, align 4, !tbaa !1
  %5 = getelementptr inbounds i8, ptr %3, i32 %4
  store i8 34, ptr %5, align 1, !tbaa !9
  %6 = add i32 %4, 1
  store i32 %6, ptr @builder_length, align 4, !tbaa !1
  %7 = icmp eq i32 %1, 0
  br i1 %7, label %8, label %13

8:                                                ; preds = %50, %2
  tail call fastcc void @reserve(i32 noundef 1) #30
  %9 = load ptr, ptr @builder, align 4, !tbaa !5
  %10 = load i32, ptr @builder_length, align 4, !tbaa !1
  %11 = getelementptr inbounds i8, ptr %9, i32 %10
  store i8 34, ptr %11, align 1, !tbaa !9
  %12 = add i32 %10, 1
  store i32 %12, ptr @builder_length, align 4, !tbaa !1
  ret void

13:                                               ; preds = %2, %50
  %14 = phi i32 [ %51, %50 ], [ 0, %2 ]
  %15 = getelementptr inbounds i8, ptr %0, i32 %14
  %16 = load i8, ptr %15, align 1, !tbaa !9
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
  tail call fastcc void @append_c(ptr noundef nonnull @.str.2) #30
  br label %50

19:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.3) #30
  br label %50

20:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.4) #30
  br label %50

21:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.5) #30
  br label %50

22:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.6) #30
  br label %50

23:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.7) #30
  br label %50

24:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.8) #30
  br label %50

25:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.9) #30
  br label %50

26:                                               ; preds = %13
  %27 = icmp ult i8 %16, 32
  %28 = icmp eq i8 %16, 127
  %29 = or i1 %27, %28
  br i1 %29, label %30, label %45

30:                                               ; preds = %26
  tail call fastcc void @append_c(ptr noundef nonnull @.str.10) #30
  %31 = lshr i32 %17, 4
  %32 = getelementptr inbounds [17 x i8], ptr @fa_b_quoted.hex, i32 0, i32 %31
  %33 = load i8, ptr %32, align 1, !tbaa !9
  tail call fastcc void @reserve(i32 noundef 1) #30
  %34 = load ptr, ptr @builder, align 4, !tbaa !5
  %35 = load i32, ptr @builder_length, align 4, !tbaa !1
  %36 = getelementptr inbounds i8, ptr %34, i32 %35
  store i8 %33, ptr %36, align 1, !tbaa !9
  %37 = add i32 %35, 1
  store i32 %37, ptr @builder_length, align 4, !tbaa !1
  %38 = and i32 %17, 15
  %39 = getelementptr inbounds [17 x i8], ptr @fa_b_quoted.hex, i32 0, i32 %38
  %40 = load i8, ptr %39, align 1, !tbaa !9
  tail call fastcc void @reserve(i32 noundef 1) #30
  %41 = load ptr, ptr @builder, align 4, !tbaa !5
  %42 = load i32, ptr @builder_length, align 4, !tbaa !1
  %43 = getelementptr inbounds i8, ptr %41, i32 %42
  store i8 %40, ptr %43, align 1, !tbaa !9
  %44 = add i32 %42, 1
  store i32 %44, ptr @builder_length, align 4, !tbaa !1
  br label %50

45:                                               ; preds = %26
  tail call fastcc void @reserve(i32 noundef 1) #30
  %46 = load ptr, ptr @builder, align 4, !tbaa !5
  %47 = load i32, ptr @builder_length, align 4, !tbaa !1
  %48 = getelementptr inbounds i8, ptr %46, i32 %47
  store i8 %16, ptr %48, align 1, !tbaa !9
  %49 = add i32 %47, 1
  store i32 %49, ptr @builder_length, align 4, !tbaa !1
  br label %50

50:                                               ; preds = %30, %45, %25, %24, %23, %22, %21, %20, %19, %18
  %51 = add nuw i32 %14, 1
  %52 = icmp eq i32 %51, %1
  br i1 %52, label %8, label %13, !llvm.loop !22
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_string_array(ptr nocapture noundef readonly %0, i32 noundef %1) local_unnamed_addr #2 {
  tail call fastcc void @reserve(i32 noundef 1) #30
  %3 = load ptr, ptr @builder, align 4, !tbaa !5
  %4 = load i32, ptr @builder_length, align 4, !tbaa !1
  %5 = getelementptr inbounds i8, ptr %3, i32 %4
  store i8 91, ptr %5, align 1, !tbaa !9
  %6 = add i32 %4, 1
  store i32 %6, ptr @builder_length, align 4, !tbaa !1
  %7 = icmp eq i32 %1, 0
  br i1 %7, label %8, label %13

8:                                                ; preds = %17, %2
  tail call fastcc void @reserve(i32 noundef 1) #30
  %9 = load ptr, ptr @builder, align 4, !tbaa !5
  %10 = load i32, ptr @builder_length, align 4, !tbaa !1
  %11 = getelementptr inbounds i8, ptr %9, i32 %10
  store i8 93, ptr %11, align 1, !tbaa !9
  %12 = add i32 %10, 1
  store i32 %12, ptr @builder_length, align 4, !tbaa !1
  ret void

13:                                               ; preds = %2, %17
  %14 = phi i32 [ %22, %17 ], [ 0, %2 ]
  %15 = icmp eq i32 %14, 0
  br i1 %15, label %17, label %16

16:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.11) #30
  br label %17

17:                                               ; preds = %16, %13
  %18 = getelementptr inbounds %struct.FaString, ptr %0, i32 %14
  %19 = load ptr, ptr %18, align 4, !tbaa !23
  %20 = getelementptr inbounds %struct.FaString, ptr %0, i32 %14, i32 1
  %21 = load i32, ptr %20, align 4, !tbaa !25
  tail call void @fa_b_quoted(ptr noundef %19, i32 noundef %21) #30
  %22 = add nuw i32 %14, 1
  %23 = icmp eq i32 %22, %1
  br i1 %23, label %8, label %13, !llvm.loop !26
}

; Function Attrs: nounwind optsize
define hidden void @fa_concat(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef readonly %2, i32 noundef %3, ptr nocapture noundef writeonly %4) local_unnamed_addr #2 {
  %6 = add i32 %3, %1
  %7 = icmp eq i32 %6, 0
  br i1 %7, label %8, label %11

8:                                                ; preds = %5
  store ptr @.str.29, ptr %4, align 4, !tbaa.struct !13
  %9 = getelementptr inbounds i8, ptr %4, i32 4
  store i32 0, ptr %9, align 4, !tbaa.struct !14
  %10 = getelementptr inbounds i8, ptr %4, i32 8
  store i32 0, ptr %10, align 4, !tbaa.struct !15
  br label %16

11:                                               ; preds = %5
  %12 = tail call ptr @fa_alloc(i32 noundef %6, i32 noundef 1) #29
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %12, ptr align 1 %0, i32 %1, i1 false)
  %13 = getelementptr inbounds i8, ptr %12, i32 %1
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %13, ptr align 1 %2, i32 %3, i1 false)
  store ptr %12, ptr %4, align 4, !tbaa !23
  %14 = getelementptr inbounds %struct.FaString, ptr %4, i32 0, i32 1
  store i32 %6, ptr %14, align 4, !tbaa !25
  %15 = getelementptr inbounds %struct.FaString, ptr %4, i32 0, i32 2
  store i32 %6, ptr %15, align 4, !tbaa !27
  br label %16

16:                                               ; preds = %11, %8
  ret void
}

; Function Attrs: optsize
declare ptr @fa_alloc(i32 noundef, i32 noundef) local_unnamed_addr #1

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: read)
define hidden i32 @fa_string_equal(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef readonly %2, i32 noundef %3) local_unnamed_addr #8 {
  %5 = icmp eq i32 %1, %3
  br i1 %5, label %6, label %25

6:                                                ; preds = %4
  %7 = icmp eq i32 %1, 0
  br i1 %7, label %25, label %8

8:                                                ; preds = %6
  %9 = load i8, ptr %0, align 1, !tbaa !9
  %10 = load i8, ptr %2, align 1, !tbaa !9
  %11 = icmp eq i8 %9, %10
  br i1 %11, label %12, label %25

12:                                               ; preds = %8, %16
  %13 = phi i32 [ %14, %16 ], [ 0, %8 ]
  %14 = add nuw i32 %13, 1
  %15 = icmp eq i32 %14, %1
  br i1 %15, label %22, label %16, !llvm.loop !28

16:                                               ; preds = %12
  %17 = getelementptr inbounds i8, ptr %0, i32 %14
  %18 = load i8, ptr %17, align 1, !tbaa !9
  %19 = getelementptr inbounds i8, ptr %2, i32 %14
  %20 = load i8, ptr %19, align 1, !tbaa !9
  %21 = icmp eq i8 %18, %20
  br i1 %21, label %12, label %22, !llvm.loop !28

22:                                               ; preds = %16, %12
  %23 = icmp uge i32 %14, %1
  %24 = zext i1 %23 to i32
  br label %25

25:                                               ; preds = %8, %22, %6, %4
  %26 = phi i32 [ 0, %4 ], [ 1, %6 ], [ 0, %8 ], [ %24, %22 ]
  ret i32 %26
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: read)
define hidden i32 @fa_string_compare(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef readonly %2, i32 noundef %3) local_unnamed_addr #8 {
  %5 = tail call i32 @llvm.umin.i32(i32 %1, i32 %3)
  %6 = icmp eq i32 %5, 0
  br i1 %6, label %11, label %7

7:                                                ; preds = %4
  %8 = load i8, ptr %0, align 1, !tbaa !9
  %9 = load i8, ptr %2, align 1, !tbaa !9
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
  br i1 %19, label %39, label %20, !llvm.loop !29

20:                                               ; preds = %16
  %21 = getelementptr inbounds i8, ptr %0, i32 %18
  %22 = load i8, ptr %21, align 1, !tbaa !9
  %23 = getelementptr inbounds i8, ptr %2, i32 %18
  %24 = load i8, ptr %23, align 1, !tbaa !9
  %25 = icmp eq i8 %22, %24
  br i1 %25, label %16, label %26, !llvm.loop !29

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
define hidden i32 @fa_next_grapheme(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2) local_unnamed_addr #9 {
  %4 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %2) #30
  ret i32 %4
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define internal fastcc i32 @next_boundary(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2) unnamed_addr #9 {
  %4 = alloca i32, align 4
  %5 = alloca i32, align 4
  store i32 %2, ptr %4, align 4, !tbaa !1
  %6 = getelementptr inbounds i8, ptr %0, i32 %2
  %7 = load i8, ptr %6, align 1, !tbaa !9
  %8 = icmp sgt i8 %7, -1
  br i1 %8, label %9, label %24

9:                                                ; preds = %3
  %10 = add i32 %2, 1
  %11 = icmp ult i32 %10, %1
  br i1 %11, label %12, label %21

12:                                               ; preds = %9
  %13 = getelementptr inbounds i8, ptr %0, i32 %10
  %14 = load i8, ptr %13, align 1, !tbaa !9
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
  %25 = call fastcc i32 @decode(ptr noundef nonnull %0, i32 noundef %1, ptr noundef nonnull %4) #30
  %26 = tail call fastcc i32 @break_kind(i32 noundef %25) #30, !range !30
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
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %5) #32
  store i32 %38, ptr %5, align 4, !tbaa !1
  %39 = call fastcc i32 @decode(ptr noundef %0, i32 noundef %1, ptr noundef nonnull %5) #30
  %40 = tail call fastcc i32 @break_kind(i32 noundef %39) #30, !range !30
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
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %5) #32
  br label %106

101:                                              ; preds = %74, %94, %88
  %102 = phi i32 [ %89, %88 ], [ %95, %94 ], [ %75, %74 ]
  %103 = phi i32 [ %92, %88 ], [ %96, %94 ], [ 0, %74 ]
  %104 = phi i32 [ 1, %88 ], [ %99, %94 ], [ 1, %74 ]
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %5) #32
  %105 = icmp ult i32 %102, %1
  br i1 %105, label %34, label %106

106:                                              ; preds = %101, %24, %100, %21
  %107 = phi i32 [ %23, %21 ], [ %38, %100 ], [ %27, %24 ], [ %102, %101 ]
  ret i32 %107
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define hidden i32 @fa_graphemes(ptr nocapture noundef readonly %0, i32 noundef %1) local_unnamed_addr #9 {
  %3 = icmp eq i32 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i32 [ 0, %2 ], [ %9, %6 ]
  ret i32 %5

6:                                                ; preds = %2, %6
  %7 = phi i32 [ %10, %6 ], [ 0, %2 ]
  %8 = phi i32 [ %9, %6 ], [ 0, %2 ]
  %9 = add i32 %8, 1
  %10 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %7) #30
  %11 = icmp ult i32 %10, %1
  br i1 %11, label %6, label %4, !llvm.loop !31
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define hidden i32 @fa_string_includes(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef readonly %2, i32 noundef %3) local_unnamed_addr #9 {
  %5 = tail call fastcc i64 @find(ptr noundef %0, i32 noundef %1, ptr noundef %2, i32 noundef %3, i32 noundef 0) #30
  %6 = icmp sgt i64 %5, -1
  %7 = zext i1 %6 to i32
  ret i32 %7
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define internal fastcc i64 @find(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef readonly %2, i32 noundef %3, i32 noundef %4) unnamed_addr #9 {
  %6 = icmp ugt i32 %3, %1
  br i1 %6, label %56, label %7

7:                                                ; preds = %5
  %8 = icmp eq i32 %3, 0
  %9 = icmp eq i32 %4, 0
  br label %10

10:                                               ; preds = %7, %51
  %11 = phi i32 [ %3, %7 ], [ %54, %51 ]
  %12 = phi i32 [ 0, %7 ], [ %53, %51 ]
  %13 = phi i64 [ -1, %7 ], [ %52, %51 ]
  %14 = getelementptr i8, ptr %0, i32 %12
  br i1 %8, label %31, label %15

15:                                               ; preds = %10
  %16 = load i8, ptr %14, align 1, !tbaa !9
  %17 = load i8, ptr %2, align 1, !tbaa !9
  %18 = icmp eq i8 %16, %17
  br i1 %18, label %19, label %51

19:                                               ; preds = %15, %23
  %20 = phi i32 [ %21, %23 ], [ 0, %15 ]
  %21 = add nuw i32 %20, 1
  %22 = icmp eq i32 %21, %3
  br i1 %22, label %29, label %23, !llvm.loop !32

23:                                               ; preds = %19
  %24 = getelementptr i8, ptr %14, i32 %21
  %25 = load i8, ptr %24, align 1, !tbaa !9
  %26 = getelementptr inbounds i8, ptr %2, i32 %21
  %27 = load i8, ptr %26, align 1, !tbaa !9
  %28 = icmp eq i8 %25, %27
  br i1 %28, label %19, label %29, !llvm.loop !32

29:                                               ; preds = %23, %19
  %30 = icmp ult i32 %21, %3
  br i1 %30, label %51, label %31

31:                                               ; preds = %10, %29
  %32 = icmp eq i32 %12, 0
  br i1 %32, label %37, label %33

33:                                               ; preds = %31, %33
  %34 = phi i32 [ %35, %33 ], [ 0, %31 ]
  %35 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %34) #30
  %36 = icmp ult i32 %35, %12
  br i1 %36, label %33, label %37, !llvm.loop !33

37:                                               ; preds = %33, %31
  %38 = phi i32 [ 0, %31 ], [ %35, %33 ]
  %39 = icmp eq i32 %38, %12
  br i1 %39, label %40, label %51

40:                                               ; preds = %37
  %41 = icmp eq i32 %11, 0
  br i1 %41, label %46, label %42

42:                                               ; preds = %40, %42
  %43 = phi i32 [ %44, %42 ], [ 0, %40 ]
  %44 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %43) #30
  %45 = icmp ult i32 %44, %11
  br i1 %45, label %42, label %46, !llvm.loop !33

46:                                               ; preds = %42, %40
  %47 = phi i32 [ 0, %40 ], [ %44, %42 ]
  %48 = icmp eq i32 %47, %11
  br i1 %48, label %49, label %51

49:                                               ; preds = %46
  %50 = zext i32 %12 to i64
  br i1 %9, label %56, label %51

51:                                               ; preds = %15, %29, %37, %46, %49
  %52 = phi i64 [ %50, %49 ], [ %13, %46 ], [ %13, %37 ], [ %13, %29 ], [ %13, %15 ]
  %53 = add i32 %12, 1
  %54 = add i32 %53, %3
  %55 = icmp ugt i32 %54, %1
  br i1 %55, label %56, label %10, !llvm.loop !34

56:                                               ; preds = %51, %49, %5
  %57 = phi i64 [ -1, %5 ], [ %50, %49 ], [ %52, %51 ]
  ret i64 %57
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define hidden i32 @fa_string_starts_with(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef readonly %2, i32 noundef %3) local_unnamed_addr #9 {
  %5 = icmp ugt i32 %3, %1
  br i1 %5, label %32, label %6

6:                                                ; preds = %4
  %7 = icmp eq i32 %3, 0
  br i1 %7, label %28, label %8

8:                                                ; preds = %6
  %9 = load i8, ptr %0, align 1, !tbaa !9
  %10 = load i8, ptr %2, align 1, !tbaa !9
  %11 = icmp eq i8 %9, %10
  br i1 %11, label %12, label %32

12:                                               ; preds = %8, %16
  %13 = phi i32 [ %14, %16 ], [ 0, %8 ]
  %14 = add nuw i32 %13, 1
  %15 = icmp eq i32 %14, %3
  br i1 %15, label %22, label %16, !llvm.loop !32

16:                                               ; preds = %12
  %17 = getelementptr i8, ptr %0, i32 %14
  %18 = load i8, ptr %17, align 1, !tbaa !9
  %19 = getelementptr inbounds i8, ptr %2, i32 %14
  %20 = load i8, ptr %19, align 1, !tbaa !9
  %21 = icmp eq i8 %18, %20
  br i1 %21, label %12, label %22, !llvm.loop !32

22:                                               ; preds = %16, %12
  %23 = icmp ult i32 %14, %3
  br i1 %23, label %32, label %24

24:                                               ; preds = %22, %24
  %25 = phi i32 [ %26, %24 ], [ 0, %22 ]
  %26 = tail call fastcc i32 @next_boundary(ptr noundef nonnull %0, i32 noundef %1, i32 noundef %25) #30
  %27 = icmp ult i32 %26, %3
  br i1 %27, label %24, label %28, !llvm.loop !33

28:                                               ; preds = %24, %6
  %29 = phi i32 [ 0, %6 ], [ %26, %24 ]
  %30 = icmp eq i32 %29, %3
  %31 = zext i1 %30 to i32
  br label %32

32:                                               ; preds = %8, %4, %28, %22
  %33 = phi i32 [ 0, %22 ], [ %31, %28 ], [ 0, %4 ], [ 0, %8 ]
  ret i32 %33
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define hidden i32 @fa_string_ends_with(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef readonly %2, i32 noundef %3) local_unnamed_addr #9 {
  %5 = icmp ugt i32 %3, %1
  br i1 %5, label %36, label %6

6:                                                ; preds = %4
  %7 = sub i32 %1, %3
  %8 = getelementptr i8, ptr %0, i32 %7
  %9 = icmp eq i32 %3, 0
  br i1 %9, label %26, label %10

10:                                               ; preds = %6
  %11 = load i8, ptr %8, align 1, !tbaa !9
  %12 = load i8, ptr %2, align 1, !tbaa !9
  %13 = icmp eq i8 %11, %12
  br i1 %13, label %14, label %36

14:                                               ; preds = %10, %18
  %15 = phi i32 [ %16, %18 ], [ 0, %10 ]
  %16 = add nuw i32 %15, 1
  %17 = icmp eq i32 %16, %3
  br i1 %17, label %24, label %18, !llvm.loop !32

18:                                               ; preds = %14
  %19 = getelementptr i8, ptr %8, i32 %16
  %20 = load i8, ptr %19, align 1, !tbaa !9
  %21 = getelementptr inbounds i8, ptr %2, i32 %16
  %22 = load i8, ptr %21, align 1, !tbaa !9
  %23 = icmp eq i8 %20, %22
  br i1 %23, label %14, label %24, !llvm.loop !32

24:                                               ; preds = %18, %14
  %25 = icmp ult i32 %16, %3
  br i1 %25, label %36, label %26

26:                                               ; preds = %6, %24
  %27 = icmp eq i32 %1, %3
  br i1 %27, label %32, label %28

28:                                               ; preds = %26, %28
  %29 = phi i32 [ %30, %28 ], [ 0, %26 ]
  %30 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %29) #30
  %31 = icmp ult i32 %30, %7
  br i1 %31, label %28, label %32, !llvm.loop !33

32:                                               ; preds = %28, %26
  %33 = phi i32 [ 0, %26 ], [ %30, %28 ]
  %34 = icmp eq i32 %33, %7
  %35 = zext i1 %34 to i32
  br label %36

36:                                               ; preds = %10, %32, %24, %4
  %37 = phi i32 [ 0, %24 ], [ 0, %4 ], [ %35, %32 ], [ 0, %10 ]
  ret i32 %37
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define hidden i64 @fa_string_index_of(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef readonly %2, i32 noundef %3, i32 noundef %4) local_unnamed_addr #9 {
  %6 = tail call fastcc i64 @find(ptr noundef %0, i32 noundef %1, ptr noundef %2, i32 noundef %3, i32 noundef %4) #30
  %7 = icmp slt i64 %6, 0
  br i1 %7, label %19, label %8

8:                                                ; preds = %5
  %9 = trunc i64 %6 to i32
  %10 = icmp eq i32 %9, 0
  br i1 %10, label %19, label %11

11:                                               ; preds = %8, %11
  %12 = phi i32 [ %15, %11 ], [ 0, %8 ]
  %13 = phi i32 [ %14, %11 ], [ 0, %8 ]
  %14 = add i32 %13, 1
  %15 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %12) #30
  %16 = icmp ult i32 %15, %9
  br i1 %16, label %11, label %17, !llvm.loop !35

17:                                               ; preds = %11
  %18 = zext i32 %14 to i64
  br label %19

19:                                               ; preds = %8, %17, %5
  %20 = phi i64 [ -1, %5 ], [ 0, %8 ], [ %18, %17 ]
  ret i64 %20
}

; Function Attrs: nounwind optsize
define hidden void @fa_string_replace(ptr noundef %0, i32 noundef %1, ptr nocapture noundef readonly %2, i32 noundef %3, ptr noundef %4, i32 noundef %5, i32 noundef %6, ptr nocapture noundef writeonly %7) local_unnamed_addr #2 {
  %9 = alloca %struct.Replacing, align 4
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  call void @llvm.lifetime.start.p0(i64 28, ptr nonnull %9) #32
  store ptr %0, ptr %9, align 4, !tbaa !36
  %10 = getelementptr inbounds %struct.Replacing, ptr %9, i32 0, i32 1
  store i32 0, ptr %10, align 4, !tbaa !38
  %11 = getelementptr inbounds %struct.Replacing, ptr %9, i32 0, i32 2
  store i32 %3, ptr %11, align 4, !tbaa !39
  %12 = getelementptr inbounds %struct.Replacing, ptr %9, i32 0, i32 3
  store ptr %4, ptr %12, align 4, !tbaa !40
  %13 = getelementptr inbounds %struct.Replacing, ptr %9, i32 0, i32 4
  store i32 %5, ptr %13, align 4, !tbaa !41
  %14 = getelementptr inbounds %struct.Replacing, ptr %9, i32 0, i32 5
  store i32 %6, ptr %14, align 4, !tbaa !42
  %15 = getelementptr inbounds %struct.Replacing, ptr %9, i32 0, i32 6
  store i32 0, ptr %15, align 4, !tbaa !43
  call fastcc void @each_match(ptr noundef %0, i32 noundef %1, ptr noundef %2, i32 noundef %3, ptr noundef nonnull @replace_one, ptr noundef nonnull %9) #30
  %16 = load i32, ptr %10, align 4, !tbaa !38
  %17 = getelementptr inbounds i8, ptr %0, i32 %16
  %18 = sub i32 %1, %16
  call fastcc void @reserve(i32 noundef %18) #30
  %19 = load ptr, ptr @builder, align 4, !tbaa !5
  %20 = load i32, ptr @builder_length, align 4, !tbaa !1
  %21 = getelementptr inbounds i8, ptr %19, i32 %20
  call void @llvm.memcpy.p0.p0.i32(ptr align 1 %21, ptr align 1 %17, i32 %18, i1 false)
  %22 = add i32 %20, %18
  store i32 %22, ptr @builder_length, align 4, !tbaa !1
  call void @fa_b_take(ptr noundef %7) #30
  call void @llvm.lifetime.end.p0(i64 28, ptr nonnull %9) #32
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @each_match(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef readonly %2, i32 noundef %3, ptr nocapture noundef readonly %4, ptr noundef %5) unnamed_addr #2 {
  %7 = icmp ugt i32 %3, %1
  br i1 %7, label %55, label %8

8:                                                ; preds = %6
  %9 = icmp eq i32 %3, 0
  %10 = tail call i32 @llvm.umax.i32(i32 %3, i32 1)
  br label %11

11:                                               ; preds = %8, %50
  %12 = phi i32 [ %3, %8 ], [ %53, %50 ]
  %13 = phi i32 [ 0, %8 ], [ %52, %50 ]
  %14 = getelementptr i8, ptr %0, i32 %13
  br i1 %9, label %31, label %15

15:                                               ; preds = %11
  %16 = load i8, ptr %14, align 1, !tbaa !9
  %17 = load i8, ptr %2, align 1, !tbaa !9
  %18 = icmp eq i8 %16, %17
  br i1 %18, label %19, label %50

19:                                               ; preds = %15, %23
  %20 = phi i32 [ %21, %23 ], [ 0, %15 ]
  %21 = add nuw i32 %20, 1
  %22 = icmp eq i32 %21, %3
  br i1 %22, label %29, label %23, !llvm.loop !32

23:                                               ; preds = %19
  %24 = getelementptr i8, ptr %14, i32 %21
  %25 = load i8, ptr %24, align 1, !tbaa !9
  %26 = getelementptr inbounds i8, ptr %2, i32 %21
  %27 = load i8, ptr %26, align 1, !tbaa !9
  %28 = icmp eq i8 %25, %27
  br i1 %28, label %19, label %29, !llvm.loop !32

29:                                               ; preds = %23, %19
  %30 = icmp ult i32 %21, %3
  br i1 %30, label %50, label %31

31:                                               ; preds = %11, %29
  %32 = icmp eq i32 %13, 0
  br i1 %32, label %37, label %33

33:                                               ; preds = %31, %33
  %34 = phi i32 [ %35, %33 ], [ 0, %31 ]
  %35 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %34) #30
  %36 = icmp ult i32 %35, %13
  br i1 %36, label %33, label %37, !llvm.loop !33

37:                                               ; preds = %33, %31
  %38 = phi i32 [ 0, %31 ], [ %35, %33 ]
  %39 = icmp eq i32 %38, %13
  br i1 %39, label %40, label %50

40:                                               ; preds = %37
  %41 = icmp eq i32 %12, 0
  br i1 %41, label %46, label %42

42:                                               ; preds = %40, %42
  %43 = phi i32 [ %44, %42 ], [ 0, %40 ]
  %44 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %43) #30
  %45 = icmp ult i32 %44, %12
  br i1 %45, label %42, label %46, !llvm.loop !33

46:                                               ; preds = %42, %40
  %47 = phi i32 [ 0, %40 ], [ %44, %42 ]
  %48 = icmp eq i32 %47, %12
  br i1 %48, label %49, label %50

49:                                               ; preds = %46
  tail call void %4(ptr noundef %5, i32 noundef %13) #29, !callees !44
  br label %50

50:                                               ; preds = %15, %29, %37, %46, %49
  %51 = phi i32 [ %10, %49 ], [ 1, %46 ], [ 1, %37 ], [ 1, %29 ], [ 1, %15 ]
  %52 = add i32 %51, %13
  %53 = add i32 %52, %3
  %54 = icmp ugt i32 %53, %1
  br i1 %54, label %55, label %11, !llvm.loop !45

55:                                               ; preds = %50, %6
  ret void
}

; Function Attrs: nounwind optsize
define internal void @replace_one(ptr nocapture noundef %0, i32 noundef %1) #2 {
  %3 = getelementptr inbounds %struct.Replacing, ptr %0, i32 0, i32 6
  %4 = load i32, ptr %3, align 4, !tbaa !43
  %5 = icmp eq i32 %4, 0
  br i1 %5, label %6, label %31

6:                                                ; preds = %2
  %7 = load ptr, ptr %0, align 4, !tbaa !36
  %8 = getelementptr inbounds %struct.Replacing, ptr %0, i32 0, i32 1
  %9 = load i32, ptr %8, align 4, !tbaa !38
  %10 = getelementptr inbounds i8, ptr %7, i32 %9
  %11 = sub i32 %1, %9
  tail call fastcc void @reserve(i32 noundef %11) #30
  %12 = load ptr, ptr @builder, align 4, !tbaa !5
  %13 = load i32, ptr @builder_length, align 4, !tbaa !1
  %14 = getelementptr inbounds i8, ptr %12, i32 %13
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %14, ptr align 1 %10, i32 %11, i1 false)
  %15 = add i32 %13, %11
  store i32 %15, ptr @builder_length, align 4, !tbaa !1
  %16 = getelementptr inbounds %struct.Replacing, ptr %0, i32 0, i32 3
  %17 = load ptr, ptr %16, align 4, !tbaa !40
  %18 = getelementptr inbounds %struct.Replacing, ptr %0, i32 0, i32 4
  %19 = load i32, ptr %18, align 4, !tbaa !41
  tail call fastcc void @reserve(i32 noundef %19) #30
  %20 = load ptr, ptr @builder, align 4, !tbaa !5
  %21 = load i32, ptr @builder_length, align 4, !tbaa !1
  %22 = getelementptr inbounds i8, ptr %20, i32 %21
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %22, ptr align 1 %17, i32 %19, i1 false)
  %23 = add i32 %21, %19
  store i32 %23, ptr @builder_length, align 4, !tbaa !1
  %24 = getelementptr inbounds %struct.Replacing, ptr %0, i32 0, i32 2
  %25 = load i32, ptr %24, align 4, !tbaa !39
  %26 = add i32 %25, %1
  store i32 %26, ptr %8, align 4, !tbaa !38
  %27 = getelementptr inbounds %struct.Replacing, ptr %0, i32 0, i32 5
  %28 = load i32, ptr %27, align 4, !tbaa !42
  %29 = icmp eq i32 %28, 0
  %30 = zext i1 %29 to i32
  store i32 %30, ptr %3, align 4, !tbaa !43
  br label %31

31:                                               ; preds = %2, %6
  ret void
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_string_characters(ptr nocapture noundef readonly %0, i32 noundef %1) local_unnamed_addr #2 {
  %3 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #29
  %4 = tail call ptr @fa_alloc(i32 noundef 48, i32 noundef 16) #29
  store ptr %4, ptr %3, align 8, !tbaa !46
  %5 = getelementptr inbounds %struct.FaArrayHeader, ptr %3, i32 0, i32 1
  store i64 0, ptr %5, align 8, !tbaa !49
  %6 = getelementptr inbounds %struct.FaArrayHeader, ptr %3, i32 0, i32 2
  store i64 4, ptr %6, align 8, !tbaa !50
  %7 = icmp eq i32 %1, 0
  br i1 %7, label %8, label %9

8:                                                ; preds = %18, %2
  ret ptr %3

9:                                                ; preds = %2, %18
  %10 = phi i32 [ %11, %18 ], [ 0, %2 ]
  %11 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %10) #30
  %12 = tail call ptr @fa_array_push_slot(ptr noundef nonnull %3, i64 noundef 12) #30
  %13 = sub i32 %11, %10
  %14 = icmp eq i32 %11, %10
  br i1 %14, label %18, label %15

15:                                               ; preds = %9
  %16 = getelementptr inbounds i8, ptr %0, i32 %10
  %17 = tail call ptr @fa_alloc(i32 noundef %13, i32 noundef 1) #29, !noalias !51
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %17, ptr align 1 %16, i32 %13, i1 false), !noalias !51
  br label %18

18:                                               ; preds = %9, %15
  %19 = phi ptr [ %17, %15 ], [ @.str.29, %9 ]
  store ptr %19, ptr %12, align 4, !tbaa.struct !13
  %20 = getelementptr inbounds i8, ptr %12, i32 4
  store i32 %13, ptr %20, align 4, !tbaa.struct !14
  %21 = getelementptr inbounds i8, ptr %12, i32 8
  store i32 %13, ptr %21, align 4, !tbaa.struct !15
  %22 = icmp ult i32 %11, %1
  br i1 %22, label %9, label %8, !llvm.loop !56
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_array_new(i64 noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #2 {
  %4 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #29
  %5 = tail call i64 @llvm.smax.i64(i64 %1, i64 %0)
  %6 = mul nsw i64 %5, %2
  %7 = trunc i64 %6 to i32
  %8 = tail call i32 @llvm.umax.i32(i32 %7, i32 1)
  %9 = tail call ptr @fa_alloc(i32 noundef %8, i32 noundef 16) #29
  store ptr %9, ptr %4, align 8, !tbaa !46
  %10 = getelementptr inbounds %struct.FaArrayHeader, ptr %4, i32 0, i32 1
  store i64 %0, ptr %10, align 8, !tbaa !49
  %11 = getelementptr inbounds %struct.FaArrayHeader, ptr %4, i32 0, i32 2
  store i64 %5, ptr %11, align 8, !tbaa !50
  ret ptr %4
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_array_push_slot(ptr nocapture noundef %0, i64 noundef %1) local_unnamed_addr #2 {
  %3 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %4 = load i64, ptr %3, align 8, !tbaa !49
  %5 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %6 = load i64, ptr %5, align 8, !tbaa !50
  %7 = icmp eq i64 %4, %6
  br i1 %7, label %10, label %8

8:                                                ; preds = %2
  %9 = load ptr, ptr %0, align 8, !tbaa !46
  br label %26

10:                                               ; preds = %2
  %11 = icmp slt i64 %4, 4
  %12 = shl nuw nsw i64 %4, 1
  %13 = select i1 %11, i64 4, i64 %12
  %14 = mul nsw i64 %13, %1
  %15 = trunc i64 %14 to i32
  %16 = tail call ptr @fa_alloc(i32 noundef %15, i32 noundef 16) #29
  %17 = load ptr, ptr %0, align 8, !tbaa !46
  %18 = load i64, ptr %3, align 8, !tbaa !49
  %19 = mul nsw i64 %18, %1
  %20 = trunc i64 %19 to i32
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %16, ptr align 1 %17, i32 %20, i1 false)
  %21 = load ptr, ptr %0, align 8, !tbaa !46
  %22 = load i64, ptr %5, align 8, !tbaa !50
  %23 = mul nsw i64 %22, %1
  %24 = trunc i64 %23 to i32
  tail call void @fa_free(ptr noundef %21, i32 noundef %24, i32 noundef 16) #29
  store ptr %16, ptr %0, align 8, !tbaa !46
  store i64 %13, ptr %5, align 8, !tbaa !50
  %25 = load i64, ptr %3, align 8, !tbaa !49
  br label %26

26:                                               ; preds = %8, %10
  %27 = phi i64 [ %4, %8 ], [ %25, %10 ]
  %28 = phi ptr [ %9, %8 ], [ %16, %10 ]
  %29 = mul nsw i64 %27, %1
  %30 = trunc i64 %29 to i32
  %31 = getelementptr inbounds i8, ptr %28, i32 %30
  %32 = add nsw i64 %27, 1
  store i64 %32, ptr %3, align 8, !tbaa !49
  ret ptr %31
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_string_split(ptr noundef %0, i32 noundef %1, ptr nocapture noundef readonly %2, i32 noundef %3) local_unnamed_addr #2 {
  %5 = alloca %struct.Splitting, align 4
  %6 = icmp eq i32 %3, 0
  br i1 %6, label %7, label %9

7:                                                ; preds = %4
  %8 = tail call ptr @fa_string_characters(ptr noundef %0, i32 noundef %1) #30
  br label %30

9:                                                ; preds = %4
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %5) #32
  store ptr %0, ptr %5, align 4, !tbaa !57
  %10 = getelementptr inbounds %struct.Splitting, ptr %5, i32 0, i32 1
  store i32 0, ptr %10, align 4, !tbaa !59
  %11 = getelementptr inbounds %struct.Splitting, ptr %5, i32 0, i32 2
  store i32 %3, ptr %11, align 4, !tbaa !60
  %12 = getelementptr inbounds %struct.Splitting, ptr %5, i32 0, i32 3
  %13 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #29
  %14 = tail call ptr @fa_alloc(i32 noundef 48, i32 noundef 16) #29
  store ptr %14, ptr %13, align 8, !tbaa !46
  %15 = getelementptr inbounds %struct.FaArrayHeader, ptr %13, i32 0, i32 1
  store i64 0, ptr %15, align 8, !tbaa !49
  %16 = getelementptr inbounds %struct.FaArrayHeader, ptr %13, i32 0, i32 2
  store i64 4, ptr %16, align 8, !tbaa !50
  store ptr %13, ptr %12, align 4, !tbaa !61
  call fastcc void @each_match(ptr noundef %0, i32 noundef %1, ptr noundef %2, i32 noundef %3, ptr noundef nonnull @split_one, ptr noundef nonnull %5) #30
  %17 = load ptr, ptr %12, align 4, !tbaa !61
  %18 = call ptr @fa_array_push_slot(ptr noundef %17, i64 noundef 12) #30
  %19 = load i32, ptr %10, align 4, !tbaa !59
  %20 = sub i32 %1, %19
  %21 = icmp eq i32 %19, %1
  br i1 %21, label %25, label %22

22:                                               ; preds = %9
  %23 = getelementptr inbounds i8, ptr %0, i32 %19
  %24 = call ptr @fa_alloc(i32 noundef %20, i32 noundef 1) #29, !noalias !62
  call void @llvm.memcpy.p0.p0.i32(ptr align 1 %24, ptr align 1 %23, i32 %20, i1 false), !noalias !62
  br label %25

25:                                               ; preds = %9, %22
  %26 = phi ptr [ %24, %22 ], [ @.str.29, %9 ]
  store ptr %26, ptr %18, align 4, !tbaa.struct !13
  %27 = getelementptr inbounds i8, ptr %18, i32 4
  store i32 %20, ptr %27, align 4, !tbaa.struct !14
  %28 = getelementptr inbounds i8, ptr %18, i32 8
  store i32 %20, ptr %28, align 4, !tbaa.struct !15
  %29 = load ptr, ptr %12, align 4, !tbaa !61
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %5) #32
  br label %30

30:                                               ; preds = %25, %7
  %31 = phi ptr [ %8, %7 ], [ %29, %25 ]
  ret ptr %31
}

; Function Attrs: nounwind optsize
define internal void @split_one(ptr nocapture noundef %0, i32 noundef %1) #2 {
  %3 = getelementptr inbounds %struct.Splitting, ptr %0, i32 0, i32 3
  %4 = load ptr, ptr %3, align 4, !tbaa !61
  %5 = tail call ptr @fa_array_push_slot(ptr noundef %4, i64 noundef 12) #30
  %6 = getelementptr inbounds %struct.Splitting, ptr %0, i32 0, i32 1
  %7 = load i32, ptr %6, align 4, !tbaa !59
  %8 = sub i32 %1, %7
  %9 = icmp eq i32 %7, %1
  br i1 %9, label %14, label %10

10:                                               ; preds = %2
  %11 = load ptr, ptr %0, align 4, !tbaa !57
  %12 = getelementptr inbounds i8, ptr %11, i32 %7
  %13 = tail call ptr @fa_alloc(i32 noundef %8, i32 noundef 1) #29, !noalias !67
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %13, ptr align 1 %12, i32 %8, i1 false), !noalias !67
  br label %14

14:                                               ; preds = %2, %10
  %15 = phi ptr [ %13, %10 ], [ @.str.29, %2 ]
  store ptr %15, ptr %5, align 4, !tbaa.struct !13
  %16 = getelementptr inbounds i8, ptr %5, i32 4
  store i32 %8, ptr %16, align 4, !tbaa.struct !14
  %17 = getelementptr inbounds i8, ptr %5, i32 8
  store i32 %8, ptr %17, align 4, !tbaa.struct !15
  %18 = getelementptr inbounds %struct.Splitting, ptr %0, i32 0, i32 2
  %19 = load i32, ptr %18, align 4, !tbaa !60
  %20 = add i32 %19, %1
  store i32 %20, ptr %6, align 4, !tbaa !59
  ret void
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_string_code_points(ptr nocapture noundef readonly %0, i32 noundef %1) local_unnamed_addr #2 {
  %3 = alloca i32, align 4
  %4 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #29
  %5 = tail call ptr @fa_alloc(i32 noundef 16, i32 noundef 16) #29
  store ptr %5, ptr %4, align 8, !tbaa !46
  %6 = getelementptr inbounds %struct.FaArrayHeader, ptr %4, i32 0, i32 1
  store i64 0, ptr %6, align 8, !tbaa !49
  %7 = getelementptr inbounds %struct.FaArrayHeader, ptr %4, i32 0, i32 2
  store i64 4, ptr %7, align 8, !tbaa !50
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %3) #32
  store i32 0, ptr %3, align 4, !tbaa !1
  %8 = icmp eq i32 %1, 0
  br i1 %8, label %9, label %10

9:                                                ; preds = %10, %2
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %3) #32
  ret ptr %4

10:                                               ; preds = %2, %10
  %11 = tail call ptr @fa_array_push_slot(ptr noundef nonnull %4, i64 noundef 4) #30
  %12 = call fastcc i32 @decode(ptr noundef %0, i32 noundef %1, ptr noundef nonnull %3) #30
  store i32 %12, ptr %11, align 4, !tbaa !17
  %13 = load i32, ptr %3, align 4, !tbaa !1
  %14 = icmp ult i32 %13, %1
  br i1 %14, label %10, label %9, !llvm.loop !72
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define internal fastcc i32 @decode(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef %2) unnamed_addr #9 {
  %4 = load i32, ptr %2, align 4, !tbaa !1
  %5 = getelementptr inbounds i8, ptr %0, i32 %4
  %6 = load i8, ptr %5, align 1, !tbaa !9
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
  %28 = load i8, ptr %27, align 1, !tbaa !9
  %29 = and i8 %28, 63
  %30 = zext nneg i8 %29 to i32
  %31 = or disjoint i32 %26, %30
  %32 = add nuw nsw i32 %17, 1
  %33 = icmp eq i32 %32, %13
  br i1 %33, label %21, label %16, !llvm.loop !73
}

; Function Attrs: nounwind optsize
define hidden noundef i32 @fa_string_get(ptr nocapture noundef readonly %0, i32 noundef %1, i64 noundef %2, ptr nocapture noundef writeonly %3) local_unnamed_addr #2 {
  %5 = icmp slt i64 %2, 0
  br i1 %5, label %6, label %20

6:                                                ; preds = %4
  %7 = icmp eq i32 %1, 0
  br i1 %7, label %16, label %8

8:                                                ; preds = %6, %8
  %9 = phi i32 [ %12, %8 ], [ 0, %6 ]
  %10 = phi i32 [ %11, %8 ], [ 0, %6 ]
  %11 = add i32 %10, 1
  %12 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %9) #30
  %13 = icmp ult i32 %12, %1
  br i1 %13, label %8, label %14, !llvm.loop !31

14:                                               ; preds = %8
  %15 = zext i32 %11 to i64
  br label %16

16:                                               ; preds = %6, %14
  %17 = phi i64 [ 0, %6 ], [ %15, %14 ]
  %18 = add nsw i64 %17, %2
  %19 = icmp slt i64 %18, 0
  br i1 %19, label %41, label %20

20:                                               ; preds = %4, %16
  %21 = phi i64 [ %18, %16 ], [ %2, %4 ]
  %22 = icmp eq i32 %1, 0
  br i1 %22, label %41, label %26

23:                                               ; preds = %26
  %24 = add nuw nsw i64 %27, 1
  %25 = icmp ult i32 %29, %1
  br i1 %25, label %26, label %41, !llvm.loop !74

26:                                               ; preds = %20, %23
  %27 = phi i64 [ %24, %23 ], [ 0, %20 ]
  %28 = phi i32 [ %29, %23 ], [ 0, %20 ]
  %29 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %28) #30
  %30 = icmp eq i64 %27, %21
  br i1 %30, label %31, label %23

31:                                               ; preds = %26
  %32 = sub i32 %29, %28
  %33 = icmp eq i32 %29, %28
  br i1 %33, label %37, label %34

34:                                               ; preds = %31
  %35 = getelementptr inbounds i8, ptr %0, i32 %28
  %36 = tail call ptr @fa_alloc(i32 noundef %32, i32 noundef 1) #29, !noalias !75
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %36, ptr align 1 %35, i32 %32, i1 false), !noalias !75
  br label %37

37:                                               ; preds = %34, %31
  %38 = phi ptr [ %36, %34 ], [ @.str.29, %31 ]
  store ptr %38, ptr %3, align 4, !tbaa.struct !13
  %39 = getelementptr inbounds i8, ptr %3, i32 4
  store i32 %32, ptr %39, align 4, !tbaa.struct !14
  %40 = getelementptr inbounds i8, ptr %3, i32 8
  store i32 %32, ptr %40, align 4, !tbaa.struct !15
  br label %41

41:                                               ; preds = %23, %37, %20, %16
  %42 = phi i32 [ 0, %16 ], [ 1, %37 ], [ 0, %20 ], [ 0, %23 ]
  ret i32 %42
}

; Function Attrs: nounwind optsize
define hidden void @fa_string_case(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef writeonly %3) local_unnamed_addr #2 {
  %5 = alloca i32, align 4
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %5) #32
  store i32 0, ptr %5, align 4, !tbaa !1
  %6 = icmp eq i32 %1, 0
  br i1 %6, label %144, label %7

7:                                                ; preds = %4
  %8 = icmp eq i32 %2, 0
  br label %9

9:                                                ; preds = %7, %141
  %10 = call fastcc i32 @decode(ptr noundef %0, i32 noundef %1, ptr noundef nonnull %5) #30
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
  tail call fastcc void @append_c(ptr noundef nonnull @.str.12) #30
  br label %141, !llvm.loop !80

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
  tail call fastcc void @reserve(i32 noundef 1) #30
  %68 = load ptr, ptr @builder, align 4, !tbaa !5
  %69 = load i32, ptr @builder_length, align 4, !tbaa !1
  %70 = getelementptr inbounds i8, ptr %68, i32 %69
  store i8 %67, ptr %70, align 1, !tbaa !9
  br label %138

71:                                               ; preds = %63
  %72 = icmp ult i32 %64, 2048
  br i1 %72, label %73, label %88

73:                                               ; preds = %32, %35, %36, %59, %34, %61, %71
  %74 = phi i32 [ %64, %71 ], [ 376, %34 ], [ %60, %59 ], [ 931, %36 ], [ 924, %35 ], [ %33, %32 ], [ 255, %61 ]
  %75 = lshr i32 %74, 6
  %76 = trunc i32 %75 to i8
  %77 = or disjoint i8 %76, -64
  tail call fastcc void @reserve(i32 noundef 1) #30
  %78 = load ptr, ptr @builder, align 4, !tbaa !5
  %79 = load i32, ptr @builder_length, align 4, !tbaa !1
  %80 = getelementptr inbounds i8, ptr %78, i32 %79
  store i8 %77, ptr %80, align 1, !tbaa !9
  %81 = add i32 %79, 1
  store i32 %81, ptr @builder_length, align 4, !tbaa !1
  %82 = trunc i32 %74 to i8
  %83 = and i8 %82, 63
  %84 = or disjoint i8 %83, -128
  tail call fastcc void @reserve(i32 noundef 1) #30
  %85 = load ptr, ptr @builder, align 4, !tbaa !5
  %86 = load i32, ptr @builder_length, align 4, !tbaa !1
  %87 = getelementptr inbounds i8, ptr %85, i32 %86
  store i8 %84, ptr %87, align 1, !tbaa !9
  br label %138

88:                                               ; preds = %71
  %89 = icmp ult i32 %64, 65536
  tail call fastcc void @reserve(i32 noundef 1) #30
  %90 = load ptr, ptr @builder, align 4, !tbaa !5
  %91 = load i32, ptr @builder_length, align 4, !tbaa !1
  %92 = getelementptr inbounds i8, ptr %90, i32 %91
  %93 = add i32 %91, 1
  br i1 %89, label %94, label %112

94:                                               ; preds = %88
  %95 = lshr i32 %64, 12
  %96 = trunc i32 %95 to i8
  %97 = or disjoint i8 %96, -32
  store i8 %97, ptr %92, align 1, !tbaa !9
  store i32 %93, ptr @builder_length, align 4, !tbaa !1
  %98 = lshr i32 %64, 6
  %99 = trunc i32 %98 to i8
  %100 = and i8 %99, 63
  %101 = or disjoint i8 %100, -128
  tail call fastcc void @reserve(i32 noundef 1) #30
  %102 = load ptr, ptr @builder, align 4, !tbaa !5
  %103 = load i32, ptr @builder_length, align 4, !tbaa !1
  %104 = getelementptr inbounds i8, ptr %102, i32 %103
  store i8 %101, ptr %104, align 1, !tbaa !9
  %105 = add i32 %103, 1
  store i32 %105, ptr @builder_length, align 4, !tbaa !1
  %106 = trunc i32 %64 to i8
  %107 = and i8 %106, 63
  %108 = or disjoint i8 %107, -128
  tail call fastcc void @reserve(i32 noundef 1) #30
  %109 = load ptr, ptr @builder, align 4, !tbaa !5
  %110 = load i32, ptr @builder_length, align 4, !tbaa !1
  %111 = getelementptr inbounds i8, ptr %109, i32 %110
  store i8 %108, ptr %111, align 1, !tbaa !9
  br label %138

112:                                              ; preds = %88
  %113 = lshr i32 %64, 18
  %114 = trunc i32 %113 to i8
  %115 = or i8 %114, -16
  store i8 %115, ptr %92, align 1, !tbaa !9
  store i32 %93, ptr @builder_length, align 4, !tbaa !1
  %116 = lshr i32 %64, 12
  %117 = trunc i32 %116 to i8
  %118 = and i8 %117, 63
  %119 = or disjoint i8 %118, -128
  tail call fastcc void @reserve(i32 noundef 1) #30
  %120 = load ptr, ptr @builder, align 4, !tbaa !5
  %121 = load i32, ptr @builder_length, align 4, !tbaa !1
  %122 = getelementptr inbounds i8, ptr %120, i32 %121
  store i8 %119, ptr %122, align 1, !tbaa !9
  %123 = add i32 %121, 1
  store i32 %123, ptr @builder_length, align 4, !tbaa !1
  %124 = lshr i32 %64, 6
  %125 = trunc i32 %124 to i8
  %126 = and i8 %125, 63
  %127 = or disjoint i8 %126, -128
  tail call fastcc void @reserve(i32 noundef 1) #30
  %128 = load ptr, ptr @builder, align 4, !tbaa !5
  %129 = load i32, ptr @builder_length, align 4, !tbaa !1
  %130 = getelementptr inbounds i8, ptr %128, i32 %129
  store i8 %127, ptr %130, align 1, !tbaa !9
  %131 = add i32 %129, 1
  store i32 %131, ptr @builder_length, align 4, !tbaa !1
  %132 = trunc i32 %64 to i8
  %133 = and i8 %132, 63
  %134 = or disjoint i8 %133, -128
  tail call fastcc void @reserve(i32 noundef 1) #30
  %135 = load ptr, ptr @builder, align 4, !tbaa !5
  %136 = load i32, ptr @builder_length, align 4, !tbaa !1
  %137 = getelementptr inbounds i8, ptr %135, i32 %136
  store i8 %134, ptr %137, align 1, !tbaa !9
  br label %138

138:                                              ; preds = %66, %73, %94, %112
  %139 = phi i32 [ %86, %73 ], [ %136, %112 ], [ %110, %94 ], [ %69, %66 ]
  %140 = add i32 %139, 1
  store i32 %140, ptr @builder_length, align 4, !tbaa !1
  br label %141

141:                                              ; preds = %138, %37
  %142 = load i32, ptr %5, align 4, !tbaa !1
  %143 = icmp ult i32 %142, %1
  br i1 %143, label %9, label %144

144:                                              ; preds = %141, %4
  tail call void @fa_b_take(ptr noundef %3) #30
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %5) #32
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_string_trim(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef writeonly %3) local_unnamed_addr #2 {
  %5 = alloca i32, align 4
  %6 = alloca i32, align 4
  %7 = and i32 %2, 1
  %8 = icmp eq i32 %7, 0
  br i1 %8, label %17, label %9

9:                                                ; preds = %4, %12
  %10 = phi i32 [ %16, %12 ], [ 0, %4 ]
  %11 = icmp ult i32 %10, %1
  br i1 %11, label %12, label %17

12:                                               ; preds = %9
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %5) #32
  store i32 %10, ptr %5, align 4, !tbaa !1
  %13 = call fastcc i32 @decode(ptr noundef %0, i32 noundef %1, ptr noundef nonnull %5) #30
  %14 = tail call fastcc i32 @is_space(i32 noundef %13) #30, !range !81
  %15 = icmp eq i32 %14, 0
  %16 = load i32, ptr %5, align 4
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %5) #32
  br i1 %15, label %17, label %9

17:                                               ; preds = %12, %9, %4
  %18 = phi i32 [ 0, %4 ], [ %10, %9 ], [ %10, %12 ]
  %19 = and i32 %2, 2
  %20 = icmp eq i32 %19, 0
  br i1 %20, label %38, label %21

21:                                               ; preds = %17, %33
  %22 = phi i32 [ %34, %33 ], [ %1, %17 ]
  %23 = icmp ugt i32 %22, %18
  br i1 %23, label %24, label %38

24:                                               ; preds = %21, %28
  %25 = phi i32 [ %26, %28 ], [ %22, %21 ]
  %26 = add i32 %25, -1
  %27 = icmp ugt i32 %26, %18
  br i1 %27, label %28, label %33

28:                                               ; preds = %24
  %29 = getelementptr inbounds i8, ptr %0, i32 %26
  %30 = load i8, ptr %29, align 1, !tbaa !9
  %31 = and i8 %30, -64
  %32 = icmp eq i8 %31, -128
  br i1 %32, label %24, label %33, !llvm.loop !82

33:                                               ; preds = %24, %28
  %34 = phi i32 [ %18, %24 ], [ %26, %28 ]
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %6) #32
  store i32 %34, ptr %6, align 4, !tbaa !1
  %35 = call fastcc i32 @decode(ptr noundef %0, i32 noundef %22, ptr noundef nonnull %6) #30
  %36 = tail call fastcc i32 @is_space(i32 noundef %35) #30, !range !81
  %37 = icmp eq i32 %36, 0
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %6) #32
  br i1 %37, label %38, label %21

38:                                               ; preds = %33, %21, %17
  %39 = phi i32 [ %1, %17 ], [ %22, %21 ], [ %22, %33 ]
  %40 = sub i32 %39, %18
  %41 = icmp eq i32 %39, %18
  br i1 %41, label %45, label %42

42:                                               ; preds = %38
  %43 = getelementptr inbounds i8, ptr %0, i32 %18
  %44 = tail call ptr @fa_alloc(i32 noundef %40, i32 noundef 1) #29, !noalias !83
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %44, ptr align 1 %43, i32 %40, i1 false), !noalias !83
  br label %45

45:                                               ; preds = %38, %42
  %46 = phi ptr [ %44, %42 ], [ @.str.29, %38 ]
  store ptr %46, ptr %3, align 4, !tbaa.struct !13
  %47 = getelementptr inbounds i8, ptr %3, i32 4
  store i32 %40, ptr %47, align 4, !tbaa.struct !14
  %48 = getelementptr inbounds i8, ptr %3, i32 8
  store i32 %40, ptr %48, align 4, !tbaa.struct !15
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(none)
define internal fastcc noundef i32 @is_space(i32 noundef %0) unnamed_addr #10 {
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
define hidden void @fa_string_repeat(ptr nocapture noundef readonly %0, i32 noundef %1, i64 noundef %2, ptr nocapture noundef writeonly %3) local_unnamed_addr #2 {
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  %5 = icmp sgt i64 %2, 0
  br i1 %5, label %7, label %6

6:                                                ; preds = %7, %4
  tail call void @fa_b_take(ptr noundef %3) #30
  ret void

7:                                                ; preds = %4, %7
  %8 = phi i64 [ %13, %7 ], [ 0, %4 ]
  tail call fastcc void @reserve(i32 noundef %1) #30
  %9 = load ptr, ptr @builder, align 4, !tbaa !5
  %10 = load i32, ptr @builder_length, align 4, !tbaa !1
  %11 = getelementptr inbounds i8, ptr %9, i32 %10
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %11, ptr align 1 %0, i32 %1, i1 false)
  %12 = add i32 %10, %1
  store i32 %12, ptr @builder_length, align 4, !tbaa !1
  %13 = add nuw nsw i64 %8, 1
  %14 = icmp eq i64 %13, %2
  br i1 %14, label %6, label %7, !llvm.loop !86
}

; Function Attrs: nounwind optsize
define hidden void @fa_array_join(ptr nocapture noundef readonly %0, ptr nocapture noundef readonly %1, i32 noundef %2, ptr nocapture noundef writeonly %3) local_unnamed_addr #2 {
  %5 = load ptr, ptr %0, align 8, !tbaa !46
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  %6 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %7 = load i64, ptr %6, align 8, !tbaa !49
  %8 = icmp sgt i64 %7, 0
  br i1 %8, label %10, label %9

9:                                                ; preds = %18, %4
  tail call void @fa_b_take(ptr noundef %3) #30
  ret void

10:                                               ; preds = %4, %18
  %11 = phi i64 [ %28, %18 ], [ 0, %4 ]
  %12 = icmp eq i64 %11, 0
  br i1 %12, label %18, label %13

13:                                               ; preds = %10
  tail call fastcc void @reserve(i32 noundef %2) #30
  %14 = load ptr, ptr @builder, align 4, !tbaa !5
  %15 = load i32, ptr @builder_length, align 4, !tbaa !1
  %16 = getelementptr inbounds i8, ptr %14, i32 %15
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %16, ptr align 1 %1, i32 %2, i1 false)
  %17 = add i32 %15, %2
  store i32 %17, ptr @builder_length, align 4, !tbaa !1
  br label %18

18:                                               ; preds = %13, %10
  %19 = trunc i64 %11 to i32
  %20 = getelementptr inbounds %struct.FaString, ptr %5, i32 %19
  %21 = load ptr, ptr %20, align 4, !tbaa !23
  %22 = getelementptr inbounds %struct.FaString, ptr %5, i32 %19, i32 1
  %23 = load i32, ptr %22, align 4, !tbaa !25
  tail call fastcc void @reserve(i32 noundef %23) #30
  %24 = load ptr, ptr @builder, align 4, !tbaa !5
  %25 = load i32, ptr @builder_length, align 4, !tbaa !1
  %26 = getelementptr inbounds i8, ptr %24, i32 %25
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %26, ptr align 1 %21, i32 %23, i1 false)
  %27 = add i32 %25, %23
  store i32 %27, ptr @builder_length, align 4, !tbaa !1
  %28 = add nuw nsw i64 %11, 1
  %29 = load i64, ptr %6, align 8, !tbaa !49
  %30 = icmp slt i64 %28, %29
  br i1 %30, label %10, label %9, !llvm.loop !87
}

; Function Attrs: nounwind optsize
define hidden void @fa_string_pad(ptr nocapture noundef readonly %0, i32 noundef %1, i64 noundef %2, ptr nocapture noundef readonly %3, i32 noundef %4, i32 noundef %5, ptr nocapture noundef writeonly %6) local_unnamed_addr #2 {
  %8 = icmp eq i32 %1, 0
  br i1 %8, label %17, label %9

9:                                                ; preds = %7, %9
  %10 = phi i32 [ %13, %9 ], [ 0, %7 ]
  %11 = phi i32 [ %12, %9 ], [ 0, %7 ]
  %12 = add i32 %11, 1
  %13 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %10) #30
  %14 = icmp ult i32 %13, %1
  br i1 %14, label %9, label %15, !llvm.loop !31

15:                                               ; preds = %9
  %16 = zext i32 %12 to i64
  br label %17

17:                                               ; preds = %15, %7
  %18 = phi i64 [ 0, %7 ], [ %16, %15 ]
  %19 = sub nsw i64 %2, %18
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  %20 = icmp eq i32 %5, 0
  br i1 %20, label %21, label %26

21:                                               ; preds = %17
  tail call fastcc void @reserve(i32 noundef %1) #30
  %22 = load ptr, ptr @builder, align 4, !tbaa !5
  %23 = load i32, ptr @builder_length, align 4, !tbaa !1
  %24 = getelementptr inbounds i8, ptr %22, i32 %23
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %24, ptr align 1 %0, i32 %1, i1 false)
  %25 = add i32 %23, %1
  store i32 %25, ptr @builder_length, align 4, !tbaa !1
  br label %26

26:                                               ; preds = %21, %17
  %27 = icmp ne i32 %4, 0
  %28 = icmp sgt i64 %19, 0
  %29 = and i1 %27, %28
  br i1 %29, label %31, label %30

30:                                               ; preds = %31, %26
  br i1 %20, label %50, label %45

31:                                               ; preds = %26, %31
  %32 = phi i32 [ %42, %31 ], [ 0, %26 ]
  %33 = phi i64 [ %43, %31 ], [ 0, %26 ]
  %34 = tail call fastcc i32 @next_boundary(ptr noundef %3, i32 noundef %4, i32 noundef %32) #30
  %35 = getelementptr inbounds i8, ptr %3, i32 %32
  %36 = sub i32 %34, %32
  tail call fastcc void @reserve(i32 noundef %36) #30
  %37 = load ptr, ptr @builder, align 4, !tbaa !5
  %38 = load i32, ptr @builder_length, align 4, !tbaa !1
  %39 = getelementptr inbounds i8, ptr %37, i32 %38
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %39, ptr align 1 %35, i32 %36, i1 false)
  %40 = add i32 %38, %36
  store i32 %40, ptr @builder_length, align 4, !tbaa !1
  %41 = icmp eq i32 %34, %4
  %42 = select i1 %41, i32 0, i32 %34
  %43 = add nuw nsw i64 %33, 1
  %44 = icmp slt i64 %43, %19
  br i1 %44, label %31, label %30, !llvm.loop !88

45:                                               ; preds = %30
  tail call fastcc void @reserve(i32 noundef %1) #30
  %46 = load ptr, ptr @builder, align 4, !tbaa !5
  %47 = load i32, ptr @builder_length, align 4, !tbaa !1
  %48 = getelementptr inbounds i8, ptr %46, i32 %47
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %48, ptr align 1 %0, i32 %1, i1 false)
  %49 = add i32 %47, %1
  store i32 %49, ptr @builder_length, align 4, !tbaa !1
  br label %50

50:                                               ; preds = %45, %30
  tail call void @fa_b_take(ptr noundef %6) #30
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_arguments_init(i32 noundef %0, ptr nocapture noundef readnone %1) local_unnamed_addr #2 {
  %3 = tail call i32 @fa_host_arguments_size() #29
  %4 = tail call i32 @llvm.umax.i32(i32 %3, i32 1)
  %5 = tail call ptr @fa_alloc(i32 noundef %4, i32 noundef 1) #29
  store ptr %5, ptr @argument_bytes, align 4, !tbaa !5
  tail call void @fa_host_arguments_copy(ptr noundef %5) #29
  store i32 0, ptr @argument_count, align 4, !tbaa !1
  %6 = icmp eq i32 %3, 0
  br i1 %6, label %7, label %13

7:                                                ; preds = %2
  %8 = tail call ptr @fa_alloc(i32 noundef 12, i32 noundef 4) #29
  store ptr %8, ptr @argument_items, align 4, !tbaa !5
  br label %23

9:                                                ; preds = %13
  store i32 %20, ptr @argument_count, align 4, !tbaa !1
  %10 = mul i32 %20, 12
  %11 = add i32 %10, 12
  %12 = tail call ptr @fa_alloc(i32 noundef %11, i32 noundef 4) #29
  store ptr %12, ptr @argument_items, align 4, !tbaa !5
  br label %24

13:                                               ; preds = %2, %13
  %14 = phi i32 [ %21, %13 ], [ 0, %2 ]
  %15 = phi i32 [ %20, %13 ], [ 0, %2 ]
  %16 = getelementptr inbounds i8, ptr %5, i32 %14
  %17 = load i8, ptr %16, align 1, !tbaa !9
  %18 = icmp eq i8 %17, 0
  %19 = zext i1 %18 to i32
  %20 = add i32 %15, %19
  %21 = add nuw i32 %14, 1
  %22 = icmp eq i32 %21, %3
  br i1 %22, label %9, label %13, !llvm.loop !89

23:                                               ; preds = %41, %7
  ret void

24:                                               ; preds = %9, %41
  %25 = phi i32 [ %42, %41 ], [ 0, %9 ]
  %26 = phi i32 [ %44, %41 ], [ 0, %9 ]
  %27 = phi i32 [ %43, %41 ], [ 0, %9 ]
  %28 = getelementptr inbounds i8, ptr %5, i32 %25
  %29 = load i8, ptr %28, align 1, !tbaa !9
  %30 = icmp eq i8 %29, 0
  br i1 %30, label %33, label %31

31:                                               ; preds = %24
  %32 = add nuw i32 %25, 1
  br label %41

33:                                               ; preds = %24
  %34 = getelementptr inbounds i8, ptr %5, i32 %27
  %35 = getelementptr inbounds %struct.FaString, ptr %12, i32 %26
  store ptr %34, ptr %35, align 4, !tbaa !23
  %36 = sub i32 %25, %27
  %37 = getelementptr inbounds %struct.FaString, ptr %12, i32 %26, i32 1
  store i32 %36, ptr %37, align 4, !tbaa !25
  %38 = getelementptr inbounds %struct.FaString, ptr %12, i32 %26, i32 2
  store i32 0, ptr %38, align 4, !tbaa !27
  %39 = add i32 %26, 1
  %40 = add nuw i32 %25, 1
  br label %41

41:                                               ; preds = %31, %33
  %42 = phi i32 [ %32, %31 ], [ %40, %33 ]
  %43 = phi i32 [ %27, %31 ], [ %40, %33 ]
  %44 = phi i32 [ %26, %31 ], [ %39, %33 ]
  %45 = icmp eq i32 %42, %3
  br i1 %45, label %23, label %24, !llvm.loop !90
}

; Function Attrs: optsize
declare i32 @fa_host_arguments_size() local_unnamed_addr #11

; Function Attrs: optsize
declare void @fa_host_arguments_copy(ptr noundef) local_unnamed_addr #12

; Function Attrs: optsize
declare void @fa_free(ptr noundef, i32 noundef, i32 noundef) local_unnamed_addr #1

; Function Attrs: nounwind optsize
define hidden void @fa_array_append(ptr nocapture noundef %0, ptr nocapture noundef readonly %1, i64 noundef %2) local_unnamed_addr #2 {
  %4 = getelementptr inbounds %struct.FaArrayHeader, ptr %1, i32 0, i32 1
  %5 = load i64, ptr %4, align 8, !tbaa !49
  %6 = icmp sgt i64 %5, 0
  br i1 %6, label %7, label %9

7:                                                ; preds = %3
  %8 = trunc i64 %2 to i32
  br label %10

9:                                                ; preds = %10, %3
  ret void

10:                                               ; preds = %7, %10
  %11 = phi i64 [ 0, %7 ], [ %17, %10 ]
  %12 = tail call ptr @fa_array_push_slot(ptr noundef %0, i64 noundef %2) #30
  %13 = load ptr, ptr %1, align 8, !tbaa !46
  %14 = mul nsw i64 %11, %2
  %15 = trunc i64 %14 to i32
  %16 = getelementptr inbounds i8, ptr %13, i32 %15
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %12, ptr align 1 %16, i32 %8, i1 false)
  %17 = add nuw nsw i64 %11, 1
  %18 = icmp eq i64 %17, %5
  br i1 %18, label %9, label %10, !llvm.loop !91
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_array_slice(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2, i8 noundef signext %3, i64 noundef %4, ptr nocapture noundef writeonly %5) local_unnamed_addr #2 {
  %7 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %8 = load i64, ptr %7, align 8, !tbaa !49
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
  %18 = tail call ptr @fa_error_index(i64 noundef %1, i64 noundef %8) #30
  br label %61

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
  %45 = tail call ptr @fa_error_index(i64 noundef %2, i64 noundef %8) #30
  br label %61

46:                                               ; preds = %38, %23
  %47 = phi i64 [ %8, %23 ], [ %43, %38 ]
  %48 = tail call i64 @llvm.smax.i64(i64 %47, i64 %24)
  %49 = sub nsw i64 %48, %24
  %50 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #29
  %51 = mul nsw i64 %49, %4
  %52 = trunc i64 %51 to i32
  %53 = tail call i32 @llvm.umax.i32(i32 %52, i32 1)
  %54 = tail call ptr @fa_alloc(i32 noundef %53, i32 noundef 16) #29
  store ptr %54, ptr %50, align 8, !tbaa !46
  %55 = getelementptr inbounds %struct.FaArrayHeader, ptr %50, i32 0, i32 1
  store i64 %49, ptr %55, align 8, !tbaa !49
  %56 = getelementptr inbounds %struct.FaArrayHeader, ptr %50, i32 0, i32 2
  store i64 %49, ptr %56, align 8, !tbaa !50
  %57 = load ptr, ptr %0, align 8, !tbaa !46
  %58 = mul nsw i64 %24, %4
  %59 = trunc i64 %58 to i32
  %60 = getelementptr inbounds i8, ptr %57, i32 %59
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %54, ptr align 1 %60, i32 %52, i1 false)
  store ptr %50, ptr %5, align 4, !tbaa !5
  br label %61

61:                                               ; preds = %44, %46, %17
  %62 = phi ptr [ %18, %17 ], [ null, %46 ], [ %45, %44 ]
  ret ptr %62
}

; Function Attrs: cold noinline nounwind optsize
define hidden noundef ptr @fa_error_index(i64 noundef %0, i64 noundef %1) local_unnamed_addr #13 {
  %3 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #29
  store i64 1, ptr %3, align 8, !tbaa !92
  %4 = getelementptr inbounds %struct.IndexError, ptr %3, i32 0, i32 1
  store i64 %0, ptr %4, align 8, !tbaa !94
  %5 = getelementptr inbounds %struct.IndexError, ptr %3, i32 0, i32 2
  store i64 %1, ptr %5, align 8, !tbaa !95
  ret ptr %3
}

; Function Attrs: nounwind optsize
define hidden void @fa_array_reserve(ptr nocapture noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #2 {
  %4 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %5 = load i64, ptr %4, align 8, !tbaa !50
  %6 = icmp slt i64 %5, %1
  br i1 %6, label %7, label %20

7:                                                ; preds = %3
  %8 = mul nsw i64 %2, %1
  %9 = trunc i64 %8 to i32
  %10 = tail call ptr @fa_alloc(i32 noundef %9, i32 noundef 16) #29
  %11 = load ptr, ptr %0, align 8, !tbaa !46
  %12 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %13 = load i64, ptr %12, align 8, !tbaa !49
  %14 = mul nsw i64 %13, %2
  %15 = trunc i64 %14 to i32
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %10, ptr align 1 %11, i32 %15, i1 false)
  %16 = load ptr, ptr %0, align 8, !tbaa !46
  %17 = load i64, ptr %4, align 8, !tbaa !50
  %18 = mul nsw i64 %17, %2
  %19 = trunc i64 %18 to i32
  tail call void @fa_free(ptr noundef %16, i32 noundef %19, i32 noundef 16) #29
  store ptr %10, ptr %0, align 8, !tbaa !46
  store i64 %1, ptr %4, align 8, !tbaa !50
  br label %20

20:                                               ; preds = %3, %7
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_array_assign(ptr noundef %0, ptr noundef readonly %1, i64 noundef %2) local_unnamed_addr #2 {
  %4 = icmp eq ptr %0, %1
  br i1 %4, label %30, label %5

5:                                                ; preds = %3
  %6 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %7 = load i64, ptr %6, align 8, !tbaa !50
  %8 = getelementptr inbounds %struct.FaArrayHeader, ptr %1, i32 0, i32 1
  %9 = load i64, ptr %8, align 8, !tbaa !49
  %10 = icmp slt i64 %7, %9
  br i1 %10, label %13, label %11

11:                                               ; preds = %5
  %12 = load ptr, ptr %0, align 8, !tbaa !46
  br label %22

13:                                               ; preds = %5
  %14 = mul nsw i64 %9, %2
  %15 = trunc i64 %14 to i32
  %16 = tail call ptr @fa_alloc(i32 noundef %15, i32 noundef 16) #29
  %17 = load ptr, ptr %0, align 8, !tbaa !46
  %18 = load i64, ptr %6, align 8, !tbaa !50
  %19 = mul nsw i64 %18, %2
  %20 = trunc i64 %19 to i32
  tail call void @fa_free(ptr noundef %17, i32 noundef %20, i32 noundef 16) #29
  store ptr %16, ptr %0, align 8, !tbaa !46
  %21 = load i64, ptr %8, align 8, !tbaa !49
  store i64 %21, ptr %6, align 8, !tbaa !50
  br label %22

22:                                               ; preds = %11, %13
  %23 = phi i64 [ %9, %11 ], [ %21, %13 ]
  %24 = phi ptr [ %12, %11 ], [ %16, %13 ]
  %25 = load ptr, ptr %1, align 8, !tbaa !46
  %26 = mul nsw i64 %23, %2
  %27 = trunc i64 %26 to i32
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %24, ptr align 1 %25, i32 %27, i1 false)
  %28 = load i64, ptr %8, align 8, !tbaa !49
  %29 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  store i64 %28, ptr %29, align 8, !tbaa !49
  br label %30

30:                                               ; preds = %3, %22
  ret void
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_array_copy(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #2 {
  %3 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %4 = load i64, ptr %3, align 8, !tbaa !49
  %5 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #29
  %6 = mul nsw i64 %4, %1
  %7 = trunc i64 %6 to i32
  %8 = tail call i32 @llvm.umax.i32(i32 %7, i32 1)
  %9 = tail call ptr @fa_alloc(i32 noundef %8, i32 noundef 16) #29
  store ptr %9, ptr %5, align 8, !tbaa !46
  %10 = getelementptr inbounds %struct.FaArrayHeader, ptr %5, i32 0, i32 1
  store i64 %4, ptr %10, align 8, !tbaa !49
  %11 = getelementptr inbounds %struct.FaArrayHeader, ptr %5, i32 0, i32 2
  store i64 %4, ptr %11, align 8, !tbaa !50
  %12 = load ptr, ptr %0, align 8, !tbaa !46
  %13 = load i64, ptr %3, align 8, !tbaa !49
  %14 = mul nsw i64 %13, %1
  %15 = trunc i64 %14 to i32
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %9, ptr align 1 %12, i32 %15, i1 false)
  ret ptr %5
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden void @fa_array_reverse(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #14 {
  %3 = alloca [64 x i8], align 16
  %4 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %5 = load i64, ptr %4, align 8, !tbaa !49
  %6 = icmp slt i64 %5, 2
  br i1 %6, label %36, label %7

7:                                                ; preds = %2
  call void @llvm.lifetime.start.p0(i64 64, ptr nonnull %3)
  %8 = add nuw i64 %5, 4294967295
  %9 = mul i64 %8, %1
  %10 = trunc i64 %9 to i32
  %11 = icmp sgt i32 %10, 0
  br i1 %11, label %12, label %35

12:                                               ; preds = %7
  %13 = load ptr, ptr %0, align 8, !tbaa !46
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
  call void @llvm.memcpy.p0.p0.i32(ptr nonnull align 16 %3, ptr align 1 %31, i32 %29, i1 false)
  %32 = getelementptr inbounds i8, ptr %20, i32 %30
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %31, ptr nonnull align 1 %32, i32 %29, i1 false)
  call void @llvm.memcpy.p0.p0.i32(ptr nonnull align 1 %32, ptr nonnull align 16 %3, i32 %29, i1 false)
  %33 = add nuw nsw i64 %26, 64
  %34 = icmp slt i64 %33, %1
  br i1 %34, label %25, label %21, !llvm.loop !97

35:                                               ; preds = %21, %7
  call void @llvm.lifetime.end.p0(i64 64, ptr nonnull %3)
  br label %36

36:                                               ; preds = %2, %35
  ret void
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_array_insert_slot(ptr nocapture noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #2 {
  %4 = tail call ptr @fa_array_push_slot(ptr noundef %0, i64 noundef %2) #30
  %5 = load ptr, ptr %0, align 8, !tbaa !46
  %6 = mul nsw i64 %2, %1
  %7 = trunc i64 %6 to i32
  %8 = getelementptr inbounds i8, ptr %5, i32 %7
  %9 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %10 = load i64, ptr %9, align 8, !tbaa !49
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
declare void @llvm.memmove.p0.p0.i32(ptr nocapture writeonly, ptr nocapture readonly, i32, i1 immarg) #7

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define hidden void @fa_array_remove(ptr nocapture noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #15 {
  %4 = load ptr, ptr %0, align 8, !tbaa !46
  %5 = mul nsw i64 %2, %1
  %6 = trunc i64 %5 to i32
  %7 = getelementptr inbounds i8, ptr %4, i32 %6
  %8 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %9 = load i64, ptr %8, align 8, !tbaa !49
  %10 = xor i64 %1, -1
  %11 = add i64 %9, %10
  %12 = mul nsw i64 %11, %2
  %13 = trunc i64 %12 to i32
  %14 = trunc i64 %2 to i32
  %15 = getelementptr inbounds i8, ptr %7, i32 %14
  tail call void @llvm.memmove.p0.p0.i32(ptr align 1 %7, ptr align 1 %15, i32 %13, i1 false)
  %16 = load i64, ptr %8, align 8, !tbaa !49
  %17 = add nsw i64 %16, -1
  store i64 %17, ptr %8, align 8, !tbaa !49
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_array_sort(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef readonly %2, ptr noundef %3) local_unnamed_addr #2 {
  %5 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %6 = load i64, ptr %5, align 8, !tbaa !49
  %7 = icmp slt i64 %6, 2
  br i1 %7, label %86, label %8

8:                                                ; preds = %4
  %9 = mul nsw i64 %6, %1
  %10 = trunc i64 %9 to i32
  %11 = load ptr, ptr %0, align 8, !tbaa !46
  %12 = tail call ptr @fa_alloc(i32 noundef %10, i32 noundef 16) #29
  %13 = trunc i64 %1 to i32
  br label %14

14:                                               ; preds = %22, %8
  %15 = phi ptr [ %11, %8 ], [ %16, %22 ]
  %16 = phi ptr [ %12, %8 ], [ %15, %22 ]
  %17 = phi i64 [ 1, %8 ], [ %18, %22 ]
  %18 = shl nsw i64 %17, 1
  br label %24

19:                                               ; preds = %22
  %20 = load ptr, ptr %0, align 8, !tbaa !46
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
  %43 = tail call i32 %2(ptr noundef %3, ptr noundef %39, ptr noundef %42) #29
  %44 = icmp slt i32 %43, 0
  %45 = mul nsw i64 %34, %1
  %46 = trunc i64 %45 to i32
  %47 = getelementptr inbounds i8, ptr %16, i32 %46
  br i1 %44, label %48, label %50

48:                                               ; preds = %33
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %47, ptr align 1 %39, i32 %13, i1 false)
  %49 = add nsw i64 %35, 1
  br label %52

50:                                               ; preds = %33
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %47, ptr align 1 %42, i32 %13, i1 false)
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
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %65, ptr align 1 %68, i32 %71, i1 false)
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
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %75, ptr align 1 %78, i32 %81, i1 false)
  %82 = icmp slt i64 %28, %6
  br i1 %82, label %24, label %22, !llvm.loop !100

83:                                               ; preds = %19
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %20, ptr align 1 %16, i32 %10, i1 false)
  br label %84

84:                                               ; preds = %19, %83
  %85 = phi ptr [ %16, %83 ], [ %15, %19 ]
  tail call void @fa_free(ptr noundef %85, i32 noundef %10, i32 noundef 16) #29
  br label %86

86:                                               ; preds = %84, %4
  ret void
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_arguments() local_unnamed_addr #2 {
  %1 = load i32, ptr @argument_count, align 4, !tbaa !1
  %2 = zext i32 %1 to i64
  %3 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #29
  %4 = mul i32 %1, 12
  %5 = tail call i32 @llvm.umax.i32(i32 %4, i32 1)
  %6 = tail call ptr @fa_alloc(i32 noundef %5, i32 noundef 16) #29
  store ptr %6, ptr %3, align 8, !tbaa !46
  %7 = getelementptr inbounds %struct.FaArrayHeader, ptr %3, i32 0, i32 1
  store i64 %2, ptr %7, align 8, !tbaa !49
  %8 = getelementptr inbounds %struct.FaArrayHeader, ptr %3, i32 0, i32 2
  store i64 %2, ptr %8, align 8, !tbaa !50
  %9 = load ptr, ptr @argument_items, align 4, !tbaa !5
  %10 = load i32, ptr @argument_count, align 4, !tbaa !1
  %11 = mul i32 %10, 12
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %6, ptr align 1 %9, i32 %11, i1 false)
  ret ptr %3
}

; Function Attrs: nounwind optsize
define hidden void @fa_array_free(ptr noundef %0, i64 noundef %1) local_unnamed_addr #2 {
  %3 = load ptr, ptr %0, align 8, !tbaa !46
  %4 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %5 = load i64, ptr %4, align 8, !tbaa !50
  %6 = mul nsw i64 %5, %1
  %7 = trunc i64 %6 to i32
  tail call void @fa_free(ptr noundef %3, i32 noundef %7, i32 noundef 16) #29
  tail call void @fa_free(ptr noundef nonnull %0, i32 noundef 24, i32 noundef 8) #29
  ret void
}

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read)
define hidden ptr @fa_segmented_at(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2, i32 noundef %3) local_unnamed_addr #16 {
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
  %20 = load ptr, ptr %19, align 4, !tbaa !5
  %21 = add i64 %17, %1
  %22 = mul nsw i64 %21, %2
  %23 = trunc i64 %22 to i32
  %24 = getelementptr inbounds i8, ptr %20, i32 %23
  ret ptr %24
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_segmented_new() local_unnamed_addr #2 {
  %1 = tail call ptr @fa_alloc(i32 noundef 208, i32 noundef 8) #29
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(208) %1, i8 0, i32 208, i1 false)
  ret ptr %1
}

; Function Attrs: mustprogress nocallback nofree nounwind willreturn memory(argmem: write)
declare void @llvm.memset.p0.i32(ptr nocapture writeonly, i8, i32, i1 immarg) #17

; Function Attrs: nounwind optsize
define hidden ptr @fa_segmented_push_slot(ptr nocapture noundef %0, i64 noundef %1, i32 noundef %2) local_unnamed_addr #2 {
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
  %29 = tail call ptr @fa_alloc(i32 noundef %28, i32 noundef 16) #29
  %30 = trunc i64 %17 to i32
  %31 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 2, i32 %30
  store ptr %29, ptr %31, align 4, !tbaa !5
  %32 = add nuw nsw i64 %17, 1
  store i64 %32, ptr %18, align 8, !tbaa !104
  br label %33

33:                                               ; preds = %21, %23
  %34 = phi i32 [ %22, %21 ], [ %30, %23 ]
  %35 = add nsw i64 %4, 1
  store i64 %35, ptr %0, align 8, !tbaa !102
  %36 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 2, i32 %34
  %37 = load ptr, ptr %36, align 4, !tbaa !5
  %38 = add i64 %16, %4
  %39 = mul nsw i64 %38, %1
  %40 = trunc i64 %39 to i32
  %41 = getelementptr inbounds i8, ptr %37, i32 %40
  ret ptr %41
}

; Function Attrs: nounwind optsize
define hidden void @fa_segmented_free(ptr noundef %0, i64 noundef %1, i32 noundef %2) local_unnamed_addr #2 {
  %4 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 1
  %5 = load i64, ptr %4, align 8, !tbaa !104
  %6 = icmp sgt i64 %5, 0
  br i1 %6, label %7, label %9

7:                                                ; preds = %3
  %8 = sext i32 %2 to i64
  br label %10

9:                                                ; preds = %10, %3
  tail call void @fa_free(ptr noundef nonnull %0, i32 noundef 208, i32 noundef 8) #29
  ret void

10:                                               ; preds = %7, %10
  %11 = phi i64 [ 0, %7 ], [ %19, %10 ]
  %12 = trunc i64 %11 to i32
  %13 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 2, i32 %12
  %14 = load ptr, ptr %13, align 4, !tbaa !5
  %15 = tail call i64 @llvm.usub.sat.i64(i64 %11, i64 1)
  %16 = add i64 %15, %8
  %17 = shl i64 %1, %16
  %18 = trunc i64 %17 to i32
  tail call void @fa_free(ptr noundef %14, i32 noundef %18, i32 noundef 16) #29
  %19 = add nuw nsw i64 %11, 1
  %20 = load i64, ptr %4, align 8, !tbaa !104
  %21 = icmp slt i64 %19, %20
  br i1 %21, label %10, label %9, !llvm.loop !105
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_segmented_flatten(ptr nocapture noundef readonly %0, i64 noundef %1, i32 noundef %2) local_unnamed_addr #2 {
  %4 = load i64, ptr %0, align 8, !tbaa !102
  %5 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #29
  %6 = mul nsw i64 %4, %1
  %7 = trunc i64 %6 to i32
  %8 = tail call i32 @llvm.umax.i32(i32 %7, i32 1)
  %9 = tail call ptr @fa_alloc(i32 noundef %8, i32 noundef 16) #29
  store ptr %9, ptr %5, align 8, !tbaa !46
  %10 = getelementptr inbounds %struct.FaArrayHeader, ptr %5, i32 0, i32 1
  store i64 %4, ptr %10, align 8, !tbaa !49
  %11 = getelementptr inbounds %struct.FaArrayHeader, ptr %5, i32 0, i32 2
  store i64 %4, ptr %11, align 8, !tbaa !50
  %12 = load i64, ptr %0, align 8, !tbaa !102
  %13 = icmp sgt i64 %12, 0
  br i1 %13, label %14, label %16

14:                                               ; preds = %3
  %15 = sext i32 %2 to i64
  br label %17

16:                                               ; preds = %17, %3
  ret ptr %5

17:                                               ; preds = %14, %17
  %18 = phi i64 [ %12, %14 ], [ %37, %17 ]
  %19 = phi i64 [ 0, %14 ], [ %36, %17 ]
  %20 = phi i64 [ 0, %14 ], [ %35, %17 ]
  %21 = tail call i64 @llvm.usub.sat.i64(i64 %19, i64 1)
  %22 = add i64 %21, %15
  %23 = shl nuw i64 1, %22
  %24 = sub nsw i64 %18, %20
  %25 = tail call i64 @llvm.smin.i64(i64 %23, i64 %24)
  %26 = load ptr, ptr %5, align 8, !tbaa !46
  %27 = mul nsw i64 %20, %1
  %28 = trunc i64 %27 to i32
  %29 = getelementptr inbounds i8, ptr %26, i32 %28
  %30 = trunc i64 %19 to i32
  %31 = getelementptr inbounds %struct.FaSegmented, ptr %0, i32 0, i32 2, i32 %30
  %32 = load ptr, ptr %31, align 4, !tbaa !5
  %33 = mul nsw i64 %25, %1
  %34 = trunc i64 %33 to i32
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %29, ptr align 1 %32, i32 %34, i1 false)
  %35 = add nsw i64 %25, %20
  %36 = add nuw nsw i64 %19, 1
  %37 = load i64, ptr %0, align 8, !tbaa !102
  %38 = icmp slt i64 %35, %37
  br i1 %38, label %17, label %16, !llvm.loop !106
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_segmented_from_array(ptr noundef %0, i64 noundef %1, i32 noundef %2) local_unnamed_addr #2 {
  %4 = tail call noundef ptr @fa_alloc(i32 noundef 208, i32 noundef 8) #29
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(208) %4, i8 0, i32 208, i1 false)
  %5 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %6 = load i64, ptr %5, align 8, !tbaa !49
  %7 = icmp sgt i64 %6, 0
  br i1 %7, label %8, label %10

8:                                                ; preds = %3
  %9 = trunc i64 %1 to i32
  br label %16

10:                                               ; preds = %16, %3
  %11 = load ptr, ptr %0, align 8, !tbaa !46
  %12 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %13 = load i64, ptr %12, align 8, !tbaa !50
  %14 = mul nsw i64 %13, %1
  %15 = trunc i64 %14 to i32
  tail call void @fa_free(ptr noundef %11, i32 noundef %15, i32 noundef 16) #29
  tail call void @fa_free(ptr noundef nonnull %0, i32 noundef 24, i32 noundef 8) #29
  ret ptr %4

16:                                               ; preds = %8, %16
  %17 = phi i64 [ 0, %8 ], [ %23, %16 ]
  %18 = tail call ptr @fa_segmented_push_slot(ptr noundef %4, i64 noundef %1, i32 noundef %2) #30
  %19 = load ptr, ptr %0, align 8, !tbaa !46
  %20 = mul nsw i64 %17, %1
  %21 = trunc i64 %20 to i32
  %22 = getelementptr inbounds i8, ptr %19, i32 %21
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %18, ptr align 1 %22, i32 %9, i1 false)
  %23 = add nuw nsw i64 %17, 1
  %24 = load i64, ptr %5, align 8, !tbaa !49
  %25 = icmp slt i64 %23, %24
  br i1 %25, label %16, label %10, !llvm.loop !107
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_heap_alloc(i32 noundef %0) local_unnamed_addr #2 {
  %2 = icmp ugt i32 %0, 8192
  br i1 %2, label %3, label %13

3:                                                ; preds = %1
  %4 = add i32 %0, 65551
  %5 = lshr i32 %4, 16
  %6 = icmp ult i32 %4, 65536
  br i1 %6, label %66, label %7

7:                                                ; preds = %3
  %8 = tail call fastcc ptr @take_pages(i32 noundef %5) #30
  %9 = icmp eq ptr %8, null
  br i1 %9, label %66, label %10

10:                                               ; preds = %7
  store i32 65535, ptr %8, align 4, !tbaa !108
  %11 = getelementptr inbounds %struct.PageHeader, ptr %8, i32 0, i32 1
  store i32 %5, ptr %11, align 4, !tbaa !110
  %12 = getelementptr inbounds i8, ptr %8, i32 16
  br label %66

13:                                               ; preds = %1
  %14 = icmp ult i32 %0, 129
  br i1 %14, label %15, label %22

15:                                               ; preds = %13
  %16 = add nuw nsw i32 %0, 15
  %17 = and i32 %16, 496
  %18 = icmp ugt i32 %0, 16
  %19 = select i1 %18, i32 %17, i32 16
  %20 = lshr exact i32 %19, 4
  %21 = add nsw i32 %20, -1
  br label %39

22:                                               ; preds = %13, %37
  %23 = phi i32 [ %26, %37 ], [ 8, %13 ]
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
  br i1 %36, label %37, label %27, !llvm.loop !111

37:                                               ; preds = %33
  %38 = shl i32 %24, 1
  br label %22

39:                                               ; preds = %27, %15
  %40 = phi i32 [ %19, %15 ], [ %31, %27 ]
  %41 = phi i32 [ %21, %15 ], [ %29, %27 ]
  %42 = getelementptr inbounds [40 x ptr], ptr @free_blocks, i32 0, i32 %41
  %43 = load ptr, ptr %42, align 4, !tbaa !5
  %44 = icmp eq ptr %43, null
  br i1 %44, label %47, label %45

45:                                               ; preds = %39
  %46 = load ptr, ptr %43, align 4, !tbaa !112
  store ptr %46, ptr %42, align 4, !tbaa !5
  br label %66

47:                                               ; preds = %39
  %48 = getelementptr inbounds [40 x ptr], ptr @class_next, i32 0, i32 %41
  %49 = load ptr, ptr %48, align 4, !tbaa !5
  %50 = getelementptr inbounds i8, ptr %49, i32 %40
  %51 = getelementptr inbounds [40 x ptr], ptr @class_end, i32 0, i32 %41
  %52 = load ptr, ptr %51, align 4, !tbaa !5
  %53 = icmp ugt ptr %50, %52
  %54 = icmp eq ptr %49, null
  %55 = or i1 %54, %53
  br i1 %55, label %56, label %63

56:                                               ; preds = %47
  %57 = tail call fastcc ptr @take_pages(i32 noundef 1) #30
  %58 = icmp eq ptr %57, null
  br i1 %58, label %66, label %59

59:                                               ; preds = %56
  store i32 %41, ptr %57, align 4, !tbaa !108
  %60 = getelementptr inbounds %struct.PageHeader, ptr %57, i32 0, i32 1
  store i32 1, ptr %60, align 4, !tbaa !110
  %61 = getelementptr inbounds i8, ptr %57, i32 16
  %62 = getelementptr inbounds i8, ptr %57, i32 65536
  store ptr %62, ptr %51, align 4, !tbaa !5
  br label %63

63:                                               ; preds = %59, %47
  %64 = phi ptr [ %61, %59 ], [ %49, %47 ]
  %65 = getelementptr inbounds i8, ptr %64, i32 %40
  store ptr %65, ptr %48, align 4, !tbaa !5
  br label %66

66:                                               ; preds = %45, %63, %56, %3, %7, %10
  %67 = phi ptr [ null, %3 ], [ %12, %10 ], [ null, %7 ], [ %43, %45 ], [ %64, %63 ], [ null, %56 ]
  ret ptr %67
}

; Function Attrs: nounwind optsize
define internal fastcc ptr @take_pages(i32 noundef %0) unnamed_addr #2 {
  br label %2

2:                                                ; preds = %6, %1
  %3 = phi ptr [ @free_spans, %1 ], [ %4, %6 ]
  %4 = load ptr, ptr %3, align 4, !tbaa !5
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
  %13 = load ptr, ptr %4, align 4, !tbaa !116
  br label %20

14:                                               ; preds = %10
  %15 = shl nuw i32 %0, 16
  %16 = getelementptr inbounds i8, ptr %4, i32 %15
  %17 = load ptr, ptr %4, align 4, !tbaa !116
  store ptr %17, ptr %16, align 4, !tbaa !116
  %18 = sub i32 %8, %0
  %19 = getelementptr inbounds %struct.FreeSpan, ptr %16, i32 0, i32 1
  store i32 %18, ptr %19, align 4, !tbaa !114
  br label %20

20:                                               ; preds = %12, %14
  %21 = phi ptr [ %16, %14 ], [ %13, %12 ]
  store ptr %21, ptr %3, align 4, !tbaa !5
  br label %40

22:                                               ; preds = %2
  %23 = tail call i32 @llvm.umax.i32(i32 %0, i32 16)
  %24 = tail call ptr @fa_pages(i32 noundef %23) #29
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
  store ptr null, ptr %30, align 4, !tbaa !116
  br label %33

33:                                               ; preds = %33, %28
  %34 = phi ptr [ @free_spans, %28 ], [ %35, %33 ]
  %35 = load ptr, ptr %34, align 4, !tbaa !5
  %36 = icmp ne ptr %35, null
  %37 = icmp ult ptr %35, %30
  %38 = and i1 %36, %37
  br i1 %38, label %33, label %39, !llvm.loop !117

39:                                               ; preds = %33
  store ptr %35, ptr %30, align 4, !tbaa !116
  store ptr %30, ptr %34, align 4, !tbaa !5
  br label %40

40:                                               ; preds = %20, %22, %39, %26
  %41 = phi ptr [ null, %22 ], [ %24, %39 ], [ %24, %26 ], [ %4, %20 ]
  ret ptr %41
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden void @fa_heap_free(ptr noundef %0) local_unnamed_addr #14 {
  %2 = ptrtoint ptr %0 to i32
  %3 = and i32 %2, -65536
  %4 = inttoptr i32 %3 to ptr
  %5 = load i32, ptr %4, align 65536, !tbaa !108
  %6 = icmp eq i32 %5, 65535
  br i1 %6, label %7, label %41

7:                                                ; preds = %1
  %8 = getelementptr inbounds %struct.PageHeader, ptr %4, i32 0, i32 1
  %9 = load i32, ptr %8, align 4, !tbaa !110
  br label %10

10:                                               ; preds = %10, %7
  %11 = phi ptr [ null, %7 ], [ %13, %10 ]
  %12 = phi ptr [ @free_spans, %7 ], [ %13, %10 ]
  %13 = load ptr, ptr %12, align 4, !tbaa !5
  %14 = icmp ne ptr %13, null
  %15 = icmp ult ptr %13, %4
  %16 = and i1 %14, %15
  br i1 %16, label %10, label %17, !llvm.loop !118

17:                                               ; preds = %10
  store ptr %13, ptr %4, align 65536, !tbaa !116
  br i1 %14, label %18, label %27

18:                                               ; preds = %17
  %19 = shl i32 %9, 16
  %20 = getelementptr inbounds i8, ptr %4, i32 %19
  %21 = icmp eq ptr %20, %13
  br i1 %21, label %22, label %27

22:                                               ; preds = %18
  %23 = getelementptr inbounds %struct.FreeSpan, ptr %13, i32 0, i32 1
  %24 = load i32, ptr %23, align 4, !tbaa !114
  %25 = add i32 %24, %9
  store i32 %25, ptr %8, align 4, !tbaa !114
  %26 = load ptr, ptr %13, align 4, !tbaa !116
  store ptr %26, ptr %4, align 65536, !tbaa !116
  br label %27

27:                                               ; preds = %22, %18, %17
  %28 = phi ptr [ %26, %22 ], [ %13, %18 ], [ null, %17 ]
  %29 = phi i32 [ %25, %22 ], [ %9, %18 ], [ %9, %17 ]
  %30 = icmp eq ptr %11, null
  br i1 %30, label %40, label %31

31:                                               ; preds = %27
  %32 = getelementptr inbounds %struct.FreeSpan, ptr %11, i32 0, i32 1
  %33 = load i32, ptr %32, align 4, !tbaa !114
  %34 = shl i32 %33, 16
  %35 = getelementptr inbounds i8, ptr %11, i32 %34
  %36 = icmp eq ptr %35, %4
  br i1 %36, label %37, label %39

37:                                               ; preds = %31
  %38 = add i32 %33, %29
  store i32 %38, ptr %32, align 4, !tbaa !114
  store ptr %28, ptr %11, align 4, !tbaa !116
  br label %44

39:                                               ; preds = %31
  store ptr %4, ptr %11, align 4, !tbaa !116
  br label %44

40:                                               ; preds = %27
  store ptr %4, ptr @free_spans, align 4, !tbaa !5
  br label %44

41:                                               ; preds = %1
  %42 = getelementptr inbounds [40 x ptr], ptr @free_blocks, i32 0, i32 %5
  %43 = load ptr, ptr %42, align 4, !tbaa !5
  store ptr %43, ptr %0, align 4, !tbaa !112
  store ptr %0, ptr %42, align 4, !tbaa !5
  br label %44

44:                                               ; preds = %40, %39, %37, %41
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_leak_check() local_unnamed_addr #2 {
  %1 = load i32, ptr @output_length, align 4, !tbaa !1
  %2 = icmp eq i32 %1, 0
  br i1 %2, label %4, label %3

3:                                                ; preds = %0
  tail call void @fa_write(i32 noundef 1, ptr noundef nonnull @output, i32 noundef %1) #29
  store i32 0, ptr @output_length, align 4, !tbaa !1
  br label %4

4:                                                ; preds = %0, %3
  %5 = load ptr, ptr @builder, align 4, !tbaa !5
  %6 = icmp eq ptr %5, null
  br i1 %6, label %9, label %7

7:                                                ; preds = %4
  %8 = load i32, ptr @builder_capacity, align 4, !tbaa !1
  tail call void @fa_free(ptr noundef nonnull %5, i32 noundef %8, i32 noundef 1) #29
  store ptr null, ptr @builder, align 4, !tbaa !5
  store i32 0, ptr @builder_capacity, align 4, !tbaa !1
  br label %9

9:                                                ; preds = %7, %4
  %10 = load ptr, ptr @argument_items, align 4, !tbaa !5
  %11 = icmp eq ptr %10, null
  br i1 %11, label %16, label %12

12:                                               ; preds = %9
  %13 = load i32, ptr @argument_count, align 4, !tbaa !1
  %14 = mul i32 %13, 12
  %15 = add i32 %14, 12
  tail call void @fa_free(ptr noundef nonnull %10, i32 noundef %15, i32 noundef 4) #29
  store ptr null, ptr @argument_items, align 4, !tbaa !5
  br label %16

16:                                               ; preds = %12, %9
  %17 = load ptr, ptr @argument_bytes, align 4, !tbaa !5
  %18 = icmp eq ptr %17, null
  br i1 %18, label %20, label %19

19:                                               ; preds = %16
  tail call void @fa_free(ptr noundef nonnull %17, i32 noundef 1, i32 noundef 1) #29
  store ptr null, ptr @argument_bytes, align 4, !tbaa !5
  br label %20

20:                                               ; preds = %19, %16
  %21 = load i32, ptr @fa_live_blocks, align 4, !tbaa !1
  %22 = icmp eq i32 %21, 0
  br i1 %22, label %26, label %23

23:                                               ; preds = %20
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  tail call fastcc void @append_c(ptr noundef nonnull @.str.13) #30
  %24 = load i32, ptr @fa_live_blocks, align 4, !tbaa !1
  %25 = zext i32 %24 to i64
  tail call void @fa_b_u64(i64 noundef %25) #30
  tail call fastcc void @append_c(ptr noundef nonnull @.str.14) #30
  tail call void @fa_b_line(i32 noundef 2) #30
  tail call void @fa_exit(i32 noundef 102) #31
  unreachable

26:                                               ; preds = %20
  ret void
}

; Function Attrs: cold noinline noreturn nounwind optsize
define hidden void @fa_overflow(i64 noundef %0, i64 noundef %1, i32 noundef %2, ptr noundef %3) local_unnamed_addr #18 {
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  tail call fastcc void @append_c(ptr noundef nonnull @.str.15) #30
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
  tail call fastcc void @append_u128(i128 noundef %13, i32 noundef %15) #30
  br label %17

16:                                               ; preds = %4
  tail call fastcc void @append_u128(i128 noundef %9, i32 noundef 0) #30
  br label %17

17:                                               ; preds = %16, %10
  tail call fastcc void @append_c(ptr noundef nonnull @.str.16) #30
  tail call fastcc void @append_c(ptr noundef %3) #30
  br label %18

18:                                               ; preds = %18, %17
  %19 = phi i32 [ 0, %17 ], [ %23, %18 ]
  %20 = getelementptr inbounds i8, ptr %3, i32 %19
  %21 = load i8, ptr %20, align 1, !tbaa !9
  %22 = icmp eq i8 %21, 0
  %23 = add i32 %19, 1
  br i1 %22, label %24, label %18, !llvm.loop !21

24:                                               ; preds = %18
  %25 = getelementptr inbounds i8, ptr %20, i32 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %25) #33
  unreachable
}

; Function Attrs: noreturn nounwind optsize
define internal fastcc void @stop_with_builder(ptr noundef %0) unnamed_addr #3 {
  tail call void @fa_flush() #30
  %2 = load ptr, ptr @builder, align 4, !tbaa !5
  %3 = load i32, ptr @builder_length, align 4, !tbaa !1
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.30, i32 noundef 6) #29
  tail call void @fa_write(i32 noundef 2, ptr noundef %2, i32 noundef %3) #29
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.31, i32 noundef 6) #29
  br label %4

4:                                                ; preds = %4, %1
  %5 = phi i32 [ 0, %1 ], [ %9, %4 ]
  %6 = getelementptr inbounds i8, ptr %0, i32 %5
  %7 = load i8, ptr %6, align 1, !tbaa !9
  %8 = icmp eq i8 %7, 0
  %9 = add i32 %5, 1
  br i1 %8, label %10, label %4, !llvm.loop !21

10:                                               ; preds = %4
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull %0, i32 noundef %5) #29
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.21, i32 noundef 1) #29
  tail call void @fa_exit(i32 noundef 101) #31
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define hidden void @fa_overflow_unknown(ptr noundef %0) local_unnamed_addr #18 {
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  tail call fastcc void @append_c(ptr noundef nonnull @.str.17) #30
  tail call fastcc void @append_c(ptr noundef %0) #30
  br label %2

2:                                                ; preds = %2, %1
  %3 = phi i32 [ 0, %1 ], [ %7, %2 ]
  %4 = getelementptr inbounds i8, ptr %0, i32 %3
  %5 = load i8, ptr %4, align 1, !tbaa !9
  %6 = icmp eq i8 %5, 0
  %7 = add i32 %3, 1
  br i1 %6, label %8, label %2, !llvm.loop !21

8:                                                ; preds = %2
  %9 = getelementptr inbounds i8, ptr %4, i32 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %9) #33
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define hidden void @fa_float_overflow(double noundef %0, ptr noundef %1) local_unnamed_addr #18 {
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  tail call fastcc void @append_c(ptr noundef nonnull @.str.15) #30
  tail call void @fa_b_f64(double noundef %0) #30
  tail call fastcc void @append_c(ptr noundef nonnull @.str.16) #30
  tail call fastcc void @append_c(ptr noundef %1) #30
  br label %3

3:                                                ; preds = %3, %2
  %4 = phi i32 [ 0, %2 ], [ %8, %3 ]
  %5 = getelementptr inbounds i8, ptr %1, i32 %4
  %6 = load i8, ptr %5, align 1, !tbaa !9
  %7 = icmp eq i8 %6, 0
  %8 = add i32 %4, 1
  br i1 %7, label %9, label %3, !llvm.loop !21

9:                                                ; preds = %3
  %10 = getelementptr inbounds i8, ptr %5, i32 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %10) #33
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define hidden void @fa_assert_failed(ptr noundef %0) local_unnamed_addr #18 {
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  tail call fastcc void @append_c(ptr noundef nonnull @.str.18) #30
  tail call fastcc void @append_c(ptr noundef %0) #30
  br label %2

2:                                                ; preds = %2, %1
  %3 = phi i32 [ 0, %1 ], [ %7, %2 ]
  %4 = getelementptr inbounds i8, ptr %0, i32 %3
  %5 = load i8, ptr %4, align 1, !tbaa !9
  %6 = icmp eq i8 %5, 0
  %7 = add i32 %3, 1
  br i1 %6, label %8, label %2, !llvm.loop !21

8:                                                ; preds = %2
  %9 = getelementptr inbounds i8, ptr %4, i32 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %9) #33
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define hidden void @fa_assert_failed_operands(ptr noundef %0) local_unnamed_addr #18 {
  %2 = alloca %struct.FaString, align 4
  call void @llvm.lifetime.start.p0(i64 12, ptr nonnull %2) #32
  call void @fa_b_take(ptr noundef nonnull %2) #30
  tail call fastcc void @append_c(ptr noundef nonnull @.str.18) #30
  tail call fastcc void @append_c(ptr noundef %0) #30
  tail call fastcc void @append_c(ptr noundef nonnull @.str.19) #30
  %3 = load ptr, ptr %2, align 4, !tbaa !23
  %4 = getelementptr inbounds %struct.FaString, ptr %2, i32 0, i32 1
  %5 = load i32, ptr %4, align 4, !tbaa !25
  tail call void @fa_b_bytes(ptr noundef %3, i32 noundef %5) #30
  br label %6

6:                                                ; preds = %6, %1
  %7 = phi i32 [ 0, %1 ], [ %11, %6 ]
  %8 = getelementptr inbounds i8, ptr %0, i32 %7
  %9 = load i8, ptr %8, align 1, !tbaa !9
  %10 = icmp eq i8 %9, 0
  %11 = add i32 %7, 1
  br i1 %10, label %12, label %6, !llvm.loop !21

12:                                               ; preds = %6
  %13 = getelementptr inbounds i8, ptr %8, i32 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %13) #33
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define hidden void @fa_uncaught(ptr noundef %0) local_unnamed_addr #18 {
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  tail call void @fa_format_union(ptr noundef %0, i32 noundef 1) #29
  tail call void @fa_flush() #30
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.20, i32 noundef 7) #29
  %2 = load ptr, ptr @builder, align 4, !tbaa !5
  %3 = load i32, ptr @builder_length, align 4, !tbaa !1
  tail call void @fa_write(i32 noundef 2, ptr noundef %2, i32 noundef %3) #29
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.21, i32 noundef 1) #29
  tail call void @fa_exit(i32 noundef 1) #31
  unreachable
}

; Function Attrs: optsize
declare void @fa_format_union(ptr noundef, i32 noundef) local_unnamed_addr #1

; Function Attrs: cold mustprogress nofree noinline norecurse nosync nounwind optsize willreturn memory(none)
define hidden noundef nonnull ptr @fa_error_division_by_zero() local_unnamed_addr #19 {
  ret ptr @division_by_zero
}

; Function Attrs: cold noinline nounwind optsize
define hidden noundef ptr @fa_error_conversion(i64 noundef %0, i64 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3) local_unnamed_addr #13 {
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
  tail call fastcc void @append_u128(i128 noundef %13, i32 noundef %15) #30
  br label %17

16:                                               ; preds = %4
  tail call fastcc void @append_u128(i128 noundef %9, i32 noundef 0) #30
  br label %17

17:                                               ; preds = %16, %10
  tail call fastcc void @append_c(ptr noundef nonnull @.str.16) #30
  tail call fastcc void @append_c(ptr noundef %3) #30
  %18 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #29
  store i64 4, ptr %18, align 8, !tbaa !119
  %19 = getelementptr inbounds %struct.MessageError, ptr %18, i32 0, i32 1
  tail call void @fa_b_take(ptr noundef nonnull %19) #30
  ret ptr %18
}

; Function Attrs: cold noinline nounwind optsize
define hidden noundef ptr @fa_error_negative_exponent(i64 noundef %0) local_unnamed_addr #13 {
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  tail call fastcc void @append_c(ptr noundef nonnull @.str.22) #30
  %2 = tail call i64 @llvm.abs.i64(i64 %0, i1 false)
  %3 = lshr i64 %0, 63
  %4 = trunc i64 %3 to i32
  tail call fastcc void @append_u64(i64 noundef %2, i32 noundef %4) #30
  %5 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #29
  store i64 4, ptr %5, align 8, !tbaa !119
  %6 = getelementptr inbounds %struct.MessageError, ptr %5, i32 0, i32 1
  tail call void @fa_b_take(ptr noundef nonnull %6) #30
  ret ptr %5
}

; Function Attrs: cold noinline nounwind optsize
define hidden noundef ptr @fa_error_float(double noundef %0, i32 noundef %1) local_unnamed_addr #13 {
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  tail call void @fa_b_f64(double noundef %0) #30
  %3 = icmp eq i32 %1, 0
  %4 = select i1 %3, ptr @.str.24, ptr @.str.23
  tail call fastcc void @append_c(ptr noundef nonnull %4) #30
  %5 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #29
  store i64 4, ptr %5, align 8, !tbaa !119
  %6 = getelementptr inbounds %struct.MessageError, ptr %5, i32 0, i32 1
  tail call void @fa_b_take(ptr noundef nonnull %6) #30
  ret ptr %5
}

; Function Attrs: cold noinline nounwind optsize
define hidden noundef ptr @fa_error_shift(i64 noundef %0, i64 noundef %1) local_unnamed_addr #13 {
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  tail call fastcc void @append_c(ptr noundef nonnull @.str.25) #30
  %3 = tail call i64 @llvm.abs.i64(i64 %0, i1 false)
  %4 = lshr i64 %0, 63
  %5 = trunc i64 %4 to i32
  tail call fastcc void @append_u64(i64 noundef %3, i32 noundef %5) #30
  tail call fastcc void @append_c(ptr noundef nonnull @.str.26) #30
  %6 = tail call i64 @llvm.abs.i64(i64 %1, i1 false)
  %7 = lshr i64 %1, 63
  %8 = trunc i64 %7 to i32
  tail call fastcc void @append_u64(i64 noundef %6, i32 noundef %8) #30
  %9 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #29
  store i64 4, ptr %9, align 8, !tbaa !119
  %10 = getelementptr inbounds %struct.MessageError, ptr %9, i32 0, i32 1
  tail call void @fa_b_take(ptr noundef nonnull %10) #30
  ret ptr %9
}

; Function Attrs: cold noinline nounwind optsize
define hidden noundef ptr @fa_error_checked(i64 noundef %0, i64 noundef %1, ptr nocapture noundef readonly %2) local_unnamed_addr #13 {
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  tail call fastcc void @append_c(ptr noundef nonnull @.str.15) #30
  %4 = zext i64 %1 to i128
  %5 = shl nuw i128 %4, 64
  %6 = zext i64 %0 to i128
  %7 = or disjoint i128 %5, %6
  %8 = icmp slt i64 %1, 0
  %9 = sub i128 0, %7
  %10 = select i1 %8, i128 %9, i128 %7
  %11 = lshr i64 %1, 63
  %12 = trunc i64 %11 to i32
  tail call fastcc void @append_u128(i128 noundef %10, i32 noundef %12) #30
  tail call fastcc void @append_c(ptr noundef nonnull @.str.16) #30
  tail call fastcc void @append_c(ptr noundef %2) #30
  %13 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #29
  store i64 4, ptr %13, align 8, !tbaa !119
  %14 = getelementptr inbounds %struct.MessageError, ptr %13, i32 0, i32 1
  tail call void @fa_b_take(ptr noundef nonnull %14) #30
  ret ptr %13
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_parse_integer(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef writeonly %2) local_unnamed_addr #2 {
  %4 = icmp eq i32 %1, 0
  br i1 %4, label %13, label %5

5:                                                ; preds = %3
  %6 = load i8, ptr %0, align 1, !tbaa !9
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
  %20 = load i8, ptr %19, align 1, !tbaa !9
  %21 = zext i8 %20 to i64
  br label %64

22:                                               ; preds = %9, %22
  %23 = phi i32 [ %30, %22 ], [ %11, %9 ]
  %24 = phi i32 [ %29, %22 ], [ 1, %9 ]
  %25 = getelementptr inbounds i8, ptr %0, i32 %23
  %26 = load i8, ptr %25, align 1, !tbaa !9
  %27 = add i8 %26, -48
  %28 = icmp ult i8 %27, 10
  %29 = select i1 %28, i32 %24, i32 0
  %30 = add nuw i32 %23, 1
  %31 = icmp eq i32 %30, %1
  br i1 %31, label %14, label %22, !llvm.loop !121

32:                                               ; preds = %9
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  br i1 %4, label %54, label %33

33:                                               ; preds = %16, %32
  tail call fastcc void @reserve(i32 noundef 1) #30
  %34 = load ptr, ptr @builder, align 4, !tbaa !5
  %35 = load i32, ptr @builder_length, align 4, !tbaa !1
  %36 = getelementptr inbounds i8, ptr %34, i32 %35
  store i8 34, ptr %36, align 1, !tbaa !9
  %37 = add i32 %35, 1
  store i32 %37, ptr @builder_length, align 4, !tbaa !1
  br label %38

38:                                               ; preds = %33, %47
  %39 = phi i32 [ 0, %33 ], [ %52, %47 ]
  %40 = getelementptr inbounds i8, ptr %0, i32 %39
  %41 = load i8, ptr %40, align 1, !tbaa !9
  switch i8 %41, label %47 [
    i8 92, label %42
    i8 34, label %42
  ]

42:                                               ; preds = %38, %38
  tail call fastcc void @reserve(i32 noundef 1) #30
  %43 = load ptr, ptr @builder, align 4, !tbaa !5
  %44 = load i32, ptr @builder_length, align 4, !tbaa !1
  %45 = getelementptr inbounds i8, ptr %43, i32 %44
  store i8 92, ptr %45, align 1, !tbaa !9
  %46 = add i32 %44, 1
  store i32 %46, ptr @builder_length, align 4, !tbaa !1
  br label %47

47:                                               ; preds = %38, %42
  tail call fastcc void @reserve(i32 noundef 1) #30
  %48 = load ptr, ptr @builder, align 4, !tbaa !5
  %49 = load i32, ptr @builder_length, align 4, !tbaa !1
  %50 = getelementptr inbounds i8, ptr %48, i32 %49
  store i8 %41, ptr %50, align 1, !tbaa !9
  %51 = add i32 %49, 1
  store i32 %51, ptr @builder_length, align 4, !tbaa !1
  %52 = add nuw i32 %39, 1
  %53 = icmp eq i32 %52, %1
  br i1 %53, label %54, label %38, !llvm.loop !122

54:                                               ; preds = %47, %32, %13
  %55 = phi ptr [ @.str.27, %13 ], [ @.str.27, %32 ], [ @.str.28, %47 ]
  tail call fastcc void @append_c(ptr noundef nonnull %55) #30
  %56 = tail call fastcc ptr @parse_error(ptr noundef %0, i32 noundef %1) #30
  br label %90

57:                                               ; preds = %64
  %58 = getelementptr inbounds i8, ptr %0, i32 %71
  %59 = load i8, ptr %58, align 1, !tbaa !9
  %60 = zext i8 %59 to i64
  %61 = sub nsw i64 47, %60
  %62 = udiv i64 %61, 10
  %63 = icmp ugt i64 %70, %62
  br i1 %63, label %73, label %64, !llvm.loop !123

64:                                               ; preds = %18, %57
  %65 = phi i64 [ %21, %18 ], [ %60, %57 ]
  %66 = phi i64 [ 0, %18 ], [ %70, %57 ]
  %67 = phi i32 [ %11, %18 ], [ %71, %57 ]
  %68 = mul nuw i64 %66, 10
  %69 = add i64 %68, -48
  %70 = add i64 %69, %65
  %71 = add nuw i32 %67, 1
  %72 = icmp eq i32 %71, %1
  br i1 %72, label %73, label %57, !llvm.loop !123

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
  tail call fastcc void @reserve(i32 noundef %1) #30
  %82 = load ptr, ptr @builder, align 4, !tbaa !5
  %83 = load i32, ptr @builder_length, align 4, !tbaa !1
  %84 = getelementptr inbounds i8, ptr %82, i32 %83
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %84, ptr nonnull align 1 %0, i32 %1, i1 false)
  %85 = add i32 %83, %1
  store i32 %85, ptr @builder_length, align 4, !tbaa !1
  tail call fastcc void @append_c(ptr noundef nonnull @.str.23) #30
  %86 = tail call fastcc ptr @parse_error(ptr noundef nonnull %0, i32 noundef %1) #30
  br label %90

87:                                               ; preds = %75
  %88 = sub i64 0, %76
  %89 = select i1 %10, i64 %76, i64 %88
  store i64 %89, ptr %2, align 8, !tbaa !124
  br label %90

90:                                               ; preds = %81, %87, %54
  %91 = phi ptr [ %56, %54 ], [ %86, %81 ], [ null, %87 ]
  ret ptr %91
}

; Function Attrs: nounwind optsize
define internal fastcc noundef ptr @parse_error(ptr nocapture noundef readonly %0, i32 noundef %1) unnamed_addr #2 {
  %3 = tail call ptr @fa_alloc(i32 noundef 32, i32 noundef 8) #29
  store i64 5, ptr %3, align 8, !tbaa !125
  %4 = icmp eq i32 %1, 0
  br i1 %4, label %7, label %5

5:                                                ; preds = %2
  %6 = tail call ptr @fa_alloc(i32 noundef %1, i32 noundef 1) #29, !noalias !127
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %6, ptr align 1 %0, i32 %1, i1 false), !noalias !127
  br label %7

7:                                                ; preds = %2, %5
  %8 = phi ptr [ %6, %5 ], [ @.str.29, %2 ]
  %9 = getelementptr inbounds %struct.ParseError, ptr %3, i32 0, i32 1
  store ptr %8, ptr %9, align 8, !tbaa.struct !13
  %10 = getelementptr inbounds %struct.ParseError, ptr %3, i32 0, i32 1, i32 1
  store i32 %1, ptr %10, align 4, !tbaa.struct !14
  %11 = getelementptr inbounds %struct.ParseError, ptr %3, i32 0, i32 1, i32 2
  store i32 %1, ptr %11, align 8, !tbaa.struct !15
  %12 = getelementptr inbounds %struct.ParseError, ptr %3, i32 0, i32 2
  tail call void @fa_b_take(ptr noundef nonnull %12) #30
  ret ptr %3
}

; Function Attrs: nounwind optsize
define hidden i64 @fa_pow_i64(i64 noundef %0, i64 noundef %1, i32 noundef %2, ptr noundef %3) local_unnamed_addr #2 {
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
  tail call void @fa_overflow_unknown(ptr noundef %3) #34
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
  tail call void @fa_overflow_unknown(ptr noundef %3) #34
  unreachable
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare { i64, i1 } @llvm.smul.with.overflow.i64(i64, i64) #20

; Function Attrs: nounwind optsize
define weak hidden double @pow(double noundef %0, double noundef %1) local_unnamed_addr #2 {
  %3 = tail call double @fa_host_pow(double noundef %0, double noundef %1) #29
  ret double %3
}

; Function Attrs: optsize
declare double @fa_host_pow(double noundef, double noundef) local_unnamed_addr #21

; Function Attrs: nounwind optsize
define weak hidden double @round(double noundef %0) local_unnamed_addr #2 {
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
declare double @llvm.trunc.f64(double) #20

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(read, inaccessiblemem: none)
define hidden ptr @fa_table_entry(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #22 {
  %4 = load ptr, ptr %0, align 8, !tbaa !130
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
  %17 = load ptr, ptr %16, align 4, !tbaa !5
  %18 = add i64 %14, %1
  %19 = mul nsw i64 %18, %2
  %20 = trunc i64 %19 to i32
  %21 = getelementptr inbounds i8, ptr %17, i32 %20
  ret ptr %21
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_table_new(i64 noundef %0, i64 noundef %1) local_unnamed_addr #2 {
  %3 = tail call ptr @fa_alloc(i32 noundef 56, i32 noundef 8) #29
  %4 = tail call i64 @llvm.smax.i64(i64 %0, i64 4)
  %5 = mul nsw i64 %4, %1
  %6 = trunc i64 %5 to i32
  %7 = tail call ptr @fa_alloc(i32 noundef %6, i32 noundef 16) #29
  store ptr %7, ptr %3, align 8, !tbaa !130
  %8 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 1
  store i64 0, ptr %8, align 8, !tbaa !132
  %9 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 2
  store i64 0, ptr %9, align 8, !tbaa !133
  %10 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 3
  store i64 %4, ptr %10, align 8, !tbaa !134
  %11 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 4
  store ptr null, ptr %11, align 8, !tbaa !135
  %12 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 6
  store i64 0, ptr %12, align 8, !tbaa !136
  %13 = shl nuw nsw i64 %4, 1
  br label %14

14:                                               ; preds = %14, %2
  %15 = phi i64 [ 8, %2 ], [ %17, %14 ]
  %16 = icmp slt i64 %15, %13
  %17 = shl nsw i64 %15, 1
  br i1 %16, label %14, label %18, !llvm.loop !137

18:                                               ; preds = %14
  tail call fastcc void @table_index(ptr noundef nonnull %3, i64 noundef %15, i64 noundef %1) #30
  ret ptr %3
}

; Function Attrs: nounwind optsize
define internal fastcc void @table_index(ptr nocapture noundef %0, i64 noundef %1, i64 noundef %2) unnamed_addr #2 {
  %4 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %5 = load ptr, ptr %4, align 8, !tbaa !135
  %6 = icmp eq ptr %5, null
  br i1 %6, label %13, label %7

7:                                                ; preds = %3
  %8 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %9 = load i64, ptr %8, align 8, !tbaa !138
  %10 = trunc i64 %9 to i32
  %11 = shl i32 %10, 3
  %12 = add i32 %11, 8
  tail call void @fa_free(ptr noundef nonnull %5, i32 noundef %12, i32 noundef 8) #29
  br label %13

13:                                               ; preds = %7, %3
  %14 = trunc i64 %1 to i32
  %15 = shl i32 %14, 3
  %16 = tail call ptr @fa_alloc(i32 noundef %15, i32 noundef 8) #29
  store ptr %16, ptr %4, align 8, !tbaa !135
  tail call void @llvm.memset.p0.i32(ptr align 8 %16, i8 0, i32 %15, i1 false)
  %17 = add nsw i64 %1, -1
  %18 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  store i64 %17, ptr %18, align 8, !tbaa !138
  %19 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 2
  %20 = load i64, ptr %19, align 8, !tbaa !133
  %21 = icmp sgt i64 %20, 0
  br i1 %21, label %22, label %25

22:                                               ; preds = %13
  %23 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 6
  %24 = load ptr, ptr %0, align 8, !tbaa !130
  br label %26

25:                                               ; preds = %72, %13
  ret void

26:                                               ; preds = %22, %72
  %27 = phi i64 [ %20, %22 ], [ %74, %72 ]
  %28 = phi i64 [ 0, %22 ], [ %73, %72 ]
  %29 = load i64, ptr %23, align 8, !tbaa !136
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
  %44 = load ptr, ptr %43, align 4, !tbaa !5
  %45 = add i64 %40, %28
  br label %46

46:                                               ; preds = %26, %39
  %47 = phi i64 [ %45, %39 ], [ %28, %26 ]
  %48 = phi ptr [ %44, %39 ], [ %24, %26 ]
  %49 = mul nsw i64 %47, %2
  %50 = trunc i64 %49 to i32
  %51 = getelementptr inbounds i8, ptr %48, i32 %50
  %52 = getelementptr inbounds i8, ptr %51, i32 8
  %53 = load i64, ptr %52, align 8, !tbaa !124
  %54 = icmp eq i64 %53, 0
  br i1 %54, label %55, label %57

55:                                               ; preds = %46
  %56 = add nuw nsw i64 %28, 1
  br label %72

57:                                               ; preds = %46
  %58 = load i64, ptr %51, align 8, !tbaa !124
  %59 = load i64, ptr %18, align 8, !tbaa !138
  %60 = load ptr, ptr %4, align 8, !tbaa !135
  br label %61

61:                                               ; preds = %61, %57
  %62 = phi i64 [ %58, %57 ], [ %68, %61 ]
  %63 = and i64 %62, %59
  %64 = trunc i64 %63 to i32
  %65 = getelementptr inbounds i64, ptr %60, i32 %64
  %66 = load i64, ptr %65, align 8, !tbaa !124
  %67 = icmp eq i64 %66, 0
  %68 = add nsw i64 %63, 1
  br i1 %67, label %69, label %61, !llvm.loop !139

69:                                               ; preds = %61
  %70 = add nuw nsw i64 %28, 1
  store i64 %70, ptr %65, align 8, !tbaa !124
  %71 = load i64, ptr %19, align 8, !tbaa !133
  br label %72

72:                                               ; preds = %55, %69
  %73 = phi i64 [ %56, %55 ], [ %70, %69 ]
  %74 = phi i64 [ %27, %55 ], [ %71, %69 ]
  %75 = icmp slt i64 %73, %74
  br i1 %75, label %26, label %25, !llvm.loop !140
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_table_new_growing(i64 noundef %0, i64 noundef %1) local_unnamed_addr #2 {
  %3 = tail call ptr @fa_alloc(i32 noundef 56, i32 noundef 8) #29
  %4 = tail call i64 @llvm.smax.i64(i64 %0, i64 4)
  %5 = tail call noundef ptr @fa_alloc(i32 noundef 208, i32 noundef 8) #29
  tail call void @llvm.memset.p0.i32(ptr noundef nonnull align 8 dereferenceable(208) %5, i8 0, i32 208, i1 false)
  store ptr %5, ptr %3, align 8, !tbaa !130
  %6 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 1
  store i64 0, ptr %6, align 8, !tbaa !132
  %7 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 2
  store i64 0, ptr %7, align 8, !tbaa !133
  %8 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 3
  store i64 %4, ptr %8, align 8, !tbaa !134
  %9 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 4
  store ptr null, ptr %9, align 8, !tbaa !135
  %10 = getelementptr inbounds %struct.FaTable, ptr %3, i32 0, i32 6
  store i64 1, ptr %10, align 8, !tbaa !136
  %11 = shl nuw nsw i64 %4, 1
  br label %12

12:                                               ; preds = %12, %2
  %13 = phi i64 [ 8, %2 ], [ %15, %12 ]
  %14 = icmp slt i64 %13, %11
  %15 = shl nsw i64 %13, 1
  br i1 %14, label %12, label %16, !llvm.loop !137

16:                                               ; preds = %12
  tail call fastcc void @table_index(ptr noundef nonnull %3, i64 noundef %13, i64 noundef %1) #30
  ret ptr %3
}

; Function Attrs: alwaysinline nounwind optsize
define hidden i64 @fa_table_find(ptr nocapture noundef readonly %0, i64 noundef %1, ptr noundef %2, ptr nocapture noundef readonly %3, i64 noundef %4) local_unnamed_addr #23 {
  %6 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %7 = load i64, ptr %6, align 8, !tbaa !138
  %8 = and i64 %7, %1
  %9 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %10 = load ptr, ptr %9, align 8, !tbaa !135
  %11 = trunc i64 %8 to i32
  %12 = getelementptr inbounds i64, ptr %10, i32 %11
  %13 = load i64, ptr %12, align 8, !tbaa !124
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
  %23 = load ptr, ptr %0, align 8, !tbaa !130
  %24 = mul nsw i64 %22, %4
  %25 = trunc i64 %24 to i32
  %26 = getelementptr inbounds i8, ptr %23, i32 %25
  %27 = load i64, ptr %26, align 8, !tbaa !124
  %28 = icmp eq i64 %27, %1
  br i1 %28, label %29, label %36

29:                                               ; preds = %21
  %30 = getelementptr inbounds i8, ptr %26, i32 16
  %31 = tail call i32 %3(ptr noundef nonnull %30, ptr noundef %2) #29
  %32 = icmp eq i32 %31, 0
  br i1 %32, label %33, label %45

33:                                               ; preds = %29
  %34 = load i64, ptr %6, align 8, !tbaa !138
  %35 = load ptr, ptr %9, align 8, !tbaa !135
  br label %36

36:                                               ; preds = %33, %21, %15
  %37 = phi ptr [ %35, %33 ], [ %16, %21 ], [ %16, %15 ]
  %38 = phi i64 [ %34, %33 ], [ %17, %21 ], [ %17, %15 ]
  %39 = add nsw i64 %19, 1
  %40 = and i64 %38, %39
  %41 = trunc i64 %40 to i32
  %42 = getelementptr inbounds i64, ptr %37, i32 %41
  %43 = load i64, ptr %42, align 8, !tbaa !124
  %44 = icmp eq i64 %43, 0
  br i1 %44, label %45, label %15

45:                                               ; preds = %29, %36, %5
  %46 = phi i64 [ -1, %5 ], [ -1, %36 ], [ %22, %29 ]
  ret i64 %46
}

; Function Attrs: alwaysinline nounwind optsize
define hidden i64 @fa_table_find_growing(ptr nocapture noundef readonly %0, i64 noundef %1, ptr noundef %2, ptr nocapture noundef readonly %3, i64 noundef %4) local_unnamed_addr #23 {
  %6 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %7 = load i64, ptr %6, align 8, !tbaa !138
  %8 = and i64 %7, %1
  %9 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %10 = load ptr, ptr %9, align 8, !tbaa !135
  %11 = trunc i64 %8 to i32
  %12 = getelementptr inbounds i64, ptr %10, i32 %11
  %13 = load i64, ptr %12, align 8, !tbaa !124
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
  %23 = load ptr, ptr %0, align 8, !tbaa !130
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
  %36 = load ptr, ptr %35, align 4, !tbaa !5
  %37 = add nsw i64 %32, %22
  %38 = mul nsw i64 %37, %4
  %39 = trunc i64 %38 to i32
  %40 = getelementptr inbounds i8, ptr %36, i32 %39
  %41 = load i64, ptr %40, align 8, !tbaa !124
  %42 = icmp eq i64 %41, %1
  br i1 %42, label %43, label %50

43:                                               ; preds = %31
  %44 = getelementptr inbounds i8, ptr %40, i32 16
  %45 = tail call i32 %3(ptr noundef nonnull %44, ptr noundef %2) #29
  %46 = icmp eq i32 %45, 0
  br i1 %46, label %47, label %59

47:                                               ; preds = %43
  %48 = load i64, ptr %6, align 8, !tbaa !138
  %49 = load ptr, ptr %9, align 8, !tbaa !135
  br label %50

50:                                               ; preds = %47, %31, %15
  %51 = phi ptr [ %49, %47 ], [ %16, %31 ], [ %16, %15 ]
  %52 = phi i64 [ %48, %47 ], [ %17, %31 ], [ %17, %15 ]
  %53 = add nsw i64 %19, 1
  %54 = and i64 %52, %53
  %55 = trunc i64 %54 to i32
  %56 = getelementptr inbounds i64, ptr %51, i32 %55
  %57 = load i64, ptr %56, align 8, !tbaa !124
  %58 = icmp eq i64 %57, 0
  br i1 %58, label %59, label %15

59:                                               ; preds = %43, %50, %5
  %60 = phi i64 [ -1, %5 ], [ -1, %50 ], [ %22, %43 ]
  ret i64 %60
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_table_append(ptr nocapture noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #2 {
  %4 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 2
  %5 = load i64, ptr %4, align 8, !tbaa !133
  %6 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 3
  %7 = load i64, ptr %6, align 8, !tbaa !134
  %8 = icmp slt i64 %5, %7
  br i1 %8, label %72, label %9

9:                                                ; preds = %3
  %10 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 6
  %11 = load i64, ptr %10, align 8, !tbaa !136
  %12 = icmp eq i64 %11, 0
  br i1 %12, label %21, label %13

13:                                               ; preds = %9
  %14 = shl nsw i64 %7, 1
  store i64 %14, ptr %6, align 8, !tbaa !134
  %15 = shl nsw i64 %7, 2
  br label %16

16:                                               ; preds = %16, %13
  %17 = phi i64 [ 8, %13 ], [ %19, %16 ]
  %18 = icmp slt i64 %17, %15
  %19 = shl nsw i64 %17, 1
  br i1 %18, label %16, label %20, !llvm.loop !137

20:                                               ; preds = %16
  tail call fastcc void @table_index(ptr noundef %0, i64 noundef %17, i64 noundef %2) #30
  br label %72

21:                                               ; preds = %9
  %22 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 1
  %23 = load i64, ptr %22, align 8, !tbaa !132
  %24 = shl nsw i64 %23, 1
  %25 = icmp sgt i64 %24, %5
  %26 = zext i1 %25 to i64
  %27 = shl nsw i64 %7, %26
  %28 = mul nsw i64 %27, %2
  %29 = trunc i64 %28 to i32
  %30 = tail call ptr @fa_alloc(i32 noundef %29, i32 noundef 16) #29
  %31 = load i64, ptr %4, align 8, !tbaa !133
  %32 = icmp sgt i64 %31, 0
  br i1 %32, label %33, label %35

33:                                               ; preds = %21
  %34 = trunc i64 %2 to i32
  br label %45

35:                                               ; preds = %62, %21
  %36 = phi i64 [ 0, %21 ], [ %64, %62 ]
  %37 = load ptr, ptr %0, align 8, !tbaa !130
  %38 = load i64, ptr %6, align 8, !tbaa !134
  %39 = mul nsw i64 %38, %2
  %40 = trunc i64 %39 to i32
  tail call void @fa_free(ptr noundef %37, i32 noundef %40, i32 noundef 16) #29
  store ptr %30, ptr %0, align 8, !tbaa !130
  store i64 %36, ptr %4, align 8, !tbaa !133
  store i64 %27, ptr %6, align 8, !tbaa !134
  %41 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %42 = load i64, ptr %41, align 8, !tbaa !138
  %43 = add nsw i64 %42, 1
  %44 = shl nsw i64 %27, 1
  br label %67

45:                                               ; preds = %62, %33
  %46 = phi i64 [ %31, %33 ], [ %63, %62 ]
  %47 = phi i64 [ 0, %33 ], [ %65, %62 ]
  %48 = phi i64 [ 0, %33 ], [ %64, %62 ]
  %49 = load ptr, ptr %0, align 8, !tbaa !130
  %50 = mul nsw i64 %47, %2
  %51 = trunc i64 %50 to i32
  %52 = getelementptr inbounds i8, ptr %49, i32 %51
  %53 = getelementptr inbounds i8, ptr %52, i32 8
  %54 = load i64, ptr %53, align 8, !tbaa !124
  %55 = icmp eq i64 %54, 0
  br i1 %55, label %62, label %56

56:                                               ; preds = %45
  %57 = mul nsw i64 %48, %2
  %58 = trunc i64 %57 to i32
  %59 = getelementptr inbounds i8, ptr %30, i32 %58
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %59, ptr nonnull align 1 %52, i32 %34, i1 false)
  %60 = add nsw i64 %48, 1
  %61 = load i64, ptr %4, align 8, !tbaa !133
  br label %62

62:                                               ; preds = %56, %45
  %63 = phi i64 [ %61, %56 ], [ %46, %45 ]
  %64 = phi i64 [ %60, %56 ], [ %48, %45 ]
  %65 = add nuw nsw i64 %47, 1
  %66 = icmp slt i64 %65, %63
  br i1 %66, label %45, label %35, !llvm.loop !141

67:                                               ; preds = %67, %35
  %68 = phi i64 [ %43, %35 ], [ %70, %67 ]
  %69 = icmp slt i64 %68, %44
  %70 = shl nsw i64 %68, 1
  br i1 %69, label %67, label %71, !llvm.loop !142

71:                                               ; preds = %67
  tail call fastcc void @table_index(ptr noundef nonnull %0, i64 noundef %68, i64 noundef %2) #30
  br label %72

72:                                               ; preds = %3, %20, %71
  %73 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 6
  %74 = load i64, ptr %73, align 8, !tbaa !136
  %75 = icmp eq i64 %74, 0
  %76 = load ptr, ptr %0, align 8, !tbaa !130
  br i1 %75, label %79, label %77

77:                                               ; preds = %72
  %78 = tail call ptr @fa_segmented_push_slot(ptr noundef %76, i64 noundef %2, i32 noundef 2) #30
  br label %84

79:                                               ; preds = %72
  %80 = load i64, ptr %4, align 8, !tbaa !133
  %81 = mul nsw i64 %80, %2
  %82 = trunc i64 %81 to i32
  %83 = getelementptr inbounds i8, ptr %76, i32 %82
  br label %84

84:                                               ; preds = %79, %77
  %85 = phi ptr [ %78, %77 ], [ %83, %79 ]
  store i64 %1, ptr %85, align 8, !tbaa !124
  %86 = getelementptr inbounds i8, ptr %85, i32 8
  store i64 1, ptr %86, align 8, !tbaa !124
  %87 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %88 = load i64, ptr %87, align 8, !tbaa !138
  %89 = and i64 %88, %1
  %90 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %91 = load ptr, ptr %90, align 8, !tbaa !135
  %92 = trunc i64 %89 to i32
  %93 = getelementptr inbounds i64, ptr %91, i32 %92
  %94 = load i64, ptr %93, align 8, !tbaa !124
  %95 = icmp sgt i64 %94, 0
  br i1 %95, label %96, label %104

96:                                               ; preds = %84, %96
  %97 = phi i64 [ %99, %96 ], [ %89, %84 ]
  %98 = add nsw i64 %97, 1
  %99 = and i64 %98, %88
  %100 = trunc i64 %99 to i32
  %101 = getelementptr inbounds i64, ptr %91, i32 %100
  %102 = load i64, ptr %101, align 8, !tbaa !124
  %103 = icmp sgt i64 %102, 0
  br i1 %103, label %96, label %104, !llvm.loop !143

104:                                              ; preds = %96, %84
  %105 = phi ptr [ %93, %84 ], [ %101, %96 ]
  %106 = load i64, ptr %4, align 8, !tbaa !133
  %107 = add nsw i64 %106, 1
  store i64 %107, ptr %105, align 8, !tbaa !124
  %108 = load i64, ptr %4, align 8, !tbaa !133
  %109 = add nsw i64 %108, 1
  store i64 %109, ptr %4, align 8, !tbaa !133
  %110 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 1
  %111 = load i64, ptr %110, align 8, !tbaa !132
  %112 = add nsw i64 %111, 1
  store i64 %112, ptr %110, align 8, !tbaa !132
  ret ptr %85
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden void @fa_table_kill(ptr nocapture noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #14 {
  %4 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 6
  %5 = load i64, ptr %4, align 8, !tbaa !136
  %6 = icmp eq i64 %5, 0
  %7 = load ptr, ptr %0, align 8, !tbaa !130
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
  %21 = load ptr, ptr %20, align 4, !tbaa !5
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
  %31 = load i64, ptr %30, align 8, !tbaa !124
  %32 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %33 = load i64, ptr %32, align 8, !tbaa !138
  %34 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %35 = load ptr, ptr %34, align 8, !tbaa !135
  %36 = add nsw i64 %1, 1
  br label %37

37:                                               ; preds = %37, %26
  %38 = phi i64 [ %31, %26 ], [ %44, %37 ]
  %39 = and i64 %38, %33
  %40 = trunc i64 %39 to i32
  %41 = getelementptr inbounds i64, ptr %35, i32 %40
  %42 = load i64, ptr %41, align 8, !tbaa !124
  %43 = icmp eq i64 %42, %36
  %44 = add nsw i64 %39, 1
  br i1 %43, label %45, label %37, !llvm.loop !144

45:                                               ; preds = %37
  store i64 -1, ptr %41, align 8, !tbaa !124
  %46 = getelementptr inbounds i8, ptr %30, i32 8
  store i64 0, ptr %46, align 8, !tbaa !124
  %47 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 1
  %48 = load i64, ptr %47, align 8, !tbaa !132
  %49 = add nsw i64 %48, -1
  store i64 %49, ptr %47, align 8, !tbaa !132
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, argmem: readwrite, inaccessiblemem: none)
define hidden void @fa_table_clear(ptr nocapture noundef %0) local_unnamed_addr #24 {
  %2 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 1
  store i64 0, ptr %2, align 8, !tbaa !132
  %3 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 2
  store i64 0, ptr %3, align 8, !tbaa !133
  %4 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 6
  %5 = load i64, ptr %4, align 8, !tbaa !136
  %6 = icmp eq i64 %5, 0
  br i1 %6, label %9, label %7

7:                                                ; preds = %1
  %8 = load ptr, ptr %0, align 8, !tbaa !130
  store i64 0, ptr %8, align 8, !tbaa !102
  br label %9

9:                                                ; preds = %7, %1
  %10 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %11 = load ptr, ptr %10, align 8, !tbaa !135
  %12 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %13 = load i64, ptr %12, align 8, !tbaa !138
  %14 = trunc i64 %13 to i32
  %15 = shl i32 %14, 3
  %16 = add i32 %15, 8
  tail call void @llvm.memset.p0.i32(ptr align 8 %11, i8 0, i32 %16, i1 false)
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_table_free(ptr noundef %0, i64 noundef %1) local_unnamed_addr #2 {
  %3 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 6
  %4 = load i64, ptr %3, align 8, !tbaa !136
  %5 = icmp eq i64 %4, 0
  %6 = load ptr, ptr %0, align 8, !tbaa !130
  br i1 %5, label %8, label %7

7:                                                ; preds = %2
  tail call void @fa_segmented_free(ptr noundef %6, i64 noundef %1, i32 noundef 2) #30
  br label %13

8:                                                ; preds = %2
  %9 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 3
  %10 = load i64, ptr %9, align 8, !tbaa !134
  %11 = mul nsw i64 %10, %1
  %12 = trunc i64 %11 to i32
  tail call void @fa_free(ptr noundef %6, i32 noundef %12, i32 noundef 16) #29
  br label %13

13:                                               ; preds = %8, %7
  %14 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 4
  %15 = load ptr, ptr %14, align 8, !tbaa !135
  %16 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 5
  %17 = load i64, ptr %16, align 8, !tbaa !138
  %18 = trunc i64 %17 to i32
  %19 = shl i32 %18, 3
  %20 = add i32 %19, 8
  tail call void @fa_free(ptr noundef %15, i32 noundef %20, i32 noundef 8) #29
  tail call void @fa_free(ptr noundef nonnull %0, i32 noundef 56, i32 noundef 8) #29
  ret void
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_table_copy(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #2 {
  %3 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 6
  %4 = load i64, ptr %3, align 8, !tbaa !136
  %5 = icmp eq i64 %4, 0
  %6 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 1
  %7 = load i64, ptr %6, align 8, !tbaa !132
  %8 = tail call i64 @llvm.smax.i64(i64 %7, i64 4)
  br i1 %5, label %11, label %9

9:                                                ; preds = %2
  %10 = tail call ptr @fa_table_new_growing(i64 noundef %8, i64 noundef %1) #30
  br label %13

11:                                               ; preds = %2
  %12 = tail call ptr @fa_table_new(i64 noundef %8, i64 noundef %1) #30
  br label %13

13:                                               ; preds = %11, %9
  %14 = phi ptr [ %10, %9 ], [ %12, %11 ]
  %15 = getelementptr inbounds %struct.FaTable, ptr %0, i32 0, i32 2
  %16 = load i64, ptr %15, align 8, !tbaa !133
  %17 = icmp sgt i64 %16, 0
  br i1 %17, label %18, label %20

18:                                               ; preds = %13
  %19 = trunc i64 %1 to i32
  br label %27

20:                                               ; preds = %73, %13
  %21 = phi i64 [ 0, %13 ], [ %75, %73 ]
  %22 = getelementptr inbounds %struct.FaTable, ptr %14, i32 0, i32 2
  store i64 %21, ptr %22, align 8, !tbaa !133
  %23 = getelementptr inbounds %struct.FaTable, ptr %14, i32 0, i32 1
  store i64 %21, ptr %23, align 8, !tbaa !132
  %24 = getelementptr inbounds %struct.FaTable, ptr %14, i32 0, i32 5
  %25 = load i64, ptr %24, align 8, !tbaa !138
  %26 = add nsw i64 %25, 1
  tail call fastcc void @table_index(ptr noundef %14, i64 noundef %26, i64 noundef %1) #30
  ret ptr %14

27:                                               ; preds = %18, %73
  %28 = phi i64 [ %16, %18 ], [ %74, %73 ]
  %29 = phi i64 [ 0, %18 ], [ %75, %73 ]
  %30 = phi i64 [ 0, %18 ], [ %76, %73 ]
  %31 = load ptr, ptr %0, align 8, !tbaa !130
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
  %45 = load i64, ptr %44, align 8, !tbaa !124
  %46 = icmp eq i64 %45, 0
  br i1 %46, label %73, label %63

47:                                               ; preds = %32, %34
  %48 = phi i64 [ %39, %34 ], [ 0, %32 ]
  %49 = phi i64 [ %37, %34 ], [ 0, %32 ]
  %50 = trunc i64 %49 to i32
  %51 = getelementptr inbounds %struct.FaSegmented, ptr %31, i32 0, i32 2, i32 %50
  %52 = load ptr, ptr %51, align 4, !tbaa !5
  %53 = add i64 %48, %30
  %54 = mul nsw i64 %53, %1
  %55 = trunc i64 %54 to i32
  %56 = getelementptr inbounds i8, ptr %52, i32 %55
  %57 = getelementptr inbounds i8, ptr %56, i32 8
  %58 = load i64, ptr %57, align 8, !tbaa !124
  %59 = icmp eq i64 %58, 0
  br i1 %59, label %73, label %60

60:                                               ; preds = %47
  %61 = load ptr, ptr %14, align 8, !tbaa !130
  %62 = tail call ptr @fa_segmented_push_slot(ptr noundef %61, i64 noundef %1, i32 noundef 2) #30
  br label %68

63:                                               ; preds = %40
  %64 = load ptr, ptr %14, align 8, !tbaa !130
  %65 = mul nsw i64 %29, %1
  %66 = trunc i64 %65 to i32
  %67 = getelementptr inbounds i8, ptr %64, i32 %66
  br label %68

68:                                               ; preds = %63, %60
  %69 = phi ptr [ %56, %60 ], [ %43, %63 ]
  %70 = phi ptr [ %62, %60 ], [ %67, %63 ]
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %70, ptr nonnull align 1 %69, i32 %19, i1 false)
  %71 = add nsw i64 %29, 1
  %72 = load i64, ptr %15, align 8, !tbaa !133
  br label %73

73:                                               ; preds = %47, %68, %40
  %74 = phi i64 [ %72, %68 ], [ %28, %40 ], [ %28, %47 ]
  %75 = phi i64 [ %71, %68 ], [ %29, %40 ], [ %29, %47 ]
  %76 = add nuw nsw i64 %30, 1
  %77 = icmp slt i64 %76, %74
  br i1 %77, label %27, label %20, !llvm.loop !145
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: read)
define hidden i64 @fa_hash_bytes(ptr nocapture noundef readonly %0, i32 noundef %1) local_unnamed_addr #8 {
  %3 = icmp eq i32 %1, 0
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
  %12 = phi i32 [ %19, %11 ], [ 0, %2 ]
  %13 = phi i64 [ %18, %11 ], [ -3750763034362895579, %2 ]
  %14 = getelementptr inbounds i8, ptr %0, i32 %12
  %15 = load i8, ptr %14, align 1, !tbaa !9
  %16 = zext i8 %15 to i64
  %17 = xor i64 %13, %16
  %18 = mul i64 %17, 1099511628211
  %19 = add nuw i32 %12, 1
  %20 = icmp eq i32 %19, %1
  br i1 %20, label %4, label %11, !llvm.loop !146
}

; Function Attrs: cold noinline nounwind optsize
define hidden noundef ptr @fa_error_key_not_found() local_unnamed_addr #13 {
  %1 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #29
  store i64 2, ptr %1, align 8, !tbaa !119
  %2 = getelementptr inbounds %struct.MessageError, ptr %1, i32 0, i32 1
  tail call void @fa_b_take(ptr noundef nonnull %2) #30
  ret ptr %1
}

; Function Attrs: nounwind optsize
define weak hidden i128 @__multi3(i128 noundef %0, i128 noundef %1) local_unnamed_addr #2 {
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
define weak hidden i128 @__udivti3(i128 noundef %0, i128 noundef %1) local_unnamed_addr #2 {
  %3 = alloca i128, align 16
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %3) #32
  %4 = call fastcc i128 @divide_u128(i128 noundef %0, i128 noundef %1, ptr noundef nonnull %3) #30
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %3) #32
  ret i128 %4
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: write)
define internal fastcc i128 @divide_u128(i128 noundef %0, i128 noundef %1, ptr nocapture noundef writeonly %2) unnamed_addr #25 {
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
  store i128 %14, ptr %2, align 16, !tbaa !147
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
  br i1 %58, label %10, label %19, !llvm.loop !149
}

; Function Attrs: nounwind optsize
define weak hidden i128 @__umodti3(i128 noundef %0, i128 noundef %1) local_unnamed_addr #2 {
  %3 = alloca i128, align 16
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %3) #32
  %4 = call fastcc i128 @divide_u128(i128 noundef %0, i128 noundef %1, ptr noundef nonnull %3) #30
  %5 = load i128, ptr %3, align 16, !tbaa !147
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %3) #32
  ret i128 %5
}

; Function Attrs: nounwind optsize
define weak hidden i128 @__divti3(i128 noundef %0, i128 noundef %1) local_unnamed_addr #2 {
  %3 = alloca i128, align 16
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %3) #32
  %4 = tail call noundef i128 @llvm.abs.i128(i128 %0, i1 false)
  %5 = tail call noundef i128 @llvm.abs.i128(i128 %1, i1 false)
  %6 = call fastcc i128 @divide_u128(i128 noundef %4, i128 noundef %5, ptr noundef nonnull %3) #30
  %7 = lshr i128 %0, 127
  %8 = trunc i128 %7 to i32
  %9 = lshr i128 %1, 127
  %10 = trunc i128 %9 to i32
  %11 = icmp eq i32 %8, %10
  %12 = sub i128 0, %6
  %13 = select i1 %11, i128 %6, i128 %12
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %3) #32
  ret i128 %13
}

; Function Attrs: nounwind optsize
define weak hidden i128 @__modti3(i128 noundef %0, i128 noundef %1) local_unnamed_addr #2 {
  %3 = alloca i128, align 16
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %3) #32
  %4 = tail call noundef i128 @llvm.abs.i128(i128 %0, i1 false)
  %5 = tail call noundef i128 @llvm.abs.i128(i128 %1, i1 false)
  %6 = call fastcc i128 @divide_u128(i128 noundef %4, i128 noundef %5, ptr noundef nonnull %3) #30
  %7 = icmp slt i128 %0, 0
  %8 = load i128, ptr %3, align 16, !tbaa !147
  %9 = sub i128 0, %8
  %10 = select i1 %7, i128 %9, i128 %8
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %3) #32
  ret i128 %10
}

; Function Attrs: optsize
declare i32 @fa_host_format_float(double noundef, i32 noundef, ptr noundef) local_unnamed_addr #26

; Function Attrs: nofree norecurse nosync nounwind optsize memory(none)
define internal fastcc i32 @break_kind(i32 noundef %0) unnamed_addr #27 {
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
  %59 = load i32, ptr %58, align 8, !tbaa !108
  %60 = icmp ugt i32 %59, %0
  br i1 %60, label %52, label %61, !llvm.loop !150

61:                                               ; preds = %55
  %62 = getelementptr inbounds %struct.Range, ptr @EXTEND, i32 %57, i32 1
  %63 = load i32, ptr %62, align 4, !tbaa !110
  %64 = icmp ult i32 %63, %0
  %65 = add nuw i32 %57, 1
  br i1 %64, label %49, label %83, !llvm.loop !150

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
  %76 = load i32, ptr %75, align 8, !tbaa !108
  %77 = icmp ugt i32 %76, %0
  br i1 %77, label %69, label %78, !llvm.loop !150

78:                                               ; preds = %72
  %79 = getelementptr inbounds %struct.Range, ptr @PICTOGRAPHIC, i32 %74, i32 1
  %80 = load i32, ptr %79, align 4, !tbaa !110
  %81 = icmp ult i32 %80, %0
  %82 = add nuw i32 %74, 1
  br i1 %81, label %66, label %83, !llvm.loop !150

83:                                               ; preds = %61, %78, %69, %35, %29, %23, %20, %18, %3, %5, %1, %44, %13, %2
  %84 = phi i32 [ 2, %2 ], [ %17, %13 ], [ %48, %44 ], [ 1, %1 ], [ 3, %5 ], [ 3, %3 ], [ 5, %18 ], [ 6, %20 ], [ 8, %23 ], [ 9, %29 ], [ 10, %35 ], [ 0, %69 ], [ 7, %78 ], [ 4, %61 ]
  ret i32 %84
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.ctlz.i64(i64, i1 immarg) #20

; Function Attrs: optsize
declare ptr @fa_pages(i32 noundef) local_unnamed_addr #1

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.abs.i64(i64, i1 immarg) #28

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.umin.i32(i32, i32) #28

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.umax.i32(i32, i32) #28

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.smin.i64(i64, i64) #28

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.smax.i64(i64, i64) #28

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.fshl.i64(i64, i64, i64) #28

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.usub.sat.i64(i64, i64) #28

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i128 @llvm.abs.i128(i128, i1 immarg) #28

attributes #0 = { nounwind optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" "wasm-export-name"="flush" }
attributes #1 = { optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #2 = { nounwind optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #3 = { noreturn nounwind optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #4 = { noreturn optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #5 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, argmem: none, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #6 = { mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite) }
attributes #7 = { mustprogress nocallback nofree nounwind willreturn memory(argmem: readwrite) }
attributes #8 = { nofree norecurse nosync nounwind optsize memory(argmem: read) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #9 = { nofree norecurse nosync nounwind optsize memory(argmem: readwrite) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #10 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #11 = { optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" "wasm-import-module"="fa" "wasm-import-name"="argumentsSize" }
attributes #12 = { optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" "wasm-import-module"="fa" "wasm-import-name"="argumentsCopy" }
attributes #13 = { cold noinline nounwind optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #14 = { nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #15 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(readwrite, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #16 = { alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #17 = { mustprogress nocallback nofree nounwind willreturn memory(argmem: write) }
attributes #18 = { cold noinline noreturn nounwind optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #19 = { cold mustprogress nofree noinline norecurse nosync nounwind optsize willreturn memory(none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #20 = { mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #21 = { optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" "wasm-import-module"="fa" "wasm-import-name"="pow" }
attributes #22 = { alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(read, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #23 = { alwaysinline nounwind optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #24 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, argmem: readwrite, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #25 = { nofree norecurse nosync nounwind optsize memory(argmem: write) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #26 = { optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" "wasm-import-module"="fa" "wasm-import-name"="formatFloat" }
attributes #27 = { nofree norecurse nosync nounwind optsize memory(none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #28 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #29 = { nounwind optsize }
attributes #30 = { optsize }
attributes #31 = { noreturn nounwind optsize }
attributes #32 = { nounwind }
attributes #33 = { noreturn optsize }
attributes #34 = { cold noreturn optsize }

!llvm.module.flags = !{!0}

!0 = !{i32 1, !"wchar_size", i32 4}
!1 = !{!2, !2, i64 0}
!2 = !{!"long", !3, i64 0}
!3 = !{!"omnipotent char", !4, i64 0}
!4 = !{!"Simple C/C++ TBAA"}
!5 = !{!6, !6, i64 0}
!6 = !{!"any pointer", !3, i64 0}
!7 = distinct !{!7, !8}
!8 = !{!"llvm.loop.mustprogress"}
!9 = !{!3, !3, i64 0}
!10 = !{!11}
!11 = distinct !{!11, !12, !"owned_copy: argument 0"}
!12 = distinct !{!12, !"owned_copy"}
!13 = !{i64 0, i64 4, !5, i64 4, i64 4, !1, i64 8, i64 4, !1}
!14 = !{i64 0, i64 4, !1, i64 4, i64 4, !1}
!15 = !{i64 0, i64 4, !1}
!16 = distinct !{!16, !8}
!17 = !{!18, !18, i64 0}
!18 = !{!"int", !3, i64 0}
!19 = distinct !{!19, !8}
!20 = distinct !{!20, !8}
!21 = distinct !{!21, !8}
!22 = distinct !{!22, !8}
!23 = !{!24, !6, i64 0}
!24 = !{!"", !6, i64 0, !2, i64 4, !2, i64 8}
!25 = !{!24, !2, i64 4}
!26 = distinct !{!26, !8}
!27 = !{!24, !2, i64 8}
!28 = distinct !{!28, !8}
!29 = distinct !{!29, !8}
!30 = !{i32 0, i32 13}
!31 = distinct !{!31, !8}
!32 = distinct !{!32, !8}
!33 = distinct !{!33, !8}
!34 = distinct !{!34, !8}
!35 = distinct !{!35, !8}
!36 = !{!37, !6, i64 0}
!37 = !{!"", !6, i64 0, !2, i64 4, !2, i64 8, !6, i64 12, !2, i64 16, !18, i64 20, !18, i64 24}
!38 = !{!37, !2, i64 4}
!39 = !{!37, !2, i64 8}
!40 = !{!37, !6, i64 12}
!41 = !{!37, !2, i64 16}
!42 = !{!37, !18, i64 20}
!43 = !{!37, !18, i64 24}
!44 = !{ptr @replace_one, ptr @split_one}
!45 = distinct !{!45, !8}
!46 = !{!47, !6, i64 0}
!47 = !{!"", !6, i64 0, !48, i64 8, !48, i64 16}
!48 = !{!"long long", !3, i64 0}
!49 = !{!47, !48, i64 8}
!50 = !{!47, !48, i64 16}
!51 = !{!52, !54}
!52 = distinct !{!52, !53, !"owned_copy: argument 0"}
!53 = distinct !{!53, !"owned_copy"}
!54 = distinct !{!54, !55, !"slice: argument 0"}
!55 = distinct !{!55, !"slice"}
!56 = distinct !{!56, !8}
!57 = !{!58, !6, i64 0}
!58 = !{!"", !6, i64 0, !2, i64 4, !2, i64 8, !6, i64 12}
!59 = !{!58, !2, i64 4}
!60 = !{!58, !2, i64 8}
!61 = !{!58, !6, i64 12}
!62 = !{!63, !65}
!63 = distinct !{!63, !64, !"owned_copy: argument 0"}
!64 = distinct !{!64, !"owned_copy"}
!65 = distinct !{!65, !66, !"slice: argument 0"}
!66 = distinct !{!66, !"slice"}
!67 = !{!68, !70}
!68 = distinct !{!68, !69, !"owned_copy: argument 0"}
!69 = distinct !{!69, !"owned_copy"}
!70 = distinct !{!70, !71, !"slice: argument 0"}
!71 = distinct !{!71, !"slice"}
!72 = distinct !{!72, !8}
!73 = distinct !{!73, !8}
!74 = distinct !{!74, !8}
!75 = !{!76, !78}
!76 = distinct !{!76, !77, !"owned_copy: argument 0"}
!77 = distinct !{!77, !"owned_copy"}
!78 = distinct !{!78, !79, !"slice: argument 0"}
!79 = distinct !{!79, !"slice"}
!80 = distinct !{!80, !8}
!81 = !{i32 0, i32 2}
!82 = distinct !{!82, !8}
!83 = !{!84}
!84 = distinct !{!84, !85, !"owned_copy: argument 0"}
!85 = distinct !{!85, !"owned_copy"}
!86 = distinct !{!86, !8}
!87 = distinct !{!87, !8}
!88 = distinct !{!88, !8}
!89 = distinct !{!89, !8}
!90 = distinct !{!90, !8}
!91 = distinct !{!91, !8}
!92 = !{!93, !48, i64 0}
!93 = !{!"", !48, i64 0, !48, i64 8, !48, i64 16}
!94 = !{!93, !48, i64 8}
!95 = !{!93, !48, i64 16}
!96 = distinct !{!96, !8}
!97 = distinct !{!97, !8}
!98 = distinct !{!98, !8}
!99 = distinct !{!99, !8}
!100 = distinct !{!100, !8}
!101 = !{i64 0, i64 65}
!102 = !{!103, !48, i64 0}
!103 = !{!"", !48, i64 0, !48, i64 8, !3, i64 16}
!104 = !{!103, !48, i64 8}
!105 = distinct !{!105, !8}
!106 = distinct !{!106, !8}
!107 = distinct !{!107, !8}
!108 = !{!109, !18, i64 0}
!109 = !{!"", !18, i64 0, !18, i64 4}
!110 = !{!109, !18, i64 4}
!111 = distinct !{!111, !8}
!112 = !{!113, !6, i64 0}
!113 = !{!"FreeBlock", !6, i64 0}
!114 = !{!115, !2, i64 4}
!115 = !{!"FreeSpan", !6, i64 0, !2, i64 4}
!116 = !{!115, !6, i64 0}
!117 = distinct !{!117, !8}
!118 = distinct !{!118, !8}
!119 = !{!120, !48, i64 0}
!120 = !{!"", !48, i64 0, !24, i64 8}
!121 = distinct !{!121, !8}
!122 = distinct !{!122, !8}
!123 = distinct !{!123, !8}
!124 = !{!48, !48, i64 0}
!125 = !{!126, !48, i64 0}
!126 = !{!"", !48, i64 0, !24, i64 8, !24, i64 20}
!127 = !{!128}
!128 = distinct !{!128, !129, !"owned_copy: argument 0"}
!129 = distinct !{!129, !"owned_copy"}
!130 = !{!131, !6, i64 0}
!131 = !{!"", !6, i64 0, !48, i64 8, !48, i64 16, !48, i64 24, !6, i64 32, !48, i64 40, !48, i64 48}
!132 = !{!131, !48, i64 8}
!133 = !{!131, !48, i64 16}
!134 = !{!131, !48, i64 24}
!135 = !{!131, !6, i64 32}
!136 = !{!131, !48, i64 48}
!137 = distinct !{!137, !8}
!138 = !{!131, !48, i64 40}
!139 = distinct !{!139, !8}
!140 = distinct !{!140, !8}
!141 = distinct !{!141, !8}
!142 = distinct !{!142, !8}
!143 = distinct !{!143, !8}
!144 = distinct !{!144, !8}
!145 = distinct !{!145, !8}
!146 = distinct !{!146, !8}
!147 = !{!148, !148, i64 0}
!148 = !{!"__int128", !3, i64 0}
!149 = distinct !{!149, !8}
!150 = distinct !{!150, !8}
