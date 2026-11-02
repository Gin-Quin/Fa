source_filename = "fa_runtime.c"

%struct.BareError = type { i64 }
%struct.Range = type { i32, i32 }
%struct.FaString = type { ptr, i32 }
%struct.Replacing = type { ptr, i32, i32, ptr, i32, i32, i32 }
%struct.FaArrayHeader = type { ptr, i64, i64 }
%struct.Splitting = type { ptr, i32, i32, ptr }
%struct.MessageError = type { i64, %struct.FaString }
%struct.IndexError = type { i64, i64, i64 }
%struct.ParseError = type { i64, %struct.FaString, %struct.FaString }

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
@argument_count = internal unnamed_addr global i32 0, align 4
@argument_items = internal unnamed_addr global ptr null, align 4
@.str.13 = private unnamed_addr constant [19 x i8] c"integer overflow: \00", align 1
@.str.14 = private unnamed_addr constant [17 x i8] c" doesn't fit in \00", align 1
@.str.15 = private unnamed_addr constant [44 x i8] c"integer overflow: the value doesn't fit in \00", align 1
@.str.16 = private unnamed_addr constant [19 x i8] c"assertion failed: \00", align 1
@.str.17 = private unnamed_addr constant [13 x i8] c"\0A       \E2\86\B3 \00", align 1
@.str.18 = private unnamed_addr constant [8 x i8] c"error: \00", align 1
@.str.19 = private unnamed_addr constant [2 x i8] c"\0A\00", align 1
@division_by_zero = internal constant %struct.BareError { i64 3 }, align 8
@.str.20 = private unnamed_addr constant [20 x i8] c"negative exponent: \00", align 1
@.str.21 = private unnamed_addr constant [24 x i8] c" doesn't fit in Integer\00", align 1
@.str.22 = private unnamed_addr constant [24 x i8] c" is not a finite number\00", align 1
@.str.23 = private unnamed_addr constant [18 x i8] c"the text is empty\00", align 1
@.str.24 = private unnamed_addr constant [20 x i8] c"\22 is not an integer\00", align 1
@builder_capacity = internal unnamed_addr global i32 0, align 4
@EXTEND = internal unnamed_addr constant [153 x %struct.Range] [%struct.Range { i32 768, i32 879 }, %struct.Range { i32 1155, i32 1161 }, %struct.Range { i32 1425, i32 1469 }, %struct.Range { i32 1471, i32 1471 }, %struct.Range { i32 1473, i32 1474 }, %struct.Range { i32 1476, i32 1477 }, %struct.Range { i32 1479, i32 1479 }, %struct.Range { i32 1552, i32 1562 }, %struct.Range { i32 1611, i32 1631 }, %struct.Range { i32 1648, i32 1648 }, %struct.Range { i32 1750, i32 1756 }, %struct.Range { i32 1759, i32 1764 }, %struct.Range { i32 1767, i32 1768 }, %struct.Range { i32 1770, i32 1773 }, %struct.Range { i32 1809, i32 1809 }, %struct.Range { i32 1840, i32 1866 }, %struct.Range { i32 1958, i32 1968 }, %struct.Range { i32 2027, i32 2035 }, %struct.Range { i32 2070, i32 2093 }, %struct.Range { i32 2137, i32 2139 }, %struct.Range { i32 2200, i32 2207 }, %struct.Range { i32 2250, i32 2273 }, %struct.Range { i32 2275, i32 2307 }, %struct.Range { i32 2362, i32 2364 }, %struct.Range { i32 2366, i32 2383 }, %struct.Range { i32 2385, i32 2391 }, %struct.Range { i32 2402, i32 2403 }, %struct.Range { i32 2433, i32 2435 }, %struct.Range { i32 2492, i32 2492 }, %struct.Range { i32 2494, i32 2509 }, %struct.Range { i32 2519, i32 2519 }, %struct.Range { i32 2530, i32 2531 }, %struct.Range { i32 2561, i32 2563 }, %struct.Range { i32 2620, i32 2641 }, %struct.Range { i32 2672, i32 2673 }, %struct.Range { i32 2677, i32 2677 }, %struct.Range { i32 2689, i32 2691 }, %struct.Range { i32 2748, i32 2748 }, %struct.Range { i32 2750, i32 2765 }, %struct.Range { i32 2786, i32 2787 }, %struct.Range { i32 2817, i32 2819 }, %struct.Range { i32 2876, i32 2876 }, %struct.Range { i32 2878, i32 2903 }, %struct.Range { i32 2914, i32 2915 }, %struct.Range { i32 2946, i32 2946 }, %struct.Range { i32 3006, i32 3021 }, %struct.Range { i32 3031, i32 3031 }, %struct.Range { i32 3072, i32 3076 }, %struct.Range { i32 3132, i32 3132 }, %struct.Range { i32 3134, i32 3158 }, %struct.Range { i32 3170, i32 3171 }, %struct.Range { i32 3201, i32 3203 }, %struct.Range { i32 3260, i32 3260 }, %struct.Range { i32 3262, i32 3286 }, %struct.Range { i32 3298, i32 3299 }, %struct.Range { i32 3328, i32 3331 }, %struct.Range { i32 3387, i32 3388 }, %struct.Range { i32 3390, i32 3405 }, %struct.Range { i32 3415, i32 3415 }, %struct.Range { i32 3426, i32 3427 }, %struct.Range { i32 3457, i32 3459 }, %struct.Range { i32 3530, i32 3551 }, %struct.Range { i32 3570, i32 3571 }, %struct.Range { i32 3633, i32 3633 }, %struct.Range { i32 3635, i32 3642 }, %struct.Range { i32 3655, i32 3662 }, %struct.Range { i32 3761, i32 3761 }, %struct.Range { i32 3763, i32 3772 }, %struct.Range { i32 3784, i32 3790 }, %struct.Range { i32 3864, i32 3865 }, %struct.Range { i32 3893, i32 3893 }, %struct.Range { i32 3895, i32 3895 }, %struct.Range { i32 3897, i32 3897 }, %struct.Range { i32 3902, i32 3903 }, %struct.Range { i32 3953, i32 3972 }, %struct.Range { i32 3974, i32 3975 }, %struct.Range { i32 3981, i32 4028 }, %struct.Range { i32 4038, i32 4038 }, %struct.Range { i32 4139, i32 4158 }, %struct.Range { i32 4182, i32 4185 }, %struct.Range { i32 4190, i32 4192 }, %struct.Range { i32 4194, i32 4196 }, %struct.Range { i32 4199, i32 4205 }, %struct.Range { i32 4209, i32 4212 }, %struct.Range { i32 4226, i32 4237 }, %struct.Range { i32 4239, i32 4239 }, %struct.Range { i32 4250, i32 4253 }, %struct.Range { i32 4957, i32 4959 }, %struct.Range { i32 5906, i32 5909 }, %struct.Range { i32 5938, i32 5940 }, %struct.Range { i32 5970, i32 5971 }, %struct.Range { i32 6002, i32 6003 }, %struct.Range { i32 6068, i32 6099 }, %struct.Range { i32 6109, i32 6109 }, %struct.Range { i32 6155, i32 6157 }, %struct.Range { i32 6159, i32 6159 }, %struct.Range { i32 6277, i32 6278 }, %struct.Range { i32 6313, i32 6313 }, %struct.Range { i32 6432, i32 6459 }, %struct.Range { i32 6679, i32 6683 }, %struct.Range { i32 6741, i32 6783 }, %struct.Range { i32 6832, i32 6911 }, %struct.Range { i32 6912, i32 6916 }, %struct.Range { i32 6964, i32 6980 }, %struct.Range { i32 7019, i32 7027 }, %struct.Range { i32 7040, i32 7042 }, %struct.Range { i32 7073, i32 7085 }, %struct.Range { i32 7142, i32 7155 }, %struct.Range { i32 7204, i32 7223 }, %struct.Range { i32 7376, i32 7378 }, %struct.Range { i32 7380, i32 7400 }, %struct.Range { i32 7405, i32 7405 }, %struct.Range { i32 7412, i32 7412 }, %struct.Range { i32 7415, i32 7417 }, %struct.Range { i32 7616, i32 7679 }, %struct.Range { i32 8204, i32 8204 }, %struct.Range { i32 8400, i32 8432 }, %struct.Range { i32 11503, i32 11505 }, %struct.Range { i32 11647, i32 11647 }, %struct.Range { i32 11744, i32 11775 }, %struct.Range { i32 12330, i32 12335 }, %struct.Range { i32 12441, i32 12442 }, %struct.Range { i32 42607, i32 42610 }, %struct.Range { i32 42612, i32 42621 }, %struct.Range { i32 42654, i32 42655 }, %struct.Range { i32 42736, i32 42737 }, %struct.Range { i32 43010, i32 43010 }, %struct.Range { i32 43014, i32 43014 }, %struct.Range { i32 43019, i32 43019 }, %struct.Range { i32 43043, i32 43047 }, %struct.Range { i32 43052, i32 43052 }, %struct.Range { i32 43136, i32 43137 }, %struct.Range { i32 43188, i32 43205 }, %struct.Range { i32 43232, i32 43249 }, %struct.Range { i32 43263, i32 43263 }, %struct.Range { i32 43302, i32 43309 }, %struct.Range { i32 43335, i32 43347 }, %struct.Range { i32 43392, i32 43395 }, %struct.Range { i32 43443, i32 43456 }, %struct.Range { i32 43561, i32 43574 }, %struct.Range { i32 43587, i32 43587 }, %struct.Range { i32 43596, i32 43597 }, %struct.Range { i32 43755, i32 43759 }, %struct.Range { i32 43765, i32 43766 }, %struct.Range { i32 44003, i32 44010 }, %struct.Range { i32 44012, i32 44013 }, %struct.Range { i32 64286, i32 64286 }, %struct.Range { i32 65024, i32 65039 }, %struct.Range { i32 65056, i32 65071 }, %struct.Range { i32 65438, i32 65439 }, %struct.Range { i32 127995, i32 127999 }, %struct.Range { i32 917536, i32 917631 }, %struct.Range { i32 917760, i32 917999 }], align 16
@PICTOGRAPHIC = internal unnamed_addr constant [22 x %struct.Range] [%struct.Range { i32 169, i32 169 }, %struct.Range { i32 174, i32 174 }, %struct.Range { i32 8252, i32 8252 }, %struct.Range { i32 8265, i32 8265 }, %struct.Range { i32 8482, i32 8482 }, %struct.Range { i32 8505, i32 8505 }, %struct.Range { i32 8596, i32 8618 }, %struct.Range { i32 8986, i32 9215 }, %struct.Range { i32 9410, i32 9410 }, %struct.Range { i32 9642, i32 9726 }, %struct.Range { i32 9728, i32 10175 }, %struct.Range { i32 10548, i32 10549 }, %struct.Range { i32 11013, i32 11093 }, %struct.Range { i32 12336, i32 12336 }, %struct.Range { i32 12349, i32 12349 }, %struct.Range { i32 12951, i32 12951 }, %struct.Range { i32 12953, i32 12953 }, %struct.Range { i32 126976, i32 127231 }, %struct.Range { i32 127245, i32 127405 }, %struct.Range { i32 127489, i32 127994 }, %struct.Range { i32 128000, i32 129791 }, %struct.Range { i32 130048, i32 131069 }], align 16
@.str.25 = private unnamed_addr constant [7 x i8] c"stop: \00", align 1
@.str.26 = private unnamed_addr constant [7 x i8] c"\0A  at \00", align 1
@llvm.used = appending global [1 x ptr] [ptr @fa_flush], section "llvm.metadata"

; Function Attrs: nounwind optsize
define hidden void @fa_flush() #0 {
  %1 = load i32, ptr @output_length, align 4, !tbaa !1
  %2 = icmp eq i32 %1, 0
  br i1 %2, label %4, label %3

3:                                                ; preds = %0
  tail call void @fa_write(i32 noundef 1, ptr noundef nonnull @output, i32 noundef %1) #24
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
  tail call void @fa_write(i32 noundef 1, ptr noundef nonnull @output, i32 noundef %5) #24
  store i32 0, ptr @output_length, align 4, !tbaa !1
  br label %9

9:                                                ; preds = %6, %8
  tail call void @fa_write(i32 noundef %0, ptr noundef %1, i32 noundef %2) #24
  br label %23

10:                                               ; preds = %3
  %11 = add i32 %5, %2
  %12 = icmp ugt i32 %11, 8192
  br i1 %12, label %13, label %19

13:                                               ; preds = %10
  %14 = icmp eq i32 %5, 0
  br i1 %14, label %16, label %15

15:                                               ; preds = %13
  tail call void @fa_write(i32 noundef 1, ptr noundef nonnull @output, i32 noundef %5) #24
  store i32 0, ptr @output_length, align 4, !tbaa !1
  br label %16

16:                                               ; preds = %13, %15
  %17 = icmp ugt i32 %2, 8192
  br i1 %17, label %18, label %19

