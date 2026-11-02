source_filename = "fa_runtime.c"

%struct.AlternateStack = type { ptr, i32, i64 }
%struct.SignalAction = type { ptr, [16 x i64], i32, ptr }
%struct.Range = type { i32, i32 }
%struct.FaString = type { ptr, i64 }
%struct.FaArray = type { ptr, i64 }

@output_length = internal unnamed_addr global i64 0, align 8
@output = internal global [8192 x i8] zeroinitializer, align 16
@builder_length = internal unnamed_addr global i64 0, align 8
@builder = internal unnamed_addr global ptr null, align 8
@.str = private unnamed_addr constant [5 x i8] c"true\00", align 1
@.str.1 = private unnamed_addr constant [6 x i8] c"false\00", align 1
@fa_b_quoted.hex = internal unnamed_addr constant [17 x i8] c"0123456789ABCDEF\00", align 16
@argument_count = internal unnamed_addr global i64 0, align 8
@argument_items = internal unnamed_addr global ptr null, align 8
@.str.13 = private unnamed_addr constant [19 x i8] c"integer overflow: \00", align 1
@.str.14 = private unnamed_addr constant [17 x i8] c" doesn't fit in \00", align 1
@.str.15 = private unnamed_addr constant [44 x i8] c"integer overflow: the value doesn't fit in \00", align 1
@.str.16 = private unnamed_addr constant [19 x i8] c"assertion failed: \00", align 1
@signal_stack = internal global [16384 x i8] zeroinitializer, align 16
@__const.fa_guard_stack.stack = private unnamed_addr constant %struct.AlternateStack { ptr @signal_stack, i32 0, i64 16384 }, align 8
@__const.fa_guard_stack.action = private unnamed_addr constant %struct.SignalAction { ptr @on_fault, [16 x i64] zeroinitializer, i32 134217728, ptr null }, align 8
@.str.17 = private unnamed_addr constant [15 x i8] c"DivisionByZero\00", align 1
@.str.18 = private unnamed_addr constant [29 x i8] c"ArithmeticError { message = \00", align 1
@.str.19 = private unnamed_addr constant [3 x i8] c" }\00", align 1
@builder_capacity = internal unnamed_addr global i64 0, align 8
@.str.20 = private unnamed_addr constant [4 x i8] c"NaN\00", align 1
@.str.22 = private unnamed_addr constant [3 x i8] c"0.\00", align 1
@.str.23 = private unnamed_addr constant [5 x i8] c"%.*e\00", align 1
@.str.24 = private unnamed_addr constant [8 x i8] c"%llde%d\00", align 1
@.str.25 = private unnamed_addr constant [6 x i8] c"%.14e\00", align 1
@EXTEND = internal unnamed_addr constant [153 x %struct.Range] [%struct.Range { i32 768, i32 879 }, %struct.Range { i32 1155, i32 1161 }, %struct.Range { i32 1425, i32 1469 }, %struct.Range { i32 1471, i32 1471 }, %struct.Range { i32 1473, i32 1474 }, %struct.Range { i32 1476, i32 1477 }, %struct.Range { i32 1479, i32 1479 }, %struct.Range { i32 1552, i32 1562 }, %struct.Range { i32 1611, i32 1631 }, %struct.Range { i32 1648, i32 1648 }, %struct.Range { i32 1750, i32 1756 }, %struct.Range { i32 1759, i32 1764 }, %struct.Range { i32 1767, i32 1768 }, %struct.Range { i32 1770, i32 1773 }, %struct.Range { i32 1809, i32 1809 }, %struct.Range { i32 1840, i32 1866 }, %struct.Range { i32 1958, i32 1968 }, %struct.Range { i32 2027, i32 2035 }, %struct.Range { i32 2070, i32 2093 }, %struct.Range { i32 2137, i32 2139 }, %struct.Range { i32 2200, i32 2207 }, %struct.Range { i32 2250, i32 2273 }, %struct.Range { i32 2275, i32 2307 }, %struct.Range { i32 2362, i32 2364 }, %struct.Range { i32 2366, i32 2383 }, %struct.Range { i32 2385, i32 2391 }, %struct.Range { i32 2402, i32 2403 }, %struct.Range { i32 2433, i32 2435 }, %struct.Range { i32 2492, i32 2492 }, %struct.Range { i32 2494, i32 2509 }, %struct.Range { i32 2519, i32 2519 }, %struct.Range { i32 2530, i32 2531 }, %struct.Range { i32 2561, i32 2563 }, %struct.Range { i32 2620, i32 2641 }, %struct.Range { i32 2672, i32 2673 }, %struct.Range { i32 2677, i32 2677 }, %struct.Range { i32 2689, i32 2691 }, %struct.Range { i32 2748, i32 2748 }, %struct.Range { i32 2750, i32 2765 }, %struct.Range { i32 2786, i32 2787 }, %struct.Range { i32 2817, i32 2819 }, %struct.Range { i32 2876, i32 2876 }, %struct.Range { i32 2878, i32 2903 }, %struct.Range { i32 2914, i32 2915 }, %struct.Range { i32 2946, i32 2946 }, %struct.Range { i32 3006, i32 3021 }, %struct.Range { i32 3031, i32 3031 }, %struct.Range { i32 3072, i32 3076 }, %struct.Range { i32 3132, i32 3132 }, %struct.Range { i32 3134, i32 3158 }, %struct.Range { i32 3170, i32 3171 }, %struct.Range { i32 3201, i32 3203 }, %struct.Range { i32 3260, i32 3260 }, %struct.Range { i32 3262, i32 3286 }, %struct.Range { i32 3298, i32 3299 }, %struct.Range { i32 3328, i32 3331 }, %struct.Range { i32 3387, i32 3388 }, %struct.Range { i32 3390, i32 3405 }, %struct.Range { i32 3415, i32 3415 }, %struct.Range { i32 3426, i32 3427 }, %struct.Range { i32 3457, i32 3459 }, %struct.Range { i32 3530, i32 3551 }, %struct.Range { i32 3570, i32 3571 }, %struct.Range { i32 3633, i32 3633 }, %struct.Range { i32 3635, i32 3642 }, %struct.Range { i32 3655, i32 3662 }, %struct.Range { i32 3761, i32 3761 }, %struct.Range { i32 3763, i32 3772 }, %struct.Range { i32 3784, i32 3790 }, %struct.Range { i32 3864, i32 3865 }, %struct.Range { i32 3893, i32 3893 }, %struct.Range { i32 3895, i32 3895 }, %struct.Range { i32 3897, i32 3897 }, %struct.Range { i32 3902, i32 3903 }, %struct.Range { i32 3953, i32 3972 }, %struct.Range { i32 3974, i32 3975 }, %struct.Range { i32 3981, i32 4028 }, %struct.Range { i32 4038, i32 4038 }, %struct.Range { i32 4139, i32 4158 }, %struct.Range { i32 4182, i32 4185 }, %struct.Range { i32 4190, i32 4192 }, %struct.Range { i32 4194, i32 4196 }, %struct.Range { i32 4199, i32 4205 }, %struct.Range { i32 4209, i32 4212 }, %struct.Range { i32 4226, i32 4237 }, %struct.Range { i32 4239, i32 4239 }, %struct.Range { i32 4250, i32 4253 }, %struct.Range { i32 4957, i32 4959 }, %struct.Range { i32 5906, i32 5909 }, %struct.Range { i32 5938, i32 5940 }, %struct.Range { i32 5970, i32 5971 }, %struct.Range { i32 6002, i32 6003 }, %struct.Range { i32 6068, i32 6099 }, %struct.Range { i32 6109, i32 6109 }, %struct.Range { i32 6155, i32 6157 }, %struct.Range { i32 6159, i32 6159 }, %struct.Range { i32 6277, i32 6278 }, %struct.Range { i32 6313, i32 6313 }, %struct.Range { i32 6432, i32 6459 }, %struct.Range { i32 6679, i32 6683 }, %struct.Range { i32 6741, i32 6783 }, %struct.Range { i32 6832, i32 6911 }, %struct.Range { i32 6912, i32 6916 }, %struct.Range { i32 6964, i32 6980 }, %struct.Range { i32 7019, i32 7027 }, %struct.Range { i32 7040, i32 7042 }, %struct.Range { i32 7073, i32 7085 }, %struct.Range { i32 7142, i32 7155 }, %struct.Range { i32 7204, i32 7223 }, %struct.Range { i32 7376, i32 7378 }, %struct.Range { i32 7380, i32 7400 }, %struct.Range { i32 7405, i32 7405 }, %struct.Range { i32 7412, i32 7412 }, %struct.Range { i32 7415, i32 7417 }, %struct.Range { i32 7616, i32 7679 }, %struct.Range { i32 8204, i32 8204 }, %struct.Range { i32 8400, i32 8432 }, %struct.Range { i32 11503, i32 11505 }, %struct.Range { i32 11647, i32 11647 }, %struct.Range { i32 11744, i32 11775 }, %struct.Range { i32 12330, i32 12335 }, %struct.Range { i32 12441, i32 12442 }, %struct.Range { i32 42607, i32 42610 }, %struct.Range { i32 42612, i32 42621 }, %struct.Range { i32 42654, i32 42655 }, %struct.Range { i32 42736, i32 42737 }, %struct.Range { i32 43010, i32 43010 }, %struct.Range { i32 43014, i32 43014 }, %struct.Range { i32 43019, i32 43019 }, %struct.Range { i32 43043, i32 43047 }, %struct.Range { i32 43052, i32 43052 }, %struct.Range { i32 43136, i32 43137 }, %struct.Range { i32 43188, i32 43205 }, %struct.Range { i32 43232, i32 43249 }, %struct.Range { i32 43263, i32 43263 }, %struct.Range { i32 43302, i32 43309 }, %struct.Range { i32 43335, i32 43347 }, %struct.Range { i32 43392, i32 43395 }, %struct.Range { i32 43443, i32 43456 }, %struct.Range { i32 43561, i32 43574 }, %struct.Range { i32 43587, i32 43587 }, %struct.Range { i32 43596, i32 43597 }, %struct.Range { i32 43755, i32 43759 }, %struct.Range { i32 43765, i32 43766 }, %struct.Range { i32 44003, i32 44010 }, %struct.Range { i32 44012, i32 44013 }, %struct.Range { i32 64286, i32 64286 }, %struct.Range { i32 65024, i32 65039 }, %struct.Range { i32 65056, i32 65071 }, %struct.Range { i32 65438, i32 65439 }, %struct.Range { i32 127995, i32 127999 }, %struct.Range { i32 917536, i32 917631 }, %struct.Range { i32 917760, i32 917999 }], align 16
@PICTOGRAPHIC = internal unnamed_addr constant [22 x %struct.Range] [%struct.Range { i32 169, i32 169 }, %struct.Range { i32 174, i32 174 }, %struct.Range { i32 8252, i32 8252 }, %struct.Range { i32 8265, i32 8265 }, %struct.Range { i32 8482, i32 8482 }, %struct.Range { i32 8505, i32 8505 }, %struct.Range { i32 8596, i32 8618 }, %struct.Range { i32 8986, i32 9215 }, %struct.Range { i32 9410, i32 9410 }, %struct.Range { i32 9642, i32 9726 }, %struct.Range { i32 9728, i32 10175 }, %struct.Range { i32 10548, i32 10549 }, %struct.Range { i32 11013, i32 11093 }, %struct.Range { i32 12336, i32 12336 }, %struct.Range { i32 12349, i32 12349 }, %struct.Range { i32 12951, i32 12951 }, %struct.Range { i32 12953, i32 12953 }, %struct.Range { i32 126976, i32 127231 }, %struct.Range { i32 127245, i32 127405 }, %struct.Range { i32 127489, i32 127994 }, %struct.Range { i32 128000, i32 129791 }, %struct.Range { i32 130048, i32 131069 }], align 16
@.str.26 = private unnamed_addr constant [7 x i8] c"stop: \00", align 1
@.str.27 = private unnamed_addr constant [7 x i8] c"\0A  at \00", align 1
@.str.28 = private unnamed_addr constant [2 x i8] c"\0A\00", align 1
@on_fault.message = internal constant [22 x i8] c"stop: stack overflow\0A\00", align 16
@.str.29 = private unnamed_addr constant [8 x i8] c"error: \00", align 1
@.str.30 = private unnamed_addr constant [20 x i8] c"negative exponent: \00", align 1

; Function Attrs: nounwind optsize
define void @fa_flush() local_unnamed_addr #0 {
  %1 = load i64, ptr @output_length, align 8, !tbaa !2
  %2 = icmp eq i64 %1, 0
  br i1 %2, label %4, label %3

3:                                                ; preds = %0
  tail call void @fa_write(i32 noundef 1, ptr noundef nonnull @output, i64 noundef %1) #16
  store i64 0, ptr @output_length, align 8, !tbaa !2
  br label %4

4:                                                ; preds = %3, %0
  ret void
}

; Function Attrs: optsize
declare void @fa_write(i32 noundef, ptr noundef, i64 noundef) local_unnamed_addr #1

; Function Attrs: nounwind optsize
define void @fa_out(i32 noundef %0, ptr noundef %1, i64 noundef %2) local_unnamed_addr #0 {
  %4 = icmp eq i32 %0, 1
  %5 = load i64, ptr @output_length, align 8, !tbaa !2
  br i1 %4, label %10, label %6

6:                                                ; preds = %3
  %7 = icmp eq i64 %5, 0
  br i1 %7, label %9, label %8

8:                                                ; preds = %6
  tail call void @fa_write(i32 noundef 1, ptr noundef nonnull @output, i64 noundef %5) #16
  store i64 0, ptr @output_length, align 8, !tbaa !2
  br label %9

9:                                                ; preds = %6, %8
  tail call void @fa_write(i32 noundef %0, ptr noundef %1, i64 noundef %2) #16
  br label %23

10:                                               ; preds = %3
  %11 = add i64 %5, %2
  %12 = icmp ugt i64 %11, 8192
  br i1 %12, label %13, label %19

13:                                               ; preds = %10
  %14 = icmp eq i64 %5, 0
  br i1 %14, label %16, label %15

15:                                               ; preds = %13
  tail call void @fa_write(i32 noundef 1, ptr noundef nonnull @output, i64 noundef %5) #16
  store i64 0, ptr @output_length, align 8, !tbaa !2
  br label %16

16:                                               ; preds = %13, %15
  %17 = icmp ugt i64 %2, 8192
  br i1 %17, label %18, label %19

18:                                               ; preds = %16
  tail call void @fa_write(i32 noundef 1, ptr noundef %1, i64 noundef %2) #16
  br label %23

19:                                               ; preds = %16, %10
  %20 = phi i64 [ %5, %10 ], [ 0, %16 ]
  %21 = getelementptr inbounds i8, ptr @output, i64 %20
  tail call void @llvm.memcpy.p0.p0.i64(ptr nonnull align 1 %21, ptr align 1 %1, i64 %2, i1 false)
  %22 = add i64 %20, %2
  store i64 %22, ptr @output_length, align 8, !tbaa !2
  br label %23