18:                                               ; preds = %16
  tail call void @fa_write(i32 noundef 1, ptr noundef %1, i32 noundef %2) #24
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
  tail call void @fa_flush() #25
  tail call void @fa_exit(i32 noundef %0) #26
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
  tail call fastcc void @reserve(i32 noundef %1) #25
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
  br i1 %5, label %6, label %18

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
  %15 = tail call ptr @fa_alloc(i32 noundef %11, i32 noundef 1) #24
  %16 = load ptr, ptr @builder, align 4, !tbaa !5
  %17 = load i32, ptr @builder_length, align 4, !tbaa !1
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %15, ptr align 1 %16, i32 %17, i1 false)
  store ptr %15, ptr @builder, align 4, !tbaa !5
  store i32 %11, ptr @builder_capacity, align 4, !tbaa !1
  br label %18

18:                                               ; preds = %1, %14
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_line(i32 noundef %0) local_unnamed_addr #2 {
  tail call fastcc void @reserve(i32 noundef 1) #25
  %2 = load ptr, ptr @builder, align 4, !tbaa !5
  %3 = load i32, ptr @builder_length, align 4, !tbaa !1
  %4 = getelementptr inbounds i8, ptr %2, i32 %3
  store i8 10, ptr %4, align 1, !tbaa !9
  %5 = add i32 %3, 1
  store i32 %5, ptr @builder_length, align 4, !tbaa !1
  tail call void @fa_out(i32 noundef %0, ptr noundef %2, i32 noundef %5) #25
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_take(ptr nocapture noundef writeonly %0) local_unnamed_addr #2 {
  %2 = load i32, ptr @builder_length, align 4
  %3 = tail call i32 @llvm.umax.i32(i32 %2, i32 1)
  %4 = tail call ptr @fa_alloc(i32 noundef %3, i32 noundef 1) #24
  %5 = load ptr, ptr @builder, align 4, !tbaa !5
  %6 = load i32, ptr @builder_length, align 4, !tbaa !1
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %4, ptr align 1 %5, i32 %6, i1 false)
  store ptr %4, ptr %0, align 4, !tbaa !10
  %7 = getelementptr inbounds %struct.FaString, ptr %0, i32 0, i32 1
  store i32 %6, ptr %7, align 4, !tbaa !12
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  ret void
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.start.p0(i64 immarg, ptr nocapture) #6

; Function Attrs: optsize
declare ptr @fa_alloc(i32 noundef, i32 noundef) local_unnamed_addr #1

; Function Attrs: mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.end.p0(i64 immarg, ptr nocapture) #6

; Function Attrs: nounwind optsize
define hidden void @fa_b_i64(i64 noundef %0) local_unnamed_addr #2 {
  %2 = tail call i64 @llvm.abs.i64(i64 %0, i1 false)
  %3 = lshr i64 %0, 63
  %4 = trunc i64 %3 to i32
  tail call fastcc void @append_u64(i64 noundef %2, i32 noundef %4) #25
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @append_u64(i64 noundef %0, i32 noundef %1) unnamed_addr #2 {
  %3 = alloca [24 x i8], align 16
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %3) #27
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
  br i1 %15, label %16, label %4, !llvm.loop !13

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
  tail call fastcc void @reserve(i32 noundef %24) #25
  %25 = load ptr, ptr @builder, align 4, !tbaa !5
  %26 = load i32, ptr @builder_length, align 4, !tbaa !1
  %27 = getelementptr inbounds i8, ptr %25, i32 %26
  call void @llvm.memcpy.p0.p0.i32(ptr align 1 %27, ptr nonnull align 1 %23, i32 %24, i1 false)
  %28 = add i32 %26, %24
  store i32 %28, ptr @builder_length, align 4, !tbaa !1
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %3) #27
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_u64(i64 noundef %0) local_unnamed_addr #2 {
  tail call fastcc void @append_u64(i64 noundef %0, i32 noundef 0) #25
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
  tail call fastcc void @append_u128(i128 noundef %9, i32 noundef %11) #25
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
  tail call fastcc void @append_u64(i64 noundef %7, i32 noundef %1) #25
  br label %80

8:                                                ; preds = %2
  %9 = lshr i128 %0, 64
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %3) #27
  %10 = lshr i128 %0, 96
  %11 = trunc i128 %10 to i32
  store i32 %11, ptr %3, align 16, !tbaa !14
  %12 = getelementptr inbounds i32, ptr %3, i32 1
  %13 = trunc i128 %9 to i32
  store i32 %13, ptr %12, align 4, !tbaa !14
  %14 = getelementptr inbounds i32, ptr %3, i32 2
  %15 = lshr i128 %0, 32
  %16 = trunc i128 %15 to i32
  store i32 %16, ptr %14, align 8, !tbaa !14
  %17 = getelementptr inbounds i32, ptr %3, i32 3
  %18 = trunc i128 %0 to i32
  store i32 %18, ptr %17, align 4, !tbaa !14
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %4) #27
  br label %19

19:                                               ; preds = %48, %8
  %20 = phi i32 [ 48, %8 ], [ %53, %48 ]
  br label %21

21:                                               ; preds = %21, %19
  %22 = phi i64 [ 0, %19 ], [ %33, %21 ]
  %23 = phi i32 [ 0, %19 ], [ %34, %21 ]
  %24 = shl nuw nsw i64 %22, 32
  %25 = getelementptr inbounds i32, ptr %3, i32 %23
  %26 = load i32, ptr %25, align 4, !tbaa !14
  %27 = zext i32 %26 to i64
  %28 = or disjoint i64 %24, %27
  %29 = freeze i64 %28
  %30 = udiv i64 %29, 1000000000
  %31 = trunc i64 %30 to i32
  store i32 %31, ptr %25, align 4, !tbaa !14
  %32 = mul i64 %30, 1000000000
  %33 = sub i64 %29, %32
  %34 = add nuw nsw i32 %23, 1
  %35 = icmp eq i32 %34, 4
  br i1 %35, label %36, label %21, !llvm.loop !16

36:                                               ; preds = %21
  %37 = trunc i64 %33 to i32
  %38 = load i32, ptr %3, align 16, !tbaa !14
  %39 = load i32, ptr %12, align 4, !tbaa !14
  %40 = or i32 %39, %38
  %41 = load i32, ptr %14, align 8, !tbaa !14
  %42 = or i32 %40, %41
  %43 = load i32, ptr %17, align 4, !tbaa !14
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
  br i1 %65, label %49, label %48, !llvm.loop !17

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
  tail call fastcc void @reserve(i32 noundef %75) #25
  %76 = load ptr, ptr @builder, align 4, !tbaa !5
  %77 = load i32, ptr @builder_length, align 4, !tbaa !1
  %78 = getelementptr inbounds i8, ptr %76, i32 %77
  call void @llvm.memcpy.p0.p0.i32(ptr align 1 %78, ptr nonnull align 1 %74, i32 %75, i1 false)
  %79 = add i32 %77, %75
  store i32 %79, ptr @builder_length, align 4, !tbaa !1
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %4) #27
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %3) #27
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
  tail call fastcc void @append_u128(i128 noundef %6, i32 noundef 0) #25
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_bool(i32 noundef %0) local_unnamed_addr #2 {
  %2 = icmp eq i32 %0, 0
  %3 = select i1 %2, ptr @.str.1, ptr @.str
  tail call fastcc void @append_c(ptr noundef nonnull %3) #25
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
  br i1 %6, label %8, label %2, !llvm.loop !18

8:                                                ; preds = %2
  tail call fastcc void @reserve(i32 noundef %3) #25
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
  tail call fastcc void @reserve(i32 noundef 40) #25
  %2 = load ptr, ptr @builder, align 4, !tbaa !5
  %3 = load i32, ptr @builder_length, align 4, !tbaa !1
  %4 = getelementptr inbounds i8, ptr %2, i32 %3
  %5 = tail call i32 @fa_host_format_float(double noundef %0, i32 noundef 0, ptr noundef %4) #24
  %6 = load i32, ptr @builder_length, align 4, !tbaa !1
  %7 = add i32 %6, %5
  store i32 %7, ptr @builder_length, align 4, !tbaa !1
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_f32(float noundef %0) local_unnamed_addr #2 {
  %2 = fpext float %0 to double
  tail call fastcc void @reserve(i32 noundef 40) #25
  %3 = load ptr, ptr @builder, align 4, !tbaa !5
  %4 = load i32, ptr @builder_length, align 4, !tbaa !1
  %5 = getelementptr inbounds i8, ptr %3, i32 %4
  %6 = tail call i32 @fa_host_format_float(double noundef %2, i32 noundef 1, ptr noundef %5) #24
  %7 = load i32, ptr @builder_length, align 4, !tbaa !1
  %8 = add i32 %7, %6
  store i32 %8, ptr @builder_length, align 4, !tbaa !1
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_percentage(double noundef %0) local_unnamed_addr #2 {
  tail call fastcc void @reserve(i32 noundef 40) #25
  %2 = load ptr, ptr @builder, align 4, !tbaa !5
  %3 = load i32, ptr @builder_length, align 4, !tbaa !1
  %4 = getelementptr inbounds i8, ptr %2, i32 %3
  %5 = tail call i32 @fa_host_format_float(double noundef %0, i32 noundef 2, ptr noundef %4) #24
  %6 = load i32, ptr @builder_length, align 4, !tbaa !1
  %7 = add i32 %6, %5
  store i32 %7, ptr @builder_length, align 4, !tbaa !1
  tail call fastcc void @reserve(i32 noundef 1) #25
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
  tail call fastcc void @reserve(i32 noundef 1) #25
  %3 = load ptr, ptr @builder, align 4, !tbaa !5
  %4 = load i32, ptr @builder_length, align 4, !tbaa !1
  %5 = getelementptr inbounds i8, ptr %3, i32 %4
  store i8 34, ptr %5, align 1, !tbaa !9
  %6 = add i32 %4, 1
  store i32 %6, ptr @builder_length, align 4, !tbaa !1
  %7 = icmp eq i32 %1, 0
  br i1 %7, label %8, label %13

8:                                                ; preds = %50, %2
  tail call fastcc void @reserve(i32 noundef 1) #25
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
  tail call fastcc void @append_c(ptr noundef nonnull @.str.2) #25
  br label %50

19:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.3) #25
  br label %50

20:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.4) #25
  br label %50

21:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.5) #25
  br label %50

22:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.6) #25
  br label %50

23:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.7) #25
  br label %50

24:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.8) #25
  br label %50

25:                                               ; preds = %13
  tail call fastcc void @append_c(ptr noundef nonnull @.str.9) #25
  br label %50

26:                                               ; preds = %13
  %27 = icmp ult i8 %16, 32
  %28 = icmp eq i8 %16, 127
  %29 = or i1 %27, %28
  br i1 %29, label %30, label %45

30:                                               ; preds = %26
  tail call fastcc void @append_c(ptr noundef nonnull @.str.10) #25
  %31 = lshr i32 %17, 4
  %32 = getelementptr inbounds [17 x i8], ptr @fa_b_quoted.hex, i32 0, i32 %31
  %33 = load i8, ptr %32, align 1, !tbaa !9
  tail call fastcc void @reserve(i32 noundef 1) #25
  %34 = load ptr, ptr @builder, align 4, !tbaa !5
  %35 = load i32, ptr @builder_length, align 4, !tbaa !1
  %36 = getelementptr inbounds i8, ptr %34, i32 %35
  store i8 %33, ptr %36, align 1, !tbaa !9
  %37 = add i32 %35, 1
  store i32 %37, ptr @builder_length, align 4, !tbaa !1
  %38 = and i32 %17, 15
  %39 = getelementptr inbounds [17 x i8], ptr @fa_b_quoted.hex, i32 0, i32 %38
  %40 = load i8, ptr %39, align 1, !tbaa !9
  tail call fastcc void @reserve(i32 noundef 1) #25
  %41 = load ptr, ptr @builder, align 4, !tbaa !5
  %42 = load i32, ptr @builder_length, align 4, !tbaa !1
  %43 = getelementptr inbounds i8, ptr %41, i32 %42
  store i8 %40, ptr %43, align 1, !tbaa !9
  %44 = add i32 %42, 1
  store i32 %44, ptr @builder_length, align 4, !tbaa !1
  br label %50

45:                                               ; preds = %26
  tail call fastcc void @reserve(i32 noundef 1) #25
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
  br i1 %52, label %8, label %13, !llvm.loop !19
}

; Function Attrs: nounwind optsize
define hidden void @fa_b_string_array(ptr nocapture noundef readonly %0, i32 noundef %1) local_unnamed_addr #2 {
  tail call fastcc void @reserve(i32 noundef 1) #25
  %3 = load ptr, ptr @builder, align 4, !tbaa !5
  %4 = load i32, ptr @builder_length, align 4, !tbaa !1
  %5 = getelementptr inbounds i8, ptr %3, i32 %4
  store i8 91, ptr %5, align 1, !tbaa !9
  %6 = add i32 %4, 1
  store i32 %6, ptr @builder_length, align 4, !tbaa !1
  %7 = icmp eq i32 %1, 0
  br i1 %7, label %8, label %13

8:                                                ; preds = %17, %2
  tail call fastcc void @reserve(i32 noundef 1) #25
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
  tail call fastcc void @append_c(ptr noundef nonnull @.str.11) #25
  br label %17