23:                                               ; preds = %19, %18, %9
  ret void
}

; Function Attrs: noreturn nounwind optsize
define void @fa_terminate(i32 noundef %0) local_unnamed_addr #2 {
  tail call void @fa_flush() #17
  tail call void @fa_exit(i32 noundef %0) #18
  unreachable
}

; Function Attrs: noreturn optsize
declare void @fa_exit(i32 noundef) local_unnamed_addr #3

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, argmem: none, inaccessiblemem: none)
define void @fa_b_reset() local_unnamed_addr #4 {
  store i64 0, ptr @builder_length, align 8, !tbaa !2
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_b_bytes(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  tail call fastcc void @reserve(i64 noundef %1) #17
  %3 = load ptr, ptr @builder, align 8, !tbaa !6
  %4 = load i64, ptr @builder_length, align 8, !tbaa !2
  %5 = getelementptr inbounds i8, ptr %3, i64 %4
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %5, ptr align 1 %0, i64 %1, i1 false)
  %6 = add i64 %4, %1
  store i64 %6, ptr @builder_length, align 8, !tbaa !2
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @reserve(i64 noundef %0) unnamed_addr #0 {
  %2 = load i64, ptr @builder_length, align 8, !tbaa !2
  %3 = add i64 %2, %0
  %4 = load i64, ptr @builder_capacity, align 8
  %5 = icmp ugt i64 %3, %4
  br i1 %5, label %6, label %18

6:                                                ; preds = %1
  %7 = icmp ult i64 %4, 256
  %8 = shl i64 %4, 1
  %9 = select i1 %7, i64 256, i64 %8
  br label %10

10:                                               ; preds = %10, %6
  %11 = phi i64 [ %9, %6 ], [ %13, %10 ]
  %12 = icmp ult i64 %11, %3
  %13 = shl i64 %11, 1
  br i1 %12, label %10, label %14, !llvm.loop !8

14:                                               ; preds = %10
  %15 = tail call ptr @fa_alloc(i64 noundef %11, i64 noundef 1) #16
  %16 = load ptr, ptr @builder, align 8, !tbaa !6
  %17 = load i64, ptr @builder_length, align 8, !tbaa !2
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %15, ptr align 1 %16, i64 %17, i1 false)
  store ptr %15, ptr @builder, align 8, !tbaa !6
  store i64 %11, ptr @builder_capacity, align 8, !tbaa !2
  br label %18

18:                                               ; preds = %1, %14
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_b_line(i32 noundef %0) local_unnamed_addr #0 {
  tail call fastcc void @reserve(i64 noundef 1) #17
  %2 = load ptr, ptr @builder, align 8, !tbaa !6
  %3 = load i64, ptr @builder_length, align 8, !tbaa !2
  %4 = getelementptr inbounds i8, ptr %2, i64 %3
  store i8 10, ptr %4, align 1, !tbaa !10
  %5 = add i64 %3, 1
  store i64 %5, ptr @builder_length, align 8, !tbaa !2
  tail call void @fa_out(i32 noundef %0, ptr noundef %2, i64 noundef %5) #17
  store i64 0, ptr @builder_length, align 8, !tbaa !2
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_b_take(ptr nocapture noundef writeonly %0) local_unnamed_addr #0 {
  %2 = load i64, ptr @builder_length, align 8
  %3 = tail call i64 @llvm.umax.i64(i64 %2, i64 1)
  %4 = tail call ptr @fa_alloc(i64 noundef %3, i64 noundef 1) #16
  %5 = load ptr, ptr @builder, align 8, !tbaa !6
  %6 = load i64, ptr @builder_length, align 8, !tbaa !2
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %4, ptr align 1 %5, i64 %6, i1 false)
  store ptr %4, ptr %0, align 8, !tbaa !11
  %7 = getelementptr inbounds %struct.FaString, ptr %0, i64 0, i32 1
  store i64 %6, ptr %7, align 8, !tbaa !13
  store i64 0, ptr @builder_length, align 8, !tbaa !2
  ret void
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.start.p0(i64 immarg, ptr nocapture) #5

; Function Attrs: optsize
declare ptr @fa_alloc(i64 noundef, i64 noundef) local_unnamed_addr #1

; Function Attrs: mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.end.p0(i64 immarg, ptr nocapture) #5

; Function Attrs: nounwind optsize
define void @fa_b_i64(i64 noundef %0) local_unnamed_addr #0 {
  %2 = tail call i64 @llvm.abs.i64(i64 %0, i1 false)
  %3 = lshr i64 %0, 63
  %4 = trunc i64 %3 to i32
  tail call fastcc void @append_u64(i64 noundef %2, i32 noundef %4) #17
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @append_u64(i64 noundef %0, i32 noundef %1) unnamed_addr #0 {
  %3 = alloca [24 x i8], align 16
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %3) #19
  br label %4

4:                                                ; preds = %4, %2
  %5 = phi i64 [ %0, %2 ], [ %12, %4 ]
  %6 = phi i64 [ 24, %2 ], [ %7, %4 ]
  %7 = add i64 %6, -1
  %8 = urem i64 %5, 10
  %9 = trunc i64 %8 to i8
  %10 = or disjoint i8 %9, 48
  %11 = getelementptr inbounds [24 x i8], ptr %3, i64 0, i64 %7
  store i8 %10, ptr %11, align 1, !tbaa !10
  %12 = udiv i64 %5, 10
  %13 = icmp ult i64 %5, 10
  br i1 %13, label %14, label %4, !llvm.loop !14

14:                                               ; preds = %4
  %15 = icmp eq i32 %1, 0
  br i1 %15, label %19, label %16

16:                                               ; preds = %14
  %17 = add i64 %6, -2
  %18 = getelementptr inbounds [24 x i8], ptr %3, i64 0, i64 %17
  store i8 45, ptr %18, align 1, !tbaa !10
  br label %19

19:                                               ; preds = %16, %14
  %20 = phi i64 [ %17, %16 ], [ %7, %14 ]
  %21 = getelementptr inbounds i8, ptr %3, i64 %20
  %22 = sub i64 24, %20
  tail call fastcc void @reserve(i64 noundef %22) #17
  %23 = load ptr, ptr @builder, align 8, !tbaa !6
  %24 = load i64, ptr @builder_length, align 8, !tbaa !2
  %25 = getelementptr inbounds i8, ptr %23, i64 %24
  call void @llvm.memcpy.p0.p0.i64(ptr align 1 %25, ptr nonnull align 1 %21, i64 %22, i1 false)
  %26 = add i64 %24, %22
  store i64 %26, ptr @builder_length, align 8, !tbaa !2
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %3) #19
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_b_u64(i64 noundef %0) local_unnamed_addr #0 {
  tail call fastcc void @append_u64(i64 noundef %0, i32 noundef 0) #17
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
  tail call fastcc void @append_u128(i64 noundef %12, i64 noundef %14, i32 noundef %9) #17
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @append_u128(i64 noundef %0, i64 noundef %1, i32 noundef %2) unnamed_addr #0 {
  %4 = alloca [4 x i32], align 16
  %5 = alloca [48 x i8], align 16
  %6 = icmp eq i64 %1, 0
  br i1 %6, label %7, label %8

7:                                                ; preds = %3
  tail call fastcc void @append_u64(i64 noundef %0, i32 noundef %2) #17
  br label %70

8:                                                ; preds = %3
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %4) #19
  %9 = lshr i64 %1, 32
  %10 = trunc i64 %9 to i32
  store i32 %10, ptr %4, align 16, !tbaa !15
  %11 = getelementptr inbounds i32, ptr %4, i64 1
  %12 = trunc i64 %1 to i32
  store i32 %12, ptr %11, align 4, !tbaa !15
  %13 = getelementptr inbounds i32, ptr %4, i64 2
  %14 = lshr i64 %0, 32
  %15 = trunc i64 %14 to i32
  store i32 %15, ptr %13, align 8, !tbaa !15
  %16 = getelementptr inbounds i32, ptr %4, i64 3
  %17 = trunc i64 %0 to i32
  store i32 %17, ptr %16, align 4, !tbaa !15
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %5) #19
  br label %18

18:                                               ; preds = %40, %8
  %19 = phi i64 [ 48, %8 ], [ %45, %40 ]
  br label %20

20:                                               ; preds = %20, %18
  %21 = phi i64 [ 0, %18 ], [ %31, %20 ]
  %22 = phi i64 [ 0, %18 ], [ %30, %20 ]
  %23 = shl nuw nsw i64 %22, 32
  %24 = getelementptr inbounds i32, ptr %4, i64 %21
  %25 = load i32, ptr %24, align 4, !tbaa !15
  %26 = zext i32 %25 to i64
  %27 = or disjoint i64 %23, %26
  %28 = udiv i64 %27, 1000000000
  %29 = trunc i64 %28 to i32
  store i32 %29, ptr %24, align 4, !tbaa !15
  %30 = urem i64 %27, 1000000000
  %31 = add nuw nsw i64 %21, 1
  %32 = icmp eq i64 %31, 4
  br i1 %32, label %33, label %20, !llvm.loop !17

33:                                               ; preds = %20
  %34 = trunc i64 %30 to i32
  %35 = load <4 x i32>, ptr %4, align 16, !tbaa !15
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
  store i8 %48, ptr %49, align 1, !tbaa !10
  %50 = udiv i32 %43, 10
  %51 = add nuw nsw i32 %42, 1
  %52 = icmp ult i32 %42, 8
  %53 = icmp ugt i32 %43, 9
  %54 = select i1 %37, i1 true, i1 %53
  %55 = select i1 %52, i1 %54, i1 false
  br i1 %55, label %41, label %40, !llvm.loop !18

56:                                               ; preds = %33, %40
  %57 = phi i64 [ %45, %40 ], [ %19, %33 ]
  %58 = icmp eq i32 %2, 0
  br i1 %58, label %62, label %59

59:                                               ; preds = %56
  %60 = add i64 %57, -1
  %61 = getelementptr inbounds [48 x i8], ptr %5, i64 0, i64 %60
  store i8 45, ptr %61, align 1, !tbaa !10
  br label %62

62:                                               ; preds = %59, %56
  %63 = phi i64 [ %60, %59 ], [ %57, %56 ]
  %64 = getelementptr inbounds i8, ptr %5, i64 %63
  %65 = sub i64 48, %63
  tail call fastcc void @reserve(i64 noundef %65) #17
  %66 = load ptr, ptr @builder, align 8, !tbaa !6
  %67 = load i64, ptr @builder_length, align 8, !tbaa !2
  %68 = getelementptr inbounds i8, ptr %66, i64 %67
  call void @llvm.memcpy.p0.p0.i64(ptr align 1 %68, ptr nonnull align 1 %64, i64 %65, i1 false)
  %69 = add i64 %67, %65
  store i64 %69, ptr @builder_length, align 8, !tbaa !2
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %5) #19
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %4) #19
  br label %70

70:                                               ; preds = %62, %7
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_b_u128(i64 noundef %0, i64 noundef %1) local_unnamed_addr #0 {
  tail call fastcc void @append_u128(i64 noundef %0, i64 noundef %1, i32 noundef 0) #17
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_b_bool(i32 noundef %0) local_unnamed_addr #0 {
  %2 = icmp eq i32 %0, 0
  %3 = select i1 %2, ptr @.str.1, ptr @.str
  br label %4

4:                                                ; preds = %4, %1
  %5 = phi i64 [ 0, %1 ], [ %9, %4 ]
  %6 = getelementptr inbounds i8, ptr %3, i64 %5
  %7 = load i8, ptr %6, align 1, !tbaa !10
  %8 = icmp eq i8 %7, 0
  %9 = add i64 %5, 1
  br i1 %8, label %10, label %4, !llvm.loop !19

10:                                               ; preds = %4
  tail call fastcc void @reserve(i64 noundef %5) #17
  %11 = load ptr, ptr @builder, align 8, !tbaa !6
  %12 = load i64, ptr @builder_length, align 8, !tbaa !2
  %13 = getelementptr inbounds i8, ptr %11, i64 %12
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %13, ptr nonnull align 1 %3, i64 %5, i1 false)
  %14 = add i64 %12, %5
  store i64 %14, ptr @builder_length, align 8, !tbaa !2
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @append_c(ptr nocapture noundef readonly %0) unnamed_addr #0 {
  br label %2

2:                                                ; preds = %2, %1
  %3 = phi i64 [ 0, %1 ], [ %7, %2 ]
  %4 = getelementptr inbounds i8, ptr %0, i64 %3
  %5 = load i8, ptr %4, align 1, !tbaa !10
  %6 = icmp eq i8 %5, 0
  %7 = add i64 %3, 1
  br i1 %6, label %8, label %2, !llvm.loop !19

8:                                                ; preds = %2
  tail call fastcc void @reserve(i64 noundef %3) #17
  %9 = load ptr, ptr @builder, align 8, !tbaa !6
  %10 = load i64, ptr @builder_length, align 8, !tbaa !2
  %11 = getelementptr inbounds i8, ptr %9, i64 %10
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %11, ptr nonnull align 1 %0, i64 %3, i1 false)
  %12 = add i64 %10, %3
  store i64 %12, ptr @builder_length, align 8, !tbaa !2
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_b_f64(double noundef %0) local_unnamed_addr #0 {
  tail call fastcc void @append_double(double noundef %0) #17
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @append_double(double noundef %0) unnamed_addr #0 {
  %2 = alloca [40 x i8], align 16
  %3 = alloca [24 x i8], align 16
  %4 = fcmp ord double %0, 0.000000e+00
  br i1 %4, label %10, label %5

5:                                                ; preds = %1
  tail call fastcc void @reserve(i64 noundef 3) #17
  %6 = load ptr, ptr @builder, align 8, !tbaa !6
  %7 = load i64, ptr @builder_length, align 8, !tbaa !2
  %8 = getelementptr inbounds i8, ptr %6, i64 %7
  tail call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(3) %8, ptr noundef nonnull align 1 dereferenceable(3) @.str.20, i64 3, i1 false)
  %9 = add i64 %7, 3
  store i64 %9, ptr @builder_length, align 8, !tbaa !2
  br label %188

10:                                               ; preds = %1
  %11 = fcmp oeq double %0, 0.000000e+00
  br i1 %11, label %12, label %17

12:                                               ; preds = %10
  tail call fastcc void @reserve(i64 noundef 1) #17
  %13 = load ptr, ptr @builder, align 8, !tbaa !6
  %14 = load i64, ptr @builder_length, align 8, !tbaa !2
  %15 = getelementptr inbounds i8, ptr %13, i64 %14
  store i8 48, ptr %15, align 1, !tbaa !10
  %16 = add i64 %14, 1
  store i64 %16, ptr @builder_length, align 8, !tbaa !2
  br label %188

17:                                               ; preds = %10
  %18 = fcmp olt double %0, 0.000000e+00
  br i1 %18, label %19, label %25