17:                                               ; preds = %16, %13
  %18 = getelementptr inbounds %struct.FaString, ptr %0, i32 %14
  %19 = load ptr, ptr %18, align 4, !tbaa !10
  %20 = getelementptr inbounds %struct.FaString, ptr %0, i32 %14, i32 1
  %21 = load i32, ptr %20, align 4, !tbaa !12
  tail call void @fa_b_quoted(ptr noundef %19, i32 noundef %21) #25
  %22 = add nuw i32 %14, 1
  %23 = icmp eq i32 %22, %1
  br i1 %23, label %8, label %13, !llvm.loop !20
}

; Function Attrs: nounwind optsize
define hidden void @fa_concat(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef readonly %2, i32 noundef %3, ptr nocapture noundef writeonly %4) local_unnamed_addr #2 {
  %6 = add i32 %3, %1
  %7 = tail call i32 @llvm.umax.i32(i32 %6, i32 1)
  %8 = tail call ptr @fa_alloc(i32 noundef %7, i32 noundef 1) #24
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %8, ptr align 1 %0, i32 %1, i1 false)
  %9 = getelementptr inbounds i8, ptr %8, i32 %1
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %9, ptr align 1 %2, i32 %3, i1 false)
  store ptr %8, ptr %4, align 4, !tbaa !10
  %10 = getelementptr inbounds %struct.FaString, ptr %4, i32 0, i32 1
  store i32 %6, ptr %10, align 4, !tbaa !12
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: read)
define hidden i32 @fa_string_equal(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef readonly %2, i32 noundef %3) local_unnamed_addr #7 {
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
  br i1 %15, label %22, label %16, !llvm.loop !21

16:                                               ; preds = %12
  %17 = getelementptr inbounds i8, ptr %0, i32 %14
  %18 = load i8, ptr %17, align 1, !tbaa !9
  %19 = getelementptr inbounds i8, ptr %2, i32 %14
  %20 = load i8, ptr %19, align 1, !tbaa !9
  %21 = icmp eq i8 %18, %20
  br i1 %21, label %12, label %22, !llvm.loop !21

22:                                               ; preds = %16, %12
  %23 = icmp uge i32 %14, %1
  %24 = zext i1 %23 to i32
  br label %25

25:                                               ; preds = %8, %22, %6, %4
  %26 = phi i32 [ 0, %4 ], [ 1, %6 ], [ 0, %8 ], [ %24, %22 ]
  ret i32 %26
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: read)
define hidden i32 @fa_string_compare(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef readonly %2, i32 noundef %3) local_unnamed_addr #7 {
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
  br i1 %19, label %39, label %20, !llvm.loop !22

20:                                               ; preds = %16
  %21 = getelementptr inbounds i8, ptr %0, i32 %18
  %22 = load i8, ptr %21, align 1, !tbaa !9
  %23 = getelementptr inbounds i8, ptr %2, i32 %18
  %24 = load i8, ptr %23, align 1, !tbaa !9
  %25 = icmp eq i8 %22, %24
  br i1 %25, label %16, label %26, !llvm.loop !22

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
define hidden i32 @fa_next_grapheme(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2) local_unnamed_addr #8 {
  %4 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %2) #25
  ret i32 %4
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define internal fastcc i32 @next_boundary(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2) unnamed_addr #8 {
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
  %25 = call fastcc i32 @decode(ptr noundef nonnull %0, i32 noundef %1, ptr noundef nonnull %4) #25
  %26 = tail call fastcc i32 @break_kind(i32 noundef %25) #25, !range !23
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
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %5) #27
  store i32 %38, ptr %5, align 4, !tbaa !1
  %39 = call fastcc i32 @decode(ptr noundef %0, i32 noundef %1, ptr noundef nonnull %5) #25
  %40 = tail call fastcc i32 @break_kind(i32 noundef %39) #25, !range !23
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
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %5) #27
  br label %106

101:                                              ; preds = %74, %94, %88
  %102 = phi i32 [ %89, %88 ], [ %95, %94 ], [ %75, %74 ]
  %103 = phi i32 [ %92, %88 ], [ %96, %94 ], [ 0, %74 ]
  %104 = phi i32 [ 1, %88 ], [ %99, %94 ], [ 1, %74 ]
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %5) #27
  %105 = icmp ult i32 %102, %1
  br i1 %105, label %34, label %106

106:                                              ; preds = %101, %24, %100, %21
  %107 = phi i32 [ %23, %21 ], [ %38, %100 ], [ %27, %24 ], [ %102, %101 ]
  ret i32 %107
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define hidden i32 @fa_graphemes(ptr nocapture noundef readonly %0, i32 noundef %1) local_unnamed_addr #8 {
  %3 = icmp eq i32 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i32 [ 0, %2 ], [ %9, %6 ]
  ret i32 %5

6:                                                ; preds = %2, %6
  %7 = phi i32 [ %10, %6 ], [ 0, %2 ]
  %8 = phi i32 [ %9, %6 ], [ 0, %2 ]
  %9 = add i32 %8, 1
  %10 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %7) #25
  %11 = icmp ult i32 %10, %1
  br i1 %11, label %6, label %4, !llvm.loop !24
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define hidden i32 @fa_string_includes(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef readonly %2, i32 noundef %3) local_unnamed_addr #8 {
  %5 = tail call fastcc i64 @find(ptr noundef %0, i32 noundef %1, ptr noundef %2, i32 noundef %3, i32 noundef 0) #25
  %6 = icmp sgt i64 %5, -1
  %7 = zext i1 %6 to i32
  ret i32 %7
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define internal fastcc i64 @find(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef readonly %2, i32 noundef %3, i32 noundef %4) unnamed_addr #8 {
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
  br i1 %22, label %29, label %23, !llvm.loop !25

23:                                               ; preds = %19
  %24 = getelementptr i8, ptr %14, i32 %21
  %25 = load i8, ptr %24, align 1, !tbaa !9
  %26 = getelementptr inbounds i8, ptr %2, i32 %21
  %27 = load i8, ptr %26, align 1, !tbaa !9
  %28 = icmp eq i8 %25, %27
  br i1 %28, label %19, label %29, !llvm.loop !25

29:                                               ; preds = %23, %19
  %30 = icmp ult i32 %21, %3
  br i1 %30, label %51, label %31

31:                                               ; preds = %10, %29
  %32 = icmp eq i32 %12, 0
  br i1 %32, label %37, label %33

33:                                               ; preds = %31, %33
  %34 = phi i32 [ %35, %33 ], [ 0, %31 ]
  %35 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %34) #25
  %36 = icmp ult i32 %35, %12
  br i1 %36, label %33, label %37, !llvm.loop !26

37:                                               ; preds = %33, %31
  %38 = phi i32 [ 0, %31 ], [ %35, %33 ]
  %39 = icmp eq i32 %38, %12
  br i1 %39, label %40, label %51

40:                                               ; preds = %37
  %41 = icmp eq i32 %11, 0
  br i1 %41, label %46, label %42

42:                                               ; preds = %40, %42
  %43 = phi i32 [ %44, %42 ], [ 0, %40 ]
  %44 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %43) #25
  %45 = icmp ult i32 %44, %11
  br i1 %45, label %42, label %46, !llvm.loop !26

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
  br i1 %55, label %56, label %10, !llvm.loop !27

56:                                               ; preds = %51, %49, %5
  %57 = phi i64 [ -1, %5 ], [ %50, %49 ], [ %52, %51 ]
  ret i64 %57
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define hidden i32 @fa_string_starts_with(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef readonly %2, i32 noundef %3) local_unnamed_addr #8 {
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
  br i1 %15, label %22, label %16, !llvm.loop !25

16:                                               ; preds = %12
  %17 = getelementptr i8, ptr %0, i32 %14
  %18 = load i8, ptr %17, align 1, !tbaa !9
  %19 = getelementptr inbounds i8, ptr %2, i32 %14
  %20 = load i8, ptr %19, align 1, !tbaa !9
  %21 = icmp eq i8 %18, %20
  br i1 %21, label %12, label %22, !llvm.loop !25

22:                                               ; preds = %16, %12
  %23 = icmp ult i32 %14, %3
  br i1 %23, label %32, label %24

24:                                               ; preds = %22, %24
  %25 = phi i32 [ %26, %24 ], [ 0, %22 ]
  %26 = tail call fastcc i32 @next_boundary(ptr noundef nonnull %0, i32 noundef %1, i32 noundef %25) #25
  %27 = icmp ult i32 %26, %3
  br i1 %27, label %24, label %28, !llvm.loop !26

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
define hidden i32 @fa_string_ends_with(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef readonly %2, i32 noundef %3) local_unnamed_addr #8 {
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
  br i1 %17, label %24, label %18, !llvm.loop !25

18:                                               ; preds = %14
  %19 = getelementptr i8, ptr %8, i32 %16
  %20 = load i8, ptr %19, align 1, !tbaa !9
  %21 = getelementptr inbounds i8, ptr %2, i32 %16
  %22 = load i8, ptr %21, align 1, !tbaa !9
  %23 = icmp eq i8 %20, %22
  br i1 %23, label %14, label %24, !llvm.loop !25

24:                                               ; preds = %18, %14
  %25 = icmp ult i32 %16, %3
  br i1 %25, label %36, label %26

26:                                               ; preds = %6, %24
  %27 = icmp eq i32 %1, %3
  br i1 %27, label %32, label %28

28:                                               ; preds = %26, %28
  %29 = phi i32 [ %30, %28 ], [ 0, %26 ]
  %30 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %29) #25
  %31 = icmp ult i32 %30, %7
  br i1 %31, label %28, label %32, !llvm.loop !26

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
define hidden i64 @fa_string_index_of(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef readonly %2, i32 noundef %3, i32 noundef %4) local_unnamed_addr #8 {
  %6 = tail call fastcc i64 @find(ptr noundef %0, i32 noundef %1, ptr noundef %2, i32 noundef %3, i32 noundef %4) #25
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
  %15 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %12) #25
  %16 = icmp ult i32 %15, %9
  br i1 %16, label %11, label %17, !llvm.loop !28

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
  call void @llvm.lifetime.start.p0(i64 28, ptr nonnull %9) #27
  store ptr %0, ptr %9, align 4, !tbaa !29
  %10 = getelementptr inbounds %struct.Replacing, ptr %9, i32 0, i32 1
  store i32 0, ptr %10, align 4, !tbaa !31
  %11 = getelementptr inbounds %struct.Replacing, ptr %9, i32 0, i32 2
  store i32 %3, ptr %11, align 4, !tbaa !32
  %12 = getelementptr inbounds %struct.Replacing, ptr %9, i32 0, i32 3
  store ptr %4, ptr %12, align 4, !tbaa !33
  %13 = getelementptr inbounds %struct.Replacing, ptr %9, i32 0, i32 4
  store i32 %5, ptr %13, align 4, !tbaa !34
  %14 = getelementptr inbounds %struct.Replacing, ptr %9, i32 0, i32 5
  store i32 %6, ptr %14, align 4, !tbaa !35
  %15 = getelementptr inbounds %struct.Replacing, ptr %9, i32 0, i32 6
  store i32 0, ptr %15, align 4, !tbaa !36
  call fastcc void @each_match(ptr noundef %0, i32 noundef %1, ptr noundef %2, i32 noundef %3, ptr noundef nonnull @replace_one, ptr noundef nonnull %9) #25
  %16 = load i32, ptr %10, align 4, !tbaa !31
  %17 = getelementptr inbounds i8, ptr %0, i32 %16
  %18 = sub i32 %1, %16
  call fastcc void @reserve(i32 noundef %18) #25
  %19 = load ptr, ptr @builder, align 4, !tbaa !5
  %20 = load i32, ptr @builder_length, align 4, !tbaa !1
  %21 = getelementptr inbounds i8, ptr %19, i32 %20
  call void @llvm.memcpy.p0.p0.i32(ptr align 1 %21, ptr align 1 %17, i32 %18, i1 false)
  %22 = add i32 %20, %18
  store i32 %22, ptr @builder_length, align 4, !tbaa !1
  %23 = call i32 @llvm.umax.i32(i32 %22, i32 1)
  %24 = call ptr @fa_alloc(i32 noundef %23, i32 noundef 1) #24
  %25 = load ptr, ptr @builder, align 4, !tbaa !5
  %26 = load i32, ptr @builder_length, align 4, !tbaa !1
  call void @llvm.memcpy.p0.p0.i32(ptr align 1 %24, ptr align 1 %25, i32 %26, i1 false)
  store ptr %24, ptr %7, align 4, !tbaa !10
  %27 = getelementptr inbounds %struct.FaString, ptr %7, i32 0, i32 1
  store i32 %26, ptr %27, align 4, !tbaa !12
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  call void @llvm.lifetime.end.p0(i64 28, ptr nonnull %9) #27
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
  br i1 %22, label %29, label %23, !llvm.loop !25

23:                                               ; preds = %19
  %24 = getelementptr i8, ptr %14, i32 %21
  %25 = load i8, ptr %24, align 1, !tbaa !9
  %26 = getelementptr inbounds i8, ptr %2, i32 %21
  %27 = load i8, ptr %26, align 1, !tbaa !9
  %28 = icmp eq i8 %25, %27
  br i1 %28, label %19, label %29, !llvm.loop !25

29:                                               ; preds = %23, %19
  %30 = icmp ult i32 %21, %3
  br i1 %30, label %50, label %31

31:                                               ; preds = %11, %29
  %32 = icmp eq i32 %13, 0
  br i1 %32, label %37, label %33

33:                                               ; preds = %31, %33
  %34 = phi i32 [ %35, %33 ], [ 0, %31 ]
  %35 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %34) #25
  %36 = icmp ult i32 %35, %13
  br i1 %36, label %33, label %37, !llvm.loop !26

37:                                               ; preds = %33, %31
  %38 = phi i32 [ 0, %31 ], [ %35, %33 ]
  %39 = icmp eq i32 %38, %13
  br i1 %39, label %40, label %50

40:                                               ; preds = %37
  %41 = icmp eq i32 %12, 0
  br i1 %41, label %46, label %42

42:                                               ; preds = %40, %42
  %43 = phi i32 [ %44, %42 ], [ 0, %40 ]
  %44 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %43) #25
  %45 = icmp ult i32 %44, %12
  br i1 %45, label %42, label %46, !llvm.loop !26

46:                                               ; preds = %42, %40
  %47 = phi i32 [ 0, %40 ], [ %44, %42 ]
  %48 = icmp eq i32 %47, %12
  br i1 %48, label %49, label %50

49:                                               ; preds = %46
  tail call void %4(ptr noundef %5, i32 noundef %13) #24, !callees !37
  br label %50

50:                                               ; preds = %15, %29, %37, %46, %49
  %51 = phi i32 [ %10, %49 ], [ 1, %46 ], [ 1, %37 ], [ 1, %29 ], [ 1, %15 ]
  %52 = add i32 %51, %13
  %53 = add i32 %52, %3
  %54 = icmp ugt i32 %53, %1
  br i1 %54, label %55, label %11, !llvm.loop !38

55:                                               ; preds = %50, %6
  ret void
}

; Function Attrs: nounwind optsize
define internal void @replace_one(ptr nocapture noundef %0, i32 noundef %1) #2 {
  %3 = getelementptr inbounds %struct.Replacing, ptr %0, i32 0, i32 6
  %4 = load i32, ptr %3, align 4, !tbaa !36
  %5 = icmp eq i32 %4, 0
  br i1 %5, label %6, label %31

6:                                                ; preds = %2
  %7 = load ptr, ptr %0, align 4, !tbaa !29
  %8 = getelementptr inbounds %struct.Replacing, ptr %0, i32 0, i32 1
  %9 = load i32, ptr %8, align 4, !tbaa !31
  %10 = getelementptr inbounds i8, ptr %7, i32 %9
  %11 = sub i32 %1, %9
  tail call fastcc void @reserve(i32 noundef %11) #25
  %12 = load ptr, ptr @builder, align 4, !tbaa !5
  %13 = load i32, ptr @builder_length, align 4, !tbaa !1
  %14 = getelementptr inbounds i8, ptr %12, i32 %13
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %14, ptr align 1 %10, i32 %11, i1 false)
  %15 = add i32 %13, %11
  store i32 %15, ptr @builder_length, align 4, !tbaa !1
  %16 = getelementptr inbounds %struct.Replacing, ptr %0, i32 0, i32 3
  %17 = load ptr, ptr %16, align 4, !tbaa !33
  %18 = getelementptr inbounds %struct.Replacing, ptr %0, i32 0, i32 4
  %19 = load i32, ptr %18, align 4, !tbaa !34
  tail call fastcc void @reserve(i32 noundef %19) #25
  %20 = load ptr, ptr @builder, align 4, !tbaa !5
  %21 = load i32, ptr @builder_length, align 4, !tbaa !1
  %22 = getelementptr inbounds i8, ptr %20, i32 %21
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %22, ptr align 1 %17, i32 %19, i1 false)
  %23 = add i32 %21, %19
  store i32 %23, ptr @builder_length, align 4, !tbaa !1
  %24 = getelementptr inbounds %struct.Replacing, ptr %0, i32 0, i32 2
  %25 = load i32, ptr %24, align 4, !tbaa !32
  %26 = add i32 %25, %1
  store i32 %26, ptr %8, align 4, !tbaa !31
  %27 = getelementptr inbounds %struct.Replacing, ptr %0, i32 0, i32 5
  %28 = load i32, ptr %27, align 4, !tbaa !35
  %29 = icmp eq i32 %28, 0
  %30 = zext i1 %29 to i32
  store i32 %30, ptr %3, align 4, !tbaa !36
  br label %31

31:                                               ; preds = %2, %6
  ret void
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_string_characters(ptr noundef %0, i32 noundef %1) local_unnamed_addr #2 {
  %3 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #24
  %4 = tail call ptr @fa_alloc(i32 noundef 32, i32 noundef 16) #24
  store ptr %4, ptr %3, align 8, !tbaa !39
  %5 = getelementptr inbounds %struct.FaArrayHeader, ptr %3, i32 0, i32 1
  store i64 0, ptr %5, align 8, !tbaa !42
  %6 = getelementptr inbounds %struct.FaArrayHeader, ptr %3, i32 0, i32 2
  store i64 4, ptr %6, align 8, !tbaa !43
  %7 = icmp eq i32 %1, 0
  br i1 %7, label %8, label %9

8:                                                ; preds = %9, %2
  ret ptr %3

9:                                                ; preds = %2, %9
  %10 = phi i32 [ %11, %9 ], [ 0, %2 ]
  %11 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %10) #25
  %12 = tail call ptr @fa_array_push_slot(ptr noundef nonnull %3, i64 noundef 8) #25
  %13 = getelementptr inbounds i8, ptr %0, i32 %10
  %14 = sub i32 %11, %10
  %15 = ptrtoint ptr %13 to i32
  store i32 %15, ptr %12, align 4
  %16 = getelementptr inbounds i8, ptr %12, i32 4
  store i32 %14, ptr %16, align 4
  %17 = icmp ult i32 %11, %1
  br i1 %17, label %9, label %8, !llvm.loop !44
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_array_new(i64 noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #2 {
  %4 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #24
  %5 = tail call i64 @llvm.smax.i64(i64 %1, i64 %0)
  %6 = mul nsw i64 %5, %2
  %7 = trunc i64 %6 to i32
  %8 = tail call i32 @llvm.umax.i32(i32 %7, i32 1)
  %9 = tail call ptr @fa_alloc(i32 noundef %8, i32 noundef 16) #24
  store ptr %9, ptr %4, align 8, !tbaa !39
  %10 = getelementptr inbounds %struct.FaArrayHeader, ptr %4, i32 0, i32 1
  store i64 %0, ptr %10, align 8, !tbaa !42
  %11 = getelementptr inbounds %struct.FaArrayHeader, ptr %4, i32 0, i32 2
  store i64 %5, ptr %11, align 8, !tbaa !43
  ret ptr %4
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_array_push_slot(ptr nocapture noundef %0, i64 noundef %1) local_unnamed_addr #2 {
  %3 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %4 = load i64, ptr %3, align 8, !tbaa !42
  %5 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 2
  %6 = load i64, ptr %5, align 8, !tbaa !43
  %7 = icmp eq i64 %4, %6
  br i1 %7, label %10, label %8

8:                                                ; preds = %2
  %9 = load ptr, ptr %0, align 8, !tbaa !39
  br label %26

10:                                               ; preds = %2
  %11 = icmp slt i64 %4, 4
  %12 = shl nuw nsw i64 %4, 1
  %13 = select i1 %11, i64 4, i64 %12
  %14 = mul nsw i64 %13, %1
  %15 = trunc i64 %14 to i32
  %16 = tail call ptr @fa_alloc(i32 noundef %15, i32 noundef 16) #24
  %17 = load ptr, ptr %0, align 8, !tbaa !39
  %18 = load i64, ptr %3, align 8, !tbaa !42
  %19 = mul nsw i64 %18, %1
  %20 = trunc i64 %19 to i32
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %16, ptr align 1 %17, i32 %20, i1 false)
  %21 = load ptr, ptr %0, align 8, !tbaa !39
  %22 = load i64, ptr %5, align 8, !tbaa !43
  %23 = mul nsw i64 %22, %1
  %24 = trunc i64 %23 to i32
  tail call void @fa_free(ptr noundef %21, i32 noundef %24, i32 noundef 16) #24
  store ptr %16, ptr %0, align 8, !tbaa !39
  store i64 %13, ptr %5, align 8, !tbaa !43
  %25 = load i64, ptr %3, align 8, !tbaa !42
  br label %26

26:                                               ; preds = %8, %10
  %27 = phi i64 [ %4, %8 ], [ %25, %10 ]
  %28 = phi ptr [ %9, %8 ], [ %16, %10 ]
  %29 = mul nsw i64 %27, %1
  %30 = trunc i64 %29 to i32
  %31 = getelementptr inbounds i8, ptr %28, i32 %30
  %32 = add nsw i64 %27, 1
  store i64 %32, ptr %3, align 8, !tbaa !42
  ret ptr %31
}

; Function Attrs: mustprogress nocallback nofree nounwind willreturn memory(argmem: readwrite)
declare void @llvm.memcpy.p0.p0.i32(ptr noalias nocapture writeonly, ptr noalias nocapture readonly, i32, i1 immarg) #9

; Function Attrs: nounwind optsize
define hidden ptr @fa_string_split(ptr noundef %0, i32 noundef %1, ptr nocapture noundef readonly %2, i32 noundef %3) local_unnamed_addr #2 {
  %5 = alloca %struct.Splitting, align 4
  %6 = icmp eq i32 %3, 0
  br i1 %6, label %7, label %9

7:                                                ; preds = %4
  %8 = tail call ptr @fa_string_characters(ptr noundef %0, i32 noundef %1) #25
  br label %25

9:                                                ; preds = %4
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %5) #27
  store ptr %0, ptr %5, align 4, !tbaa !45
  %10 = getelementptr inbounds %struct.Splitting, ptr %5, i32 0, i32 1
  store i32 0, ptr %10, align 4, !tbaa !47
  %11 = getelementptr inbounds %struct.Splitting, ptr %5, i32 0, i32 2
  store i32 %3, ptr %11, align 4, !tbaa !48
  %12 = getelementptr inbounds %struct.Splitting, ptr %5, i32 0, i32 3
  %13 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #24
  %14 = tail call ptr @fa_alloc(i32 noundef 32, i32 noundef 16) #24
  store ptr %14, ptr %13, align 8, !tbaa !39
  %15 = getelementptr inbounds %struct.FaArrayHeader, ptr %13, i32 0, i32 1
  store i64 0, ptr %15, align 8, !tbaa !42
  %16 = getelementptr inbounds %struct.FaArrayHeader, ptr %13, i32 0, i32 2
  store i64 4, ptr %16, align 8, !tbaa !43
  store ptr %13, ptr %12, align 4, !tbaa !49
  call fastcc void @each_match(ptr noundef %0, i32 noundef %1, ptr noundef %2, i32 noundef %3, ptr noundef nonnull @split_one, ptr noundef nonnull %5) #25
  %17 = load ptr, ptr %12, align 4, !tbaa !49
  %18 = call ptr @fa_array_push_slot(ptr noundef %17, i64 noundef 8) #25
  %19 = load i32, ptr %10, align 4, !tbaa !47
  %20 = getelementptr inbounds i8, ptr %0, i32 %19
  %21 = sub i32 %1, %19
  %22 = ptrtoint ptr %20 to i32
  store i32 %22, ptr %18, align 4
  %23 = getelementptr inbounds i8, ptr %18, i32 4
  store i32 %21, ptr %23, align 4
  %24 = load ptr, ptr %12, align 4, !tbaa !49
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %5) #27
  br label %25

25:                                               ; preds = %9, %7
  %26 = phi ptr [ %8, %7 ], [ %24, %9 ]
  ret ptr %26
}