19:                                               ; preds = %17
  tail call fastcc void @reserve(i64 noundef 1) #17
  %20 = load ptr, ptr @builder, align 8, !tbaa !6
  %21 = load i64, ptr @builder_length, align 8, !tbaa !2
  %22 = getelementptr inbounds i8, ptr %20, i64 %21
  store i8 45, ptr %22, align 1, !tbaa !10
  %23 = add i64 %21, 1
  store i64 %23, ptr @builder_length, align 8, !tbaa !2
  %24 = fneg double %0
  br label %25

25:                                               ; preds = %19, %17
  %26 = phi double [ %24, %19 ], [ %0, %17 ]
  %27 = fcmp ogt double %26, 0x7FEFFFFFFFFFFFFF
  br i1 %27, label %28, label %33

28:                                               ; preds = %25
  tail call fastcc void @reserve(i64 noundef 8) #17
  %29 = load ptr, ptr @builder, align 8, !tbaa !6
  %30 = load i64, ptr @builder_length, align 8, !tbaa !2
  %31 = getelementptr inbounds i8, ptr %29, i64 %30
  store i64 8751735898823355977, ptr %31, align 1
  %32 = add i64 %30, 8
  store i64 %32, ptr @builder_length, align 8, !tbaa !2
  br label %188

33:                                               ; preds = %25
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %3) #19
  call void @llvm.lifetime.start.p0(i64 40, ptr nonnull %2) #19
  br label %34

34:                                               ; preds = %34, %33
  %35 = phi i32 [ 1, %33 ], [ %40, %34 ]
  %36 = add nsw i32 %35, -1
  %37 = call i32 (ptr, i64, ptr, ...) @snprintf(ptr noundef nonnull %2, i64 noundef 40, ptr noundef nonnull @.str.23, i32 noundef %36, double noundef %26) #16
  %38 = call double @strtod(ptr noundef nonnull %2, ptr noundef null) #16
  %39 = fcmp oeq double %38, %26
  %40 = add nuw nsw i32 %35, 1
  %41 = icmp eq i32 %40, 18
  %42 = select i1 %39, i1 true, i1 %41
  br i1 %42, label %43, label %34, !llvm.loop !20

43:                                               ; preds = %34
  %44 = load i8, ptr %2, align 16, !tbaa !10
  %45 = icmp eq i8 %44, 101
  br i1 %45, label %61, label %46

46:                                               ; preds = %43, %56
  %47 = phi i8 [ %59, %56 ], [ %44, %43 ]
  %48 = phi ptr [ %58, %56 ], [ %2, %43 ]
  %49 = phi i32 [ %57, %56 ], [ 0, %43 ]
  %50 = add i8 %47, -48
  %51 = icmp ult i8 %50, 10
  br i1 %51, label %52, label %56

52:                                               ; preds = %46
  %53 = sext i32 %49 to i64
  %54 = getelementptr inbounds i8, ptr %3, i64 %53
  store i8 %47, ptr %54, align 1, !tbaa !10
  %55 = add nsw i32 %49, 1
  br label %56

56:                                               ; preds = %52, %46
  %57 = phi i32 [ %55, %52 ], [ %49, %46 ]
  %58 = getelementptr inbounds i8, ptr %48, i64 1
  %59 = load i8, ptr %58, align 1, !tbaa !10
  %60 = icmp eq i8 %59, 101
  br i1 %60, label %61, label %46, !llvm.loop !21

61:                                               ; preds = %56, %43
  %62 = phi i32 [ 0, %43 ], [ %57, %56 ]
  %63 = phi ptr [ %2, %43 ], [ %58, %56 ]
  %64 = getelementptr inbounds i8, ptr %63, i64 1
  %65 = load i8, ptr %64, align 1, !tbaa !10
  %66 = getelementptr inbounds i8, ptr %63, i64 2
  %67 = load i8, ptr %66, align 1, !tbaa !10
  %68 = icmp eq i8 %67, 0
  br i1 %68, label %69, label %73

69:                                               ; preds = %73, %61
  %70 = phi i32 [ 0, %61 ], [ %80, %73 ]
  %71 = zext i32 %62 to i64
  %72 = call i32 @llvm.smin.i32(i32 %62, i32 1)
  br label %84

73:                                               ; preds = %61, %73
  %74 = phi i8 [ %82, %73 ], [ %67, %61 ]
  %75 = phi i32 [ %80, %73 ], [ 0, %61 ]
  %76 = phi ptr [ %81, %73 ], [ %66, %61 ]
  %77 = sext i8 %74 to i32
  %78 = mul nsw i32 %75, 10
  %79 = add nsw i32 %77, -48
  %80 = add i32 %79, %78
  %81 = getelementptr inbounds i8, ptr %76, i64 1
  %82 = load i8, ptr %81, align 1, !tbaa !10
  %83 = icmp eq i8 %82, 0
  br i1 %83, label %69, label %73, !llvm.loop !22

84:                                               ; preds = %88, %69
  %85 = phi i64 [ %71, %69 ], [ %89, %88 ]
  %86 = trunc i64 %85 to i32
  %87 = icmp sgt i32 %86, 1
  br i1 %87, label %88, label %93

88:                                               ; preds = %84
  %89 = add nsw i64 %85, -1
  %90 = getelementptr inbounds i8, ptr %3, i64 %89
  %91 = load i8, ptr %90, align 1, !tbaa !10
  %92 = icmp eq i8 %91, 48
  br i1 %92, label %84, label %93, !llvm.loop !23

93:                                               ; preds = %84, %88
  %94 = phi i32 [ %72, %84 ], [ %86, %88 ]
  %95 = icmp eq i8 %65, 45
  %96 = sub nsw i32 0, %70
  %97 = select i1 %95, i32 %96, i32 %70
  %98 = add nsw i32 %97, 1
  call void @llvm.lifetime.end.p0(i64 40, ptr nonnull %2) #19
  %99 = icmp sle i32 %94, %98
  %100 = icmp slt i32 %97, 21
  %101 = and i1 %100, %99
  br i1 %101, label %102, label %117

102:                                              ; preds = %93
  %103 = sext i32 %94 to i64
  call fastcc void @reserve(i64 noundef %103) #17
  %104 = load ptr, ptr @builder, align 8, !tbaa !6
  %105 = load i64, ptr @builder_length, align 8, !tbaa !2
  %106 = getelementptr inbounds i8, ptr %104, i64 %105
  call void @llvm.memcpy.p0.p0.i64(ptr align 1 %106, ptr nonnull align 16 %3, i64 %103, i1 false)
  %107 = add i64 %105, %103
  store i64 %107, ptr @builder_length, align 8, !tbaa !2
  %108 = icmp sgt i32 %94, %97
  br i1 %108, label %187, label %109

109:                                              ; preds = %102, %109
  %110 = phi i32 [ %115, %109 ], [ %94, %102 ]
  call fastcc void @reserve(i64 noundef 1) #17
  %111 = load ptr, ptr @builder, align 8, !tbaa !6
  %112 = load i64, ptr @builder_length, align 8, !tbaa !2
  %113 = getelementptr inbounds i8, ptr %111, i64 %112
  store i8 48, ptr %113, align 1, !tbaa !10
  %114 = add i64 %112, 1
  store i64 %114, ptr @builder_length, align 8, !tbaa !2
  %115 = add i32 %110, 1
  %116 = icmp eq i32 %110, %97
  br i1 %116, label %187, label %109, !llvm.loop !24

117:                                              ; preds = %93
  %118 = icmp ult i32 %97, 21
  br i1 %118, label %119, label %136

119:                                              ; preds = %117
  %120 = zext nneg i32 %98 to i64
  call fastcc void @reserve(i64 noundef %120) #17
  %121 = load ptr, ptr @builder, align 8, !tbaa !6
  %122 = load i64, ptr @builder_length, align 8, !tbaa !2
  %123 = getelementptr inbounds i8, ptr %121, i64 %122
  call void @llvm.memcpy.p0.p0.i64(ptr align 1 %123, ptr nonnull align 16 %3, i64 %120, i1 false)
  %124 = add i64 %122, %120
  store i64 %124, ptr @builder_length, align 8, !tbaa !2
  call fastcc void @reserve(i64 noundef 1) #17
  %125 = load ptr, ptr @builder, align 8, !tbaa !6
  %126 = load i64, ptr @builder_length, align 8, !tbaa !2
  %127 = getelementptr inbounds i8, ptr %125, i64 %126
  store i8 46, ptr %127, align 1, !tbaa !10
  %128 = add i64 %126, 1
  store i64 %128, ptr @builder_length, align 8, !tbaa !2
  %129 = getelementptr inbounds i8, ptr %3, i64 %120
  %130 = sub nsw i32 %94, %98
  %131 = sext i32 %130 to i64
  call fastcc void @reserve(i64 noundef %131) #17
  %132 = load ptr, ptr @builder, align 8, !tbaa !6
  %133 = load i64, ptr @builder_length, align 8, !tbaa !2
  %134 = getelementptr inbounds i8, ptr %132, i64 %133
  call void @llvm.memcpy.p0.p0.i64(ptr align 1 %134, ptr nonnull align 1 %129, i64 %131, i1 false)
  %135 = add i64 %133, %131
  store i64 %135, ptr @builder_length, align 8, !tbaa !2
  br label %187

136:                                              ; preds = %117
  %137 = icmp ugt i32 %97, -7
  br i1 %137, label %138, label %155

138:                                              ; preds = %136
  call fastcc void @append_c(ptr noundef nonnull @.str.22) #17
  %139 = icmp eq i32 %97, -1
  br i1 %139, label %140, label %146

140:                                              ; preds = %146, %138
  %141 = sext i32 %94 to i64
  call fastcc void @reserve(i64 noundef %141) #17
  %142 = load ptr, ptr @builder, align 8, !tbaa !6
  %143 = load i64, ptr @builder_length, align 8, !tbaa !2
  %144 = getelementptr inbounds i8, ptr %142, i64 %143
  call void @llvm.memcpy.p0.p0.i64(ptr align 1 %144, ptr nonnull align 16 %3, i64 %141, i1 false)
  %145 = add i64 %143, %141
  store i64 %145, ptr @builder_length, align 8, !tbaa !2
  br label %187

146:                                              ; preds = %138, %146
  %147 = phi i32 [ %152, %146 ], [ 0, %138 ]
  call fastcc void @reserve(i64 noundef 1) #17
  %148 = load ptr, ptr @builder, align 8, !tbaa !6
  %149 = load i64, ptr @builder_length, align 8, !tbaa !2
  %150 = getelementptr inbounds i8, ptr %148, i64 %149
  store i8 48, ptr %150, align 1, !tbaa !10
  %151 = add i64 %149, 1
  store i64 %151, ptr @builder_length, align 8, !tbaa !2
  %152 = add nuw nsw i32 %147, 1
  %153 = xor i32 %97, %152
  %154 = icmp eq i32 %153, -1
  br i1 %154, label %140, label %146, !llvm.loop !25

155:                                              ; preds = %136
  %156 = load i8, ptr %3, align 16, !tbaa !10
  call fastcc void @reserve(i64 noundef 1) #17
  %157 = load ptr, ptr @builder, align 8, !tbaa !6
  %158 = load i64, ptr @builder_length, align 8, !tbaa !2
  %159 = getelementptr inbounds i8, ptr %157, i64 %158
  store i8 %156, ptr %159, align 1, !tbaa !10
  %160 = add i64 %158, 1
  store i64 %160, ptr @builder_length, align 8, !tbaa !2
  %161 = icmp sgt i32 %94, 1
  br i1 %161, label %162, label %174

162:                                              ; preds = %155
  call fastcc void @reserve(i64 noundef 1) #17
  %163 = load ptr, ptr @builder, align 8, !tbaa !6
  %164 = load i64, ptr @builder_length, align 8, !tbaa !2
  %165 = getelementptr inbounds i8, ptr %163, i64 %164
  store i8 46, ptr %165, align 1, !tbaa !10
  %166 = add i64 %164, 1
  store i64 %166, ptr @builder_length, align 8, !tbaa !2
  %167 = getelementptr inbounds i8, ptr %3, i64 1
  %168 = add nsw i32 %94, -1
  %169 = zext nneg i32 %168 to i64
  call fastcc void @reserve(i64 noundef %169) #17
  %170 = load ptr, ptr @builder, align 8, !tbaa !6
  %171 = load i64, ptr @builder_length, align 8, !tbaa !2
  %172 = getelementptr inbounds i8, ptr %170, i64 %171
  call void @llvm.memcpy.p0.p0.i64(ptr align 1 %172, ptr nonnull align 1 %167, i64 %169, i1 false)
  %173 = add i64 %171, %169
  store i64 %173, ptr @builder_length, align 8, !tbaa !2
  br label %174

174:                                              ; preds = %162, %155
  call fastcc void @reserve(i64 noundef 1) #17
  %175 = load ptr, ptr @builder, align 8, !tbaa !6
  %176 = load i64, ptr @builder_length, align 8, !tbaa !2
  %177 = getelementptr inbounds i8, ptr %175, i64 %176
  store i8 101, ptr %177, align 1, !tbaa !10
  %178 = add i64 %176, 1
  store i64 %178, ptr @builder_length, align 8, !tbaa !2
  %179 = icmp slt i32 %97, 0
  %180 = select i1 %179, i8 45, i8 43
  call fastcc void @reserve(i64 noundef 1) #17
  %181 = load ptr, ptr @builder, align 8, !tbaa !6
  %182 = load i64, ptr @builder_length, align 8, !tbaa !2
  %183 = getelementptr inbounds i8, ptr %181, i64 %182
  store i8 %180, ptr %183, align 1, !tbaa !10
  %184 = add i64 %182, 1
  store i64 %184, ptr @builder_length, align 8, !tbaa !2
  %185 = call i32 @llvm.abs.i32(i32 %70, i1 true)
  %186 = zext nneg i32 %185 to i64
  call fastcc void @append_u64(i64 noundef %186, i32 noundef 0) #17
  br label %187

187:                                              ; preds = %109, %102, %119, %174, %140
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %3) #19
  br label %188

188:                                              ; preds = %187, %28, %12, %5
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
  tail call fastcc void @append_double(double noundef %9) #17
  br label %107

11:                                               ; preds = %1
  call void @llvm.lifetime.start.p0(i64 40, ptr nonnull %3) #19
  %12 = getelementptr inbounds i64, ptr %4, i64 1
  %13 = getelementptr inbounds i64, ptr %4, i64 2
  %14 = insertelement <2 x double> poison, double %9, i64 0
  %15 = shufflevector <2 x double> %14, <2 x double> poison, <2 x i32> zeroinitializer
  br label %16

16:                                               ; preds = %102, %11
  %17 = phi i32 [ 1, %11 ], [ %103, %102 ]
  %18 = add nsw i32 %17, -1
  %19 = call i32 (ptr, i64, ptr, ...) @snprintf(ptr noundef nonnull %3, i64 noundef 40, ptr noundef nonnull @.str.23, i32 noundef %18, double noundef %9) #16
  %20 = load i8, ptr %3, align 16, !tbaa !10
  %21 = icmp eq i8 %20, 45
  %22 = zext i1 %21 to i64
  %23 = getelementptr inbounds i8, ptr %3, i64 %22
  %24 = load i8, ptr %23, align 1, !tbaa !10
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
  %37 = load i8, ptr %36, align 1, !tbaa !10
  %38 = icmp eq i8 %37, 101
  br i1 %38, label %39, label %26, !llvm.loop !26