; Function Attrs: nounwind optsize
define internal void @split_one(ptr nocapture noundef %0, i32 noundef %1) #2 {
  %3 = getelementptr inbounds %struct.Splitting, ptr %0, i32 0, i32 3
  %4 = load ptr, ptr %3, align 4, !tbaa !49
  %5 = tail call ptr @fa_array_push_slot(ptr noundef %4, i64 noundef 8) #25
  %6 = load ptr, ptr %0, align 4, !tbaa !45
  %7 = getelementptr inbounds %struct.Splitting, ptr %0, i32 0, i32 1
  %8 = load i32, ptr %7, align 4, !tbaa !47
  %9 = getelementptr inbounds i8, ptr %6, i32 %8
  %10 = sub i32 %1, %8
  %11 = ptrtoint ptr %9 to i32
  store i32 %11, ptr %5, align 4
  %12 = getelementptr inbounds i8, ptr %5, i32 4
  store i32 %10, ptr %12, align 4
  %13 = getelementptr inbounds %struct.Splitting, ptr %0, i32 0, i32 2
  %14 = load i32, ptr %13, align 4, !tbaa !48
  %15 = add i32 %14, %1
  store i32 %15, ptr %7, align 4, !tbaa !47
  ret void
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_string_code_points(ptr nocapture noundef readonly %0, i32 noundef %1) local_unnamed_addr #2 {
  %3 = alloca i32, align 4
  %4 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #24
  %5 = tail call ptr @fa_alloc(i32 noundef 16, i32 noundef 16) #24
  store ptr %5, ptr %4, align 8, !tbaa !39
  %6 = getelementptr inbounds %struct.FaArrayHeader, ptr %4, i32 0, i32 1
  store i64 0, ptr %6, align 8, !tbaa !42
  %7 = getelementptr inbounds %struct.FaArrayHeader, ptr %4, i32 0, i32 2
  store i64 4, ptr %7, align 8, !tbaa !43
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %3) #27
  store i32 0, ptr %3, align 4, !tbaa !1
  %8 = icmp eq i32 %1, 0
  br i1 %8, label %9, label %10

9:                                                ; preds = %10, %2
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %3) #27
  ret ptr %4

10:                                               ; preds = %2, %10
  %11 = tail call ptr @fa_array_push_slot(ptr noundef nonnull %4, i64 noundef 4) #25
  %12 = call fastcc i32 @decode(ptr noundef %0, i32 noundef %1, ptr noundef nonnull %3) #25
  store i32 %12, ptr %11, align 4, !tbaa !14
  %13 = load i32, ptr %3, align 4, !tbaa !1
  %14 = icmp ult i32 %13, %1
  br i1 %14, label %10, label %9, !llvm.loop !50
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define internal fastcc i32 @decode(ptr nocapture noundef readonly %0, i32 noundef %1, ptr nocapture noundef %2) unnamed_addr #8 {
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
  br i1 %33, label %21, label %16, !llvm.loop !51
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define hidden noundef i32 @fa_string_get(ptr noundef %0, i32 noundef %1, i64 noundef %2, ptr nocapture noundef writeonly %3) local_unnamed_addr #8 {
  %5 = icmp slt i64 %2, 0
  br i1 %5, label %6, label %20

6:                                                ; preds = %4
  %7 = icmp eq i32 %1, 0
  br i1 %7, label %16, label %8

8:                                                ; preds = %6, %8
  %9 = phi i32 [ %12, %8 ], [ 0, %6 ]
  %10 = phi i32 [ %11, %8 ], [ 0, %6 ]
  %11 = add i32 %10, 1
  %12 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %9) #25
  %13 = icmp ult i32 %12, %1
  br i1 %13, label %8, label %14, !llvm.loop !24

14:                                               ; preds = %8
  %15 = zext i32 %11 to i64
  br label %16

16:                                               ; preds = %6, %14
  %17 = phi i64 [ 0, %6 ], [ %15, %14 ]
  %18 = add nsw i64 %17, %2
  %19 = icmp slt i64 %18, 0
  br i1 %19, label %36, label %20

20:                                               ; preds = %4, %16
  %21 = phi i64 [ %18, %16 ], [ %2, %4 ]
  %22 = icmp eq i32 %1, 0
  br i1 %22, label %36, label %26

23:                                               ; preds = %26
  %24 = add nuw nsw i64 %27, 1
  %25 = icmp ult i32 %29, %1
  br i1 %25, label %26, label %36, !llvm.loop !52

26:                                               ; preds = %20, %23
  %27 = phi i64 [ %24, %23 ], [ 0, %20 ]
  %28 = phi i32 [ %29, %23 ], [ 0, %20 ]
  %29 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %28) #25
  %30 = icmp eq i64 %27, %21
  br i1 %30, label %31, label %23

31:                                               ; preds = %26
  %32 = getelementptr inbounds i8, ptr %0, i32 %28
  %33 = sub i32 %29, %28
  %34 = ptrtoint ptr %32 to i32
  store i32 %34, ptr %3, align 4
  %35 = getelementptr inbounds i8, ptr %3, i32 4
  store i32 %33, ptr %35, align 4
  br label %36

36:                                               ; preds = %23, %31, %20, %16
  %37 = phi i32 [ 0, %16 ], [ 1, %31 ], [ 0, %20 ], [ 0, %23 ]
  ret i32 %37
}

; Function Attrs: nounwind optsize
define hidden void @fa_string_case(ptr nocapture noundef readonly %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef writeonly %3) local_unnamed_addr #2 {
  %5 = alloca i32, align 4
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %5) #27
  store i32 0, ptr %5, align 4, !tbaa !1
  %6 = icmp eq i32 %1, 0
  br i1 %6, label %146, label %7

7:                                                ; preds = %4
  %8 = icmp eq i32 %2, 0
  br label %9

9:                                                ; preds = %7, %141
  %10 = call fastcc i32 @decode(ptr noundef %0, i32 noundef %1, ptr noundef nonnull %5) #25
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
  tail call fastcc void @append_c(ptr noundef nonnull @.str.12) #25
  br label %141, !llvm.loop !53

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
  tail call fastcc void @reserve(i32 noundef 1) #25
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
  tail call fastcc void @reserve(i32 noundef 1) #25
  %78 = load ptr, ptr @builder, align 4, !tbaa !5
  %79 = load i32, ptr @builder_length, align 4, !tbaa !1
  %80 = getelementptr inbounds i8, ptr %78, i32 %79
  store i8 %77, ptr %80, align 1, !tbaa !9
  %81 = add i32 %79, 1
  store i32 %81, ptr @builder_length, align 4, !tbaa !1
  %82 = trunc i32 %74 to i8
  %83 = and i8 %82, 63
  %84 = or disjoint i8 %83, -128
  tail call fastcc void @reserve(i32 noundef 1) #25
  %85 = load ptr, ptr @builder, align 4, !tbaa !5
  %86 = load i32, ptr @builder_length, align 4, !tbaa !1
  %87 = getelementptr inbounds i8, ptr %85, i32 %86
  store i8 %84, ptr %87, align 1, !tbaa !9
  br label %138

88:                                               ; preds = %71
  %89 = icmp ult i32 %64, 65536
  tail call fastcc void @reserve(i32 noundef 1) #25
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
  tail call fastcc void @reserve(i32 noundef 1) #25
  %102 = load ptr, ptr @builder, align 4, !tbaa !5
  %103 = load i32, ptr @builder_length, align 4, !tbaa !1
  %104 = getelementptr inbounds i8, ptr %102, i32 %103
  store i8 %101, ptr %104, align 1, !tbaa !9
  %105 = add i32 %103, 1
  store i32 %105, ptr @builder_length, align 4, !tbaa !1
  %106 = trunc i32 %64 to i8
  %107 = and i8 %106, 63
  %108 = or disjoint i8 %107, -128
  tail call fastcc void @reserve(i32 noundef 1) #25
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
  tail call fastcc void @reserve(i32 noundef 1) #25
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
  tail call fastcc void @reserve(i32 noundef 1) #25
  %128 = load ptr, ptr @builder, align 4, !tbaa !5
  %129 = load i32, ptr @builder_length, align 4, !tbaa !1
  %130 = getelementptr inbounds i8, ptr %128, i32 %129
  store i8 %127, ptr %130, align 1, !tbaa !9
  %131 = add i32 %129, 1
  store i32 %131, ptr @builder_length, align 4, !tbaa !1
  %132 = trunc i32 %64 to i8
  %133 = and i8 %132, 63
  %134 = or disjoint i8 %133, -128
  tail call fastcc void @reserve(i32 noundef 1) #25
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

144:                                              ; preds = %141
  %145 = load i32, ptr @builder_length, align 4
  br label %146

146:                                              ; preds = %144, %4
  %147 = phi i32 [ %145, %144 ], [ 0, %4 ]
  %148 = tail call i32 @llvm.umax.i32(i32 %147, i32 1)
  %149 = tail call ptr @fa_alloc(i32 noundef %148, i32 noundef 1) #24
  %150 = load ptr, ptr @builder, align 4, !tbaa !5
  %151 = load i32, ptr @builder_length, align 4, !tbaa !1
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %149, ptr align 1 %150, i32 %151, i1 false)
  store ptr %149, ptr %3, align 4, !tbaa !10
  %152 = getelementptr inbounds %struct.FaString, ptr %3, i32 0, i32 1
  store i32 %151, ptr %152, align 4, !tbaa !12
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %5) #27
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define hidden void @fa_string_trim(ptr noundef %0, i32 noundef %1, i32 noundef %2, ptr nocapture noundef writeonly %3) local_unnamed_addr #8 {
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
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %5) #27
  store i32 %10, ptr %5, align 4, !tbaa !1
  %13 = call fastcc i32 @decode(ptr noundef %0, i32 noundef %1, ptr noundef nonnull %5) #25
  %14 = tail call fastcc i32 @is_space(i32 noundef %13) #25, !range !54
  %15 = icmp eq i32 %14, 0
  %16 = load i32, ptr %5, align 4
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %5) #27
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
  br i1 %32, label %24, label %33, !llvm.loop !55

33:                                               ; preds = %24, %28
  %34 = phi i32 [ %18, %24 ], [ %26, %28 ]
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %6) #27
  store i32 %34, ptr %6, align 4, !tbaa !1
  %35 = call fastcc i32 @decode(ptr noundef %0, i32 noundef %22, ptr noundef nonnull %6) #25
  %36 = tail call fastcc i32 @is_space(i32 noundef %35) #25, !range !54
  %37 = icmp eq i32 %36, 0
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %6) #27
  br i1 %37, label %38, label %21

38:                                               ; preds = %33, %21, %17
  %39 = phi i32 [ %1, %17 ], [ %22, %21 ], [ %22, %33 ]
  %40 = getelementptr inbounds i8, ptr %0, i32 %18
  store ptr %40, ptr %3, align 4, !tbaa !10
  %41 = sub i32 %39, %18
  %42 = getelementptr inbounds %struct.FaString, ptr %3, i32 0, i32 1
  store i32 %41, ptr %42, align 4, !tbaa !12
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
  br i1 %5, label %13, label %6

6:                                                ; preds = %13, %4
  %7 = phi i32 [ 0, %4 ], [ %18, %13 ]
  %8 = tail call i32 @llvm.umax.i32(i32 %7, i32 1)
  %9 = tail call ptr @fa_alloc(i32 noundef %8, i32 noundef 1) #24
  %10 = load ptr, ptr @builder, align 4, !tbaa !5
  %11 = load i32, ptr @builder_length, align 4, !tbaa !1
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %9, ptr align 1 %10, i32 %11, i1 false)
  store ptr %9, ptr %3, align 4, !tbaa !10
  %12 = getelementptr inbounds %struct.FaString, ptr %3, i32 0, i32 1
  store i32 %11, ptr %12, align 4, !tbaa !12
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  ret void

13:                                               ; preds = %4, %13
  %14 = phi i64 [ %19, %13 ], [ 0, %4 ]
  tail call fastcc void @reserve(i32 noundef %1) #25
  %15 = load ptr, ptr @builder, align 4, !tbaa !5
  %16 = load i32, ptr @builder_length, align 4, !tbaa !1
  %17 = getelementptr inbounds i8, ptr %15, i32 %16
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %17, ptr align 1 %0, i32 %1, i1 false)
  %18 = add i32 %16, %1
  store i32 %18, ptr @builder_length, align 4, !tbaa !1
  %19 = add nuw nsw i64 %14, 1
  %20 = icmp eq i64 %19, %2
  br i1 %20, label %6, label %13, !llvm.loop !56
}

; Function Attrs: nounwind optsize
define hidden void @fa_string_pad(ptr nocapture noundef readonly %0, i32 noundef %1, i64 noundef %2, ptr nocapture noundef readonly %3, i32 noundef %4, i32 noundef %5, ptr nocapture noundef writeonly %6) local_unnamed_addr #2 {
  %8 = icmp eq i32 %1, 0
  br i1 %8, label %17, label %9

9:                                                ; preds = %7, %9
  %10 = phi i32 [ %13, %9 ], [ 0, %7 ]
  %11 = phi i32 [ %12, %9 ], [ 0, %7 ]
  %12 = add i32 %11, 1
  %13 = tail call fastcc i32 @next_boundary(ptr noundef %0, i32 noundef %1, i32 noundef %10) #25
  %14 = icmp ult i32 %13, %1
  br i1 %14, label %9, label %15, !llvm.loop !24

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
  tail call fastcc void @reserve(i32 noundef %1) #25
  %22 = load ptr, ptr @builder, align 4, !tbaa !5
  %23 = load i32, ptr @builder_length, align 4, !tbaa !1
  %24 = getelementptr inbounds i8, ptr %22, i32 %23
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %24, ptr align 1 %0, i32 %1, i1 false)
  %25 = add i32 %23, %1
  store i32 %25, ptr @builder_length, align 4, !tbaa !1
  br label %26

26:                                               ; preds = %21, %17
  %27 = phi i32 [ %25, %21 ], [ 0, %17 ]
  %28 = icmp ne i32 %4, 0
  %29 = icmp sgt i64 %19, 0
  %30 = and i1 %28, %29
  br i1 %30, label %33, label %31

31:                                               ; preds = %33, %26
  %32 = phi i32 [ %27, %26 ], [ %42, %33 ]
  br i1 %20, label %52, label %47

33:                                               ; preds = %26, %33
  %34 = phi i32 [ %44, %33 ], [ 0, %26 ]
  %35 = phi i64 [ %45, %33 ], [ 0, %26 ]
  %36 = tail call fastcc i32 @next_boundary(ptr noundef %3, i32 noundef %4, i32 noundef %34) #25
  %37 = getelementptr inbounds i8, ptr %3, i32 %34
  %38 = sub i32 %36, %34
  tail call fastcc void @reserve(i32 noundef %38) #25
  %39 = load ptr, ptr @builder, align 4, !tbaa !5
  %40 = load i32, ptr @builder_length, align 4, !tbaa !1
  %41 = getelementptr inbounds i8, ptr %39, i32 %40
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %41, ptr align 1 %37, i32 %38, i1 false)
  %42 = add i32 %40, %38
  store i32 %42, ptr @builder_length, align 4, !tbaa !1
  %43 = icmp eq i32 %36, %4
  %44 = select i1 %43, i32 0, i32 %36
  %45 = add nuw nsw i64 %35, 1
  %46 = icmp slt i64 %45, %19
  br i1 %46, label %33, label %31, !llvm.loop !57

47:                                               ; preds = %31
  tail call fastcc void @reserve(i32 noundef %1) #25
  %48 = load ptr, ptr @builder, align 4, !tbaa !5
  %49 = load i32, ptr @builder_length, align 4, !tbaa !1
  %50 = getelementptr inbounds i8, ptr %48, i32 %49
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %50, ptr align 1 %0, i32 %1, i1 false)
  %51 = add i32 %49, %1
  store i32 %51, ptr @builder_length, align 4, !tbaa !1
  br label %52

52:                                               ; preds = %47, %31
  %53 = phi i32 [ %51, %47 ], [ %32, %31 ]
  %54 = tail call i32 @llvm.umax.i32(i32 %53, i32 1)
  %55 = tail call ptr @fa_alloc(i32 noundef %54, i32 noundef 1) #24
  %56 = load ptr, ptr @builder, align 4, !tbaa !5
  %57 = load i32, ptr @builder_length, align 4, !tbaa !1
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %55, ptr align 1 %56, i32 %57, i1 false)
  store ptr %55, ptr %6, align 4, !tbaa !10
  %58 = getelementptr inbounds %struct.FaString, ptr %6, i32 0, i32 1
  store i32 %57, ptr %58, align 4, !tbaa !12
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_arguments_init(i32 noundef %0, ptr nocapture noundef readnone %1) local_unnamed_addr #2 {
  %3 = tail call i32 @fa_host_arguments_size() #24
  %4 = tail call i32 @llvm.umax.i32(i32 %3, i32 1)
  %5 = tail call ptr @fa_alloc(i32 noundef %4, i32 noundef 1) #24
  tail call void @fa_host_arguments_copy(ptr noundef %5) #24
  store i32 0, ptr @argument_count, align 4, !tbaa !1
  %6 = icmp eq i32 %3, 0
  br i1 %6, label %7, label %13

7:                                                ; preds = %2
  %8 = tail call ptr @fa_alloc(i32 noundef 8, i32 noundef 4) #24
  store ptr %8, ptr @argument_items, align 4, !tbaa !5
  br label %23

9:                                                ; preds = %13
  store i32 %20, ptr @argument_count, align 4, !tbaa !1
  %10 = shl i32 %20, 3
  %11 = add i32 %10, 8
  %12 = tail call ptr @fa_alloc(i32 noundef %11, i32 noundef 4) #24
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
  br i1 %22, label %9, label %13, !llvm.loop !58

23:                                               ; preds = %40, %7
  ret void

24:                                               ; preds = %9, %40
  %25 = phi i32 [ %41, %40 ], [ 0, %9 ]
  %26 = phi i32 [ %43, %40 ], [ 0, %9 ]
  %27 = phi i32 [ %42, %40 ], [ 0, %9 ]
  %28 = getelementptr inbounds i8, ptr %5, i32 %25
  %29 = load i8, ptr %28, align 1, !tbaa !9
  %30 = icmp eq i8 %29, 0
  br i1 %30, label %33, label %31

31:                                               ; preds = %24
  %32 = add nuw i32 %25, 1
  br label %40

33:                                               ; preds = %24
  %34 = getelementptr inbounds i8, ptr %5, i32 %27
  %35 = getelementptr inbounds %struct.FaString, ptr %12, i32 %26
  store ptr %34, ptr %35, align 4, !tbaa !10
  %36 = sub i32 %25, %27
  %37 = getelementptr inbounds %struct.FaString, ptr %12, i32 %26, i32 1
  store i32 %36, ptr %37, align 4, !tbaa !12
  %38 = add i32 %26, 1
  %39 = add nuw i32 %25, 1
  br label %40

40:                                               ; preds = %31, %33
  %41 = phi i32 [ %32, %31 ], [ %39, %33 ]
  %42 = phi i32 [ %27, %31 ], [ %39, %33 ]
  %43 = phi i32 [ %26, %31 ], [ %38, %33 ]
  %44 = icmp eq i32 %41, %3
  br i1 %44, label %23, label %24, !llvm.loop !59
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
  %5 = load i64, ptr %4, align 8, !tbaa !42
  %6 = icmp sgt i64 %5, 0
  br i1 %6, label %7, label %9

7:                                                ; preds = %3
  %8 = trunc i64 %2 to i32
  br label %10

9:                                                ; preds = %10, %3
  ret void

10:                                               ; preds = %7, %10
  %11 = phi i64 [ 0, %7 ], [ %17, %10 ]
  %12 = tail call ptr @fa_array_push_slot(ptr noundef %0, i64 noundef %2) #25
  %13 = load ptr, ptr %1, align 8, !tbaa !39
  %14 = mul nsw i64 %11, %2
  %15 = trunc i64 %14 to i32
  %16 = getelementptr inbounds i8, ptr %13, i32 %15
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %12, ptr align 1 %16, i32 %8, i1 false)
  %17 = add nuw nsw i64 %11, 1
  %18 = icmp eq i64 %17, %5
  br i1 %18, label %9, label %10, !llvm.loop !60
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_array_copy(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #2 {
  %3 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %4 = load i64, ptr %3, align 8, !tbaa !42
  %5 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #24
  %6 = mul nsw i64 %4, %1
  %7 = trunc i64 %6 to i32
  %8 = tail call i32 @llvm.umax.i32(i32 %7, i32 1)
  %9 = tail call ptr @fa_alloc(i32 noundef %8, i32 noundef 16) #24
  store ptr %9, ptr %5, align 8, !tbaa !39
  %10 = getelementptr inbounds %struct.FaArrayHeader, ptr %5, i32 0, i32 1
  store i64 %4, ptr %10, align 8, !tbaa !42
  %11 = getelementptr inbounds %struct.FaArrayHeader, ptr %5, i32 0, i32 2
  store i64 %4, ptr %11, align 8, !tbaa !43
  %12 = load ptr, ptr %0, align 8, !tbaa !39
  %13 = load i64, ptr %3, align 8, !tbaa !42
  %14 = mul nsw i64 %13, %1
  %15 = trunc i64 %14 to i32
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %9, ptr align 1 %12, i32 %15, i1 false)
  ret ptr %5
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define hidden void @fa_array_reverse(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #13 {
  %3 = alloca [64 x i8], align 16
  %4 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %5 = load i64, ptr %4, align 8, !tbaa !42
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
  %13 = load ptr, ptr %0, align 8, !tbaa !39
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
  br i1 %24, label %18, label %35, !llvm.loop !61

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
  br i1 %34, label %25, label %21, !llvm.loop !62

35:                                               ; preds = %21, %7
  call void @llvm.lifetime.end.p0(i64 64, ptr nonnull %3)
  br label %36

36:                                               ; preds = %2, %35
  ret void
}

; Function Attrs: nounwind optsize
define hidden ptr @fa_array_insert_slot(ptr nocapture noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #2 {
  %4 = tail call ptr @fa_array_push_slot(ptr noundef %0, i64 noundef %2) #25
  %5 = load ptr, ptr %0, align 8, !tbaa !39
  %6 = mul nsw i64 %2, %1
  %7 = trunc i64 %6 to i32
  %8 = getelementptr inbounds i8, ptr %5, i32 %7
  %9 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %10 = load i64, ptr %9, align 8, !tbaa !42
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

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define hidden void @fa_array_remove(ptr nocapture noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #14 {
  %4 = load ptr, ptr %0, align 8, !tbaa !39
  %5 = mul nsw i64 %2, %1
  %6 = trunc i64 %5 to i32
  %7 = getelementptr inbounds i8, ptr %4, i32 %6
  %8 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %9 = load i64, ptr %8, align 8, !tbaa !42
  %10 = xor i64 %1, -1
  %11 = add i64 %9, %10
  %12 = mul nsw i64 %11, %2
  %13 = trunc i64 %12 to i32
  %14 = trunc i64 %2 to i32
  %15 = getelementptr inbounds i8, ptr %7, i32 %14
  tail call void @llvm.memmove.p0.p0.i32(ptr align 1 %7, ptr align 1 %15, i32 %13, i1 false)
  %16 = load i64, ptr %8, align 8, !tbaa !42
  %17 = add nsw i64 %16, -1
  store i64 %17, ptr %8, align 8, !tbaa !42
  ret void
}

; Function Attrs: nounwind optsize
define hidden void @fa_array_sort(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef readonly %2, ptr noundef %3) local_unnamed_addr #2 {
  %5 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i32 0, i32 1
  %6 = load i64, ptr %5, align 8, !tbaa !42
  %7 = icmp slt i64 %6, 2
  br i1 %7, label %86, label %8

8:                                                ; preds = %4
  %9 = mul nsw i64 %6, %1
  %10 = trunc i64 %9 to i32
  %11 = load ptr, ptr %0, align 8, !tbaa !39
  %12 = tail call ptr @fa_alloc(i32 noundef %10, i32 noundef 16) #24
  %13 = trunc i64 %1 to i32
  br label %14

14:                                               ; preds = %22, %8
  %15 = phi ptr [ %11, %8 ], [ %16, %22 ]
  %16 = phi ptr [ %12, %8 ], [ %15, %22 ]
  %17 = phi i64 [ 1, %8 ], [ %18, %22 ]
  %18 = shl nsw i64 %17, 1
  br label %24

19:                                               ; preds = %22
  %20 = load ptr, ptr %0, align 8, !tbaa !39
  %21 = icmp eq ptr %16, %20
  br i1 %21, label %84, label %83

22:                                               ; preds = %59
  %23 = icmp slt i64 %18, %6
  br i1 %23, label %14, label %19, !llvm.loop !63

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
  %43 = tail call i32 %2(ptr noundef %3, ptr noundef %39, ptr noundef %42) #24
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
  br i1 %58, label %33, label %59, !llvm.loop !64

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
  br i1 %82, label %24, label %22, !llvm.loop !65

83:                                               ; preds = %19
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %20, ptr align 1 %16, i32 %10, i1 false)
  br label %84

84:                                               ; preds = %19, %83
  %85 = phi ptr [ %16, %83 ], [ %15, %19 ]
  tail call void @fa_free(ptr noundef %85, i32 noundef %10, i32 noundef 16) #24
  br label %86

86:                                               ; preds = %84, %4
  ret void
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_arguments() local_unnamed_addr #2 {
  %1 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #24
  %2 = load ptr, ptr @argument_items, align 4, !tbaa !5
  store ptr %2, ptr %1, align 8, !tbaa !39
  %3 = load i32, ptr @argument_count, align 4, !tbaa !1
  %4 = zext i32 %3 to i64
  %5 = getelementptr inbounds %struct.FaArrayHeader, ptr %1, i32 0, i32 1
  store i64 %4, ptr %5, align 8, !tbaa !42
  %6 = getelementptr inbounds %struct.FaArrayHeader, ptr %1, i32 0, i32 2
  store i64 %4, ptr %6, align 8, !tbaa !43
  ret ptr %1
}

; Function Attrs: cold noinline noreturn nounwind optsize
define hidden void @fa_overflow(i64 noundef %0, i64 noundef %1, i32 noundef %2, ptr noundef %3) local_unnamed_addr #15 {
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  tail call fastcc void @append_c(ptr noundef nonnull @.str.13) #25
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
  tail call fastcc void @append_u128(i128 noundef %13, i32 noundef %15) #25
  br label %17

16:                                               ; preds = %4
  tail call fastcc void @append_u128(i128 noundef %9, i32 noundef 0) #25
  br label %17

17:                                               ; preds = %16, %10
  tail call fastcc void @append_c(ptr noundef nonnull @.str.14) #25
  tail call fastcc void @append_c(ptr noundef %3) #25
  br label %18