39:                                               ; preds = %26, %16
  %40 = phi ptr [ %23, %16 ], [ %36, %26 ]
  %41 = phi i64 [ 0, %16 ], [ %35, %26 ]
  %42 = getelementptr inbounds i8, ptr %40, i64 1
  %43 = load i8, ptr %42, align 1, !tbaa !10
  %44 = icmp eq i8 %43, 45
  %45 = getelementptr inbounds i8, ptr %40, i64 2
  %46 = load i8, ptr %45, align 1, !tbaa !10
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
  %57 = load i8, ptr %56, align 1, !tbaa !10
  %58 = icmp eq i8 %57, 0
  br i1 %58, label %59, label %48, !llvm.loop !27

59:                                               ; preds = %48, %39
  %60 = phi i32 [ 0, %39 ], [ %55, %48 ]
  %61 = sub nsw i32 0, %60
  %62 = select i1 %44, i32 %61, i32 %60
  %63 = sub nsw i32 %62, %18
  %64 = sub nsw i64 0, %41
  %65 = select i1 %21, i64 %64, i64 %41
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %4) #19
  store i64 %65, ptr %4, align 16, !tbaa !2
  %66 = add nsw i64 %65, -1
  store i64 %66, ptr %12, align 8, !tbaa !2
  %67 = add nsw i64 %65, 1
  store i64 %67, ptr %13, align 16, !tbaa !2
  br label %71

68:                                               ; preds = %94
  %69 = add nuw nsw i64 %72, 1
  %70 = icmp eq i64 %69, 3
  br i1 %70, label %100, label %71, !llvm.loop !28

71:                                               ; preds = %68, %59
  %72 = phi i64 [ 0, %59 ], [ %69, %68 ]
  %73 = phi double [ 0.000000e+00, %59 ], [ %96, %68 ]
  %74 = phi i32 [ 0, %59 ], [ %95, %68 ]
  %75 = getelementptr inbounds [3 x i64], ptr %4, i64 0, i64 %72
  %76 = load i64, ptr %75, align 8, !tbaa !2
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %2) #19
  %77 = call i32 (ptr, i64, ptr, ...) @snprintf(ptr noundef nonnull %2, i64 noundef 48, ptr noundef nonnull @.str.24, i64 noundef %76, i32 noundef %63) #16
  %78 = call double @strtod(ptr noundef nonnull %2, ptr noundef null) #16
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %2) #19
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
  call fastcc void @append_double(double noundef %96) #17
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %4) #19
  br label %106

102:                                              ; preds = %100
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %4) #19
  %103 = add nuw nsw i32 %17, 1
  %104 = icmp eq i32 %103, 10
  br i1 %104, label %105, label %16, !llvm.loop !29

105:                                              ; preds = %102
  call fastcc void @append_double(double noundef %9) #17
  br label %106

106:                                              ; preds = %105, %101
  call void @llvm.lifetime.end.p0(i64 40, ptr nonnull %3) #19
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
  tail call fastcc void @append_double(double noundef %3) #17
  br label %12

9:                                                ; preds = %1
  call void @llvm.lifetime.start.p0(i64 40, ptr nonnull %2) #19
  %10 = call i32 (ptr, i64, ptr, ...) @snprintf(ptr noundef nonnull %2, i64 noundef 40, ptr noundef nonnull @.str.25, double noundef %3) #16
  %11 = call double @strtod(ptr noundef nonnull %2, ptr noundef null) #16
  call fastcc void @append_double(double noundef %11) #17
  call void @llvm.lifetime.end.p0(i64 40, ptr nonnull %2) #19
  br label %12

12:                                               ; preds = %8, %9
  call fastcc void @reserve(i64 noundef 1) #17
  %13 = load ptr, ptr @builder, align 8, !tbaa !6
  %14 = load i64, ptr @builder_length, align 8, !tbaa !2
  %15 = getelementptr inbounds i8, ptr %13, i64 %14
  store i8 37, ptr %15, align 1, !tbaa !10
  %16 = add i64 %14, 1
  store i64 %16, ptr @builder_length, align 8, !tbaa !2
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_b_quoted(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  tail call fastcc void @reserve(i64 noundef 1) #17
  %3 = load ptr, ptr @builder, align 8, !tbaa !6
  %4 = load i64, ptr @builder_length, align 8, !tbaa !2
  %5 = getelementptr inbounds i8, ptr %3, i64 %4
  store i8 34, ptr %5, align 1, !tbaa !10
  %6 = add i64 %4, 1
  store i64 %6, ptr @builder_length, align 8, !tbaa !2
  %7 = icmp eq i64 %1, 0
  br i1 %7, label %8, label %13

8:                                                ; preds = %88, %2
  tail call fastcc void @reserve(i64 noundef 1) #17
  %9 = load ptr, ptr @builder, align 8, !tbaa !6
  %10 = load i64, ptr @builder_length, align 8, !tbaa !2
  %11 = getelementptr inbounds i8, ptr %9, i64 %10
  store i8 34, ptr %11, align 1, !tbaa !10
  %12 = add i64 %10, 1
  store i64 %12, ptr @builder_length, align 8, !tbaa !2
  ret void

13:                                               ; preds = %2, %88
  %14 = phi i64 [ %90, %88 ], [ 0, %2 ]
  %15 = getelementptr inbounds i8, ptr %0, i64 %14
  %16 = load i8, ptr %15, align 1, !tbaa !10
  %17 = zext i8 %16 to i32
  switch i8 %16, label %58 [
    i8 92, label %53
    i8 34, label %48
    i8 10, label %43
    i8 13, label %38
    i8 9, label %33
    i8 0, label %28
    i8 123, label %23
    i8 125, label %18
  ]

18:                                               ; preds = %13
  tail call fastcc void @reserve(i64 noundef 2) #17
  %19 = load ptr, ptr @builder, align 8, !tbaa !6
  %20 = load i64, ptr @builder_length, align 8, !tbaa !2
  %21 = getelementptr inbounds i8, ptr %19, i64 %20
  store i16 32125, ptr %21, align 1
  %22 = add i64 %20, 2
  br label %88

23:                                               ; preds = %13
  tail call fastcc void @reserve(i64 noundef 2) #17
  %24 = load ptr, ptr @builder, align 8, !tbaa !6
  %25 = load i64, ptr @builder_length, align 8, !tbaa !2
  %26 = getelementptr inbounds i8, ptr %24, i64 %25
  store i16 31611, ptr %26, align 1
  %27 = add i64 %25, 2
  br label %88

28:                                               ; preds = %13
  tail call fastcc void @reserve(i64 noundef 2) #17
  %29 = load ptr, ptr @builder, align 8, !tbaa !6
  %30 = load i64, ptr @builder_length, align 8, !tbaa !2
  %31 = getelementptr inbounds i8, ptr %29, i64 %30
  store i16 12380, ptr %31, align 1
  %32 = add i64 %30, 2
  br label %88

33:                                               ; preds = %13
  tail call fastcc void @reserve(i64 noundef 2) #17
  %34 = load ptr, ptr @builder, align 8, !tbaa !6
  %35 = load i64, ptr @builder_length, align 8, !tbaa !2
  %36 = getelementptr inbounds i8, ptr %34, i64 %35
  store i16 29788, ptr %36, align 1
  %37 = add i64 %35, 2
  br label %88

38:                                               ; preds = %13
  tail call fastcc void @reserve(i64 noundef 2) #17
  %39 = load ptr, ptr @builder, align 8, !tbaa !6
  %40 = load i64, ptr @builder_length, align 8, !tbaa !2
  %41 = getelementptr inbounds i8, ptr %39, i64 %40
  store i16 29276, ptr %41, align 1
  %42 = add i64 %40, 2
  br label %88

43:                                               ; preds = %13
  tail call fastcc void @reserve(i64 noundef 2) #17
  %44 = load ptr, ptr @builder, align 8, !tbaa !6
  %45 = load i64, ptr @builder_length, align 8, !tbaa !2
  %46 = getelementptr inbounds i8, ptr %44, i64 %45
  store i16 28252, ptr %46, align 1
  %47 = add i64 %45, 2
  br label %88

48:                                               ; preds = %13
  tail call fastcc void @reserve(i64 noundef 2) #17
  %49 = load ptr, ptr @builder, align 8, !tbaa !6
  %50 = load i64, ptr @builder_length, align 8, !tbaa !2
  %51 = getelementptr inbounds i8, ptr %49, i64 %50
  store i16 8796, ptr %51, align 1
  %52 = add i64 %50, 2
  br label %88

53:                                               ; preds = %13
  tail call fastcc void @reserve(i64 noundef 2) #17
  %54 = load ptr, ptr @builder, align 8, !tbaa !6
  %55 = load i64, ptr @builder_length, align 8, !tbaa !2
  %56 = getelementptr inbounds i8, ptr %54, i64 %55
  store i16 23644, ptr %56, align 1
  %57 = add i64 %55, 2
  br label %88

58:                                               ; preds = %13
  %59 = icmp ult i8 %16, 32
  %60 = icmp eq i8 %16, 127
  %61 = or i1 %59, %60
  br i1 %61, label %62, label %83

62:                                               ; preds = %58
  tail call fastcc void @reserve(i64 noundef 2) #17
  %63 = load ptr, ptr @builder, align 8, !tbaa !6
  %64 = load i64, ptr @builder_length, align 8, !tbaa !2
  %65 = getelementptr inbounds i8, ptr %63, i64 %64
  store i16 30812, ptr %65, align 1
  %66 = add i64 %64, 2
  store i64 %66, ptr @builder_length, align 8, !tbaa !2
  %67 = lshr i32 %17, 4
  %68 = zext nneg i32 %67 to i64
  %69 = getelementptr inbounds [17 x i8], ptr @fa_b_quoted.hex, i64 0, i64 %68
  %70 = load i8, ptr %69, align 1, !tbaa !10
  tail call fastcc void @reserve(i64 noundef 1) #17
  %71 = load ptr, ptr @builder, align 8, !tbaa !6
  %72 = load i64, ptr @builder_length, align 8, !tbaa !2
  %73 = getelementptr inbounds i8, ptr %71, i64 %72
  store i8 %70, ptr %73, align 1, !tbaa !10
  %74 = add i64 %72, 1
  store i64 %74, ptr @builder_length, align 8, !tbaa !2
  %75 = and i32 %17, 15
  %76 = zext nneg i32 %75 to i64
  %77 = getelementptr inbounds [17 x i8], ptr @fa_b_quoted.hex, i64 0, i64 %76
  %78 = load i8, ptr %77, align 1, !tbaa !10
  tail call fastcc void @reserve(i64 noundef 1) #17
  %79 = load ptr, ptr @builder, align 8, !tbaa !6
  %80 = load i64, ptr @builder_length, align 8, !tbaa !2
  %81 = getelementptr inbounds i8, ptr %79, i64 %80
  store i8 %78, ptr %81, align 1, !tbaa !10
  %82 = add i64 %80, 1
  br label %88

83:                                               ; preds = %58
  tail call fastcc void @reserve(i64 noundef 1) #17
  %84 = load ptr, ptr @builder, align 8, !tbaa !6
  %85 = load i64, ptr @builder_length, align 8, !tbaa !2
  %86 = getelementptr inbounds i8, ptr %84, i64 %85
  store i8 %16, ptr %86, align 1, !tbaa !10
  %87 = add i64 %85, 1
  br label %88

88:                                               ; preds = %62, %83, %18, %23, %28, %33, %38, %43, %48, %53
  %89 = phi i64 [ %82, %62 ], [ %87, %83 ], [ %22, %18 ], [ %27, %23 ], [ %32, %28 ], [ %37, %33 ], [ %42, %38 ], [ %47, %43 ], [ %52, %48 ], [ %57, %53 ]
  store i64 %89, ptr @builder_length, align 8, !tbaa !2
  %90 = add nuw i64 %14, 1
  %91 = icmp eq i64 %90, %1
  br i1 %91, label %8, label %13, !llvm.loop !30
}

; Function Attrs: nounwind optsize
define void @fa_b_string_array(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  tail call fastcc void @reserve(i64 noundef 1) #17
  %3 = load ptr, ptr @builder, align 8, !tbaa !6
  %4 = load i64, ptr @builder_length, align 8, !tbaa !2
  %5 = getelementptr inbounds i8, ptr %3, i64 %4
  store i8 91, ptr %5, align 1, !tbaa !10
  %6 = add i64 %4, 1
  store i64 %6, ptr @builder_length, align 8, !tbaa !2
  %7 = icmp eq i64 %1, 0
  br i1 %7, label %8, label %13

8:                                                ; preds = %21, %2
  tail call fastcc void @reserve(i64 noundef 1) #17
  %9 = load ptr, ptr @builder, align 8, !tbaa !6
  %10 = load i64, ptr @builder_length, align 8, !tbaa !2
  %11 = getelementptr inbounds i8, ptr %9, i64 %10
  store i8 93, ptr %11, align 1, !tbaa !10
  %12 = add i64 %10, 1
  store i64 %12, ptr @builder_length, align 8, !tbaa !2
  ret void

13:                                               ; preds = %2, %21
  %14 = phi i64 [ %26, %21 ], [ 0, %2 ]
  %15 = icmp eq i64 %14, 0
  br i1 %15, label %21, label %16

16:                                               ; preds = %13
  tail call fastcc void @reserve(i64 noundef 2) #17
  %17 = load ptr, ptr @builder, align 8, !tbaa !6
  %18 = load i64, ptr @builder_length, align 8, !tbaa !2
  %19 = getelementptr inbounds i8, ptr %17, i64 %18
  store i16 8236, ptr %19, align 1
  %20 = add i64 %18, 2
  store i64 %20, ptr @builder_length, align 8, !tbaa !2
  br label %21

21:                                               ; preds = %16, %13
  %22 = getelementptr inbounds %struct.FaString, ptr %0, i64 %14
  %23 = load ptr, ptr %22, align 8, !tbaa !11
  %24 = getelementptr inbounds %struct.FaString, ptr %0, i64 %14, i32 1
  %25 = load i64, ptr %24, align 8, !tbaa !13
  tail call void @fa_b_quoted(ptr noundef %23, i64 noundef %25) #17
  %26 = add nuw i64 %14, 1
  %27 = icmp eq i64 %26, %1
  br i1 %27, label %8, label %13, !llvm.loop !31
}

; Function Attrs: nounwind optsize
define void @fa_concat(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef readonly %2, i64 noundef %3, ptr nocapture noundef writeonly %4) local_unnamed_addr #0 {
  %6 = add i64 %3, %1
  %7 = tail call i64 @llvm.umax.i64(i64 %6, i64 1)
  %8 = tail call ptr @fa_alloc(i64 noundef %7, i64 noundef 1) #16
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %8, ptr align 1 %0, i64 %1, i1 false)
  %9 = getelementptr inbounds i8, ptr %8, i64 %1
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %9, ptr align 1 %2, i64 %3, i1 false)
  store ptr %8, ptr %4, align 8, !tbaa !11
  %10 = getelementptr inbounds %struct.FaString, ptr %4, i64 0, i32 1
  store i64 %6, ptr %10, align 8, !tbaa !13
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: read)
define i32 @fa_string_equal(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef readonly %2, i64 noundef %3) local_unnamed_addr #6 {
  %5 = icmp eq i64 %1, %3
  br i1 %5, label %6, label %25

6:                                                ; preds = %4
  %7 = icmp eq i64 %1, 0
  br i1 %7, label %25, label %8

8:                                                ; preds = %6
  %9 = load i8, ptr %0, align 1, !tbaa !10
  %10 = load i8, ptr %2, align 1, !tbaa !10
  %11 = icmp eq i8 %9, %10
  br i1 %11, label %12, label %25

12:                                               ; preds = %8, %16
  %13 = phi i64 [ %14, %16 ], [ 0, %8 ]
  %14 = add nuw i64 %13, 1
  %15 = icmp eq i64 %14, %1
  br i1 %15, label %22, label %16, !llvm.loop !32

16:                                               ; preds = %12
  %17 = getelementptr inbounds i8, ptr %0, i64 %14
  %18 = load i8, ptr %17, align 1, !tbaa !10
  %19 = getelementptr inbounds i8, ptr %2, i64 %14
  %20 = load i8, ptr %19, align 1, !tbaa !10
  %21 = icmp eq i8 %18, %20
  br i1 %21, label %12, label %22, !llvm.loop !32

22:                                               ; preds = %16, %12
  %23 = icmp uge i64 %14, %1
  %24 = zext i1 %23 to i32
  br label %25

25:                                               ; preds = %8, %22, %6, %4
  %26 = phi i32 [ 0, %4 ], [ 1, %6 ], [ 0, %8 ], [ %24, %22 ]
  ret i32 %26
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: read)
define i32 @fa_string_compare(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef readonly %2, i64 noundef %3) local_unnamed_addr #6 {
  %5 = tail call i64 @llvm.umin.i64(i64 %1, i64 %3)
  %6 = icmp eq i64 %5, 0
  br i1 %6, label %11, label %7

7:                                                ; preds = %4
  %8 = load i8, ptr %0, align 1, !tbaa !10
  %9 = load i8, ptr %2, align 1, !tbaa !10
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
  br i1 %19, label %39, label %20, !llvm.loop !33

20:                                               ; preds = %16
  %21 = getelementptr inbounds i8, ptr %0, i64 %18
  %22 = load i8, ptr %21, align 1, !tbaa !10
  %23 = getelementptr inbounds i8, ptr %2, i64 %18
  %24 = load i8, ptr %23, align 1, !tbaa !10
  %25 = icmp eq i8 %22, %24
  br i1 %25, label %16, label %26, !llvm.loop !33

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
define i64 @fa_next_grapheme(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #7 {
  %4 = tail call fastcc i64 @next_boundary(ptr noundef %0, i64 noundef %1, i64 noundef %2) #17
  ret i64 %4
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define internal fastcc i64 @next_boundary(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2) unnamed_addr #7 {
  %4 = alloca i64, align 8
  %5 = alloca i64, align 8
  store i64 %2, ptr %4, align 8, !tbaa !2
  %6 = getelementptr inbounds i8, ptr %0, i64 %2
  %7 = load i8, ptr %6, align 1, !tbaa !10
  %8 = icmp sgt i8 %7, -1
  br i1 %8, label %9, label %24

9:                                                ; preds = %3
  %10 = add i64 %2, 1
  %11 = icmp ult i64 %10, %1
  br i1 %11, label %12, label %21

12:                                               ; preds = %9
  %13 = getelementptr inbounds i8, ptr %0, i64 %10
  %14 = load i8, ptr %13, align 1, !tbaa !10
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
  %25 = call fastcc i32 @decode(ptr noundef nonnull %0, i64 noundef %1, ptr noundef nonnull %4) #17
  %26 = tail call fastcc i32 @break_kind(i32 noundef %25) #17, !range !34
  %27 = load i64, ptr %4, align 8, !tbaa !2
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
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %5) #19
  store i64 %38, ptr %5, align 8, !tbaa !2
  %39 = call fastcc i32 @decode(ptr noundef %0, i64 noundef %1, ptr noundef nonnull %5) #17
  %40 = tail call fastcc i32 @break_kind(i32 noundef %39) #17, !range !34
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
  %75 = load i64, ptr %5, align 8, !tbaa !2
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
  %84 = load i64, ptr %5, align 8, !tbaa !2
  %85 = add nuw nsw i32 %36, 1
  br label %94

86:                                               ; preds = %53, %52, %52, %52, %52, %34
  %87 = load i64, ptr %5, align 8, !tbaa !2
  br label %94

88:                                               ; preds = %59
  %89 = load i64, ptr %5, align 8, !tbaa !2
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
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %5) #19
  br label %106

101:                                              ; preds = %74, %94, %88
  %102 = phi i64 [ %89, %88 ], [ %95, %94 ], [ %75, %74 ]
  %103 = phi i32 [ %92, %88 ], [ %96, %94 ], [ 0, %74 ]
  %104 = phi i32 [ 1, %88 ], [ %99, %94 ], [ 1, %74 ]
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %5) #19
  %105 = icmp ult i64 %102, %1
  br i1 %105, label %34, label %106

106:                                              ; preds = %101, %24, %100, %21
  %107 = phi i64 [ %23, %21 ], [ %38, %100 ], [ %27, %24 ], [ %102, %101 ]
  ret i64 %107
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define i64 @fa_graphemes(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #7 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i64 [ 0, %2 ], [ %9, %6 ]
  ret i64 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %10, %6 ], [ 0, %2 ]
  %8 = phi i64 [ %9, %6 ], [ 0, %2 ]
  %9 = add i64 %8, 1
  %10 = tail call fastcc i64 @next_boundary(ptr noundef %0, i64 noundef %1, i64 noundef %7) #17
  %11 = icmp ult i64 %10, %1
  br i1 %11, label %6, label %4, !llvm.loop !35
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define i32 @fa_string_includes(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef readonly %2, i64 noundef %3) local_unnamed_addr #7 {
  %5 = tail call fastcc i64 @find(ptr noundef %0, i64 noundef %1, ptr noundef %2, i64 noundef %3, i32 noundef 0) #17
  %6 = icmp sgt i64 %5, -1
  %7 = zext i1 %6 to i32
  ret i32 %7
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define internal fastcc i64 @find(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef readonly %2, i64 noundef %3, i32 noundef %4) unnamed_addr #7 {
  %6 = icmp ugt i64 %3, %1
  br i1 %6, label %56, label %7

7:                                                ; preds = %5
  %8 = icmp eq i64 %3, 0
  %9 = icmp ne i32 %4, 0
  br label %10

10:                                               ; preds = %7, %51
  %11 = phi i64 [ %3, %7 ], [ %54, %51 ]
  %12 = phi i64 [ 0, %7 ], [ %53, %51 ]
  %13 = phi i64 [ -1, %7 ], [ %52, %51 ]
  %14 = getelementptr i8, ptr %0, i64 %12
  br i1 %8, label %31, label %15

15:                                               ; preds = %10
  %16 = load i8, ptr %14, align 1, !tbaa !10
  %17 = load i8, ptr %2, align 1, !tbaa !10
  %18 = icmp eq i8 %16, %17
  br i1 %18, label %19, label %51

19:                                               ; preds = %15, %23
  %20 = phi i64 [ %21, %23 ], [ 0, %15 ]
  %21 = add nuw i64 %20, 1
  %22 = icmp eq i64 %21, %3
  br i1 %22, label %29, label %23, !llvm.loop !36

23:                                               ; preds = %19
  %24 = getelementptr i8, ptr %14, i64 %21
  %25 = load i8, ptr %24, align 1, !tbaa !10
  %26 = getelementptr inbounds i8, ptr %2, i64 %21
  %27 = load i8, ptr %26, align 1, !tbaa !10
  %28 = icmp eq i8 %25, %27
  br i1 %28, label %19, label %29, !llvm.loop !36

29:                                               ; preds = %23, %19
  %30 = icmp ult i64 %21, %3
  br i1 %30, label %51, label %31

31:                                               ; preds = %10, %29
  %32 = icmp eq i64 %12, 0
  br i1 %32, label %37, label %33

33:                                               ; preds = %31, %33
  %34 = phi i64 [ %35, %33 ], [ 0, %31 ]
  %35 = tail call fastcc i64 @next_boundary(ptr noundef %0, i64 noundef %1, i64 noundef %34) #17
  %36 = icmp ult i64 %35, %12
  br i1 %36, label %33, label %37, !llvm.loop !37

37:                                               ; preds = %33, %31
  %38 = phi i64 [ 0, %31 ], [ %35, %33 ]
  %39 = icmp eq i64 %38, %12
  br i1 %39, label %40, label %51

40:                                               ; preds = %37
  %41 = icmp eq i64 %11, 0
  br i1 %41, label %46, label %42

42:                                               ; preds = %40, %42
  %43 = phi i64 [ %44, %42 ], [ 0, %40 ]
  %44 = tail call fastcc i64 @next_boundary(ptr noundef %0, i64 noundef %1, i64 noundef %43) #17
  %45 = icmp ult i64 %44, %11
  br i1 %45, label %42, label %46, !llvm.loop !37

46:                                               ; preds = %42, %40
  %47 = phi i64 [ 0, %40 ], [ %44, %42 ]
  %48 = icmp ne i64 %47, %11
  %49 = or i1 %48, %9
  %50 = select i1 %48, i64 %13, i64 %12
  br i1 %49, label %51, label %56

51:                                               ; preds = %15, %46, %29, %37
  %52 = phi i64 [ %50, %46 ], [ %13, %37 ], [ %13, %29 ], [ %13, %15 ]
  %53 = add i64 %12, 1
  %54 = add i64 %53, %3
  %55 = icmp ugt i64 %54, %1
  br i1 %55, label %56, label %10, !llvm.loop !38

56:                                               ; preds = %51, %46, %5
  %57 = phi i64 [ -1, %5 ], [ %12, %46 ], [ %52, %51 ]
  ret i64 %57
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define i32 @fa_string_starts_with(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef readonly %2, i64 noundef %3) local_unnamed_addr #7 {
  %5 = icmp ugt i64 %3, %1
  br i1 %5, label %32, label %6

6:                                                ; preds = %4
  %7 = icmp eq i64 %3, 0
  br i1 %7, label %28, label %8

8:                                                ; preds = %6
  %9 = load i8, ptr %0, align 1, !tbaa !10
  %10 = load i8, ptr %2, align 1, !tbaa !10
  %11 = icmp eq i8 %9, %10
  br i1 %11, label %12, label %32

12:                                               ; preds = %8, %16
  %13 = phi i64 [ %14, %16 ], [ 0, %8 ]
  %14 = add nuw i64 %13, 1
  %15 = icmp eq i64 %14, %3
  br i1 %15, label %22, label %16, !llvm.loop !36

16:                                               ; preds = %12
  %17 = getelementptr i8, ptr %0, i64 %14
  %18 = load i8, ptr %17, align 1, !tbaa !10
  %19 = getelementptr inbounds i8, ptr %2, i64 %14
  %20 = load i8, ptr %19, align 1, !tbaa !10
  %21 = icmp eq i8 %18, %20
  br i1 %21, label %12, label %22, !llvm.loop !36

22:                                               ; preds = %16, %12
  %23 = icmp ult i64 %14, %3
  br i1 %23, label %32, label %24

24:                                               ; preds = %22, %24
  %25 = phi i64 [ %26, %24 ], [ 0, %22 ]
  %26 = tail call fastcc i64 @next_boundary(ptr noundef nonnull %0, i64 noundef %1, i64 noundef %25) #17
  %27 = icmp ult i64 %26, %3
  br i1 %27, label %24, label %28, !llvm.loop !37

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
define i32 @fa_string_ends_with(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef readonly %2, i64 noundef %3) local_unnamed_addr #7 {
  %5 = icmp ugt i64 %3, %1
  br i1 %5, label %36, label %6

6:                                                ; preds = %4
  %7 = sub i64 %1, %3
  %8 = getelementptr i8, ptr %0, i64 %7
  %9 = icmp eq i64 %3, 0
  br i1 %9, label %26, label %10

10:                                               ; preds = %6
  %11 = load i8, ptr %8, align 1, !tbaa !10
  %12 = load i8, ptr %2, align 1, !tbaa !10
  %13 = icmp eq i8 %11, %12
  br i1 %13, label %14, label %36

14:                                               ; preds = %10, %18
  %15 = phi i64 [ %16, %18 ], [ 0, %10 ]
  %16 = add nuw i64 %15, 1
  %17 = icmp eq i64 %16, %3
  br i1 %17, label %24, label %18, !llvm.loop !36

18:                                               ; preds = %14
  %19 = getelementptr i8, ptr %8, i64 %16
  %20 = load i8, ptr %19, align 1, !tbaa !10
  %21 = getelementptr inbounds i8, ptr %2, i64 %16
  %22 = load i8, ptr %21, align 1, !tbaa !10
  %23 = icmp eq i8 %20, %22
  br i1 %23, label %14, label %24, !llvm.loop !36

24:                                               ; preds = %18, %14
  %25 = icmp ult i64 %16, %3
  br i1 %25, label %36, label %26

26:                                               ; preds = %6, %24
  %27 = icmp eq i64 %1, %3
  br i1 %27, label %32, label %28

28:                                               ; preds = %26, %28
  %29 = phi i64 [ %30, %28 ], [ 0, %26 ]
  %30 = tail call fastcc i64 @next_boundary(ptr noundef %0, i64 noundef %1, i64 noundef %29) #17
  %31 = icmp ult i64 %30, %7
  br i1 %31, label %28, label %32, !llvm.loop !37

32:                                               ; preds = %28, %26
  %33 = phi i64 [ 0, %26 ], [ %30, %28 ]
  %34 = icmp eq i64 %33, %7
  %35 = zext i1 %34 to i32
  br label %36

36:                                               ; preds = %10, %32, %24, %4
  %37 = phi i32 [ 0, %24 ], [ 0, %4 ], [ %35, %32 ], [ 0, %10 ]
  ret i32 %37
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define i64 @fa_string_index_of(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef readonly %2, i64 noundef %3, i32 noundef %4) local_unnamed_addr #7 {
  %6 = tail call fastcc i64 @find(ptr noundef %0, i64 noundef %1, ptr noundef %2, i64 noundef %3, i32 noundef %4) #17
  %7 = icmp slt i64 %6, 0
  br i1 %7, label %16, label %8

8:                                                ; preds = %5
  %9 = icmp eq i64 %6, 0
  br i1 %9, label %16, label %10