18:                                               ; preds = %18, %17
  %19 = phi i32 [ 0, %17 ], [ %23, %18 ]
  %20 = getelementptr inbounds i8, ptr %3, i32 %19
  %21 = load i8, ptr %20, align 1, !tbaa !9
  %22 = icmp eq i8 %21, 0
  %23 = add i32 %19, 1
  br i1 %22, label %24, label %18, !llvm.loop !18

24:                                               ; preds = %18
  %25 = getelementptr inbounds i8, ptr %20, i32 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %25) #28
  unreachable
}

; Function Attrs: noreturn nounwind optsize
define internal fastcc void @stop_with_builder(ptr noundef %0) unnamed_addr #3 {
  tail call void @fa_flush() #25
  %2 = load ptr, ptr @builder, align 4, !tbaa !5
  %3 = load i32, ptr @builder_length, align 4, !tbaa !1
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.25, i32 noundef 6) #24
  tail call void @fa_write(i32 noundef 2, ptr noundef %2, i32 noundef %3) #24
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.26, i32 noundef 6) #24
  br label %4

4:                                                ; preds = %4, %1
  %5 = phi i32 [ 0, %1 ], [ %9, %4 ]
  %6 = getelementptr inbounds i8, ptr %0, i32 %5
  %7 = load i8, ptr %6, align 1, !tbaa !9
  %8 = icmp eq i8 %7, 0
  %9 = add i32 %5, 1
  br i1 %8, label %10, label %4, !llvm.loop !18

10:                                               ; preds = %4
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull %0, i32 noundef %5) #24
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.19, i32 noundef 1) #24
  tail call void @fa_exit(i32 noundef 101) #26
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define hidden void @fa_overflow_unknown(ptr noundef %0) local_unnamed_addr #15 {
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  tail call fastcc void @append_c(ptr noundef nonnull @.str.15) #25
  tail call fastcc void @append_c(ptr noundef %0) #25
  br label %2

2:                                                ; preds = %2, %1
  %3 = phi i32 [ 0, %1 ], [ %7, %2 ]
  %4 = getelementptr inbounds i8, ptr %0, i32 %3
  %5 = load i8, ptr %4, align 1, !tbaa !9
  %6 = icmp eq i8 %5, 0
  %7 = add i32 %3, 1
  br i1 %6, label %8, label %2, !llvm.loop !18

8:                                                ; preds = %2
  %9 = getelementptr inbounds i8, ptr %4, i32 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %9) #28
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define hidden void @fa_float_overflow(double noundef %0, ptr noundef %1) local_unnamed_addr #15 {
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  tail call fastcc void @append_c(ptr noundef nonnull @.str.13) #25
  tail call void @fa_b_f64(double noundef %0) #25
  tail call fastcc void @append_c(ptr noundef nonnull @.str.14) #25
  tail call fastcc void @append_c(ptr noundef %1) #25
  br label %3

3:                                                ; preds = %3, %2
  %4 = phi i32 [ 0, %2 ], [ %8, %3 ]
  %5 = getelementptr inbounds i8, ptr %1, i32 %4
  %6 = load i8, ptr %5, align 1, !tbaa !9
  %7 = icmp eq i8 %6, 0
  %8 = add i32 %4, 1
  br i1 %7, label %9, label %3, !llvm.loop !18

9:                                                ; preds = %3
  %10 = getelementptr inbounds i8, ptr %5, i32 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %10) #28
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define hidden void @fa_assert_failed(ptr noundef %0) local_unnamed_addr #15 {
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  tail call fastcc void @append_c(ptr noundef nonnull @.str.16) #25
  tail call fastcc void @append_c(ptr noundef %0) #25
  br label %2

2:                                                ; preds = %2, %1
  %3 = phi i32 [ 0, %1 ], [ %7, %2 ]
  %4 = getelementptr inbounds i8, ptr %0, i32 %3
  %5 = load i8, ptr %4, align 1, !tbaa !9
  %6 = icmp eq i8 %5, 0
  %7 = add i32 %3, 1
  br i1 %6, label %8, label %2, !llvm.loop !18

8:                                                ; preds = %2
  %9 = getelementptr inbounds i8, ptr %4, i32 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %9) #28
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define hidden void @fa_assert_failed_operands(ptr noundef %0) local_unnamed_addr #15 {
  %2 = alloca %struct.FaString, align 4
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %2) #27
  call void @fa_b_take(ptr noundef nonnull %2) #25
  tail call fastcc void @append_c(ptr noundef nonnull @.str.16) #25
  tail call fastcc void @append_c(ptr noundef %0) #25
  tail call fastcc void @append_c(ptr noundef nonnull @.str.17) #25
  %3 = load ptr, ptr %2, align 4, !tbaa !10
  %4 = getelementptr inbounds %struct.FaString, ptr %2, i32 0, i32 1
  %5 = load i32, ptr %4, align 4, !tbaa !12
  tail call void @fa_b_bytes(ptr noundef %3, i32 noundef %5) #25
  br label %6

6:                                                ; preds = %6, %1
  %7 = phi i32 [ 0, %1 ], [ %11, %6 ]
  %8 = getelementptr inbounds i8, ptr %0, i32 %7
  %9 = load i8, ptr %8, align 1, !tbaa !9
  %10 = icmp eq i8 %9, 0
  %11 = add i32 %7, 1
  br i1 %10, label %12, label %6, !llvm.loop !18

12:                                               ; preds = %6
  %13 = getelementptr inbounds i8, ptr %8, i32 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %13) #28
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define hidden void @fa_uncaught(ptr noundef %0) local_unnamed_addr #15 {
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  tail call void @fa_format_union(ptr noundef %0, i32 noundef 1) #24
  tail call void @fa_flush() #25
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.18, i32 noundef 7) #24
  %2 = load ptr, ptr @builder, align 4, !tbaa !5
  %3 = load i32, ptr @builder_length, align 4, !tbaa !1
  tail call void @fa_write(i32 noundef 2, ptr noundef %2, i32 noundef %3) #24
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.19, i32 noundef 1) #24
  tail call void @fa_exit(i32 noundef 1) #26
  unreachable
}

; Function Attrs: optsize
declare void @fa_format_union(ptr noundef, i32 noundef) local_unnamed_addr #1

; Function Attrs: cold mustprogress nofree noinline norecurse nosync nounwind optsize willreturn memory(none)
define hidden noundef nonnull ptr @fa_error_division_by_zero() local_unnamed_addr #16 {
  ret ptr @division_by_zero
}

; Function Attrs: cold noinline nounwind optsize
define hidden noundef ptr @fa_error_conversion(i64 noundef %0, i64 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3) local_unnamed_addr #17 {
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
  tail call fastcc void @append_u128(i128 noundef %13, i32 noundef %15) #25
  br label %17

16:                                               ; preds = %4
  tail call fastcc void @append_u128(i128 noundef %9, i32 noundef 0) #25
  br label %17

17:                                               ; preds = %16, %10
  tail call fastcc void @append_c(ptr noundef nonnull @.str.14) #25
  tail call fastcc void @append_c(ptr noundef %3) #25
  %18 = tail call fastcc ptr @arithmetic_error_from_builder() #25
  ret ptr %18
}

; Function Attrs: nounwind optsize
define internal fastcc noundef ptr @arithmetic_error_from_builder() unnamed_addr #2 {
  %1 = tail call ptr @fa_alloc(i32 noundef 16, i32 noundef 8) #24
  store i64 4, ptr %1, align 8, !tbaa !66
  %2 = getelementptr inbounds %struct.MessageError, ptr %1, i32 0, i32 1
  %3 = load i32, ptr @builder_length, align 4
  %4 = tail call i32 @llvm.umax.i32(i32 %3, i32 1)
  %5 = tail call ptr @fa_alloc(i32 noundef %4, i32 noundef 1) #24
  %6 = load ptr, ptr @builder, align 4, !tbaa !5
  %7 = load i32, ptr @builder_length, align 4, !tbaa !1
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %5, ptr align 1 %6, i32 %7, i1 false)
  store ptr %5, ptr %2, align 4, !tbaa !10
  %8 = getelementptr inbounds %struct.MessageError, ptr %1, i32 0, i32 1, i32 1
  store i32 %7, ptr %8, align 4, !tbaa !12
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  ret ptr %1
}

; Function Attrs: cold noinline nounwind optsize
define hidden noundef ptr @fa_error_negative_exponent(i64 noundef %0) local_unnamed_addr #17 {
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  tail call fastcc void @append_c(ptr noundef nonnull @.str.20) #25
  %2 = tail call i64 @llvm.abs.i64(i64 %0, i1 false)
  %3 = lshr i64 %0, 63
  %4 = trunc i64 %3 to i32
  tail call fastcc void @append_u64(i64 noundef %2, i32 noundef %4) #25
  %5 = tail call fastcc ptr @arithmetic_error_from_builder() #25
  ret ptr %5
}

; Function Attrs: cold noinline nounwind optsize
define hidden noundef ptr @fa_error_float(double noundef %0, i32 noundef %1) local_unnamed_addr #17 {
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  tail call void @fa_b_f64(double noundef %0) #25
  %3 = icmp eq i32 %1, 0
  %4 = select i1 %3, ptr @.str.22, ptr @.str.21
  tail call fastcc void @append_c(ptr noundef nonnull %4) #25
  %5 = tail call fastcc ptr @arithmetic_error_from_builder() #25
  ret ptr %5
}

; Function Attrs: cold noinline nounwind optsize
define hidden noundef ptr @fa_error_index(i64 noundef %0, i64 noundef %1) local_unnamed_addr #17 {
  %3 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #24
  store i64 1, ptr %3, align 8, !tbaa !68
  %4 = getelementptr inbounds %struct.IndexError, ptr %3, i32 0, i32 1
  store i64 %0, ptr %4, align 8, !tbaa !70
  %5 = getelementptr inbounds %struct.IndexError, ptr %3, i32 0, i32 2
  store i64 %1, ptr %5, align 8, !tbaa !71
  ret ptr %3
}

; Function Attrs: nounwind optsize
define hidden noundef ptr @fa_parse_integer(ptr noundef %0, i32 noundef %1, ptr nocapture noundef writeonly %2) local_unnamed_addr #2 {
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
  br i1 %31, label %14, label %22, !llvm.loop !72

32:                                               ; preds = %9
  store i32 0, ptr @builder_length, align 4, !tbaa !1
  br i1 %4, label %54, label %33

33:                                               ; preds = %16, %32
  tail call fastcc void @reserve(i32 noundef 1) #25
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
  tail call fastcc void @reserve(i32 noundef 1) #25
  %43 = load ptr, ptr @builder, align 4, !tbaa !5
  %44 = load i32, ptr @builder_length, align 4, !tbaa !1
  %45 = getelementptr inbounds i8, ptr %43, i32 %44
  store i8 92, ptr %45, align 1, !tbaa !9
  %46 = add i32 %44, 1
  store i32 %46, ptr @builder_length, align 4, !tbaa !1
  br label %47

47:                                               ; preds = %38, %42
  tail call fastcc void @reserve(i32 noundef 1) #25
  %48 = load ptr, ptr @builder, align 4, !tbaa !5
  %49 = load i32, ptr @builder_length, align 4, !tbaa !1
  %50 = getelementptr inbounds i8, ptr %48, i32 %49
  store i8 %41, ptr %50, align 1, !tbaa !9
  %51 = add i32 %49, 1
  store i32 %51, ptr @builder_length, align 4, !tbaa !1
  %52 = add nuw i32 %39, 1
  %53 = icmp eq i32 %52, %1
  br i1 %53, label %54, label %38, !llvm.loop !73

54:                                               ; preds = %47, %32, %13
  %55 = phi ptr [ @.str.23, %13 ], [ @.str.23, %32 ], [ @.str.24, %47 ]
  tail call fastcc void @append_c(ptr noundef nonnull %55) #25
  %56 = tail call fastcc ptr @parse_error(ptr noundef %0, i32 noundef %1) #25
  br label %90

57:                                               ; preds = %64
  %58 = getelementptr inbounds i8, ptr %0, i32 %71
  %59 = load i8, ptr %58, align 1, !tbaa !9
  %60 = zext i8 %59 to i64
  %61 = sub nsw i64 47, %60
  %62 = udiv i64 %61, 10
  %63 = icmp ugt i64 %70, %62
  br i1 %63, label %73, label %64, !llvm.loop !74

64:                                               ; preds = %18, %57
  %65 = phi i64 [ %21, %18 ], [ %60, %57 ]
  %66 = phi i64 [ 0, %18 ], [ %70, %57 ]
  %67 = phi i32 [ %11, %18 ], [ %71, %57 ]
  %68 = mul nuw i64 %66, 10
  %69 = add i64 %68, -48
  %70 = add i64 %69, %65
  %71 = add nuw i32 %67, 1
  %72 = icmp eq i32 %71, %1
  br i1 %72, label %73, label %57, !llvm.loop !74

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
  tail call fastcc void @reserve(i32 noundef %1) #25
  %82 = load ptr, ptr @builder, align 4, !tbaa !5
  %83 = load i32, ptr @builder_length, align 4, !tbaa !1
  %84 = getelementptr inbounds i8, ptr %82, i32 %83
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %84, ptr nonnull align 1 %0, i32 %1, i1 false)
  %85 = add i32 %83, %1
  store i32 %85, ptr @builder_length, align 4, !tbaa !1
  tail call fastcc void @append_c(ptr noundef nonnull @.str.21) #25
  %86 = tail call fastcc ptr @parse_error(ptr noundef nonnull %0, i32 noundef %1) #25
  br label %90

87:                                               ; preds = %75
  %88 = sub i64 0, %76
  %89 = select i1 %10, i64 %76, i64 %88
  store i64 %89, ptr %2, align 8, !tbaa !75
  br label %90

90:                                               ; preds = %81, %87, %54
  %91 = phi ptr [ %56, %54 ], [ %86, %81 ], [ null, %87 ]
  ret ptr %91
}

; Function Attrs: nounwind optsize
define internal fastcc noundef ptr @parse_error(ptr noundef %0, i32 noundef %1) unnamed_addr #2 {
  %3 = tail call ptr @fa_alloc(i32 noundef 24, i32 noundef 8) #24
  store i64 5, ptr %3, align 8, !tbaa !76
  %4 = getelementptr inbounds %struct.ParseError, ptr %3, i32 0, i32 1
  store ptr %0, ptr %4, align 8, !tbaa !78
  %5 = getelementptr inbounds %struct.ParseError, ptr %3, i32 0, i32 1, i32 1
  store i32 %1, ptr %5, align 4, !tbaa !79
  %6 = getelementptr inbounds %struct.ParseError, ptr %3, i32 0, i32 2
  %7 = load i32, ptr @builder_length, align 4
  %8 = tail call i32 @llvm.umax.i32(i32 %7, i32 1)
  %9 = tail call ptr @fa_alloc(i32 noundef %8, i32 noundef 1) #24
  %10 = load ptr, ptr @builder, align 4, !tbaa !5
  %11 = load i32, ptr @builder_length, align 4, !tbaa !1
  tail call void @llvm.memcpy.p0.p0.i32(ptr align 1 %9, ptr align 1 %10, i32 %11, i1 false)
  store ptr %9, ptr %6, align 4, !tbaa !10
  %12 = getelementptr inbounds %struct.ParseError, ptr %3, i32 0, i32 2, i32 1
  store i32 %11, ptr %12, align 4, !tbaa !12
  store i32 0, ptr @builder_length, align 4, !tbaa !1
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
  tail call void @fa_overflow_unknown(ptr noundef %3) #29
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
  tail call void @fa_overflow_unknown(ptr noundef %3) #29
  unreachable
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare { i64, i1 } @llvm.smul.with.overflow.i64(i64, i64) #18

; Function Attrs: nounwind optsize
define weak hidden double @pow(double noundef %0, double noundef %1) local_unnamed_addr #2 {
  %3 = tail call double @fa_host_pow(double noundef %0, double noundef %1) #24
  ret double %3
}

; Function Attrs: optsize
declare double @fa_host_pow(double noundef, double noundef) local_unnamed_addr #19

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
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %3) #27
  %4 = call fastcc i128 @divide_u128(i128 noundef %0, i128 noundef %1, ptr noundef nonnull %3) #25
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %3) #27
  ret i128 %4
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: write)
define internal fastcc i128 @divide_u128(i128 noundef %0, i128 noundef %1, ptr nocapture noundef writeonly %2) unnamed_addr #20 {
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
  store i128 %14, ptr %2, align 16, !tbaa !80
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
  br i1 %58, label %10, label %19, !llvm.loop !82
}

; Function Attrs: nounwind optsize
define weak hidden i128 @__umodti3(i128 noundef %0, i128 noundef %1) local_unnamed_addr #2 {
  %3 = alloca i128, align 16
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %3) #27
  %4 = call fastcc i128 @divide_u128(i128 noundef %0, i128 noundef %1, ptr noundef nonnull %3) #25
  %5 = load i128, ptr %3, align 16, !tbaa !80
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %3) #27
  ret i128 %5
}

; Function Attrs: nounwind optsize
define weak hidden i128 @__divti3(i128 noundef %0, i128 noundef %1) local_unnamed_addr #2 {
  %3 = alloca i128, align 16
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %3) #27
  %4 = tail call noundef i128 @llvm.abs.i128(i128 %0, i1 false)
  %5 = tail call noundef i128 @llvm.abs.i128(i128 %1, i1 false)
  %6 = call fastcc i128 @divide_u128(i128 noundef %4, i128 noundef %5, ptr noundef nonnull %3) #25
  %7 = lshr i128 %0, 127
  %8 = trunc i128 %7 to i32
  %9 = lshr i128 %1, 127
  %10 = trunc i128 %9 to i32
  %11 = icmp eq i32 %8, %10
  %12 = sub i128 0, %6
  %13 = select i1 %11, i128 %6, i128 %12
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %3) #27
  ret i128 %13
}

; Function Attrs: nounwind optsize
define weak hidden i128 @__modti3(i128 noundef %0, i128 noundef %1) local_unnamed_addr #2 {
  %3 = alloca i128, align 16
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %3) #27
  %4 = tail call noundef i128 @llvm.abs.i128(i128 %0, i1 false)
  %5 = tail call noundef i128 @llvm.abs.i128(i128 %1, i1 false)
  %6 = call fastcc i128 @divide_u128(i128 noundef %4, i128 noundef %5, ptr noundef nonnull %3) #25
  %7 = icmp slt i128 %0, 0
  %8 = load i128, ptr %3, align 16, !tbaa !80
  %9 = sub i128 0, %8
  %10 = select i1 %7, i128 %9, i128 %8
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %3) #27
  ret i128 %10
}

; Function Attrs: optsize
declare i32 @fa_host_format_float(double noundef, i32 noundef, ptr noundef) local_unnamed_addr #21

; Function Attrs: nofree norecurse nosync nounwind optsize memory(none)
define internal fastcc i32 @break_kind(i32 noundef %0) unnamed_addr #22 {
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
  %59 = load i32, ptr %58, align 8, !tbaa !83
  %60 = icmp ugt i32 %59, %0
  br i1 %60, label %52, label %61, !llvm.loop !85

61:                                               ; preds = %55
  %62 = getelementptr inbounds %struct.Range, ptr @EXTEND, i32 %57, i32 1
  %63 = load i32, ptr %62, align 4, !tbaa !86
  %64 = icmp ult i32 %63, %0
  %65 = add nuw i32 %57, 1
  br i1 %64, label %49, label %83, !llvm.loop !85

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
  %76 = load i32, ptr %75, align 8, !tbaa !83
  %77 = icmp ugt i32 %76, %0
  br i1 %77, label %69, label %78, !llvm.loop !85

78:                                               ; preds = %72
  %79 = getelementptr inbounds %struct.Range, ptr @PICTOGRAPHIC, i32 %74, i32 1
  %80 = load i32, ptr %79, align 4, !tbaa !86
  %81 = icmp ult i32 %80, %0
  %82 = add nuw i32 %74, 1
  br i1 %81, label %66, label %83, !llvm.loop !85

83:                                               ; preds = %61, %78, %69, %35, %29, %23, %20, %18, %3, %5, %1, %44, %13, %2
  %84 = phi i32 [ 2, %2 ], [ %17, %13 ], [ %48, %44 ], [ 1, %1 ], [ 3, %5 ], [ 3, %3 ], [ 5, %18 ], [ 6, %20 ], [ 8, %23 ], [ 9, %29 ], [ 10, %35 ], [ 0, %69 ], [ 7, %78 ], [ 4, %61 ]
  ret i32 %84
}

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.umax.i32(i32, i32) #23

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.abs.i64(i64, i1 immarg) #23

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.umin.i32(i32, i32) #23

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.smin.i64(i64, i64) #23

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.fshl.i64(i64, i64, i64) #23

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.smax.i64(i64, i64) #23

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i128 @llvm.abs.i128(i128, i1 immarg) #23

attributes #0 = { nounwind optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" "wasm-export-name"="flush" }
attributes #1 = { optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #2 = { nounwind optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #3 = { noreturn nounwind optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #4 = { noreturn optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #5 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, argmem: none, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #6 = { mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite) }
attributes #7 = { nofree norecurse nosync nounwind optsize memory(argmem: read) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #8 = { nofree norecurse nosync nounwind optsize memory(argmem: readwrite) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #9 = { mustprogress nocallback nofree nounwind willreturn memory(argmem: readwrite) }
attributes #10 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #11 = { optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" "wasm-import-module"="fa" "wasm-import-name"="argumentsSize" }
attributes #12 = { optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" "wasm-import-module"="fa" "wasm-import-name"="argumentsCopy" }
attributes #13 = { nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #14 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(readwrite, inaccessiblemem: none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #15 = { cold noinline noreturn nounwind optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #16 = { cold mustprogress nofree noinline norecurse nosync nounwind optsize willreturn memory(none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #17 = { cold noinline nounwind optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #18 = { mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #19 = { optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" "wasm-import-module"="fa" "wasm-import-name"="pow" }
attributes #20 = { nofree norecurse nosync nounwind optsize memory(argmem: write) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #21 = { optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" "wasm-import-module"="fa" "wasm-import-name"="formatFloat" }
attributes #22 = { nofree norecurse nosync nounwind optsize memory(none) "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #23 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #24 = { nounwind optsize }
attributes #25 = { optsize }
attributes #26 = { noreturn nounwind optsize }
attributes #27 = { nounwind }
attributes #28 = { noreturn optsize }
attributes #29 = { cold noreturn optsize }

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
!10 = !{!11, !6, i64 0}
!11 = !{!"", !6, i64 0, !2, i64 4}
!12 = !{!11, !2, i64 4}
!13 = distinct !{!13, !8}
!14 = !{!15, !15, i64 0}
!15 = !{!"int", !3, i64 0}
!16 = distinct !{!16, !8}
!17 = distinct !{!17, !8}
!18 = distinct !{!18, !8}
!19 = distinct !{!19, !8}
!20 = distinct !{!20, !8}
!21 = distinct !{!21, !8}
!22 = distinct !{!22, !8}
!23 = !{i32 0, i32 13}
!24 = distinct !{!24, !8}
!25 = distinct !{!25, !8}
!26 = distinct !{!26, !8}
!27 = distinct !{!27, !8}
!28 = distinct !{!28, !8}
!29 = !{!30, !6, i64 0}
!30 = !{!"", !6, i64 0, !2, i64 4, !2, i64 8, !6, i64 12, !2, i64 16, !15, i64 20, !15, i64 24}
!31 = !{!30, !2, i64 4}
!32 = !{!30, !2, i64 8}
!33 = !{!30, !6, i64 12}
!34 = !{!30, !2, i64 16}
!35 = !{!30, !15, i64 20}
!36 = !{!30, !15, i64 24}
!37 = !{ptr @replace_one, ptr @split_one}
!38 = distinct !{!38, !8}
!39 = !{!40, !6, i64 0}
!40 = !{!"", !6, i64 0, !41, i64 8, !41, i64 16}
!41 = !{!"long long", !3, i64 0}
!42 = !{!40, !41, i64 8}
!43 = !{!40, !41, i64 16}
!44 = distinct !{!44, !8}
!45 = !{!46, !6, i64 0}
!46 = !{!"", !6, i64 0, !2, i64 4, !2, i64 8, !6, i64 12}
!47 = !{!46, !2, i64 4}
!48 = !{!46, !2, i64 8}
!49 = !{!46, !6, i64 12}
!50 = distinct !{!50, !8}
!51 = distinct !{!51, !8}
!52 = distinct !{!52, !8}
!53 = distinct !{!53, !8}
!54 = !{i32 0, i32 2}
!55 = distinct !{!55, !8}
!56 = distinct !{!56, !8}
!57 = distinct !{!57, !8}
!58 = distinct !{!58, !8}
!59 = distinct !{!59, !8}
!60 = distinct !{!60, !8}
!61 = distinct !{!61, !8}
!62 = distinct !{!62, !8}
!63 = distinct !{!63, !8}
!64 = distinct !{!64, !8}
!65 = distinct !{!65, !8}
!66 = !{!67, !41, i64 0}
!67 = !{!"", !41, i64 0, !11, i64 8}
!68 = !{!69, !41, i64 0}
!69 = !{!"", !41, i64 0, !41, i64 8, !41, i64 16}
!70 = !{!69, !41, i64 8}
!71 = !{!69, !41, i64 16}
!72 = distinct !{!72, !8}
!73 = distinct !{!73, !8}
!74 = distinct !{!74, !8}
!75 = !{!41, !41, i64 0}
!76 = !{!77, !41, i64 0}
!77 = !{!"", !41, i64 0, !11, i64 8, !11, i64 16}
!78 = !{!77, !6, i64 8}
!79 = !{!77, !2, i64 12}
!80 = !{!81, !81, i64 0}
!81 = !{!"__int128", !3, i64 0}
!82 = distinct !{!82, !8}
!83 = !{!84, !15, i64 0}
!84 = !{!"", !15, i64 0, !15, i64 4}
!85 = distinct !{!85, !8}
!86 = !{!84, !15, i64 4}