10:                                               ; preds = %8, %10
  %11 = phi i64 [ %14, %10 ], [ 0, %8 ]
  %12 = phi i64 [ %13, %10 ], [ 0, %8 ]
  %13 = add i64 %12, 1
  %14 = tail call fastcc i64 @next_boundary(ptr noundef %0, i64 noundef %1, i64 noundef %11) #17
  %15 = icmp ult i64 %14, %6
  br i1 %15, label %10, label %16, !llvm.loop !39

16:                                               ; preds = %10, %8, %5
  %17 = phi i64 [ -1, %5 ], [ 0, %8 ], [ %13, %10 ]
  ret i64 %17
}

; Function Attrs: nounwind optsize
define void @fa_string_case(ptr nocapture noundef readonly %0, i64 noundef %1, i32 noundef %2, ptr nocapture noundef writeonly %3) local_unnamed_addr #0 {
  %5 = alloca i64, align 8
  store i64 0, ptr @builder_length, align 8, !tbaa !2
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %5) #19
  store i64 0, ptr %5, align 8, !tbaa !2
  %6 = icmp eq i64 %1, 0
  br i1 %6, label %149, label %7

7:                                                ; preds = %4
  %8 = icmp eq i32 %2, 0
  br label %9

9:                                                ; preds = %7, %145
  %10 = call fastcc i32 @decode(ptr noundef %0, i64 noundef %1, ptr noundef nonnull %5) #17
  br i1 %8, label %42, label %11

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
  br label %67

29:                                               ; preds = %19
  %30 = and i32 %10, -16
  %31 = icmp eq i32 %30, 1104
  br i1 %31, label %32, label %34

32:                                               ; preds = %29
  %33 = add nsw i32 %10, -80
  br label %77

34:                                               ; preds = %29
  switch i32 %10, label %67 [
    i32 255, label %77
    i32 181, label %40
    i32 962, label %41
    i32 223, label %35
  ]

35:                                               ; preds = %34
  tail call fastcc void @reserve(i64 noundef 2) #17
  %36 = load ptr, ptr @builder, align 8, !tbaa !6
  %37 = load i64, ptr @builder_length, align 8, !tbaa !2
  %38 = getelementptr inbounds i8, ptr %36, i64 %37
  store i16 21331, ptr %38, align 1
  %39 = add i64 %37, 2
  br label %145, !llvm.loop !40

40:                                               ; preds = %34
  br label %77

41:                                               ; preds = %34
  br label %77

42:                                               ; preds = %9
  %43 = add i32 %10, -65
  %44 = icmp ult i32 %43, 26
  br i1 %44, label %58, label %45

45:                                               ; preds = %42
  %46 = add i32 %10, -192
  %47 = icmp ult i32 %46, 31
  %48 = icmp ne i32 %10, 215
  %49 = and i1 %48, %47
  br i1 %49, label %58, label %50

50:                                               ; preds = %45
  %51 = add i32 %10, -913
  %52 = icmp ult i32 %51, 25
  %53 = icmp ne i32 %10, 930
  %54 = and i1 %53, %52
  %55 = add i32 %10, -1040
  %56 = icmp ult i32 %55, 32
  %57 = or i1 %56, %54
  br i1 %57, label %58, label %60

58:                                               ; preds = %50, %45, %42
  %59 = add nuw nsw i32 %10, 32
  br label %67

60:                                               ; preds = %50
  %61 = and i32 %10, -16
  %62 = icmp eq i32 %61, 1024
  br i1 %62, label %63, label %65

63:                                               ; preds = %60
  %64 = or disjoint i32 %10, 80
  br label %77

65:                                               ; preds = %60
  %66 = icmp eq i32 %10, 376
  br i1 %66, label %77, label %67

67:                                               ; preds = %34, %65, %58, %27
  %68 = phi i32 [ %28, %27 ], [ %59, %58 ], [ %10, %65 ], [ %10, %34 ]
  %69 = icmp ult i32 %68, 128
  br i1 %69, label %70, label %75

70:                                               ; preds = %67
  %71 = trunc i32 %68 to i8
  tail call fastcc void @reserve(i64 noundef 1) #17
  %72 = load ptr, ptr @builder, align 8, !tbaa !6
  %73 = load i64, ptr @builder_length, align 8, !tbaa !2
  %74 = getelementptr inbounds i8, ptr %72, i64 %73
  store i8 %71, ptr %74, align 1, !tbaa !10
  br label %142

75:                                               ; preds = %67
  %76 = icmp ult i32 %68, 2048
  br i1 %76, label %77, label %92

77:                                               ; preds = %32, %40, %41, %63, %34, %65, %75
  %78 = phi i32 [ %68, %75 ], [ 376, %34 ], [ %64, %63 ], [ 931, %41 ], [ 924, %40 ], [ %33, %32 ], [ 255, %65 ]
  %79 = lshr i32 %78, 6
  %80 = trunc i32 %79 to i8
  %81 = or disjoint i8 %80, -64
  tail call fastcc void @reserve(i64 noundef 1) #17
  %82 = load ptr, ptr @builder, align 8, !tbaa !6
  %83 = load i64, ptr @builder_length, align 8, !tbaa !2
  %84 = getelementptr inbounds i8, ptr %82, i64 %83
  store i8 %81, ptr %84, align 1, !tbaa !10
  %85 = add i64 %83, 1
  store i64 %85, ptr @builder_length, align 8, !tbaa !2
  %86 = trunc i32 %78 to i8
  %87 = and i8 %86, 63
  %88 = or disjoint i8 %87, -128
  tail call fastcc void @reserve(i64 noundef 1) #17
  %89 = load ptr, ptr @builder, align 8, !tbaa !6
  %90 = load i64, ptr @builder_length, align 8, !tbaa !2
  %91 = getelementptr inbounds i8, ptr %89, i64 %90
  store i8 %88, ptr %91, align 1, !tbaa !10
  br label %142

92:                                               ; preds = %75
  %93 = icmp ult i32 %68, 65536
  tail call fastcc void @reserve(i64 noundef 1) #17
  %94 = load ptr, ptr @builder, align 8, !tbaa !6
  %95 = load i64, ptr @builder_length, align 8, !tbaa !2
  %96 = getelementptr inbounds i8, ptr %94, i64 %95
  %97 = add i64 %95, 1
  br i1 %93, label %98, label %116

98:                                               ; preds = %92
  %99 = lshr i32 %68, 12
  %100 = trunc i32 %99 to i8
  %101 = or disjoint i8 %100, -32
  store i8 %101, ptr %96, align 1, !tbaa !10
  store i64 %97, ptr @builder_length, align 8, !tbaa !2
  %102 = lshr i32 %68, 6
  %103 = trunc i32 %102 to i8
  %104 = and i8 %103, 63
  %105 = or disjoint i8 %104, -128
  tail call fastcc void @reserve(i64 noundef 1) #17
  %106 = load ptr, ptr @builder, align 8, !tbaa !6
  %107 = load i64, ptr @builder_length, align 8, !tbaa !2
  %108 = getelementptr inbounds i8, ptr %106, i64 %107
  store i8 %105, ptr %108, align 1, !tbaa !10
  %109 = add i64 %107, 1
  store i64 %109, ptr @builder_length, align 8, !tbaa !2
  %110 = trunc i32 %68 to i8
  %111 = and i8 %110, 63
  %112 = or disjoint i8 %111, -128
  tail call fastcc void @reserve(i64 noundef 1) #17
  %113 = load ptr, ptr @builder, align 8, !tbaa !6
  %114 = load i64, ptr @builder_length, align 8, !tbaa !2
  %115 = getelementptr inbounds i8, ptr %113, i64 %114
  store i8 %112, ptr %115, align 1, !tbaa !10
  br label %142

116:                                              ; preds = %92
  %117 = lshr i32 %68, 18
  %118 = trunc i32 %117 to i8
  %119 = or i8 %118, -16
  store i8 %119, ptr %96, align 1, !tbaa !10
  store i64 %97, ptr @builder_length, align 8, !tbaa !2
  %120 = lshr i32 %68, 12
  %121 = trunc i32 %120 to i8
  %122 = and i8 %121, 63
  %123 = or disjoint i8 %122, -128
  tail call fastcc void @reserve(i64 noundef 1) #17
  %124 = load ptr, ptr @builder, align 8, !tbaa !6
  %125 = load i64, ptr @builder_length, align 8, !tbaa !2
  %126 = getelementptr inbounds i8, ptr %124, i64 %125
  store i8 %123, ptr %126, align 1, !tbaa !10
  %127 = add i64 %125, 1
  store i64 %127, ptr @builder_length, align 8, !tbaa !2
  %128 = lshr i32 %68, 6
  %129 = trunc i32 %128 to i8
  %130 = and i8 %129, 63
  %131 = or disjoint i8 %130, -128
  tail call fastcc void @reserve(i64 noundef 1) #17
  %132 = load ptr, ptr @builder, align 8, !tbaa !6
  %133 = load i64, ptr @builder_length, align 8, !tbaa !2
  %134 = getelementptr inbounds i8, ptr %132, i64 %133
  store i8 %131, ptr %134, align 1, !tbaa !10
  %135 = add i64 %133, 1
  store i64 %135, ptr @builder_length, align 8, !tbaa !2
  %136 = trunc i32 %68 to i8
  %137 = and i8 %136, 63
  %138 = or disjoint i8 %137, -128
  tail call fastcc void @reserve(i64 noundef 1) #17
  %139 = load ptr, ptr @builder, align 8, !tbaa !6
  %140 = load i64, ptr @builder_length, align 8, !tbaa !2
  %141 = getelementptr inbounds i8, ptr %139, i64 %140
  store i8 %138, ptr %141, align 1, !tbaa !10
  br label %142

142:                                              ; preds = %70, %77, %98, %116
  %143 = phi i64 [ %90, %77 ], [ %140, %116 ], [ %114, %98 ], [ %73, %70 ]
  %144 = add i64 %143, 1
  br label %145

145:                                              ; preds = %142, %35
  %146 = phi i64 [ %39, %35 ], [ %144, %142 ]
  store i64 %146, ptr @builder_length, align 8, !tbaa !2
  %147 = load i64, ptr %5, align 8, !tbaa !2
  %148 = icmp ult i64 %147, %1
  br i1 %148, label %9, label %149

149:                                              ; preds = %145, %4
  %150 = phi i64 [ 0, %4 ], [ %146, %145 ]
  %151 = tail call i64 @llvm.umax.i64(i64 %150, i64 1)
  %152 = tail call ptr @fa_alloc(i64 noundef %151, i64 noundef 1) #16
  %153 = load ptr, ptr @builder, align 8, !tbaa !6
  %154 = load i64, ptr @builder_length, align 8, !tbaa !2
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %152, ptr align 1 %153, i64 %154, i1 false)
  store ptr %152, ptr %3, align 8, !tbaa !11
  %155 = getelementptr inbounds %struct.FaString, ptr %3, i64 0, i32 1
  store i64 %154, ptr %155, align 8, !tbaa !13
  store i64 0, ptr @builder_length, align 8, !tbaa !2
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %5) #19
  ret void
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define internal fastcc i32 @decode(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef %2) unnamed_addr #7 {
  %4 = load i64, ptr %2, align 8, !tbaa !2
  %5 = getelementptr inbounds i8, ptr %0, i64 %4
  %6 = load i8, ptr %5, align 1, !tbaa !10
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
  store i64 %25, ptr %2, align 8, !tbaa !2
  ret i32 %24

26:                                               ; preds = %17
  %27 = shl i32 %19, 6
  %28 = getelementptr inbounds i8, ptr %0, i64 %20
  %29 = load i8, ptr %28, align 1, !tbaa !10
  %30 = and i8 %29, 63
  %31 = zext nneg i8 %30 to i32
  %32 = or disjoint i32 %27, %31
  %33 = add nuw nsw i64 %18, 1
  %34 = icmp eq i64 %33, %14
  br i1 %34, label %22, label %17, !llvm.loop !41
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define void @fa_string_trim(ptr noundef %0, i64 noundef %1, i32 noundef %2, ptr nocapture noundef writeonly %3) local_unnamed_addr #7 {
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
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %5) #19
  store i64 %10, ptr %5, align 8, !tbaa !2
  %13 = call fastcc i32 @decode(ptr noundef %0, i64 noundef %1, ptr noundef nonnull %5) #17
  %14 = tail call fastcc i32 @is_space(i32 noundef %13) #17, !range !42
  %15 = icmp eq i32 %14, 0
  %16 = load i64, ptr %5, align 8
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %5) #19
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
  %30 = load i8, ptr %29, align 1, !tbaa !10
  %31 = and i8 %30, -64
  %32 = icmp eq i8 %31, -128
  br i1 %32, label %24, label %33, !llvm.loop !43

33:                                               ; preds = %24, %28
  %34 = phi i64 [ %18, %24 ], [ %26, %28 ]
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %6) #19
  store i64 %34, ptr %6, align 8, !tbaa !2
  %35 = call fastcc i32 @decode(ptr noundef %0, i64 noundef %22, ptr noundef nonnull %6) #17
  %36 = tail call fastcc i32 @is_space(i32 noundef %35) #17, !range !42
  %37 = icmp eq i32 %36, 0
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %6) #19
  br i1 %37, label %38, label %21

38:                                               ; preds = %33, %21, %17
  %39 = phi i64 [ %1, %17 ], [ %22, %21 ], [ %22, %33 ]
  %40 = getelementptr inbounds i8, ptr %0, i64 %18
  store ptr %40, ptr %3, align 8, !tbaa !11
  %41 = sub i64 %39, %18
  %42 = getelementptr inbounds %struct.FaString, ptr %3, i64 0, i32 1
  store i64 %41, ptr %42, align 8, !tbaa !13
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(none)
define internal fastcc noundef i32 @is_space(i32 noundef %0) unnamed_addr #8 {
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
  store i64 0, ptr @builder_length, align 8, !tbaa !2
  %5 = icmp sgt i64 %2, 0
  br i1 %5, label %13, label %6

6:                                                ; preds = %13, %4
  %7 = phi i64 [ 0, %4 ], [ %18, %13 ]
  %8 = tail call i64 @llvm.umax.i64(i64 %7, i64 1)
  %9 = tail call ptr @fa_alloc(i64 noundef %8, i64 noundef 1) #16
  %10 = load ptr, ptr @builder, align 8, !tbaa !6
  %11 = load i64, ptr @builder_length, align 8, !tbaa !2
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %9, ptr align 1 %10, i64 %11, i1 false)
  store ptr %9, ptr %3, align 8, !tbaa !11
  %12 = getelementptr inbounds %struct.FaString, ptr %3, i64 0, i32 1
  store i64 %11, ptr %12, align 8, !tbaa !13
  store i64 0, ptr @builder_length, align 8, !tbaa !2
  ret void

13:                                               ; preds = %4, %13
  %14 = phi i64 [ %19, %13 ], [ 0, %4 ]
  tail call fastcc void @reserve(i64 noundef %1) #17
  %15 = load ptr, ptr @builder, align 8, !tbaa !6
  %16 = load i64, ptr @builder_length, align 8, !tbaa !2
  %17 = getelementptr inbounds i8, ptr %15, i64 %16
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %17, ptr align 1 %0, i64 %1, i1 false)
  %18 = add i64 %16, %1
  store i64 %18, ptr @builder_length, align 8, !tbaa !2
  %19 = add nuw nsw i64 %14, 1
  %20 = icmp eq i64 %19, %2
  br i1 %20, label %6, label %13, !llvm.loop !44
}

; Function Attrs: nounwind optsize
define void @fa_string_pad(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2, ptr nocapture noundef readonly %3, i64 noundef %4, i32 noundef %5, ptr nocapture noundef writeonly %6) local_unnamed_addr #0 {
  %8 = icmp eq i64 %1, 0
  br i1 %8, label %15, label %9

9:                                                ; preds = %7, %9
  %10 = phi i64 [ %13, %9 ], [ 0, %7 ]
  %11 = phi i64 [ %12, %9 ], [ 0, %7 ]
  %12 = add i64 %11, 1
  %13 = tail call fastcc i64 @next_boundary(ptr noundef %0, i64 noundef %1, i64 noundef %10) #17
  %14 = icmp ult i64 %13, %1
  br i1 %14, label %9, label %15, !llvm.loop !35

15:                                               ; preds = %9, %7
  %16 = phi i64 [ 0, %7 ], [ %12, %9 ]
  %17 = sub nsw i64 %2, %16
  store i64 0, ptr @builder_length, align 8, !tbaa !2
  %18 = icmp eq i32 %5, 0
  br i1 %18, label %19, label %24

19:                                               ; preds = %15
  tail call fastcc void @reserve(i64 noundef %1) #17
  %20 = load ptr, ptr @builder, align 8, !tbaa !6
  %21 = load i64, ptr @builder_length, align 8, !tbaa !2
  %22 = getelementptr inbounds i8, ptr %20, i64 %21
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %22, ptr align 1 %0, i64 %1, i1 false)
  %23 = add i64 %21, %1
  store i64 %23, ptr @builder_length, align 8, !tbaa !2
  br label %24

24:                                               ; preds = %19, %15
  %25 = phi i64 [ %23, %19 ], [ 0, %15 ]
  %26 = icmp ne i64 %4, 0
  %27 = icmp sgt i64 %17, 0
  %28 = and i1 %26, %27
  br i1 %28, label %31, label %29

29:                                               ; preds = %31, %24
  %30 = phi i64 [ %25, %24 ], [ %40, %31 ]
  br i1 %18, label %50, label %45

31:                                               ; preds = %24, %31
  %32 = phi i64 [ %42, %31 ], [ 0, %24 ]
  %33 = phi i64 [ %43, %31 ], [ 0, %24 ]
  %34 = tail call fastcc i64 @next_boundary(ptr noundef %3, i64 noundef %4, i64 noundef %32) #17
  %35 = getelementptr inbounds i8, ptr %3, i64 %32
  %36 = sub i64 %34, %32
  tail call fastcc void @reserve(i64 noundef %36) #17
  %37 = load ptr, ptr @builder, align 8, !tbaa !6
  %38 = load i64, ptr @builder_length, align 8, !tbaa !2
  %39 = getelementptr inbounds i8, ptr %37, i64 %38
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %39, ptr align 1 %35, i64 %36, i1 false)
  %40 = add i64 %38, %36
  store i64 %40, ptr @builder_length, align 8, !tbaa !2
  %41 = icmp eq i64 %34, %4
  %42 = select i1 %41, i64 0, i64 %34
  %43 = add nuw nsw i64 %33, 1
  %44 = icmp slt i64 %43, %17
  br i1 %44, label %31, label %29, !llvm.loop !45

45:                                               ; preds = %29
  tail call fastcc void @reserve(i64 noundef %1) #17
  %46 = load ptr, ptr @builder, align 8, !tbaa !6
  %47 = load i64, ptr @builder_length, align 8, !tbaa !2
  %48 = getelementptr inbounds i8, ptr %46, i64 %47
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %48, ptr align 1 %0, i64 %1, i1 false)
  %49 = add i64 %47, %1
  store i64 %49, ptr @builder_length, align 8, !tbaa !2
  br label %50

50:                                               ; preds = %45, %29
  %51 = phi i64 [ %49, %45 ], [ %30, %29 ]
  %52 = tail call i64 @llvm.umax.i64(i64 %51, i64 1)
  %53 = tail call ptr @fa_alloc(i64 noundef %52, i64 noundef 1) #16
  %54 = load ptr, ptr @builder, align 8, !tbaa !6
  %55 = load i64, ptr @builder_length, align 8, !tbaa !2
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %53, ptr align 1 %54, i64 %55, i1 false)
  store ptr %53, ptr %6, align 8, !tbaa !11
  %56 = getelementptr inbounds %struct.FaString, ptr %6, i64 0, i32 1
  store i64 %55, ptr %56, align 8, !tbaa !13
  store i64 0, ptr @builder_length, align 8, !tbaa !2
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_arguments_init(i32 noundef %0, ptr nocapture noundef readonly %1) local_unnamed_addr #0 {
  %3 = icmp sgt i32 %0, 1
  %4 = zext nneg i32 %0 to i64
  %5 = add nsw i64 %4, -1
  %6 = select i1 %3, i64 %5, i64 0
  store i64 %6, ptr @argument_count, align 8, !tbaa !2
  %7 = shl nsw i64 %6, 4
  %8 = add nsw i64 %7, 16
  %9 = tail call ptr @fa_alloc(i64 noundef %8, i64 noundef 8) #16
  store ptr %9, ptr @argument_items, align 8, !tbaa !6
  %10 = load i64, ptr @argument_count, align 8, !tbaa !2
  %11 = icmp eq i64 %10, 0
  br i1 %11, label %12, label %13

12:                                               ; preds = %25, %2
  ret void

13:                                               ; preds = %2, %25
  %14 = phi i64 [ %15, %25 ], [ 0, %2 ]
  %15 = add nuw i64 %14, 1
  %16 = getelementptr inbounds ptr, ptr %1, i64 %15
  %17 = load ptr, ptr %16, align 8, !tbaa !6
  %18 = getelementptr inbounds %struct.FaString, ptr %9, i64 %14
  store ptr %17, ptr %18, align 8, !tbaa !11
  br label %19

19:                                               ; preds = %19, %13
  %20 = phi i64 [ 0, %13 ], [ %24, %19 ]
  %21 = getelementptr inbounds i8, ptr %17, i64 %20
  %22 = load i8, ptr %21, align 1, !tbaa !10
  %23 = icmp eq i8 %22, 0
  %24 = add i64 %20, 1
  br i1 %23, label %25, label %19, !llvm.loop !19

25:                                               ; preds = %19
  %26 = getelementptr inbounds %struct.FaString, ptr %9, i64 %14, i32 1
  store i64 %20, ptr %26, align 8, !tbaa !13
  %27 = icmp eq i64 %15, %10
  br i1 %27, label %12, label %13, !llvm.loop !46
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(read, argmem: write, inaccessiblemem: none)
define void @fa_arguments(ptr nocapture noundef writeonly %0) local_unnamed_addr #9 {
  %2 = load ptr, ptr @argument_items, align 8, !tbaa !6
  store ptr %2, ptr %0, align 8, !tbaa !11
  %3 = load i64, ptr @argument_count, align 8, !tbaa !2
  %4 = getelementptr inbounds %struct.FaArray, ptr %0, i64 0, i32 1
  store i64 %3, ptr %4, align 8, !tbaa !13
  ret void
}

; Function Attrs: cold noinline noreturn nounwind optsize
define void @fa_overflow(i64 noundef %0, i64 noundef %1, i32 noundef %2, ptr noundef %3) local_unnamed_addr #10 {
  store i64 0, ptr @builder_length, align 8, !tbaa !2
  tail call fastcc void @reserve(i64 noundef 18) #17
  %5 = load ptr, ptr @builder, align 8, !tbaa !6
  %6 = load i64, ptr @builder_length, align 8, !tbaa !2
  %7 = getelementptr inbounds i8, ptr %5, i64 %6
  tail call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(18) %7, ptr noundef nonnull align 1 dereferenceable(18) @.str.13, i64 18, i1 false)
  %8 = add i64 %6, 18
  store i64 %8, ptr @builder_length, align 8, !tbaa !2
  %9 = icmp eq i32 %2, 0
  br i1 %9, label %23, label %10

10:                                               ; preds = %4
  %11 = zext i64 %1 to i128
  %12 = shl nuw i128 %11, 64
  %13 = zext i64 %0 to i128
  %14 = or disjoint i128 %12, %13
  %15 = icmp slt i64 %1, 0
  %16 = lshr i64 %1, 63
  %17 = trunc i64 %16 to i32
  %18 = sub i128 0, %14
  %19 = select i1 %15, i128 %18, i128 %14
  %20 = trunc i128 %19 to i64
  %21 = lshr i128 %19, 64
  %22 = trunc i128 %21 to i64
  tail call fastcc void @append_u128(i64 noundef %20, i64 noundef %22, i32 noundef %17) #17
  br label %24

23:                                               ; preds = %4
  tail call fastcc void @append_u128(i64 noundef %0, i64 noundef %1, i32 noundef 0) #17
  br label %24

24:                                               ; preds = %23, %10
  tail call fastcc void @append_c(ptr noundef nonnull @.str.14) #17
  tail call fastcc void @append_c(ptr noundef %3) #17
  br label %25

25:                                               ; preds = %25, %24
  %26 = phi i64 [ 0, %24 ], [ %30, %25 ]
  %27 = getelementptr inbounds i8, ptr %3, i64 %26
  %28 = load i8, ptr %27, align 1, !tbaa !10
  %29 = icmp eq i8 %28, 0
  %30 = add i64 %26, 1
  br i1 %29, label %31, label %25, !llvm.loop !19

31:                                               ; preds = %25
  %32 = getelementptr inbounds i8, ptr %3, i64 %26
  %33 = getelementptr inbounds i8, ptr %32, i64 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %33) #20
  unreachable
}

; Function Attrs: noreturn nounwind optsize
define internal fastcc void @stop_with_builder(ptr noundef %0) unnamed_addr #2 {
  tail call void @fa_flush() #17
  %2 = load ptr, ptr @builder, align 8, !tbaa !6
  %3 = load i64, ptr @builder_length, align 8, !tbaa !2
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.26, i64 noundef 6) #16
  tail call void @fa_write(i32 noundef 2, ptr noundef %2, i64 noundef %3) #16
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.27, i64 noundef 6) #16
  br label %4

4:                                                ; preds = %4, %1
  %5 = phi i64 [ 0, %1 ], [ %9, %4 ]
  %6 = getelementptr inbounds i8, ptr %0, i64 %5
  %7 = load i8, ptr %6, align 1, !tbaa !10
  %8 = icmp eq i8 %7, 0
  %9 = add i64 %5, 1
  br i1 %8, label %10, label %4, !llvm.loop !19

10:                                               ; preds = %4
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull %0, i64 noundef %5) #16
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.28, i64 noundef 1) #16
  tail call void @fa_exit(i32 noundef 101) #18
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define void @fa_overflow_unknown(ptr noundef %0) local_unnamed_addr #10 {
  store i64 0, ptr @builder_length, align 8, !tbaa !2
  tail call fastcc void @append_c(ptr noundef nonnull @.str.15) #17
  tail call fastcc void @append_c(ptr noundef %0) #17
  br label %2

2:                                                ; preds = %2, %1
  %3 = phi i64 [ 0, %1 ], [ %7, %2 ]
  %4 = getelementptr inbounds i8, ptr %0, i64 %3
  %5 = load i8, ptr %4, align 1, !tbaa !10
  %6 = icmp eq i8 %5, 0
  %7 = add i64 %3, 1
  br i1 %6, label %8, label %2, !llvm.loop !19

8:                                                ; preds = %2
  %9 = getelementptr inbounds i8, ptr %0, i64 %3
  %10 = getelementptr inbounds i8, ptr %9, i64 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %10) #20
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define void @fa_float_overflow(double noundef %0, ptr noundef %1) local_unnamed_addr #10 {
  store i64 0, ptr @builder_length, align 8, !tbaa !2
  tail call fastcc void @append_c(ptr noundef nonnull @.str.13) #17
  tail call void @fa_b_f64(double noundef %0) #17
  tail call fastcc void @append_c(ptr noundef nonnull @.str.14) #17
  tail call fastcc void @append_c(ptr noundef %1) #17
  br label %3

3:                                                ; preds = %3, %2
  %4 = phi i64 [ 0, %2 ], [ %8, %3 ]
  %5 = getelementptr inbounds i8, ptr %1, i64 %4
  %6 = load i8, ptr %5, align 1, !tbaa !10
  %7 = icmp eq i8 %6, 0
  %8 = add i64 %4, 1
  br i1 %7, label %9, label %3, !llvm.loop !19

9:                                                ; preds = %3
  %10 = getelementptr inbounds i8, ptr %1, i64 %4
  %11 = getelementptr inbounds i8, ptr %10, i64 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %11) #20
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define void @fa_assert_failed(ptr noundef %0) local_unnamed_addr #10 {
  store i64 0, ptr @builder_length, align 8, !tbaa !2
  tail call fastcc void @append_c(ptr noundef nonnull @.str.16) #17
  tail call fastcc void @append_c(ptr noundef %0) #17
  br label %2

2:                                                ; preds = %2, %1
  %3 = phi i64 [ 0, %1 ], [ %7, %2 ]
  %4 = getelementptr inbounds i8, ptr %0, i64 %3
  %5 = load i8, ptr %4, align 1, !tbaa !10
  %6 = icmp eq i8 %5, 0
  %7 = add i64 %3, 1
  br i1 %6, label %8, label %2, !llvm.loop !19

8:                                                ; preds = %2
  %9 = getelementptr inbounds i8, ptr %0, i64 %3
  %10 = getelementptr inbounds i8, ptr %9, i64 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %10) #20
  unreachable
}

; Function Attrs: nounwind optsize
define void @fa_guard_stack() local_unnamed_addr #0 {
  %1 = alloca %struct.AlternateStack, align 8
  %2 = alloca %struct.SignalAction, align 8
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %1) #19
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %1, ptr noundef nonnull align 8 dereferenceable(24) @__const.fa_guard_stack.stack, i64 24, i1 false)
  %3 = call i32 @sigaltstack(ptr noundef nonnull %1, ptr noundef null) #16
  call void @llvm.lifetime.start.p0(i64 152, ptr nonnull %2) #19
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(152) %2, ptr noundef nonnull align 8 dereferenceable(152) @__const.fa_guard_stack.action, i64 152, i1 false)
  %4 = call i32 @sigaction(i32 noundef 11, ptr noundef nonnull %2, ptr noundef null) #16
  call void @llvm.lifetime.end.p0(i64 152, ptr nonnull %2) #19
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %1) #19
  ret void
}

; Function Attrs: mustprogress nocallback nofree nounwind willreturn memory(argmem: readwrite)
declare void @llvm.memcpy.p0.p0.i64(ptr noalias nocapture writeonly, ptr noalias nocapture readonly, i64, i1 immarg) #11

; Function Attrs: optsize
declare i32 @sigaltstack(ptr noundef, ptr noundef) local_unnamed_addr #1

; Function Attrs: noreturn nounwind optsize
define internal void @on_fault(i32 %0) #2 {
  tail call void @fa_flush() #17
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @on_fault.message, i64 noundef 21) #16
  tail call void @_exit(i32 noundef 101) #18
  unreachable
}

; Function Attrs: optsize
declare i32 @sigaction(i32 noundef, ptr noundef, ptr noundef) local_unnamed_addr #1

; Function Attrs: cold noinline noreturn nounwind optsize
define void @fa_division_by_zero() local_unnamed_addr #10 {
  store i64 0, ptr @builder_length, align 8, !tbaa !2
  tail call fastcc void @append_c(ptr noundef nonnull @.str.17) #17
  tail call fastcc void @uncaught_with_builder() #20
  unreachable
}

; Function Attrs: noreturn nounwind optsize
define internal fastcc void @uncaught_with_builder() unnamed_addr #2 {
  tail call void @fa_flush() #17
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.29, i64 noundef 7) #16
  %1 = load ptr, ptr @builder, align 8, !tbaa !6
  %2 = load i64, ptr @builder_length, align 8, !tbaa !2
  tail call void @fa_write(i32 noundef 2, ptr noundef %1, i64 noundef %2) #16
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.28, i64 noundef 1) #16
  tail call void @fa_exit(i32 noundef 1) #18
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define void @fa_conversion_error(i64 noundef %0, i64 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3) local_unnamed_addr #10 {
  %5 = alloca %struct.FaString, align 8
  store i64 0, ptr @builder_length, align 8, !tbaa !2
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
  br label %20

20:                                               ; preds = %4, %7
  %21 = phi i32 [ %14, %7 ], [ 0, %4 ]
  %22 = phi i64 [ %19, %7 ], [ %1, %4 ]
  %23 = phi i64 [ %17, %7 ], [ %0, %4 ]
  tail call fastcc void @append_u128(i64 noundef %23, i64 noundef %22, i32 noundef %21) #17
  tail call fastcc void @append_c(ptr noundef nonnull @.str.14) #17
  tail call fastcc void @append_c(ptr noundef %3) #17
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %5) #19
  call void @fa_b_take(ptr noundef nonnull %5) #17
  tail call fastcc void @append_c(ptr noundef nonnull @.str.18) #17
  %24 = load ptr, ptr %5, align 8, !tbaa !11
  %25 = getelementptr inbounds %struct.FaString, ptr %5, i64 0, i32 1
  %26 = load i64, ptr %25, align 8, !tbaa !13
  tail call void @fa_b_quoted(ptr noundef %24, i64 noundef %26) #17
  tail call fastcc void @append_c(ptr noundef nonnull @.str.19) #17
  tail call fastcc void @uncaught_with_builder() #20
  unreachable
}

; Function Attrs: nounwind optsize
define i64 @fa_pow_i64(i64 noundef %0, i64 noundef %1, i32 noundef %2, ptr noundef %3) local_unnamed_addr #0 {
  %5 = icmp slt i64 %1, 0
  br i1 %5, label %8, label %6

6:                                                ; preds = %4
  %7 = icmp ne i32 %2, 0
  br label %9

8:                                                ; preds = %4
  tail call fastcc void @negative_exponent(i64 noundef %1) #21
  unreachable

9:                                                ; preds = %6, %25
  %10 = phi i64 [ %26, %25 ], [ %1, %6 ]
  %11 = phi i64 [ %29, %25 ], [ %0, %6 ]
  %12 = phi i64 [ %22, %25 ], [ 1, %6 ]
  %13 = and i64 %10, 1
  %14 = icmp eq i64 %13, 0
  br i1 %14, label %21, label %15

15:                                               ; preds = %9
  %16 = tail call { i64, i1 } @llvm.smul.with.overflow.i64(i64 %12, i64 %11)
  %17 = extractvalue { i64, i1 } %16, 1
  %18 = extractvalue { i64, i1 } %16, 0
  %19 = and i1 %7, %17
  br i1 %19, label %20, label %21

20:                                               ; preds = %15
  tail call void @fa_overflow_unknown(ptr noundef %3) #21
  unreachable

21:                                               ; preds = %15, %9
  %22 = phi i64 [ %18, %15 ], [ %12, %9 ]
  %23 = icmp ult i64 %10, 2
  br i1 %23, label %24, label %25

24:                                               ; preds = %21
  ret i64 %22

25:                                               ; preds = %21
  %26 = lshr i64 %10, 1
  %27 = tail call { i64, i1 } @llvm.smul.with.overflow.i64(i64 %11, i64 %11)
  %28 = extractvalue { i64, i1 } %27, 1
  %29 = extractvalue { i64, i1 } %27, 0
  %30 = and i1 %7, %28
  br i1 %30, label %31, label %9

31:                                               ; preds = %25
  tail call void @fa_overflow_unknown(ptr noundef %3) #21
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define internal fastcc void @negative_exponent(i64 noundef %0) unnamed_addr #10 {
  %2 = alloca %struct.FaString, align 8
  store i64 0, ptr @builder_length, align 8, !tbaa !2
  tail call fastcc void @append_c(ptr noundef nonnull @.str.30) #17
  tail call void @fa_b_i64(i64 noundef %0) #17
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %2) #19
  call void @fa_b_take(ptr noundef nonnull %2) #17
  tail call fastcc void @append_c(ptr noundef nonnull @.str.18) #17
  %3 = load ptr, ptr %2, align 8, !tbaa !11
  %4 = getelementptr inbounds %struct.FaString, ptr %2, i64 0, i32 1
  %5 = load i64, ptr %4, align 8, !tbaa !13
  tail call void @fa_b_quoted(ptr noundef %3, i64 noundef %5) #17
  tail call fastcc void @append_c(ptr noundef nonnull @.str.19) #17
  tail call fastcc void @uncaught_with_builder() #20
  unreachable
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare { i64, i1 } @llvm.smul.with.overflow.i64(i64, i64) #12

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
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %5) #19
  %6 = call fastcc { i64, i64 } @divide_u128(i64 noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3, ptr noundef nonnull %5) #17
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %5) #19
  ret { i64, i64 } %6
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: write)
define internal fastcc { i64, i64 } @divide_u128(i64 noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3, ptr nocapture noundef writeonly %4) unnamed_addr #13 {
  br label %13

6:                                                ; preds = %45
  %7 = zext i64 %46 to i128
  %8 = shl nuw i128 %7, 64
  %9 = zext i64 %47 to i128
  %10 = or disjoint i128 %8, %9
  store i128 %10, ptr %4, align 16, !tbaa !47
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
  br i1 %51, label %6, label %13, !llvm.loop !49
}

; Function Attrs: nounwind optsize
define weak { i64, i64 } @__umodti3(i64 noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3) local_unnamed_addr #0 {
  %5 = alloca i128, align 16
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %5) #19
  %6 = call fastcc { i64, i64 } @divide_u128(i64 noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3, ptr noundef nonnull %5) #17
  %7 = load i128, ptr %5, align 16, !tbaa !47
  %8 = trunc i128 %7 to i64
  %9 = lshr i128 %7, 64
  %10 = trunc i128 %9 to i64
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %5) #19
  %11 = insertvalue { i64, i64 } poison, i64 %8, 0
  %12 = insertvalue { i64, i64 } %11, i64 %10, 1
  ret { i64, i64 } %12
}

; Function Attrs: nounwind optsize
define weak { i64, i64 } @__divti3(i64 noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3) local_unnamed_addr #0 {
  %5 = alloca i128, align 16
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %5) #19
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
  %22 = call fastcc { i64, i64 } @divide_u128(i64 noundef %11, i64 noundef %13, i64 noundef %19, i64 noundef %21, ptr noundef nonnull %5) #17
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
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %5) #19
  %36 = insertvalue { i64, i64 } poison, i64 %33, 0
  %37 = insertvalue { i64, i64 } %36, i64 %35, 1
  ret { i64, i64 } %37
}

; Function Attrs: nounwind optsize
define weak { i64, i64 } @__modti3(i64 noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3) local_unnamed_addr #0 {
  %5 = alloca i128, align 16
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %5) #19
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
  %22 = call fastcc { i64, i64 } @divide_u128(i64 noundef %11, i64 noundef %13, i64 noundef %19, i64 noundef %21, ptr noundef nonnull %5) #17
  %23 = load i128, ptr %5, align 16
  %24 = sub i128 0, %23
  %25 = icmp slt i64 %1, 0
  %26 = select i1 %25, i128 %24, i128 %23
  %27 = trunc i128 %26 to i64
  %28 = lshr i128 %26, 64
  %29 = trunc i128 %28 to i64
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %5) #19
  %30 = insertvalue { i64, i64 } poison, i64 %27, 0
  %31 = insertvalue { i64, i64 } %30, i64 %29, 1
  ret { i64, i64 } %31
}

; Function Attrs: optsize
declare i32 @snprintf(ptr noundef, i64 noundef, ptr noundef, ...) local_unnamed_addr #1

; Function Attrs: optsize
declare double @strtod(ptr noundef, ptr noundef) local_unnamed_addr #1

; Function Attrs: nofree norecurse nosync nounwind optsize memory(none)
define internal fastcc i32 @break_kind(i32 noundef %0) unnamed_addr #14 {
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
  %59 = load i32, ptr %58, align 8, !tbaa !50
  %60 = icmp ugt i32 %59, %0
  br i1 %60, label %52, label %61, !llvm.loop !52

61:                                               ; preds = %55
  %62 = getelementptr inbounds %struct.Range, ptr @EXTEND, i64 %57, i32 1
  %63 = load i32, ptr %62, align 4, !tbaa !53
  %64 = icmp ult i32 %63, %0
  %65 = add nuw i64 %57, 1
  br i1 %64, label %49, label %83, !llvm.loop !52

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
  %76 = load i32, ptr %75, align 8, !tbaa !50
  %77 = icmp ugt i32 %76, %0
  br i1 %77, label %69, label %78, !llvm.loop !52

78:                                               ; preds = %72
  %79 = getelementptr inbounds %struct.Range, ptr @PICTOGRAPHIC, i64 %74, i32 1
  %80 = load i32, ptr %79, align 4, !tbaa !53
  %81 = icmp ult i32 %80, %0
  %82 = add nuw i64 %74, 1
  br i1 %81, label %66, label %83, !llvm.loop !52

83:                                               ; preds = %61, %78, %69, %35, %29, %23, %20, %18, %3, %5, %1, %44, %13, %2
  %84 = phi i32 [ 2, %2 ], [ %17, %13 ], [ %48, %44 ], [ 1, %1 ], [ 3, %5 ], [ 3, %3 ], [ 5, %18 ], [ 6, %20 ], [ 8, %23 ], [ 9, %29 ], [ 10, %35 ], [ 0, %69 ], [ 7, %78 ], [ 4, %61 ]
  ret i32 %84
}

; Function Attrs: noreturn optsize
declare void @_exit(i32 noundef) local_unnamed_addr #3

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.umax.i64(i64, i64) #15

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.abs.i64(i64, i1 immarg) #15

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.umin.i64(i64, i64) #15

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.fshl.i64(i64, i64, i64) #15

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i128 @llvm.abs.i128(i128, i1 immarg) #15

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.smin.i32(i32, i32) #15

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.abs.i32(i32, i1 immarg) #15

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.vector.reduce.or.v4i32(<4 x i32>) #15

attributes #0 = { nounwind optsize "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #1 = { optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #2 = { noreturn nounwind optsize "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #3 = { noreturn optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #4 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, argmem: none, inaccessiblemem: none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #5 = { mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite) }
attributes #6 = { nofree norecurse nosync nounwind optsize memory(argmem: read) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #7 = { nofree norecurse nosync nounwind optsize memory(argmem: readwrite) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #8 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #9 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(read, argmem: write, inaccessiblemem: none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #10 = { cold noinline noreturn nounwind optsize "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #11 = { mustprogress nocallback nofree nounwind willreturn memory(argmem: readwrite) }
attributes #12 = { mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #13 = { nofree norecurse nosync nounwind optsize memory(argmem: write) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #14 = { nofree norecurse nosync nounwind optsize memory(none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #15 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #16 = { nounwind optsize }
attributes #17 = { optsize }
attributes #18 = { noreturn nounwind optsize }
attributes #19 = { nounwind }
attributes #20 = { noreturn optsize }
attributes #21 = { cold noreturn optsize }

!llvm.module.flags = !{!0, !1}

!0 = !{i32 1, !"wchar_size", i32 4}
!1 = !{i32 8, !"PIC Level", i32 2}
!2 = !{!3, !3, i64 0}
!3 = !{!"long", !4, i64 0}
!4 = !{!"omnipotent char", !5, i64 0}
!5 = !{!"Simple C/C++ TBAA"}
!6 = !{!7, !7, i64 0}
!7 = !{!"any pointer", !4, i64 0}
!8 = distinct !{!8, !9}
!9 = !{!"llvm.loop.mustprogress"}
!10 = !{!4, !4, i64 0}
!11 = !{!12, !7, i64 0}
!12 = !{!"", !7, i64 0, !3, i64 8}
!13 = !{!12, !3, i64 8}
!14 = distinct !{!14, !9}
!15 = !{!16, !16, i64 0}
!16 = !{!"int", !4, i64 0}
!17 = distinct !{!17, !9}
!18 = distinct !{!18, !9}
!19 = distinct !{!19, !9}
!20 = distinct !{!20, !9}
!21 = distinct !{!21, !9}
!22 = distinct !{!22, !9}
!23 = distinct !{!23, !9}
!24 = distinct !{!24, !9}
!25 = distinct !{!25, !9}
!26 = distinct !{!26, !9}
!27 = distinct !{!27, !9}
!28 = distinct !{!28, !9}
!29 = distinct !{!29, !9}
!30 = distinct !{!30, !9}
!31 = distinct !{!31, !9}
!32 = distinct !{!32, !9}
!33 = distinct !{!33, !9}
!34 = !{i32 0, i32 13}
!35 = distinct !{!35, !9}
!36 = distinct !{!36, !9}
!37 = distinct !{!37, !9}
!38 = distinct !{!38, !9}
!39 = distinct !{!39, !9}
!40 = distinct !{!40, !9}
!41 = distinct !{!41, !9}
!42 = !{i32 0, i32 2}
!43 = distinct !{!43, !9}
!44 = distinct !{!44, !9}
!45 = distinct !{!45, !9}
!46 = distinct !{!46, !9}
!47 = !{!48, !48, i64 0}
!48 = !{!"__int128", !4, i64 0}
!49 = distinct !{!49, !9}
!50 = !{!51, !16, i64 0}
!51 = !{!"", !16, i64 0, !16, i64 4}
!52 = distinct !{!52, !9}
!53 = !{!51, !16, i64 4}
