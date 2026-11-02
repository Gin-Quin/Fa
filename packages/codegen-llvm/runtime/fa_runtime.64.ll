source_filename = "fa_runtime.c"

%struct.AlternateStack = type { ptr, i32, i64 }
%struct.SignalAction = type { ptr, [16 x i64], i32, ptr }
%struct.BareError = type { i64 }
%struct.Range = type { i32, i32 }
%struct.FaString = type { ptr, i64, i64 }
%struct.Replacing = type { ptr, i64, i64, ptr, i64, i32, i32 }
%struct.FaArrayHeader = type { ptr, i64, i64 }
%struct.Splitting = type { ptr, i64, i64, ptr }
%struct.IndexError = type { i64, i64, i64 }
%struct.FaSegmented = type { i64, i64, [48 x ptr] }
%struct.PageHeader = type { i32, i32 }
%struct.FreeSpan = type { ptr, i64 }
%struct.MessageError = type { i64, %struct.FaString }
%struct.ParseError = type { i64, %struct.FaString, %struct.FaString }
%struct.FaTable = type { ptr, i64, i64, i64, ptr, i64, i64 }

@output_length = internal unnamed_addr global i64 0, align 8
@output = internal global [8192 x i8] zeroinitializer, align 16
@builder_length = internal unnamed_addr global i64 0, align 8
@builder = internal unnamed_addr global ptr null, align 8
@.str = private unnamed_addr constant [5 x i8] c"true\00", align 1
@.str.1 = private unnamed_addr constant [6 x i8] c"false\00", align 1
@fa_b_quoted.hex = internal unnamed_addr constant [17 x i8] c"0123456789ABCDEF\00", align 16
@argument_count = internal unnamed_addr global i64 0, align 8
@argument_items = internal unnamed_addr global ptr null, align 8
@free_blocks = internal unnamed_addr global [40 x ptr] zeroinitializer, align 16
@class_next = internal unnamed_addr global [40 x ptr] zeroinitializer, align 16
@class_end = internal unnamed_addr global [40 x ptr] zeroinitializer, align 16
@builder_capacity = internal unnamed_addr global i64 0, align 8
@fa_live_blocks = local_unnamed_addr global i64 0, align 8
@.str.13 = private unnamed_addr constant [7 x i8] c"leak: \00", align 1
@.str.14 = private unnamed_addr constant [8 x i8] c" blocks\00", align 1
@.str.15 = private unnamed_addr constant [19 x i8] c"integer overflow: \00", align 1
@.str.16 = private unnamed_addr constant [17 x i8] c" doesn't fit in \00", align 1
@.str.17 = private unnamed_addr constant [44 x i8] c"integer overflow: the value doesn't fit in \00", align 1
@.str.18 = private unnamed_addr constant [19 x i8] c"assertion failed: \00", align 1
@signal_stack = internal global [16384 x i8] zeroinitializer, align 16
@__const.fa_guard_stack.stack = private unnamed_addr constant %struct.AlternateStack { ptr @signal_stack, i32 0, i64 16384 }, align 8
@__const.fa_guard_stack.action = private unnamed_addr constant %struct.SignalAction { ptr @on_fault, [16 x i64] zeroinitializer, i32 134217728, ptr null }, align 8
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
@.str.30 = private unnamed_addr constant [4 x i8] c"NaN\00", align 1
@.str.32 = private unnamed_addr constant [3 x i8] c"0.\00", align 1
@.str.33 = private unnamed_addr constant [5 x i8] c"%.*e\00", align 1
@.str.34 = private unnamed_addr constant [8 x i8] c"%llde%d\00", align 1
@.str.35 = private unnamed_addr constant [6 x i8] c"%.14e\00", align 1
@EXTEND = internal unnamed_addr constant [153 x %struct.Range] [%struct.Range { i32 768, i32 879 }, %struct.Range { i32 1155, i32 1161 }, %struct.Range { i32 1425, i32 1469 }, %struct.Range { i32 1471, i32 1471 }, %struct.Range { i32 1473, i32 1474 }, %struct.Range { i32 1476, i32 1477 }, %struct.Range { i32 1479, i32 1479 }, %struct.Range { i32 1552, i32 1562 }, %struct.Range { i32 1611, i32 1631 }, %struct.Range { i32 1648, i32 1648 }, %struct.Range { i32 1750, i32 1756 }, %struct.Range { i32 1759, i32 1764 }, %struct.Range { i32 1767, i32 1768 }, %struct.Range { i32 1770, i32 1773 }, %struct.Range { i32 1809, i32 1809 }, %struct.Range { i32 1840, i32 1866 }, %struct.Range { i32 1958, i32 1968 }, %struct.Range { i32 2027, i32 2035 }, %struct.Range { i32 2070, i32 2093 }, %struct.Range { i32 2137, i32 2139 }, %struct.Range { i32 2200, i32 2207 }, %struct.Range { i32 2250, i32 2273 }, %struct.Range { i32 2275, i32 2307 }, %struct.Range { i32 2362, i32 2364 }, %struct.Range { i32 2366, i32 2383 }, %struct.Range { i32 2385, i32 2391 }, %struct.Range { i32 2402, i32 2403 }, %struct.Range { i32 2433, i32 2435 }, %struct.Range { i32 2492, i32 2492 }, %struct.Range { i32 2494, i32 2509 }, %struct.Range { i32 2519, i32 2519 }, %struct.Range { i32 2530, i32 2531 }, %struct.Range { i32 2561, i32 2563 }, %struct.Range { i32 2620, i32 2641 }, %struct.Range { i32 2672, i32 2673 }, %struct.Range { i32 2677, i32 2677 }, %struct.Range { i32 2689, i32 2691 }, %struct.Range { i32 2748, i32 2748 }, %struct.Range { i32 2750, i32 2765 }, %struct.Range { i32 2786, i32 2787 }, %struct.Range { i32 2817, i32 2819 }, %struct.Range { i32 2876, i32 2876 }, %struct.Range { i32 2878, i32 2903 }, %struct.Range { i32 2914, i32 2915 }, %struct.Range { i32 2946, i32 2946 }, %struct.Range { i32 3006, i32 3021 }, %struct.Range { i32 3031, i32 3031 }, %struct.Range { i32 3072, i32 3076 }, %struct.Range { i32 3132, i32 3132 }, %struct.Range { i32 3134, i32 3158 }, %struct.Range { i32 3170, i32 3171 }, %struct.Range { i32 3201, i32 3203 }, %struct.Range { i32 3260, i32 3260 }, %struct.Range { i32 3262, i32 3286 }, %struct.Range { i32 3298, i32 3299 }, %struct.Range { i32 3328, i32 3331 }, %struct.Range { i32 3387, i32 3388 }, %struct.Range { i32 3390, i32 3405 }, %struct.Range { i32 3415, i32 3415 }, %struct.Range { i32 3426, i32 3427 }, %struct.Range { i32 3457, i32 3459 }, %struct.Range { i32 3530, i32 3551 }, %struct.Range { i32 3570, i32 3571 }, %struct.Range { i32 3633, i32 3633 }, %struct.Range { i32 3635, i32 3642 }, %struct.Range { i32 3655, i32 3662 }, %struct.Range { i32 3761, i32 3761 }, %struct.Range { i32 3763, i32 3772 }, %struct.Range { i32 3784, i32 3790 }, %struct.Range { i32 3864, i32 3865 }, %struct.Range { i32 3893, i32 3893 }, %struct.Range { i32 3895, i32 3895 }, %struct.Range { i32 3897, i32 3897 }, %struct.Range { i32 3902, i32 3903 }, %struct.Range { i32 3953, i32 3972 }, %struct.Range { i32 3974, i32 3975 }, %struct.Range { i32 3981, i32 4028 }, %struct.Range { i32 4038, i32 4038 }, %struct.Range { i32 4139, i32 4158 }, %struct.Range { i32 4182, i32 4185 }, %struct.Range { i32 4190, i32 4192 }, %struct.Range { i32 4194, i32 4196 }, %struct.Range { i32 4199, i32 4205 }, %struct.Range { i32 4209, i32 4212 }, %struct.Range { i32 4226, i32 4237 }, %struct.Range { i32 4239, i32 4239 }, %struct.Range { i32 4250, i32 4253 }, %struct.Range { i32 4957, i32 4959 }, %struct.Range { i32 5906, i32 5909 }, %struct.Range { i32 5938, i32 5940 }, %struct.Range { i32 5970, i32 5971 }, %struct.Range { i32 6002, i32 6003 }, %struct.Range { i32 6068, i32 6099 }, %struct.Range { i32 6109, i32 6109 }, %struct.Range { i32 6155, i32 6157 }, %struct.Range { i32 6159, i32 6159 }, %struct.Range { i32 6277, i32 6278 }, %struct.Range { i32 6313, i32 6313 }, %struct.Range { i32 6432, i32 6459 }, %struct.Range { i32 6679, i32 6683 }, %struct.Range { i32 6741, i32 6783 }, %struct.Range { i32 6832, i32 6911 }, %struct.Range { i32 6912, i32 6916 }, %struct.Range { i32 6964, i32 6980 }, %struct.Range { i32 7019, i32 7027 }, %struct.Range { i32 7040, i32 7042 }, %struct.Range { i32 7073, i32 7085 }, %struct.Range { i32 7142, i32 7155 }, %struct.Range { i32 7204, i32 7223 }, %struct.Range { i32 7376, i32 7378 }, %struct.Range { i32 7380, i32 7400 }, %struct.Range { i32 7405, i32 7405 }, %struct.Range { i32 7412, i32 7412 }, %struct.Range { i32 7415, i32 7417 }, %struct.Range { i32 7616, i32 7679 }, %struct.Range { i32 8204, i32 8204 }, %struct.Range { i32 8400, i32 8432 }, %struct.Range { i32 11503, i32 11505 }, %struct.Range { i32 11647, i32 11647 }, %struct.Range { i32 11744, i32 11775 }, %struct.Range { i32 12330, i32 12335 }, %struct.Range { i32 12441, i32 12442 }, %struct.Range { i32 42607, i32 42610 }, %struct.Range { i32 42612, i32 42621 }, %struct.Range { i32 42654, i32 42655 }, %struct.Range { i32 42736, i32 42737 }, %struct.Range { i32 43010, i32 43010 }, %struct.Range { i32 43014, i32 43014 }, %struct.Range { i32 43019, i32 43019 }, %struct.Range { i32 43043, i32 43047 }, %struct.Range { i32 43052, i32 43052 }, %struct.Range { i32 43136, i32 43137 }, %struct.Range { i32 43188, i32 43205 }, %struct.Range { i32 43232, i32 43249 }, %struct.Range { i32 43263, i32 43263 }, %struct.Range { i32 43302, i32 43309 }, %struct.Range { i32 43335, i32 43347 }, %struct.Range { i32 43392, i32 43395 }, %struct.Range { i32 43443, i32 43456 }, %struct.Range { i32 43561, i32 43574 }, %struct.Range { i32 43587, i32 43587 }, %struct.Range { i32 43596, i32 43597 }, %struct.Range { i32 43755, i32 43759 }, %struct.Range { i32 43765, i32 43766 }, %struct.Range { i32 44003, i32 44010 }, %struct.Range { i32 44012, i32 44013 }, %struct.Range { i32 64286, i32 64286 }, %struct.Range { i32 65024, i32 65039 }, %struct.Range { i32 65056, i32 65071 }, %struct.Range { i32 65438, i32 65439 }, %struct.Range { i32 127995, i32 127999 }, %struct.Range { i32 917536, i32 917631 }, %struct.Range { i32 917760, i32 917999 }], align 16
@PICTOGRAPHIC = internal unnamed_addr constant [22 x %struct.Range] [%struct.Range { i32 169, i32 169 }, %struct.Range { i32 174, i32 174 }, %struct.Range { i32 8252, i32 8252 }, %struct.Range { i32 8265, i32 8265 }, %struct.Range { i32 8482, i32 8482 }, %struct.Range { i32 8505, i32 8505 }, %struct.Range { i32 8596, i32 8618 }, %struct.Range { i32 8986, i32 9215 }, %struct.Range { i32 9410, i32 9410 }, %struct.Range { i32 9642, i32 9726 }, %struct.Range { i32 9728, i32 10175 }, %struct.Range { i32 10548, i32 10549 }, %struct.Range { i32 11013, i32 11093 }, %struct.Range { i32 12336, i32 12336 }, %struct.Range { i32 12349, i32 12349 }, %struct.Range { i32 12951, i32 12951 }, %struct.Range { i32 12953, i32 12953 }, %struct.Range { i32 126976, i32 127231 }, %struct.Range { i32 127245, i32 127405 }, %struct.Range { i32 127489, i32 127994 }, %struct.Range { i32 128000, i32 129791 }, %struct.Range { i32 130048, i32 131069 }], align 16
@free_spans = internal unnamed_addr global ptr null, align 8
@.str.36 = private unnamed_addr constant [7 x i8] c"stop: \00", align 1
@.str.37 = private unnamed_addr constant [7 x i8] c"\0A  at \00", align 1
@on_fault.message = internal constant [22 x i8] c"stop: stack overflow\0A\00", align 16

; Function Attrs: nounwind optsize
define void @fa_flush() local_unnamed_addr #0 {
  %1 = load i64, ptr @output_length, align 8, !tbaa !2
  %2 = icmp eq i64 %1, 0
  br i1 %2, label %4, label %3

3:                                                ; preds = %0
  tail call void @fa_write(i32 noundef 1, ptr noundef nonnull @output, i64 noundef %1) #24
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
  tail call void @fa_write(i32 noundef 1, ptr noundef nonnull @output, i64 noundef %5) #24
  store i64 0, ptr @output_length, align 8, !tbaa !2
  br label %9

9:                                                ; preds = %6, %8
  tail call void @fa_write(i32 noundef %0, ptr noundef %1, i64 noundef %2) #24
  br label %23

10:                                               ; preds = %3
  %11 = add i64 %5, %2
  %12 = icmp ugt i64 %11, 8192
  br i1 %12, label %13, label %19

13:                                               ; preds = %10
  %14 = icmp eq i64 %5, 0
  br i1 %14, label %16, label %15

15:                                               ; preds = %13
  tail call void @fa_write(i32 noundef 1, ptr noundef nonnull @output, i64 noundef %5) #24
  store i64 0, ptr @output_length, align 8, !tbaa !2
  br label %16

16:                                               ; preds = %13, %15
  %17 = icmp ugt i64 %2, 8192
  br i1 %17, label %18, label %19

18:                                               ; preds = %16
  tail call void @fa_write(i32 noundef 1, ptr noundef %1, i64 noundef %2) #24
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
  tail call void @fa_flush() #25
  tail call void @fa_exit(i32 noundef %0) #26
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
  tail call fastcc void @reserve(i64 noundef %1) #25
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
  br i1 %5, label %6, label %22

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
  %15 = tail call ptr @fa_alloc(i64 noundef %11, i64 noundef 1) #24
  %16 = load ptr, ptr @builder, align 8, !tbaa !6
  %17 = load i64, ptr @builder_length, align 8, !tbaa !2
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %15, ptr align 1 %16, i64 %17, i1 false)
  %18 = icmp eq ptr %16, null
  br i1 %18, label %21, label %19

19:                                               ; preds = %14
  %20 = load i64, ptr @builder_capacity, align 8, !tbaa !2
  tail call void @fa_free(ptr noundef nonnull %16, i64 noundef %20, i64 noundef 1) #24
  br label %21

21:                                               ; preds = %19, %14
  store ptr %15, ptr @builder, align 8, !tbaa !6
  store i64 %11, ptr @builder_capacity, align 8, !tbaa !2
  br label %22

22:                                               ; preds = %1, %21
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_b_line(i32 noundef %0) local_unnamed_addr #0 {
  tail call fastcc void @reserve(i64 noundef 1) #25
  %2 = load ptr, ptr @builder, align 8, !tbaa !6
  %3 = load i64, ptr @builder_length, align 8, !tbaa !2
  %4 = getelementptr inbounds i8, ptr %2, i64 %3
  store i8 10, ptr %4, align 1, !tbaa !10
  %5 = add i64 %3, 1
  store i64 %5, ptr @builder_length, align 8, !tbaa !2
  tail call void @fa_out(i32 noundef %0, ptr noundef %2, i64 noundef %5) #25
  store i64 0, ptr @builder_length, align 8, !tbaa !2
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_b_take(ptr nocapture noundef writeonly %0) local_unnamed_addr #0 {
  %2 = load i64, ptr @builder_length, align 8, !tbaa !2
  %3 = icmp eq i64 %2, 0
  br i1 %3, label %7, label %4

4:                                                ; preds = %1
  %5 = load ptr, ptr @builder, align 8, !tbaa !6
  %6 = tail call ptr @fa_alloc(i64 noundef %2, i64 noundef 1) #24, !noalias !11
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %6, ptr align 1 %5, i64 %2, i1 false), !noalias !11
  br label %7

7:                                                ; preds = %1, %4
  %8 = phi ptr [ %6, %4 ], [ @.str.29, %1 ]
  store ptr %8, ptr %0, align 8, !tbaa.struct !14
  %9 = getelementptr inbounds i8, ptr %0, i64 8
  store i64 %2, ptr %9, align 8, !tbaa.struct !15
  %10 = getelementptr inbounds i8, ptr %0, i64 16
  store i64 %2, ptr %10, align 8, !tbaa.struct !16
  store i64 0, ptr @builder_length, align 8, !tbaa !2
  ret void
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.start.p0(i64 immarg, ptr nocapture) #5

; Function Attrs: mustprogress nocallback nofree nounwind willreturn memory(argmem: readwrite)
declare void @llvm.memcpy.p0.p0.i64(ptr noalias nocapture writeonly, ptr noalias nocapture readonly, i64, i1 immarg) #6

; Function Attrs: mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.end.p0(i64 immarg, ptr nocapture) #5

; Function Attrs: nounwind optsize
define void @fa_b_i64(i64 noundef %0) local_unnamed_addr #0 {
  %2 = tail call i64 @llvm.abs.i64(i64 %0, i1 false)
  %3 = lshr i64 %0, 63
  %4 = trunc i64 %3 to i32
  tail call fastcc void @append_u64(i64 noundef %2, i32 noundef %4) #25
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @append_u64(i64 noundef %0, i32 noundef %1) unnamed_addr #0 {
  %3 = alloca [24 x i8], align 16
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %3) #27
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
  br i1 %13, label %14, label %4, !llvm.loop !17

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
  tail call fastcc void @reserve(i64 noundef %22) #25
  %23 = load ptr, ptr @builder, align 8, !tbaa !6
  %24 = load i64, ptr @builder_length, align 8, !tbaa !2
  %25 = getelementptr inbounds i8, ptr %23, i64 %24
  call void @llvm.memcpy.p0.p0.i64(ptr align 1 %25, ptr nonnull align 1 %21, i64 %22, i1 false)
  %26 = add i64 %24, %22
  store i64 %26, ptr @builder_length, align 8, !tbaa !2
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %3) #27
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_b_u64(i64 noundef %0) local_unnamed_addr #0 {
  tail call fastcc void @append_u64(i64 noundef %0, i32 noundef 0) #25
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
  tail call fastcc void @append_u128(i64 noundef %12, i64 noundef %14, i32 noundef %9) #25
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @append_u128(i64 noundef %0, i64 noundef %1, i32 noundef %2) unnamed_addr #0 {
  %4 = alloca [4 x i32], align 16
  %5 = alloca [48 x i8], align 16
  %6 = icmp eq i64 %1, 0
  br i1 %6, label %7, label %8

7:                                                ; preds = %3
  tail call fastcc void @append_u64(i64 noundef %0, i32 noundef %2) #25
  br label %70

8:                                                ; preds = %3
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %4) #27
  %9 = lshr i64 %1, 32
  %10 = trunc i64 %9 to i32
  store i32 %10, ptr %4, align 16, !tbaa !18
  %11 = getelementptr inbounds i32, ptr %4, i64 1
  %12 = trunc i64 %1 to i32
  store i32 %12, ptr %11, align 4, !tbaa !18
  %13 = getelementptr inbounds i32, ptr %4, i64 2
  %14 = lshr i64 %0, 32
  %15 = trunc i64 %14 to i32
  store i32 %15, ptr %13, align 8, !tbaa !18
  %16 = getelementptr inbounds i32, ptr %4, i64 3
  %17 = trunc i64 %0 to i32
  store i32 %17, ptr %16, align 4, !tbaa !18
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %5) #27
  br label %18

18:                                               ; preds = %40, %8
  %19 = phi i64 [ 48, %8 ], [ %45, %40 ]
  br label %20

20:                                               ; preds = %20, %18
  %21 = phi i64 [ 0, %18 ], [ %31, %20 ]
  %22 = phi i64 [ 0, %18 ], [ %30, %20 ]
  %23 = shl nuw nsw i64 %22, 32
  %24 = getelementptr inbounds i32, ptr %4, i64 %21
  %25 = load i32, ptr %24, align 4, !tbaa !18
  %26 = zext i32 %25 to i64
  %27 = or disjoint i64 %23, %26
  %28 = udiv i64 %27, 1000000000
  %29 = trunc i64 %28 to i32
  store i32 %29, ptr %24, align 4, !tbaa !18
  %30 = urem i64 %27, 1000000000
  %31 = add nuw nsw i64 %21, 1
  %32 = icmp eq i64 %31, 4
  br i1 %32, label %33, label %20, !llvm.loop !20

33:                                               ; preds = %20
  %34 = trunc i64 %30 to i32
  %35 = load <4 x i32>, ptr %4, align 16, !tbaa !18
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
  br i1 %55, label %41, label %40, !llvm.loop !21

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
  tail call fastcc void @reserve(i64 noundef %65) #25
  %66 = load ptr, ptr @builder, align 8, !tbaa !6
  %67 = load i64, ptr @builder_length, align 8, !tbaa !2
  %68 = getelementptr inbounds i8, ptr %66, i64 %67
  call void @llvm.memcpy.p0.p0.i64(ptr align 1 %68, ptr nonnull align 1 %64, i64 %65, i1 false)
  %69 = add i64 %67, %65
  store i64 %69, ptr @builder_length, align 8, !tbaa !2
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %5) #27
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %4) #27
  br label %70

70:                                               ; preds = %62, %7
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_b_u128(i64 noundef %0, i64 noundef %1) local_unnamed_addr #0 {
  tail call fastcc void @append_u128(i64 noundef %0, i64 noundef %1, i32 noundef 0) #25
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
  br i1 %8, label %10, label %4, !llvm.loop !22

10:                                               ; preds = %4
  tail call fastcc void @reserve(i64 noundef %5) #25
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
  br i1 %6, label %8, label %2, !llvm.loop !22

8:                                                ; preds = %2
  tail call fastcc void @reserve(i64 noundef %3) #25
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
  tail call fastcc void @append_double(double noundef %0) #25
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @append_double(double noundef %0) unnamed_addr #0 {
  %2 = alloca [40 x i8], align 16
  %3 = alloca [24 x i8], align 16
  %4 = fcmp ord double %0, 0.000000e+00
  br i1 %4, label %10, label %5

5:                                                ; preds = %1
  tail call fastcc void @reserve(i64 noundef 3) #25
  %6 = load ptr, ptr @builder, align 8, !tbaa !6
  %7 = load i64, ptr @builder_length, align 8, !tbaa !2
  %8 = getelementptr inbounds i8, ptr %6, i64 %7
  tail call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(3) %8, ptr noundef nonnull align 1 dereferenceable(3) @.str.30, i64 3, i1 false)
  %9 = add i64 %7, 3
  store i64 %9, ptr @builder_length, align 8, !tbaa !2
  br label %188

10:                                               ; preds = %1
  %11 = fcmp oeq double %0, 0.000000e+00
  br i1 %11, label %12, label %17

12:                                               ; preds = %10
  tail call fastcc void @reserve(i64 noundef 1) #25
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
  tail call fastcc void @reserve(i64 noundef 1) #25
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
  tail call fastcc void @reserve(i64 noundef 8) #25
  %29 = load ptr, ptr @builder, align 8, !tbaa !6
  %30 = load i64, ptr @builder_length, align 8, !tbaa !2
  %31 = getelementptr inbounds i8, ptr %29, i64 %30
  store i64 8751735898823355977, ptr %31, align 1
  %32 = add i64 %30, 8
  store i64 %32, ptr @builder_length, align 8, !tbaa !2
  br label %188

33:                                               ; preds = %25
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %3) #27
  call void @llvm.lifetime.start.p0(i64 40, ptr nonnull %2) #27
  br label %34

34:                                               ; preds = %34, %33
  %35 = phi i32 [ 1, %33 ], [ %40, %34 ]
  %36 = add nsw i32 %35, -1
  %37 = call i32 (ptr, i64, ptr, ...) @snprintf(ptr noundef nonnull %2, i64 noundef 40, ptr noundef nonnull @.str.33, i32 noundef %36, double noundef %26) #24
  %38 = call double @strtod(ptr noundef nonnull %2, ptr noundef null) #24
  %39 = fcmp oeq double %38, %26
  %40 = add nuw nsw i32 %35, 1
  %41 = icmp eq i32 %40, 18
  %42 = select i1 %39, i1 true, i1 %41
  br i1 %42, label %43, label %34, !llvm.loop !23

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
  br i1 %60, label %61, label %46, !llvm.loop !24

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
  br i1 %83, label %69, label %73, !llvm.loop !25

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
  br i1 %92, label %84, label %93, !llvm.loop !26

93:                                               ; preds = %84, %88
  %94 = phi i32 [ %72, %84 ], [ %86, %88 ]
  %95 = icmp eq i8 %65, 45
  %96 = sub nsw i32 0, %70
  %97 = select i1 %95, i32 %96, i32 %70
  %98 = add nsw i32 %97, 1
  call void @llvm.lifetime.end.p0(i64 40, ptr nonnull %2) #27
  %99 = icmp sle i32 %94, %98
  %100 = icmp slt i32 %97, 21
  %101 = and i1 %100, %99
  br i1 %101, label %102, label %117

102:                                              ; preds = %93
  %103 = sext i32 %94 to i64
  call fastcc void @reserve(i64 noundef %103) #25
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
  call fastcc void @reserve(i64 noundef 1) #25
  %111 = load ptr, ptr @builder, align 8, !tbaa !6
  %112 = load i64, ptr @builder_length, align 8, !tbaa !2
  %113 = getelementptr inbounds i8, ptr %111, i64 %112
  store i8 48, ptr %113, align 1, !tbaa !10
  %114 = add i64 %112, 1
  store i64 %114, ptr @builder_length, align 8, !tbaa !2
  %115 = add i32 %110, 1
  %116 = icmp eq i32 %110, %97
  br i1 %116, label %187, label %109, !llvm.loop !27

117:                                              ; preds = %93
  %118 = icmp ult i32 %97, 21
  br i1 %118, label %119, label %136

119:                                              ; preds = %117
  %120 = zext nneg i32 %98 to i64
  call fastcc void @reserve(i64 noundef %120) #25
  %121 = load ptr, ptr @builder, align 8, !tbaa !6
  %122 = load i64, ptr @builder_length, align 8, !tbaa !2
  %123 = getelementptr inbounds i8, ptr %121, i64 %122
  call void @llvm.memcpy.p0.p0.i64(ptr align 1 %123, ptr nonnull align 16 %3, i64 %120, i1 false)
  %124 = add i64 %122, %120
  store i64 %124, ptr @builder_length, align 8, !tbaa !2
  call fastcc void @reserve(i64 noundef 1) #25
  %125 = load ptr, ptr @builder, align 8, !tbaa !6
  %126 = load i64, ptr @builder_length, align 8, !tbaa !2
  %127 = getelementptr inbounds i8, ptr %125, i64 %126
  store i8 46, ptr %127, align 1, !tbaa !10
  %128 = add i64 %126, 1
  store i64 %128, ptr @builder_length, align 8, !tbaa !2
  %129 = getelementptr inbounds i8, ptr %3, i64 %120
  %130 = sub nsw i32 %94, %98
  %131 = sext i32 %130 to i64
  call fastcc void @reserve(i64 noundef %131) #25
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
  call fastcc void @append_c(ptr noundef nonnull @.str.32) #25
  %139 = icmp eq i32 %97, -1
  br i1 %139, label %140, label %146

140:                                              ; preds = %146, %138
  %141 = sext i32 %94 to i64
  call fastcc void @reserve(i64 noundef %141) #25
  %142 = load ptr, ptr @builder, align 8, !tbaa !6
  %143 = load i64, ptr @builder_length, align 8, !tbaa !2
  %144 = getelementptr inbounds i8, ptr %142, i64 %143
  call void @llvm.memcpy.p0.p0.i64(ptr align 1 %144, ptr nonnull align 16 %3, i64 %141, i1 false)
  %145 = add i64 %143, %141
  store i64 %145, ptr @builder_length, align 8, !tbaa !2
  br label %187

146:                                              ; preds = %138, %146
  %147 = phi i32 [ %152, %146 ], [ 0, %138 ]
  call fastcc void @reserve(i64 noundef 1) #25
  %148 = load ptr, ptr @builder, align 8, !tbaa !6
  %149 = load i64, ptr @builder_length, align 8, !tbaa !2
  %150 = getelementptr inbounds i8, ptr %148, i64 %149
  store i8 48, ptr %150, align 1, !tbaa !10
  %151 = add i64 %149, 1
  store i64 %151, ptr @builder_length, align 8, !tbaa !2
  %152 = add nuw nsw i32 %147, 1
  %153 = xor i32 %97, %152
  %154 = icmp eq i32 %153, -1
  br i1 %154, label %140, label %146, !llvm.loop !28

155:                                              ; preds = %136
  %156 = load i8, ptr %3, align 16, !tbaa !10
  call fastcc void @reserve(i64 noundef 1) #25
  %157 = load ptr, ptr @builder, align 8, !tbaa !6
  %158 = load i64, ptr @builder_length, align 8, !tbaa !2
  %159 = getelementptr inbounds i8, ptr %157, i64 %158
  store i8 %156, ptr %159, align 1, !tbaa !10
  %160 = add i64 %158, 1
  store i64 %160, ptr @builder_length, align 8, !tbaa !2
  %161 = icmp sgt i32 %94, 1
  br i1 %161, label %162, label %174

162:                                              ; preds = %155
  call fastcc void @reserve(i64 noundef 1) #25
  %163 = load ptr, ptr @builder, align 8, !tbaa !6
  %164 = load i64, ptr @builder_length, align 8, !tbaa !2
  %165 = getelementptr inbounds i8, ptr %163, i64 %164
  store i8 46, ptr %165, align 1, !tbaa !10
  %166 = add i64 %164, 1
  store i64 %166, ptr @builder_length, align 8, !tbaa !2
  %167 = getelementptr inbounds i8, ptr %3, i64 1
  %168 = add nsw i32 %94, -1
  %169 = zext nneg i32 %168 to i64
  call fastcc void @reserve(i64 noundef %169) #25
  %170 = load ptr, ptr @builder, align 8, !tbaa !6
  %171 = load i64, ptr @builder_length, align 8, !tbaa !2
  %172 = getelementptr inbounds i8, ptr %170, i64 %171
  call void @llvm.memcpy.p0.p0.i64(ptr align 1 %172, ptr nonnull align 1 %167, i64 %169, i1 false)
  %173 = add i64 %171, %169
  store i64 %173, ptr @builder_length, align 8, !tbaa !2
  br label %174

174:                                              ; preds = %162, %155
  call fastcc void @reserve(i64 noundef 1) #25
  %175 = load ptr, ptr @builder, align 8, !tbaa !6
  %176 = load i64, ptr @builder_length, align 8, !tbaa !2
  %177 = getelementptr inbounds i8, ptr %175, i64 %176
  store i8 101, ptr %177, align 1, !tbaa !10
  %178 = add i64 %176, 1
  store i64 %178, ptr @builder_length, align 8, !tbaa !2
  %179 = icmp slt i32 %97, 0
  %180 = select i1 %179, i8 45, i8 43
  call fastcc void @reserve(i64 noundef 1) #25
  %181 = load ptr, ptr @builder, align 8, !tbaa !6
  %182 = load i64, ptr @builder_length, align 8, !tbaa !2
  %183 = getelementptr inbounds i8, ptr %181, i64 %182
  store i8 %180, ptr %183, align 1, !tbaa !10
  %184 = add i64 %182, 1
  store i64 %184, ptr @builder_length, align 8, !tbaa !2
  %185 = call i32 @llvm.abs.i32(i32 %70, i1 true)
  %186 = zext nneg i32 %185 to i64
  call fastcc void @append_u64(i64 noundef %186, i32 noundef 0) #25
  br label %187

187:                                              ; preds = %109, %102, %119, %174, %140
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %3) #27
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
  tail call fastcc void @append_double(double noundef %9) #25
  br label %107

11:                                               ; preds = %1
  call void @llvm.lifetime.start.p0(i64 40, ptr nonnull %3) #27
  %12 = getelementptr inbounds i64, ptr %4, i64 1
  %13 = getelementptr inbounds i64, ptr %4, i64 2
  %14 = insertelement <2 x double> poison, double %9, i64 0
  %15 = shufflevector <2 x double> %14, <2 x double> poison, <2 x i32> zeroinitializer
  br label %16

16:                                               ; preds = %102, %11
  %17 = phi i32 [ 1, %11 ], [ %103, %102 ]
  %18 = add nsw i32 %17, -1
  %19 = call i32 (ptr, i64, ptr, ...) @snprintf(ptr noundef nonnull %3, i64 noundef 40, ptr noundef nonnull @.str.33, i32 noundef %18, double noundef %9) #24
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
  br i1 %38, label %39, label %26, !llvm.loop !29

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
  br i1 %58, label %59, label %48, !llvm.loop !30

59:                                               ; preds = %48, %39
  %60 = phi i32 [ 0, %39 ], [ %55, %48 ]
  %61 = sub nsw i32 0, %60
  %62 = select i1 %44, i32 %61, i32 %60
  %63 = sub nsw i32 %62, %18
  %64 = sub nsw i64 0, %41
  %65 = select i1 %21, i64 %64, i64 %41
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %4) #27
  store i64 %65, ptr %4, align 16, !tbaa !2
  %66 = add nsw i64 %65, -1
  store i64 %66, ptr %12, align 8, !tbaa !2
  %67 = add nsw i64 %65, 1
  store i64 %67, ptr %13, align 16, !tbaa !2
  br label %71

68:                                               ; preds = %94
  %69 = add nuw nsw i64 %72, 1
  %70 = icmp eq i64 %69, 3
  br i1 %70, label %100, label %71, !llvm.loop !31

71:                                               ; preds = %68, %59
  %72 = phi i64 [ 0, %59 ], [ %69, %68 ]
  %73 = phi double [ 0.000000e+00, %59 ], [ %96, %68 ]
  %74 = phi i32 [ 0, %59 ], [ %95, %68 ]
  %75 = getelementptr inbounds [3 x i64], ptr %4, i64 0, i64 %72
  %76 = load i64, ptr %75, align 8, !tbaa !2
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %2) #27
  %77 = call i32 (ptr, i64, ptr, ...) @snprintf(ptr noundef nonnull %2, i64 noundef 48, ptr noundef nonnull @.str.34, i64 noundef %76, i32 noundef %63) #24
  %78 = call double @strtod(ptr noundef nonnull %2, ptr noundef null) #24
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %2) #27
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
  call fastcc void @append_double(double noundef %96) #25
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %4) #27
  br label %106

102:                                              ; preds = %100
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %4) #27
  %103 = add nuw nsw i32 %17, 1
  %104 = icmp eq i32 %103, 10
  br i1 %104, label %105, label %16, !llvm.loop !32

105:                                              ; preds = %102
  call fastcc void @append_double(double noundef %9) #25
  br label %106

106:                                              ; preds = %105, %101
  call void @llvm.lifetime.end.p0(i64 40, ptr nonnull %3) #27
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
  tail call fastcc void @append_double(double noundef %3) #25
  br label %12

9:                                                ; preds = %1
  call void @llvm.lifetime.start.p0(i64 40, ptr nonnull %2) #27
  %10 = call i32 (ptr, i64, ptr, ...) @snprintf(ptr noundef nonnull %2, i64 noundef 40, ptr noundef nonnull @.str.35, double noundef %3) #24
  %11 = call double @strtod(ptr noundef nonnull %2, ptr noundef null) #24
  call fastcc void @append_double(double noundef %11) #25
  call void @llvm.lifetime.end.p0(i64 40, ptr nonnull %2) #27
  br label %12

12:                                               ; preds = %8, %9
  call fastcc void @reserve(i64 noundef 1) #25
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
  tail call fastcc void @reserve(i64 noundef 1) #25
  %3 = load ptr, ptr @builder, align 8, !tbaa !6
  %4 = load i64, ptr @builder_length, align 8, !tbaa !2
  %5 = getelementptr inbounds i8, ptr %3, i64 %4
  store i8 34, ptr %5, align 1, !tbaa !10
  %6 = add i64 %4, 1
  store i64 %6, ptr @builder_length, align 8, !tbaa !2
  %7 = icmp eq i64 %1, 0
  br i1 %7, label %8, label %13

8:                                                ; preds = %88, %2
  tail call fastcc void @reserve(i64 noundef 1) #25
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
  tail call fastcc void @reserve(i64 noundef 2) #25
  %19 = load ptr, ptr @builder, align 8, !tbaa !6
  %20 = load i64, ptr @builder_length, align 8, !tbaa !2
  %21 = getelementptr inbounds i8, ptr %19, i64 %20
  store i16 32125, ptr %21, align 1
  %22 = add i64 %20, 2
  br label %88

23:                                               ; preds = %13
  tail call fastcc void @reserve(i64 noundef 2) #25
  %24 = load ptr, ptr @builder, align 8, !tbaa !6
  %25 = load i64, ptr @builder_length, align 8, !tbaa !2
  %26 = getelementptr inbounds i8, ptr %24, i64 %25
  store i16 31611, ptr %26, align 1
  %27 = add i64 %25, 2
  br label %88

28:                                               ; preds = %13
  tail call fastcc void @reserve(i64 noundef 2) #25
  %29 = load ptr, ptr @builder, align 8, !tbaa !6
  %30 = load i64, ptr @builder_length, align 8, !tbaa !2
  %31 = getelementptr inbounds i8, ptr %29, i64 %30
  store i16 12380, ptr %31, align 1
  %32 = add i64 %30, 2
  br label %88

33:                                               ; preds = %13
  tail call fastcc void @reserve(i64 noundef 2) #25
  %34 = load ptr, ptr @builder, align 8, !tbaa !6
  %35 = load i64, ptr @builder_length, align 8, !tbaa !2
  %36 = getelementptr inbounds i8, ptr %34, i64 %35
  store i16 29788, ptr %36, align 1
  %37 = add i64 %35, 2
  br label %88

38:                                               ; preds = %13
  tail call fastcc void @reserve(i64 noundef 2) #25
  %39 = load ptr, ptr @builder, align 8, !tbaa !6
  %40 = load i64, ptr @builder_length, align 8, !tbaa !2
  %41 = getelementptr inbounds i8, ptr %39, i64 %40
  store i16 29276, ptr %41, align 1
  %42 = add i64 %40, 2
  br label %88

43:                                               ; preds = %13
  tail call fastcc void @reserve(i64 noundef 2) #25
  %44 = load ptr, ptr @builder, align 8, !tbaa !6
  %45 = load i64, ptr @builder_length, align 8, !tbaa !2
  %46 = getelementptr inbounds i8, ptr %44, i64 %45
  store i16 28252, ptr %46, align 1
  %47 = add i64 %45, 2
  br label %88

48:                                               ; preds = %13
  tail call fastcc void @reserve(i64 noundef 2) #25
  %49 = load ptr, ptr @builder, align 8, !tbaa !6
  %50 = load i64, ptr @builder_length, align 8, !tbaa !2
  %51 = getelementptr inbounds i8, ptr %49, i64 %50
  store i16 8796, ptr %51, align 1
  %52 = add i64 %50, 2
  br label %88

53:                                               ; preds = %13
  tail call fastcc void @reserve(i64 noundef 2) #25
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
  tail call fastcc void @reserve(i64 noundef 2) #25
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
  tail call fastcc void @reserve(i64 noundef 1) #25
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
  tail call fastcc void @reserve(i64 noundef 1) #25
  %79 = load ptr, ptr @builder, align 8, !tbaa !6
  %80 = load i64, ptr @builder_length, align 8, !tbaa !2
  %81 = getelementptr inbounds i8, ptr %79, i64 %80
  store i8 %78, ptr %81, align 1, !tbaa !10
  %82 = add i64 %80, 1
  br label %88

83:                                               ; preds = %58
  tail call fastcc void @reserve(i64 noundef 1) #25
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
  br i1 %91, label %8, label %13, !llvm.loop !33
}

; Function Attrs: nounwind optsize
define void @fa_b_string_array(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  tail call fastcc void @reserve(i64 noundef 1) #25
  %3 = load ptr, ptr @builder, align 8, !tbaa !6
  %4 = load i64, ptr @builder_length, align 8, !tbaa !2
  %5 = getelementptr inbounds i8, ptr %3, i64 %4
  store i8 91, ptr %5, align 1, !tbaa !10
  %6 = add i64 %4, 1
  store i64 %6, ptr @builder_length, align 8, !tbaa !2
  %7 = icmp eq i64 %1, 0
  br i1 %7, label %8, label %13

8:                                                ; preds = %21, %2
  tail call fastcc void @reserve(i64 noundef 1) #25
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
  tail call fastcc void @reserve(i64 noundef 2) #25
  %17 = load ptr, ptr @builder, align 8, !tbaa !6
  %18 = load i64, ptr @builder_length, align 8, !tbaa !2
  %19 = getelementptr inbounds i8, ptr %17, i64 %18
  store i16 8236, ptr %19, align 1
  %20 = add i64 %18, 2
  store i64 %20, ptr @builder_length, align 8, !tbaa !2
  br label %21

21:                                               ; preds = %16, %13
  %22 = getelementptr inbounds %struct.FaString, ptr %0, i64 %14
  %23 = load ptr, ptr %22, align 8, !tbaa !34
  %24 = getelementptr inbounds %struct.FaString, ptr %0, i64 %14, i32 1
  %25 = load i64, ptr %24, align 8, !tbaa !36
  tail call void @fa_b_quoted(ptr noundef %23, i64 noundef %25) #25
  %26 = add nuw i64 %14, 1
  %27 = icmp eq i64 %26, %1
  br i1 %27, label %8, label %13, !llvm.loop !37
}

; Function Attrs: nounwind optsize
define void @fa_concat(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef readonly %2, i64 noundef %3, ptr nocapture noundef writeonly %4) local_unnamed_addr #0 {
  %6 = add i64 %3, %1
  %7 = icmp eq i64 %6, 0
  br i1 %7, label %8, label %11

8:                                                ; preds = %5
  store ptr @.str.29, ptr %4, align 8, !tbaa.struct !14
  %9 = getelementptr inbounds i8, ptr %4, i64 8
  store i64 0, ptr %9, align 8, !tbaa.struct !15
  %10 = getelementptr inbounds i8, ptr %4, i64 16
  store i64 0, ptr %10, align 8, !tbaa.struct !16
  br label %16

11:                                               ; preds = %5
  %12 = tail call ptr @fa_alloc(i64 noundef %6, i64 noundef 1) #24
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %12, ptr align 1 %0, i64 %1, i1 false)
  %13 = getelementptr inbounds i8, ptr %12, i64 %1
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %13, ptr align 1 %2, i64 %3, i1 false)
  store ptr %12, ptr %4, align 8, !tbaa !34
  %14 = getelementptr inbounds %struct.FaString, ptr %4, i64 0, i32 1
  store i64 %6, ptr %14, align 8, !tbaa !36
  %15 = getelementptr inbounds %struct.FaString, ptr %4, i64 0, i32 2
  store i64 %6, ptr %15, align 8, !tbaa !38
  br label %16

16:                                               ; preds = %11, %8
  ret void
}

; Function Attrs: optsize
declare ptr @fa_alloc(i64 noundef, i64 noundef) local_unnamed_addr #1

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: read)
define i32 @fa_string_equal(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef readonly %2, i64 noundef %3) local_unnamed_addr #7 {
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
  br i1 %15, label %22, label %16, !llvm.loop !39

16:                                               ; preds = %12
  %17 = getelementptr inbounds i8, ptr %0, i64 %14
  %18 = load i8, ptr %17, align 1, !tbaa !10
  %19 = getelementptr inbounds i8, ptr %2, i64 %14
  %20 = load i8, ptr %19, align 1, !tbaa !10
  %21 = icmp eq i8 %18, %20
  br i1 %21, label %12, label %22, !llvm.loop !39

22:                                               ; preds = %16, %12
  %23 = icmp uge i64 %14, %1
  %24 = zext i1 %23 to i32
  br label %25

25:                                               ; preds = %8, %22, %6, %4
  %26 = phi i32 [ 0, %4 ], [ 1, %6 ], [ 0, %8 ], [ %24, %22 ]
  ret i32 %26
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: read)
define i32 @fa_string_compare(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef readonly %2, i64 noundef %3) local_unnamed_addr #7 {
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
  br i1 %19, label %39, label %20, !llvm.loop !40

20:                                               ; preds = %16
  %21 = getelementptr inbounds i8, ptr %0, i64 %18
  %22 = load i8, ptr %21, align 1, !tbaa !10
  %23 = getelementptr inbounds i8, ptr %2, i64 %18
  %24 = load i8, ptr %23, align 1, !tbaa !10
  %25 = icmp eq i8 %22, %24
  br i1 %25, label %16, label %26, !llvm.loop !40

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
define i64 @fa_next_grapheme(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #8 {
  %4 = tail call fastcc i64 @next_boundary(ptr noundef %0, i64 noundef %1, i64 noundef %2) #25
  ret i64 %4
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define internal fastcc i64 @next_boundary(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2) unnamed_addr #8 {
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
  %25 = call fastcc i32 @decode(ptr noundef nonnull %0, i64 noundef %1, ptr noundef nonnull %4) #25
  %26 = tail call fastcc i32 @break_kind(i32 noundef %25) #25, !range !41
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
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %5) #27
  store i64 %38, ptr %5, align 8, !tbaa !2
  %39 = call fastcc i32 @decode(ptr noundef %0, i64 noundef %1, ptr noundef nonnull %5) #25
  %40 = tail call fastcc i32 @break_kind(i32 noundef %39) #25, !range !41
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
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %5) #27
  br label %106

101:                                              ; preds = %74, %94, %88
  %102 = phi i64 [ %89, %88 ], [ %95, %94 ], [ %75, %74 ]
  %103 = phi i32 [ %92, %88 ], [ %96, %94 ], [ 0, %74 ]
  %104 = phi i32 [ 1, %88 ], [ %99, %94 ], [ 1, %74 ]
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %5) #27
  %105 = icmp ult i64 %102, %1
  br i1 %105, label %34, label %106

106:                                              ; preds = %101, %24, %100, %21
  %107 = phi i64 [ %23, %21 ], [ %38, %100 ], [ %27, %24 ], [ %102, %101 ]
  ret i64 %107
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define i64 @fa_graphemes(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #8 {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %6, %2
  %5 = phi i64 [ 0, %2 ], [ %9, %6 ]
  ret i64 %5

6:                                                ; preds = %2, %6
  %7 = phi i64 [ %10, %6 ], [ 0, %2 ]
  %8 = phi i64 [ %9, %6 ], [ 0, %2 ]
  %9 = add i64 %8, 1
  %10 = tail call fastcc i64 @next_boundary(ptr noundef %0, i64 noundef %1, i64 noundef %7) #25
  %11 = icmp ult i64 %10, %1
  br i1 %11, label %6, label %4, !llvm.loop !42
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define i32 @fa_string_includes(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef readonly %2, i64 noundef %3) local_unnamed_addr #8 {
  %5 = tail call fastcc i64 @find(ptr noundef %0, i64 noundef %1, ptr noundef %2, i64 noundef %3, i32 noundef 0) #25
  %6 = icmp sgt i64 %5, -1
  %7 = zext i1 %6 to i32
  ret i32 %7
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define internal fastcc i64 @find(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef readonly %2, i64 noundef %3, i32 noundef %4) unnamed_addr #8 {
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
  br i1 %22, label %29, label %23, !llvm.loop !43

23:                                               ; preds = %19
  %24 = getelementptr i8, ptr %14, i64 %21
  %25 = load i8, ptr %24, align 1, !tbaa !10
  %26 = getelementptr inbounds i8, ptr %2, i64 %21
  %27 = load i8, ptr %26, align 1, !tbaa !10
  %28 = icmp eq i8 %25, %27
  br i1 %28, label %19, label %29, !llvm.loop !43

29:                                               ; preds = %23, %19
  %30 = icmp ult i64 %21, %3
  br i1 %30, label %51, label %31

31:                                               ; preds = %10, %29
  %32 = icmp eq i64 %12, 0
  br i1 %32, label %37, label %33

33:                                               ; preds = %31, %33
  %34 = phi i64 [ %35, %33 ], [ 0, %31 ]
  %35 = tail call fastcc i64 @next_boundary(ptr noundef %0, i64 noundef %1, i64 noundef %34) #25
  %36 = icmp ult i64 %35, %12
  br i1 %36, label %33, label %37, !llvm.loop !44

37:                                               ; preds = %33, %31
  %38 = phi i64 [ 0, %31 ], [ %35, %33 ]
  %39 = icmp eq i64 %38, %12
  br i1 %39, label %40, label %51

40:                                               ; preds = %37
  %41 = icmp eq i64 %11, 0
  br i1 %41, label %46, label %42

42:                                               ; preds = %40, %42
  %43 = phi i64 [ %44, %42 ], [ 0, %40 ]
  %44 = tail call fastcc i64 @next_boundary(ptr noundef %0, i64 noundef %1, i64 noundef %43) #25
  %45 = icmp ult i64 %44, %11
  br i1 %45, label %42, label %46, !llvm.loop !44

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
  br i1 %55, label %56, label %10, !llvm.loop !45

56:                                               ; preds = %51, %46, %5
  %57 = phi i64 [ -1, %5 ], [ %12, %46 ], [ %52, %51 ]
  ret i64 %57
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define i32 @fa_string_starts_with(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef readonly %2, i64 noundef %3) local_unnamed_addr #8 {
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
  br i1 %15, label %22, label %16, !llvm.loop !43

16:                                               ; preds = %12
  %17 = getelementptr i8, ptr %0, i64 %14
  %18 = load i8, ptr %17, align 1, !tbaa !10
  %19 = getelementptr inbounds i8, ptr %2, i64 %14
  %20 = load i8, ptr %19, align 1, !tbaa !10
  %21 = icmp eq i8 %18, %20
  br i1 %21, label %12, label %22, !llvm.loop !43

22:                                               ; preds = %16, %12
  %23 = icmp ult i64 %14, %3
  br i1 %23, label %32, label %24

24:                                               ; preds = %22, %24
  %25 = phi i64 [ %26, %24 ], [ 0, %22 ]
  %26 = tail call fastcc i64 @next_boundary(ptr noundef nonnull %0, i64 noundef %1, i64 noundef %25) #25
  %27 = icmp ult i64 %26, %3
  br i1 %27, label %24, label %28, !llvm.loop !44

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
define i32 @fa_string_ends_with(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef readonly %2, i64 noundef %3) local_unnamed_addr #8 {
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
  br i1 %17, label %24, label %18, !llvm.loop !43

18:                                               ; preds = %14
  %19 = getelementptr i8, ptr %8, i64 %16
  %20 = load i8, ptr %19, align 1, !tbaa !10
  %21 = getelementptr inbounds i8, ptr %2, i64 %16
  %22 = load i8, ptr %21, align 1, !tbaa !10
  %23 = icmp eq i8 %20, %22
  br i1 %23, label %14, label %24, !llvm.loop !43

24:                                               ; preds = %18, %14
  %25 = icmp ult i64 %16, %3
  br i1 %25, label %36, label %26

26:                                               ; preds = %6, %24
  %27 = icmp eq i64 %1, %3
  br i1 %27, label %32, label %28

28:                                               ; preds = %26, %28
  %29 = phi i64 [ %30, %28 ], [ 0, %26 ]
  %30 = tail call fastcc i64 @next_boundary(ptr noundef %0, i64 noundef %1, i64 noundef %29) #25
  %31 = icmp ult i64 %30, %7
  br i1 %31, label %28, label %32, !llvm.loop !44

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
define i64 @fa_string_index_of(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef readonly %2, i64 noundef %3, i32 noundef %4) local_unnamed_addr #8 {
  %6 = tail call fastcc i64 @find(ptr noundef %0, i64 noundef %1, ptr noundef %2, i64 noundef %3, i32 noundef %4) #25
  %7 = icmp slt i64 %6, 0
  br i1 %7, label %16, label %8

8:                                                ; preds = %5
  %9 = icmp eq i64 %6, 0
  br i1 %9, label %16, label %10

10:                                               ; preds = %8, %10
  %11 = phi i64 [ %14, %10 ], [ 0, %8 ]
  %12 = phi i64 [ %13, %10 ], [ 0, %8 ]
  %13 = add i64 %12, 1
  %14 = tail call fastcc i64 @next_boundary(ptr noundef %0, i64 noundef %1, i64 noundef %11) #25
  %15 = icmp ult i64 %14, %6
  br i1 %15, label %10, label %16, !llvm.loop !46

16:                                               ; preds = %10, %8, %5
  %17 = phi i64 [ -1, %5 ], [ 0, %8 ], [ %13, %10 ]
  ret i64 %17
}

; Function Attrs: nounwind optsize
define void @fa_string_replace(ptr noundef %0, i64 noundef %1, ptr nocapture noundef readonly %2, i64 noundef %3, ptr noundef %4, i64 noundef %5, i32 noundef %6, ptr nocapture noundef writeonly %7) local_unnamed_addr #0 {
  %9 = alloca %struct.Replacing, align 8
  store i64 0, ptr @builder_length, align 8, !tbaa !2
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %9) #27
  store ptr %0, ptr %9, align 8, !tbaa !47
  %10 = getelementptr inbounds %struct.Replacing, ptr %9, i64 0, i32 1
  store i64 0, ptr %10, align 8, !tbaa !49
  %11 = getelementptr inbounds %struct.Replacing, ptr %9, i64 0, i32 2
  store i64 %3, ptr %11, align 8, !tbaa !50
  %12 = getelementptr inbounds %struct.Replacing, ptr %9, i64 0, i32 3
  store ptr %4, ptr %12, align 8, !tbaa !51
  %13 = getelementptr inbounds %struct.Replacing, ptr %9, i64 0, i32 4
  store i64 %5, ptr %13, align 8, !tbaa !52
  %14 = getelementptr inbounds %struct.Replacing, ptr %9, i64 0, i32 5
  store i32 %6, ptr %14, align 8, !tbaa !53
  %15 = getelementptr inbounds %struct.Replacing, ptr %9, i64 0, i32 6
  store i32 0, ptr %15, align 4, !tbaa !54
  call fastcc void @each_match(ptr noundef %0, i64 noundef %1, ptr noundef %2, i64 noundef %3, ptr noundef nonnull @replace_one, ptr noundef nonnull %9) #25
  %16 = load i64, ptr %10, align 8, !tbaa !49
  %17 = getelementptr inbounds i8, ptr %0, i64 %16
  %18 = sub i64 %1, %16
  call fastcc void @reserve(i64 noundef %18) #25
  %19 = load ptr, ptr @builder, align 8, !tbaa !6
  %20 = load i64, ptr @builder_length, align 8, !tbaa !2
  %21 = getelementptr inbounds i8, ptr %19, i64 %20
  call void @llvm.memcpy.p0.p0.i64(ptr align 1 %21, ptr align 1 %17, i64 %18, i1 false)
  %22 = add i64 %20, %18
  store i64 %22, ptr @builder_length, align 8, !tbaa !2
  call void @fa_b_take(ptr noundef %7) #25
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %9) #27
  ret void
}

; Function Attrs: nounwind optsize
define internal fastcc void @each_match(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef readonly %2, i64 noundef %3, ptr nocapture noundef readonly %4, ptr noundef %5) unnamed_addr #0 {
  %7 = icmp ugt i64 %3, %1
  br i1 %7, label %55, label %8

8:                                                ; preds = %6
  %9 = icmp eq i64 %3, 0
  %10 = tail call i64 @llvm.umax.i64(i64 %3, i64 1)
  br label %11

11:                                               ; preds = %8, %50
  %12 = phi i64 [ %3, %8 ], [ %53, %50 ]
  %13 = phi i64 [ 0, %8 ], [ %52, %50 ]
  %14 = getelementptr i8, ptr %0, i64 %13
  br i1 %9, label %31, label %15

15:                                               ; preds = %11
  %16 = load i8, ptr %14, align 1, !tbaa !10
  %17 = load i8, ptr %2, align 1, !tbaa !10
  %18 = icmp eq i8 %16, %17
  br i1 %18, label %19, label %50

19:                                               ; preds = %15, %23
  %20 = phi i64 [ %21, %23 ], [ 0, %15 ]
  %21 = add nuw i64 %20, 1
  %22 = icmp eq i64 %21, %3
  br i1 %22, label %29, label %23, !llvm.loop !43

23:                                               ; preds = %19
  %24 = getelementptr i8, ptr %14, i64 %21
  %25 = load i8, ptr %24, align 1, !tbaa !10
  %26 = getelementptr inbounds i8, ptr %2, i64 %21
  %27 = load i8, ptr %26, align 1, !tbaa !10
  %28 = icmp eq i8 %25, %27
  br i1 %28, label %19, label %29, !llvm.loop !43

29:                                               ; preds = %23, %19
  %30 = icmp ult i64 %21, %3
  br i1 %30, label %50, label %31

31:                                               ; preds = %11, %29
  %32 = icmp eq i64 %13, 0
  br i1 %32, label %37, label %33

33:                                               ; preds = %31, %33
  %34 = phi i64 [ %35, %33 ], [ 0, %31 ]
  %35 = tail call fastcc i64 @next_boundary(ptr noundef %0, i64 noundef %1, i64 noundef %34) #25
  %36 = icmp ult i64 %35, %13
  br i1 %36, label %33, label %37, !llvm.loop !44

37:                                               ; preds = %33, %31
  %38 = phi i64 [ 0, %31 ], [ %35, %33 ]
  %39 = icmp eq i64 %38, %13
  br i1 %39, label %40, label %50

40:                                               ; preds = %37
  %41 = icmp eq i64 %12, 0
  br i1 %41, label %46, label %42

42:                                               ; preds = %40, %42
  %43 = phi i64 [ %44, %42 ], [ 0, %40 ]
  %44 = tail call fastcc i64 @next_boundary(ptr noundef %0, i64 noundef %1, i64 noundef %43) #25
  %45 = icmp ult i64 %44, %12
  br i1 %45, label %42, label %46, !llvm.loop !44

46:                                               ; preds = %42, %40
  %47 = phi i64 [ 0, %40 ], [ %44, %42 ]
  %48 = icmp eq i64 %47, %12
  br i1 %48, label %49, label %50

49:                                               ; preds = %46
  tail call void %4(ptr noundef %5, i64 noundef %13) #24, !callees !55
  br label %50

50:                                               ; preds = %15, %29, %37, %46, %49
  %51 = phi i64 [ %10, %49 ], [ 1, %46 ], [ 1, %37 ], [ 1, %29 ], [ 1, %15 ]
  %52 = add i64 %51, %13
  %53 = add i64 %52, %3
  %54 = icmp ugt i64 %53, %1
  br i1 %54, label %55, label %11, !llvm.loop !56

55:                                               ; preds = %50, %6
  ret void
}

; Function Attrs: nounwind optsize
define internal void @replace_one(ptr nocapture noundef %0, i64 noundef %1) #0 {
  %3 = getelementptr inbounds %struct.Replacing, ptr %0, i64 0, i32 6
  %4 = load i32, ptr %3, align 4, !tbaa !54
  %5 = icmp eq i32 %4, 0
  br i1 %5, label %6, label %31

6:                                                ; preds = %2
  %7 = load ptr, ptr %0, align 8, !tbaa !47
  %8 = getelementptr inbounds %struct.Replacing, ptr %0, i64 0, i32 1
  %9 = load i64, ptr %8, align 8, !tbaa !49
  %10 = getelementptr inbounds i8, ptr %7, i64 %9
  %11 = sub i64 %1, %9
  tail call fastcc void @reserve(i64 noundef %11) #25
  %12 = load ptr, ptr @builder, align 8, !tbaa !6
  %13 = load i64, ptr @builder_length, align 8, !tbaa !2
  %14 = getelementptr inbounds i8, ptr %12, i64 %13
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %14, ptr align 1 %10, i64 %11, i1 false)
  %15 = add i64 %13, %11
  store i64 %15, ptr @builder_length, align 8, !tbaa !2
  %16 = getelementptr inbounds %struct.Replacing, ptr %0, i64 0, i32 3
  %17 = load ptr, ptr %16, align 8, !tbaa !51
  %18 = getelementptr inbounds %struct.Replacing, ptr %0, i64 0, i32 4
  %19 = load i64, ptr %18, align 8, !tbaa !52
  tail call fastcc void @reserve(i64 noundef %19) #25
  %20 = load ptr, ptr @builder, align 8, !tbaa !6
  %21 = load i64, ptr @builder_length, align 8, !tbaa !2
  %22 = getelementptr inbounds i8, ptr %20, i64 %21
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %22, ptr align 1 %17, i64 %19, i1 false)
  %23 = add i64 %21, %19
  store i64 %23, ptr @builder_length, align 8, !tbaa !2
  %24 = getelementptr inbounds %struct.Replacing, ptr %0, i64 0, i32 2
  %25 = load i64, ptr %24, align 8, !tbaa !50
  %26 = add i64 %25, %1
  store i64 %26, ptr %8, align 8, !tbaa !49
  %27 = getelementptr inbounds %struct.Replacing, ptr %0, i64 0, i32 5
  %28 = load i32, ptr %27, align 8, !tbaa !53
  %29 = icmp eq i32 %28, 0
  %30 = zext i1 %29 to i32
  store i32 %30, ptr %3, align 4, !tbaa !54
  br label %31

31:                                               ; preds = %2, %6
  ret void
}

; Function Attrs: nounwind optsize
define ptr @fa_string_characters(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = tail call ptr @fa_alloc(i64 noundef 24, i64 noundef 8) #24
  %4 = tail call ptr @fa_alloc(i64 noundef 96, i64 noundef 16) #24
  store ptr %4, ptr %3, align 8, !tbaa !34
  %5 = getelementptr inbounds %struct.FaArrayHeader, ptr %3, i64 0, i32 1
  store i64 0, ptr %5, align 8, !tbaa !36
  %6 = getelementptr inbounds %struct.FaArrayHeader, ptr %3, i64 0, i32 2
  store i64 4, ptr %6, align 8, !tbaa !38
  %7 = icmp eq i64 %1, 0
  br i1 %7, label %8, label %9

8:                                                ; preds = %18, %2
  ret ptr %3

9:                                                ; preds = %2, %18
  %10 = phi i64 [ %11, %18 ], [ 0, %2 ]
  %11 = tail call fastcc i64 @next_boundary(ptr noundef %0, i64 noundef %1, i64 noundef %10) #25
  %12 = tail call ptr @fa_array_push_slot(ptr noundef nonnull %3, i64 noundef 24) #25
  %13 = sub i64 %11, %10
  %14 = icmp eq i64 %11, %10
  br i1 %14, label %18, label %15

15:                                               ; preds = %9
  %16 = getelementptr inbounds i8, ptr %0, i64 %10
  %17 = tail call ptr @fa_alloc(i64 noundef %13, i64 noundef 1) #24, !noalias !57
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %17, ptr align 1 %16, i64 %13, i1 false), !noalias !57
  br label %18

18:                                               ; preds = %9, %15
  %19 = phi ptr [ %17, %15 ], [ @.str.29, %9 ]
  store ptr %19, ptr %12, align 8, !tbaa.struct !14
  %20 = getelementptr inbounds i8, ptr %12, i64 8
  store i64 %13, ptr %20, align 8, !tbaa.struct !15
  %21 = getelementptr inbounds i8, ptr %12, i64 16
  store i64 %13, ptr %21, align 8, !tbaa.struct !16
  %22 = icmp ult i64 %11, %1
  br i1 %22, label %9, label %8, !llvm.loop !62
}

; Function Attrs: nounwind optsize
define ptr @fa_array_new(i64 noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #0 {
  %4 = tail call ptr @fa_alloc(i64 noundef 24, i64 noundef 8) #24
  %5 = tail call i64 @llvm.smax.i64(i64 %1, i64 %0)
  %6 = mul nsw i64 %5, %2
  %7 = tail call i64 @llvm.umax.i64(i64 %6, i64 1)
  %8 = tail call ptr @fa_alloc(i64 noundef %7, i64 noundef 16) #24
  store ptr %8, ptr %4, align 8, !tbaa !34
  %9 = getelementptr inbounds %struct.FaArrayHeader, ptr %4, i64 0, i32 1
  store i64 %0, ptr %9, align 8, !tbaa !36
  %10 = getelementptr inbounds %struct.FaArrayHeader, ptr %4, i64 0, i32 2
  store i64 %5, ptr %10, align 8, !tbaa !38
  ret ptr %4
}

; Function Attrs: nounwind optsize
define ptr @fa_array_push_slot(ptr nocapture noundef %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %4 = load i64, ptr %3, align 8, !tbaa !36
  %5 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 2
  %6 = load i64, ptr %5, align 8, !tbaa !38
  %7 = icmp eq i64 %4, %6
  br i1 %7, label %10, label %8

8:                                                ; preds = %2
  %9 = load ptr, ptr %0, align 8, !tbaa !34
  br label %23

10:                                               ; preds = %2
  %11 = icmp slt i64 %4, 4
  %12 = shl nuw nsw i64 %4, 1
  %13 = select i1 %11, i64 4, i64 %12
  %14 = mul nsw i64 %13, %1
  %15 = tail call ptr @fa_alloc(i64 noundef %14, i64 noundef 16) #24
  %16 = load ptr, ptr %0, align 8, !tbaa !34
  %17 = load i64, ptr %3, align 8, !tbaa !36
  %18 = mul nsw i64 %17, %1
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %15, ptr align 1 %16, i64 %18, i1 false)
  %19 = load ptr, ptr %0, align 8, !tbaa !34
  %20 = load i64, ptr %5, align 8, !tbaa !38
  %21 = mul nsw i64 %20, %1
  tail call void @fa_free(ptr noundef %19, i64 noundef %21, i64 noundef 16) #24
  store ptr %15, ptr %0, align 8, !tbaa !34
  store i64 %13, ptr %5, align 8, !tbaa !38
  %22 = load i64, ptr %3, align 8, !tbaa !36
  br label %23

23:                                               ; preds = %8, %10
  %24 = phi i64 [ %4, %8 ], [ %22, %10 ]
  %25 = phi ptr [ %9, %8 ], [ %15, %10 ]
  %26 = mul nsw i64 %24, %1
  %27 = getelementptr inbounds i8, ptr %25, i64 %26
  %28 = add nsw i64 %24, 1
  store i64 %28, ptr %3, align 8, !tbaa !36
  ret ptr %27
}

; Function Attrs: nounwind optsize
define ptr @fa_string_split(ptr noundef %0, i64 noundef %1, ptr nocapture noundef readonly %2, i64 noundef %3) local_unnamed_addr #0 {
  %5 = alloca %struct.Splitting, align 8
  %6 = icmp eq i64 %3, 0
  br i1 %6, label %7, label %9

7:                                                ; preds = %4
  %8 = tail call ptr @fa_string_characters(ptr noundef %0, i64 noundef %1) #25
  br label %30

9:                                                ; preds = %4
  call void @llvm.lifetime.start.p0(i64 32, ptr nonnull %5) #27
  store ptr %0, ptr %5, align 8, !tbaa !63
  %10 = getelementptr inbounds %struct.Splitting, ptr %5, i64 0, i32 1
  store i64 0, ptr %10, align 8, !tbaa !65
  %11 = getelementptr inbounds %struct.Splitting, ptr %5, i64 0, i32 2
  store i64 %3, ptr %11, align 8, !tbaa !66
  %12 = getelementptr inbounds %struct.Splitting, ptr %5, i64 0, i32 3
  %13 = tail call ptr @fa_alloc(i64 noundef 24, i64 noundef 8) #24
  %14 = tail call ptr @fa_alloc(i64 noundef 96, i64 noundef 16) #24
  store ptr %14, ptr %13, align 8, !tbaa !34
  %15 = getelementptr inbounds %struct.FaArrayHeader, ptr %13, i64 0, i32 1
  store i64 0, ptr %15, align 8, !tbaa !36
  %16 = getelementptr inbounds %struct.FaArrayHeader, ptr %13, i64 0, i32 2
  store i64 4, ptr %16, align 8, !tbaa !38
  store ptr %13, ptr %12, align 8, !tbaa !67
  call fastcc void @each_match(ptr noundef %0, i64 noundef %1, ptr noundef %2, i64 noundef %3, ptr noundef nonnull @split_one, ptr noundef nonnull %5) #25
  %17 = load ptr, ptr %12, align 8, !tbaa !67
  %18 = call ptr @fa_array_push_slot(ptr noundef %17, i64 noundef 24) #25
  %19 = load i64, ptr %10, align 8, !tbaa !65
  %20 = sub i64 %1, %19
  %21 = icmp eq i64 %19, %1
  br i1 %21, label %25, label %22

22:                                               ; preds = %9
  %23 = getelementptr inbounds i8, ptr %0, i64 %19
  %24 = call ptr @fa_alloc(i64 noundef %20, i64 noundef 1) #24, !noalias !68
  call void @llvm.memcpy.p0.p0.i64(ptr align 1 %24, ptr align 1 %23, i64 %20, i1 false), !noalias !68
  br label %25

25:                                               ; preds = %9, %22
  %26 = phi ptr [ %24, %22 ], [ @.str.29, %9 ]
  store ptr %26, ptr %18, align 8, !tbaa.struct !14
  %27 = getelementptr inbounds i8, ptr %18, i64 8
  store i64 %20, ptr %27, align 8, !tbaa.struct !15
  %28 = getelementptr inbounds i8, ptr %18, i64 16
  store i64 %20, ptr %28, align 8, !tbaa.struct !16
  %29 = load ptr, ptr %12, align 8, !tbaa !67
  call void @llvm.lifetime.end.p0(i64 32, ptr nonnull %5) #27
  br label %30

30:                                               ; preds = %25, %7
  %31 = phi ptr [ %8, %7 ], [ %29, %25 ]
  ret ptr %31
}

; Function Attrs: nounwind optsize
define internal void @split_one(ptr nocapture noundef %0, i64 noundef %1) #0 {
  %3 = getelementptr inbounds %struct.Splitting, ptr %0, i64 0, i32 3
  %4 = load ptr, ptr %3, align 8, !tbaa !67
  %5 = tail call ptr @fa_array_push_slot(ptr noundef %4, i64 noundef 24) #25
  %6 = getelementptr inbounds %struct.Splitting, ptr %0, i64 0, i32 1
  %7 = load i64, ptr %6, align 8, !tbaa !65
  %8 = sub i64 %1, %7
  %9 = icmp eq i64 %7, %1
  br i1 %9, label %14, label %10

10:                                               ; preds = %2
  %11 = load ptr, ptr %0, align 8, !tbaa !63
  %12 = getelementptr inbounds i8, ptr %11, i64 %7
  %13 = tail call ptr @fa_alloc(i64 noundef %8, i64 noundef 1) #24, !noalias !73
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %13, ptr align 1 %12, i64 %8, i1 false), !noalias !73
  br label %14

14:                                               ; preds = %2, %10
  %15 = phi ptr [ %13, %10 ], [ @.str.29, %2 ]
  store ptr %15, ptr %5, align 8, !tbaa.struct !14
  %16 = getelementptr inbounds i8, ptr %5, i64 8
  store i64 %8, ptr %16, align 8, !tbaa.struct !15
  %17 = getelementptr inbounds i8, ptr %5, i64 16
  store i64 %8, ptr %17, align 8, !tbaa.struct !16
  %18 = getelementptr inbounds %struct.Splitting, ptr %0, i64 0, i32 2
  %19 = load i64, ptr %18, align 8, !tbaa !66
  %20 = add i64 %19, %1
  store i64 %20, ptr %6, align 8, !tbaa !65
  ret void
}

; Function Attrs: nounwind optsize
define ptr @fa_string_code_points(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = alloca i64, align 8
  %4 = tail call ptr @fa_alloc(i64 noundef 24, i64 noundef 8) #24
  %5 = tail call ptr @fa_alloc(i64 noundef 16, i64 noundef 16) #24
  store ptr %5, ptr %4, align 8, !tbaa !34
  %6 = getelementptr inbounds %struct.FaArrayHeader, ptr %4, i64 0, i32 1
  store i64 0, ptr %6, align 8, !tbaa !36
  %7 = getelementptr inbounds %struct.FaArrayHeader, ptr %4, i64 0, i32 2
  store i64 4, ptr %7, align 8, !tbaa !38
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %3) #27
  store i64 0, ptr %3, align 8, !tbaa !2
  %8 = icmp eq i64 %1, 0
  br i1 %8, label %9, label %10

9:                                                ; preds = %10, %2
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %3) #27
  ret ptr %4

10:                                               ; preds = %2, %10
  %11 = tail call ptr @fa_array_push_slot(ptr noundef nonnull %4, i64 noundef 4) #25
  %12 = call fastcc i32 @decode(ptr noundef %0, i64 noundef %1, ptr noundef nonnull %3) #25
  store i32 %12, ptr %11, align 4, !tbaa !18
  %13 = load i64, ptr %3, align 8, !tbaa !2
  %14 = icmp ult i64 %13, %1
  br i1 %14, label %10, label %9, !llvm.loop !78
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: readwrite)
define internal fastcc i32 @decode(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef %2) unnamed_addr #8 {
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
  br i1 %34, label %22, label %17, !llvm.loop !79
}

; Function Attrs: nounwind optsize
define noundef i32 @fa_string_get(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2, ptr nocapture noundef writeonly %3) local_unnamed_addr #0 {
  %5 = icmp slt i64 %2, 0
  br i1 %5, label %6, label %18

6:                                                ; preds = %4
  %7 = icmp eq i64 %1, 0
  br i1 %7, label %14, label %8

8:                                                ; preds = %6, %8
  %9 = phi i64 [ %12, %8 ], [ 0, %6 ]
  %10 = phi i64 [ %11, %8 ], [ 0, %6 ]
  %11 = add i64 %10, 1
  %12 = tail call fastcc i64 @next_boundary(ptr noundef %0, i64 noundef %1, i64 noundef %9) #25
  %13 = icmp ult i64 %12, %1
  br i1 %13, label %8, label %14, !llvm.loop !42

14:                                               ; preds = %8, %6
  %15 = phi i64 [ 0, %6 ], [ %11, %8 ]
  %16 = add nsw i64 %15, %2
  %17 = icmp slt i64 %16, 0
  br i1 %17, label %39, label %18

18:                                               ; preds = %4, %14
  %19 = phi i64 [ %16, %14 ], [ %2, %4 ]
  %20 = icmp eq i64 %1, 0
  br i1 %20, label %39, label %24

21:                                               ; preds = %24
  %22 = add nuw nsw i64 %25, 1
  %23 = icmp ult i64 %27, %1
  br i1 %23, label %24, label %39, !llvm.loop !80

24:                                               ; preds = %18, %21
  %25 = phi i64 [ %22, %21 ], [ 0, %18 ]
  %26 = phi i64 [ %27, %21 ], [ 0, %18 ]
  %27 = tail call fastcc i64 @next_boundary(ptr noundef %0, i64 noundef %1, i64 noundef %26) #25
  %28 = icmp eq i64 %25, %19
  br i1 %28, label %29, label %21

29:                                               ; preds = %24
  %30 = sub i64 %27, %26
  %31 = icmp eq i64 %27, %26
  br i1 %31, label %35, label %32

32:                                               ; preds = %29
  %33 = getelementptr inbounds i8, ptr %0, i64 %26
  %34 = tail call ptr @fa_alloc(i64 noundef %30, i64 noundef 1) #24, !noalias !81
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %34, ptr align 1 %33, i64 %30, i1 false), !noalias !81
  br label %35

35:                                               ; preds = %32, %29
  %36 = phi ptr [ %34, %32 ], [ @.str.29, %29 ]
  store ptr %36, ptr %3, align 8, !tbaa.struct !14
  %37 = getelementptr inbounds i8, ptr %3, i64 8
  store i64 %30, ptr %37, align 8, !tbaa.struct !15
  %38 = getelementptr inbounds i8, ptr %3, i64 16
  store i64 %30, ptr %38, align 8, !tbaa.struct !16
  br label %39

39:                                               ; preds = %21, %35, %18, %14
  %40 = phi i32 [ 0, %14 ], [ 1, %35 ], [ 0, %18 ], [ 0, %21 ]
  ret i32 %40
}

; Function Attrs: nounwind optsize
define void @fa_string_case(ptr nocapture noundef readonly %0, i64 noundef %1, i32 noundef %2, ptr nocapture noundef writeonly %3) local_unnamed_addr #0 {
  %5 = alloca i64, align 8
  store i64 0, ptr @builder_length, align 8, !tbaa !2
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %5) #27
  store i64 0, ptr %5, align 8, !tbaa !2
  %6 = icmp eq i64 %1, 0
  br i1 %6, label %149, label %7

7:                                                ; preds = %4
  %8 = icmp eq i32 %2, 0
  br label %9

9:                                                ; preds = %7, %145
  %10 = call fastcc i32 @decode(ptr noundef %0, i64 noundef %1, ptr noundef nonnull %5) #25
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
  tail call fastcc void @reserve(i64 noundef 2) #25
  %36 = load ptr, ptr @builder, align 8, !tbaa !6
  %37 = load i64, ptr @builder_length, align 8, !tbaa !2
  %38 = getelementptr inbounds i8, ptr %36, i64 %37
  store i16 21331, ptr %38, align 1
  %39 = add i64 %37, 2
  br label %145, !llvm.loop !86

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
  tail call fastcc void @reserve(i64 noundef 1) #25
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
  tail call fastcc void @reserve(i64 noundef 1) #25
  %82 = load ptr, ptr @builder, align 8, !tbaa !6
  %83 = load i64, ptr @builder_length, align 8, !tbaa !2
  %84 = getelementptr inbounds i8, ptr %82, i64 %83
  store i8 %81, ptr %84, align 1, !tbaa !10
  %85 = add i64 %83, 1
  store i64 %85, ptr @builder_length, align 8, !tbaa !2
  %86 = trunc i32 %78 to i8
  %87 = and i8 %86, 63
  %88 = or disjoint i8 %87, -128
  tail call fastcc void @reserve(i64 noundef 1) #25
  %89 = load ptr, ptr @builder, align 8, !tbaa !6
  %90 = load i64, ptr @builder_length, align 8, !tbaa !2
  %91 = getelementptr inbounds i8, ptr %89, i64 %90
  store i8 %88, ptr %91, align 1, !tbaa !10
  br label %142

92:                                               ; preds = %75
  %93 = icmp ult i32 %68, 65536
  tail call fastcc void @reserve(i64 noundef 1) #25
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
  tail call fastcc void @reserve(i64 noundef 1) #25
  %106 = load ptr, ptr @builder, align 8, !tbaa !6
  %107 = load i64, ptr @builder_length, align 8, !tbaa !2
  %108 = getelementptr inbounds i8, ptr %106, i64 %107
  store i8 %105, ptr %108, align 1, !tbaa !10
  %109 = add i64 %107, 1
  store i64 %109, ptr @builder_length, align 8, !tbaa !2
  %110 = trunc i32 %68 to i8
  %111 = and i8 %110, 63
  %112 = or disjoint i8 %111, -128
  tail call fastcc void @reserve(i64 noundef 1) #25
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
  tail call fastcc void @reserve(i64 noundef 1) #25
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
  tail call fastcc void @reserve(i64 noundef 1) #25
  %132 = load ptr, ptr @builder, align 8, !tbaa !6
  %133 = load i64, ptr @builder_length, align 8, !tbaa !2
  %134 = getelementptr inbounds i8, ptr %132, i64 %133
  store i8 %131, ptr %134, align 1, !tbaa !10
  %135 = add i64 %133, 1
  store i64 %135, ptr @builder_length, align 8, !tbaa !2
  %136 = trunc i32 %68 to i8
  %137 = and i8 %136, 63
  %138 = or disjoint i8 %137, -128
  tail call fastcc void @reserve(i64 noundef 1) #25
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
  tail call void @fa_b_take(ptr noundef %3) #25
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %5) #27
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
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %5) #27
  store i64 %10, ptr %5, align 8, !tbaa !2
  %13 = call fastcc i32 @decode(ptr noundef %0, i64 noundef %1, ptr noundef nonnull %5) #25
  %14 = tail call fastcc i32 @is_space(i32 noundef %13) #25, !range !87
  %15 = icmp eq i32 %14, 0
  %16 = load i64, ptr %5, align 8
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %5) #27
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
  br i1 %32, label %24, label %33, !llvm.loop !88

33:                                               ; preds = %24, %28
  %34 = phi i64 [ %18, %24 ], [ %26, %28 ]
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %6) #27
  store i64 %34, ptr %6, align 8, !tbaa !2
  %35 = call fastcc i32 @decode(ptr noundef %0, i64 noundef %22, ptr noundef nonnull %6) #25
  %36 = tail call fastcc i32 @is_space(i32 noundef %35) #25, !range !87
  %37 = icmp eq i32 %36, 0
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %6) #27
  br i1 %37, label %38, label %21

38:                                               ; preds = %33, %21, %17
  %39 = phi i64 [ %1, %17 ], [ %22, %21 ], [ %22, %33 ]
  %40 = sub i64 %39, %18
  %41 = icmp eq i64 %39, %18
  br i1 %41, label %45, label %42

42:                                               ; preds = %38
  %43 = getelementptr inbounds i8, ptr %0, i64 %18
  %44 = tail call ptr @fa_alloc(i64 noundef %40, i64 noundef 1) #24, !noalias !89
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %44, ptr align 1 %43, i64 %40, i1 false), !noalias !89
  br label %45

45:                                               ; preds = %38, %42
  %46 = phi ptr [ %44, %42 ], [ @.str.29, %38 ]
  store ptr %46, ptr %3, align 8, !tbaa.struct !14
  %47 = getelementptr inbounds i8, ptr %3, i64 8
  store i64 %40, ptr %47, align 8, !tbaa.struct !15
  %48 = getelementptr inbounds i8, ptr %3, i64 16
  store i64 %40, ptr %48, align 8, !tbaa.struct !16
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(none)
define internal fastcc noundef i32 @is_space(i32 noundef %0) unnamed_addr #9 {
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
  br i1 %5, label %7, label %6

6:                                                ; preds = %7, %4
  tail call void @fa_b_take(ptr noundef %3) #25
  ret void

7:                                                ; preds = %4, %7
  %8 = phi i64 [ %13, %7 ], [ 0, %4 ]
  tail call fastcc void @reserve(i64 noundef %1) #25
  %9 = load ptr, ptr @builder, align 8, !tbaa !6
  %10 = load i64, ptr @builder_length, align 8, !tbaa !2
  %11 = getelementptr inbounds i8, ptr %9, i64 %10
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %11, ptr align 1 %0, i64 %1, i1 false)
  %12 = add i64 %10, %1
  store i64 %12, ptr @builder_length, align 8, !tbaa !2
  %13 = add nuw nsw i64 %8, 1
  %14 = icmp eq i64 %13, %2
  br i1 %14, label %6, label %7, !llvm.loop !92
}

; Function Attrs: nounwind optsize
define void @fa_array_join(ptr nocapture noundef readonly %0, ptr nocapture noundef readonly %1, i64 noundef %2, ptr nocapture noundef writeonly %3) local_unnamed_addr #0 {
  %5 = load ptr, ptr %0, align 8, !tbaa !34
  store i64 0, ptr @builder_length, align 8, !tbaa !2
  %6 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %7 = load i64, ptr %6, align 8, !tbaa !36
  %8 = icmp sgt i64 %7, 0
  br i1 %8, label %10, label %9

9:                                                ; preds = %18, %4
  tail call void @fa_b_take(ptr noundef %3) #25
  ret void

10:                                               ; preds = %4, %18
  %11 = phi i64 [ %27, %18 ], [ 0, %4 ]
  %12 = icmp eq i64 %11, 0
  br i1 %12, label %18, label %13

13:                                               ; preds = %10
  tail call fastcc void @reserve(i64 noundef %2) #25
  %14 = load ptr, ptr @builder, align 8, !tbaa !6
  %15 = load i64, ptr @builder_length, align 8, !tbaa !2
  %16 = getelementptr inbounds i8, ptr %14, i64 %15
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %16, ptr align 1 %1, i64 %2, i1 false)
  %17 = add i64 %15, %2
  store i64 %17, ptr @builder_length, align 8, !tbaa !2
  br label %18

18:                                               ; preds = %13, %10
  %19 = getelementptr inbounds %struct.FaString, ptr %5, i64 %11
  %20 = load ptr, ptr %19, align 8, !tbaa !34
  %21 = getelementptr inbounds %struct.FaString, ptr %5, i64 %11, i32 1
  %22 = load i64, ptr %21, align 8, !tbaa !36
  tail call fastcc void @reserve(i64 noundef %22) #25
  %23 = load ptr, ptr @builder, align 8, !tbaa !6
  %24 = load i64, ptr @builder_length, align 8, !tbaa !2
  %25 = getelementptr inbounds i8, ptr %23, i64 %24
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %25, ptr align 1 %20, i64 %22, i1 false)
  %26 = add i64 %24, %22
  store i64 %26, ptr @builder_length, align 8, !tbaa !2
  %27 = add nuw nsw i64 %11, 1
  %28 = load i64, ptr %6, align 8, !tbaa !36
  %29 = icmp slt i64 %27, %28
  br i1 %29, label %10, label %9, !llvm.loop !93
}

; Function Attrs: nounwind optsize
define void @fa_string_pad(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2, ptr nocapture noundef readonly %3, i64 noundef %4, i32 noundef %5, ptr nocapture noundef writeonly %6) local_unnamed_addr #0 {
  %8 = icmp eq i64 %1, 0
  br i1 %8, label %15, label %9

9:                                                ; preds = %7, %9
  %10 = phi i64 [ %13, %9 ], [ 0, %7 ]
  %11 = phi i64 [ %12, %9 ], [ 0, %7 ]
  %12 = add i64 %11, 1
  %13 = tail call fastcc i64 @next_boundary(ptr noundef %0, i64 noundef %1, i64 noundef %10) #25
  %14 = icmp ult i64 %13, %1
  br i1 %14, label %9, label %15, !llvm.loop !42

15:                                               ; preds = %9, %7
  %16 = phi i64 [ 0, %7 ], [ %12, %9 ]
  %17 = sub nsw i64 %2, %16
  store i64 0, ptr @builder_length, align 8, !tbaa !2
  %18 = icmp eq i32 %5, 0
  br i1 %18, label %19, label %24

19:                                               ; preds = %15
  tail call fastcc void @reserve(i64 noundef %1) #25
  %20 = load ptr, ptr @builder, align 8, !tbaa !6
  %21 = load i64, ptr @builder_length, align 8, !tbaa !2
  %22 = getelementptr inbounds i8, ptr %20, i64 %21
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %22, ptr align 1 %0, i64 %1, i1 false)
  %23 = add i64 %21, %1
  store i64 %23, ptr @builder_length, align 8, !tbaa !2
  br label %24

24:                                               ; preds = %19, %15
  %25 = icmp ne i64 %4, 0
  %26 = icmp sgt i64 %17, 0
  %27 = and i1 %25, %26
  br i1 %27, label %29, label %28

28:                                               ; preds = %29, %24
  br i1 %18, label %48, label %43

29:                                               ; preds = %24, %29
  %30 = phi i64 [ %40, %29 ], [ 0, %24 ]
  %31 = phi i64 [ %41, %29 ], [ 0, %24 ]
  %32 = tail call fastcc i64 @next_boundary(ptr noundef %3, i64 noundef %4, i64 noundef %30) #25
  %33 = getelementptr inbounds i8, ptr %3, i64 %30
  %34 = sub i64 %32, %30
  tail call fastcc void @reserve(i64 noundef %34) #25
  %35 = load ptr, ptr @builder, align 8, !tbaa !6
  %36 = load i64, ptr @builder_length, align 8, !tbaa !2
  %37 = getelementptr inbounds i8, ptr %35, i64 %36
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %37, ptr align 1 %33, i64 %34, i1 false)
  %38 = add i64 %36, %34
  store i64 %38, ptr @builder_length, align 8, !tbaa !2
  %39 = icmp eq i64 %32, %4
  %40 = select i1 %39, i64 0, i64 %32
  %41 = add nuw nsw i64 %31, 1
  %42 = icmp slt i64 %41, %17
  br i1 %42, label %29, label %28, !llvm.loop !94

43:                                               ; preds = %28
  tail call fastcc void @reserve(i64 noundef %1) #25
  %44 = load ptr, ptr @builder, align 8, !tbaa !6
  %45 = load i64, ptr @builder_length, align 8, !tbaa !2
  %46 = getelementptr inbounds i8, ptr %44, i64 %45
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %46, ptr align 1 %0, i64 %1, i1 false)
  %47 = add i64 %45, %1
  store i64 %47, ptr @builder_length, align 8, !tbaa !2
  br label %48

48:                                               ; preds = %43, %28
  tail call void @fa_b_take(ptr noundef %6) #25
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_arguments_init(i32 noundef %0, ptr nocapture noundef readonly %1) local_unnamed_addr #0 {
  %3 = icmp sgt i32 %0, 1
  %4 = zext nneg i32 %0 to i64
  %5 = add nsw i64 %4, -1
  %6 = select i1 %3, i64 %5, i64 0
  store i64 %6, ptr @argument_count, align 8, !tbaa !2
  %7 = mul nsw i64 %6, 24
  %8 = add nsw i64 %7, 24
  %9 = tail call ptr @fa_alloc(i64 noundef %8, i64 noundef 8) #24
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
  store ptr %17, ptr %18, align 8, !tbaa !34
  br label %19

19:                                               ; preds = %19, %13
  %20 = phi i64 [ 0, %13 ], [ %24, %19 ]
  %21 = getelementptr inbounds i8, ptr %17, i64 %20
  %22 = load i8, ptr %21, align 1, !tbaa !10
  %23 = icmp eq i8 %22, 0
  %24 = add i64 %20, 1
  br i1 %23, label %25, label %19, !llvm.loop !22

25:                                               ; preds = %19
  %26 = getelementptr inbounds %struct.FaString, ptr %9, i64 %14, i32 1
  store i64 %20, ptr %26, align 8, !tbaa !36
  %27 = getelementptr inbounds %struct.FaString, ptr %9, i64 %14, i32 2
  store i64 0, ptr %27, align 8, !tbaa !38
  %28 = icmp eq i64 %15, %10
  br i1 %28, label %12, label %13, !llvm.loop !95
}

; Function Attrs: optsize
declare void @fa_free(ptr noundef, i64 noundef, i64 noundef) local_unnamed_addr #1

; Function Attrs: nounwind optsize
define void @fa_array_append(ptr nocapture noundef %0, ptr nocapture noundef readonly %1, i64 noundef %2) local_unnamed_addr #0 {
  %4 = getelementptr inbounds %struct.FaArrayHeader, ptr %1, i64 0, i32 1
  %5 = load i64, ptr %4, align 8, !tbaa !36
  %6 = icmp sgt i64 %5, 0
  br i1 %6, label %8, label %7

7:                                                ; preds = %8, %3
  ret void

8:                                                ; preds = %3, %8
  %9 = phi i64 [ %14, %8 ], [ 0, %3 ]
  %10 = tail call ptr @fa_array_push_slot(ptr noundef %0, i64 noundef %2) #25
  %11 = load ptr, ptr %1, align 8, !tbaa !34
  %12 = mul nsw i64 %9, %2
  %13 = getelementptr inbounds i8, ptr %11, i64 %12
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %10, ptr align 1 %13, i64 %2, i1 false)
  %14 = add nuw nsw i64 %9, 1
  %15 = icmp eq i64 %14, %5
  br i1 %15, label %7, label %8, !llvm.loop !96
}

; Function Attrs: nounwind optsize
define noundef ptr @fa_array_slice(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2, i8 noundef signext %3, i64 noundef %4, ptr nocapture noundef writeonly %5) local_unnamed_addr #0 {
  %7 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %8 = load i64, ptr %7, align 8, !tbaa !36
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
  %18 = tail call ptr @fa_error_index(i64 noundef %1, i64 noundef %8) #25
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
  %45 = tail call ptr @fa_error_index(i64 noundef %2, i64 noundef %8) #25
  br label %59

46:                                               ; preds = %38, %23
  %47 = phi i64 [ %8, %23 ], [ %43, %38 ]
  %48 = tail call i64 @llvm.smax.i64(i64 %47, i64 %24)
  %49 = sub nsw i64 %48, %24
  %50 = tail call ptr @fa_alloc(i64 noundef 24, i64 noundef 8) #24
  %51 = mul nsw i64 %49, %4
  %52 = tail call i64 @llvm.umax.i64(i64 %51, i64 1)
  %53 = tail call ptr @fa_alloc(i64 noundef %52, i64 noundef 16) #24
  store ptr %53, ptr %50, align 8, !tbaa !34
  %54 = getelementptr inbounds %struct.FaArrayHeader, ptr %50, i64 0, i32 1
  store i64 %49, ptr %54, align 8, !tbaa !36
  %55 = getelementptr inbounds %struct.FaArrayHeader, ptr %50, i64 0, i32 2
  store i64 %49, ptr %55, align 8, !tbaa !38
  %56 = load ptr, ptr %0, align 8, !tbaa !34
  %57 = mul nsw i64 %24, %4
  %58 = getelementptr inbounds i8, ptr %56, i64 %57
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %53, ptr align 1 %58, i64 %51, i1 false)
  store ptr %50, ptr %5, align 8, !tbaa !6
  br label %59

59:                                               ; preds = %44, %46, %17
  %60 = phi ptr [ %18, %17 ], [ null, %46 ], [ %45, %44 ]
  ret ptr %60
}

; Function Attrs: cold noinline nounwind optsize
define noundef ptr @fa_error_index(i64 noundef %0, i64 noundef %1) local_unnamed_addr #10 {
  %3 = tail call ptr @fa_alloc(i64 noundef 24, i64 noundef 8) #24
  store i64 1, ptr %3, align 8, !tbaa !97
  %4 = getelementptr inbounds %struct.IndexError, ptr %3, i64 0, i32 1
  store i64 %0, ptr %4, align 8, !tbaa !99
  %5 = getelementptr inbounds %struct.IndexError, ptr %3, i64 0, i32 2
  store i64 %1, ptr %5, align 8, !tbaa !100
  ret ptr %3
}

; Function Attrs: nounwind optsize
define void @fa_array_reserve(ptr nocapture noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #0 {
  %4 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 2
  %5 = load i64, ptr %4, align 8, !tbaa !38
  %6 = icmp slt i64 %5, %1
  br i1 %6, label %7, label %17

7:                                                ; preds = %3
  %8 = mul nsw i64 %2, %1
  %9 = tail call ptr @fa_alloc(i64 noundef %8, i64 noundef 16) #24
  %10 = load ptr, ptr %0, align 8, !tbaa !34
  %11 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %12 = load i64, ptr %11, align 8, !tbaa !36
  %13 = mul nsw i64 %12, %2
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %9, ptr align 1 %10, i64 %13, i1 false)
  %14 = load ptr, ptr %0, align 8, !tbaa !34
  %15 = load i64, ptr %4, align 8, !tbaa !38
  %16 = mul nsw i64 %15, %2
  tail call void @fa_free(ptr noundef %14, i64 noundef %16, i64 noundef 16) #24
  store ptr %9, ptr %0, align 8, !tbaa !34
  store i64 %1, ptr %4, align 8, !tbaa !38
  br label %17

17:                                               ; preds = %3, %7
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_array_assign(ptr noundef %0, ptr noundef readonly %1, i64 noundef %2) local_unnamed_addr #0 {
  %4 = icmp eq ptr %0, %1
  br i1 %4, label %27, label %5

5:                                                ; preds = %3
  %6 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 2
  %7 = load i64, ptr %6, align 8, !tbaa !38
  %8 = getelementptr inbounds %struct.FaArrayHeader, ptr %1, i64 0, i32 1
  %9 = load i64, ptr %8, align 8, !tbaa !36
  %10 = icmp slt i64 %7, %9
  br i1 %10, label %13, label %11

11:                                               ; preds = %5
  %12 = load ptr, ptr %0, align 8, !tbaa !34
  br label %20

13:                                               ; preds = %5
  %14 = mul nsw i64 %9, %2
  %15 = tail call ptr @fa_alloc(i64 noundef %14, i64 noundef 16) #24
  %16 = load ptr, ptr %0, align 8, !tbaa !34
  %17 = load i64, ptr %6, align 8, !tbaa !38
  %18 = mul nsw i64 %17, %2
  tail call void @fa_free(ptr noundef %16, i64 noundef %18, i64 noundef 16) #24
  store ptr %15, ptr %0, align 8, !tbaa !34
  %19 = load i64, ptr %8, align 8, !tbaa !36
  store i64 %19, ptr %6, align 8, !tbaa !38
  br label %20

20:                                               ; preds = %11, %13
  %21 = phi i64 [ %9, %11 ], [ %19, %13 ]
  %22 = phi ptr [ %12, %11 ], [ %15, %13 ]
  %23 = load ptr, ptr %1, align 8, !tbaa !34
  %24 = mul nsw i64 %21, %2
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %22, ptr align 1 %23, i64 %24, i1 false)
  %25 = load i64, ptr %8, align 8, !tbaa !36
  %26 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  store i64 %25, ptr %26, align 8, !tbaa !36
  br label %27

27:                                               ; preds = %3, %20
  ret void
}

; Function Attrs: nounwind optsize
define ptr @fa_array_copy(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %4 = load i64, ptr %3, align 8, !tbaa !36
  %5 = tail call ptr @fa_alloc(i64 noundef 24, i64 noundef 8) #24
  %6 = mul nsw i64 %4, %1
  %7 = tail call i64 @llvm.umax.i64(i64 %6, i64 1)
  %8 = tail call ptr @fa_alloc(i64 noundef %7, i64 noundef 16) #24
  store ptr %8, ptr %5, align 8, !tbaa !34
  %9 = getelementptr inbounds %struct.FaArrayHeader, ptr %5, i64 0, i32 1
  store i64 %4, ptr %9, align 8, !tbaa !36
  %10 = getelementptr inbounds %struct.FaArrayHeader, ptr %5, i64 0, i32 2
  store i64 %4, ptr %10, align 8, !tbaa !38
  %11 = load ptr, ptr %0, align 8, !tbaa !34
  %12 = load i64, ptr %3, align 8, !tbaa !36
  %13 = mul nsw i64 %12, %1
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %8, ptr align 1 %11, i64 %13, i1 false)
  ret ptr %5
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define void @fa_array_reverse(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #11 {
  %3 = alloca [64 x i8], align 16
  %4 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %5 = load i64, ptr %4, align 8, !tbaa !36
  %6 = icmp slt i64 %5, 2
  br i1 %6, label %32, label %7

7:                                                ; preds = %2
  call void @llvm.lifetime.start.p0(i64 64, ptr nonnull %3)
  %8 = add nsw i64 %5, -1
  %9 = mul nsw i64 %8, %1
  %10 = icmp sgt i64 %9, 0
  br i1 %10, label %11, label %31

11:                                               ; preds = %7
  %12 = load ptr, ptr %0, align 8, !tbaa !34
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
  br i1 %22, label %16, label %31, !llvm.loop !101

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
  br i1 %30, label %23, label %19, !llvm.loop !102

31:                                               ; preds = %19, %7
  call void @llvm.lifetime.end.p0(i64 64, ptr nonnull %3)
  br label %32

32:                                               ; preds = %2, %31
  ret void
}

; Function Attrs: nounwind optsize
define ptr @fa_array_insert_slot(ptr nocapture noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #0 {
  %4 = tail call ptr @fa_array_push_slot(ptr noundef %0, i64 noundef %2) #25
  %5 = load ptr, ptr %0, align 8, !tbaa !34
  %6 = mul nsw i64 %2, %1
  %7 = getelementptr inbounds i8, ptr %5, i64 %6
  %8 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %9 = load i64, ptr %8, align 8, !tbaa !36
  %10 = xor i64 %1, -1
  %11 = add i64 %9, %10
  %12 = mul nsw i64 %11, %2
  %13 = getelementptr inbounds i8, ptr %7, i64 %2
  tail call void @llvm.memmove.p0.p0.i64(ptr align 1 %13, ptr align 1 %7, i64 %12, i1 false)
  ret ptr %7
}

; Function Attrs: mustprogress nocallback nofree nounwind willreturn memory(argmem: readwrite)
declare void @llvm.memmove.p0.p0.i64(ptr nocapture writeonly, ptr nocapture readonly, i64, i1 immarg) #6

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(readwrite, inaccessiblemem: none)
define void @fa_array_remove(ptr nocapture noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #12 {
  %4 = load ptr, ptr %0, align 8, !tbaa !34
  %5 = mul nsw i64 %2, %1
  %6 = getelementptr inbounds i8, ptr %4, i64 %5
  %7 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %8 = load i64, ptr %7, align 8, !tbaa !36
  %9 = xor i64 %1, -1
  %10 = add i64 %8, %9
  %11 = mul nsw i64 %10, %2
  %12 = getelementptr inbounds i8, ptr %6, i64 %2
  tail call void @llvm.memmove.p0.p0.i64(ptr align 1 %6, ptr align 1 %12, i64 %11, i1 false)
  %13 = load i64, ptr %7, align 8, !tbaa !36
  %14 = add nsw i64 %13, -1
  store i64 %14, ptr %7, align 8, !tbaa !36
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_array_sort(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef readonly %2, ptr noundef %3) local_unnamed_addr #0 {
  %5 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %6 = load i64, ptr %5, align 8, !tbaa !36
  %7 = icmp slt i64 %6, 2
  br i1 %7, label %75, label %8

8:                                                ; preds = %4
  %9 = mul nsw i64 %6, %1
  %10 = load ptr, ptr %0, align 8, !tbaa !34
  %11 = tail call ptr @fa_alloc(i64 noundef %9, i64 noundef 16) #24
  br label %12

12:                                               ; preds = %20, %8
  %13 = phi ptr [ %10, %8 ], [ %14, %20 ]
  %14 = phi ptr [ %11, %8 ], [ %13, %20 ]
  %15 = phi i64 [ 1, %8 ], [ %16, %20 ]
  %16 = shl nsw i64 %15, 1
  br label %22

17:                                               ; preds = %20
  %18 = load ptr, ptr %0, align 8, !tbaa !34
  %19 = icmp eq ptr %14, %18
  br i1 %19, label %73, label %72

20:                                               ; preds = %54
  %21 = icmp slt i64 %16, %6
  br i1 %21, label %12, label %17, !llvm.loop !103

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
  %39 = tail call i32 %2(ptr noundef %3, ptr noundef %36, ptr noundef %38) #24
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
  br i1 %53, label %31, label %54, !llvm.loop !104

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
  br i1 %71, label %22, label %20, !llvm.loop !105

72:                                               ; preds = %17
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %18, ptr align 1 %14, i64 %9, i1 false)
  br label %73

73:                                               ; preds = %17, %72
  %74 = phi ptr [ %14, %72 ], [ %13, %17 ]
  tail call void @fa_free(ptr noundef %74, i64 noundef %9, i64 noundef 16) #24
  br label %75

75:                                               ; preds = %73, %4
  ret void
}

; Function Attrs: nounwind optsize
define ptr @fa_arguments() local_unnamed_addr #0 {
  %1 = load i64, ptr @argument_count, align 8, !tbaa !2
  %2 = tail call ptr @fa_alloc(i64 noundef 24, i64 noundef 8) #24
  %3 = mul nsw i64 %1, 24
  %4 = tail call i64 @llvm.umax.i64(i64 %3, i64 1)
  %5 = tail call ptr @fa_alloc(i64 noundef %4, i64 noundef 16) #24
  store ptr %5, ptr %2, align 8, !tbaa !34
  %6 = getelementptr inbounds %struct.FaArrayHeader, ptr %2, i64 0, i32 1
  store i64 %1, ptr %6, align 8, !tbaa !36
  %7 = getelementptr inbounds %struct.FaArrayHeader, ptr %2, i64 0, i32 2
  store i64 %1, ptr %7, align 8, !tbaa !38
  %8 = load ptr, ptr @argument_items, align 8, !tbaa !6
  %9 = load i64, ptr @argument_count, align 8, !tbaa !2
  %10 = mul nsw i64 %9, 24
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %5, ptr align 1 %8, i64 %10, i1 false)
  ret ptr %2
}

; Function Attrs: nounwind optsize
define void @fa_array_free(ptr noundef %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = load ptr, ptr %0, align 8, !tbaa !34
  %4 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 2
  %5 = load i64, ptr %4, align 8, !tbaa !38
  %6 = mul nsw i64 %5, %1
  tail call void @fa_free(ptr noundef %3, i64 noundef %6, i64 noundef 16) #24
  tail call void @fa_free(ptr noundef nonnull %0, i64 noundef 24, i64 noundef 8) #24
  ret void
}

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read)
define ptr @fa_segmented_at(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2, i32 noundef %3) local_unnamed_addr #13 {
  %5 = zext nneg i32 %3 to i64
  %6 = lshr i64 %1, %5
  %7 = icmp eq i64 %6, 0
  br i1 %7, label %15, label %8

8:                                                ; preds = %4
  %9 = tail call i64 @llvm.ctlz.i64(i64 %6, i1 true), !range !106
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
  %19 = load ptr, ptr %18, align 8, !tbaa !6
  %20 = add i64 %16, %1
  %21 = mul nsw i64 %20, %2
  %22 = getelementptr inbounds i8, ptr %19, i64 %21
  ret ptr %22
}

; Function Attrs: nounwind optsize
define noundef ptr @fa_segmented_new() local_unnamed_addr #0 {
  %1 = tail call ptr @fa_alloc(i64 noundef 400, i64 noundef 8) #24
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(400) %1, i8 0, i64 400, i1 false)
  ret ptr %1
}

; Function Attrs: mustprogress nocallback nofree nounwind willreturn memory(argmem: write)
declare void @llvm.memset.p0.i64(ptr nocapture writeonly, i8, i64, i1 immarg) #14

; Function Attrs: nounwind optsize
define ptr @fa_segmented_push_slot(ptr nocapture noundef %0, i64 noundef %1, i32 noundef %2) local_unnamed_addr #0 {
  %4 = load i64, ptr %0, align 8, !tbaa !107
  %5 = zext nneg i32 %2 to i64
  %6 = lshr i64 %4, %5
  %7 = icmp eq i64 %6, 0
  br i1 %7, label %15, label %8

8:                                                ; preds = %3
  %9 = tail call i64 @llvm.ctlz.i64(i64 %6, i1 true), !range !106
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
  %19 = load i64, ptr %18, align 8, !tbaa !109
  %20 = icmp slt i64 %17, %19
  br i1 %20, label %21, label %24

21:                                               ; preds = %15
  %22 = getelementptr inbounds %struct.FaSegmented, ptr %0, i64 0, i32 2, i64 %17
  %23 = load ptr, ptr %22, align 8, !tbaa !6
  br label %32

24:                                               ; preds = %15
  %25 = sext i32 %2 to i64
  %26 = tail call i64 @llvm.usub.sat.i64(i64 %17, i64 1)
  %27 = add nsw i64 %26, %25
  %28 = shl i64 %1, %27
  %29 = tail call ptr @fa_alloc(i64 noundef %28, i64 noundef 16) #24
  %30 = getelementptr inbounds %struct.FaSegmented, ptr %0, i64 0, i32 2, i64 %17
  store ptr %29, ptr %30, align 8, !tbaa !6
  %31 = add nuw nsw i64 %17, 1
  store i64 %31, ptr %18, align 8, !tbaa !109
  br label %32

32:                                               ; preds = %21, %24
  %33 = phi ptr [ %23, %21 ], [ %29, %24 ]
  %34 = add nsw i64 %4, 1
  store i64 %34, ptr %0, align 8, !tbaa !107
  %35 = add i64 %16, %4
  %36 = mul nsw i64 %35, %1
  %37 = getelementptr inbounds i8, ptr %33, i64 %36
  ret ptr %37
}

; Function Attrs: nounwind optsize
define void @fa_segmented_free(ptr noundef %0, i64 noundef %1, i32 noundef %2) local_unnamed_addr #0 {
  %4 = getelementptr inbounds %struct.FaSegmented, ptr %0, i64 0, i32 1
  %5 = load i64, ptr %4, align 8, !tbaa !109
  %6 = icmp sgt i64 %5, 0
  br i1 %6, label %7, label %9

7:                                                ; preds = %3
  %8 = sext i32 %2 to i64
  br label %10

9:                                                ; preds = %10, %3
  tail call void @fa_free(ptr noundef nonnull %0, i64 noundef 400, i64 noundef 8) #24
  ret void

10:                                               ; preds = %7, %10
  %11 = phi i64 [ 0, %7 ], [ %17, %10 ]
  %12 = getelementptr inbounds %struct.FaSegmented, ptr %0, i64 0, i32 2, i64 %11
  %13 = load ptr, ptr %12, align 8, !tbaa !6
  %14 = tail call i64 @llvm.usub.sat.i64(i64 %11, i64 1)
  %15 = add i64 %14, %8
  %16 = shl i64 %1, %15
  tail call void @fa_free(ptr noundef %13, i64 noundef %16, i64 noundef 16) #24
  %17 = add nuw nsw i64 %11, 1
  %18 = load i64, ptr %4, align 8, !tbaa !109
  %19 = icmp slt i64 %17, %18
  br i1 %19, label %10, label %9, !llvm.loop !110
}

; Function Attrs: nounwind optsize
define ptr @fa_segmented_flatten(ptr nocapture noundef readonly %0, i64 noundef %1, i32 noundef %2) local_unnamed_addr #0 {
  %4 = load i64, ptr %0, align 8, !tbaa !107
  %5 = tail call ptr @fa_alloc(i64 noundef 24, i64 noundef 8) #24
  %6 = mul nsw i64 %4, %1
  %7 = tail call i64 @llvm.umax.i64(i64 %6, i64 1)
  %8 = tail call ptr @fa_alloc(i64 noundef %7, i64 noundef 16) #24
  store ptr %8, ptr %5, align 8, !tbaa !34
  %9 = getelementptr inbounds %struct.FaArrayHeader, ptr %5, i64 0, i32 1
  store i64 %4, ptr %9, align 8, !tbaa !36
  %10 = getelementptr inbounds %struct.FaArrayHeader, ptr %5, i64 0, i32 2
  store i64 %4, ptr %10, align 8, !tbaa !38
  %11 = load i64, ptr %0, align 8, !tbaa !107
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
  %25 = load ptr, ptr %5, align 8, !tbaa !34
  %26 = mul nsw i64 %19, %1
  %27 = getelementptr inbounds i8, ptr %25, i64 %26
  %28 = getelementptr inbounds %struct.FaSegmented, ptr %0, i64 0, i32 2, i64 %18
  %29 = load ptr, ptr %28, align 8, !tbaa !6
  %30 = mul nsw i64 %24, %1
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %27, ptr align 1 %29, i64 %30, i1 false)
  %31 = add nsw i64 %24, %19
  %32 = add nuw nsw i64 %18, 1
  %33 = load i64, ptr %0, align 8, !tbaa !107
  %34 = icmp slt i64 %31, %33
  br i1 %34, label %16, label %15, !llvm.loop !111
}

; Function Attrs: nounwind optsize
define noundef ptr @fa_segmented_from_array(ptr noundef %0, i64 noundef %1, i32 noundef %2) local_unnamed_addr #0 {
  %4 = tail call noundef ptr @fa_alloc(i64 noundef 400, i64 noundef 8) #24
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(400) %4, i8 0, i64 400, i1 false)
  %5 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 1
  %6 = load i64, ptr %5, align 8, !tbaa !36
  %7 = icmp sgt i64 %6, 0
  br i1 %7, label %13, label %8

8:                                                ; preds = %13, %3
  %9 = load ptr, ptr %0, align 8, !tbaa !34
  %10 = getelementptr inbounds %struct.FaArrayHeader, ptr %0, i64 0, i32 2
  %11 = load i64, ptr %10, align 8, !tbaa !38
  %12 = mul nsw i64 %11, %1
  tail call void @fa_free(ptr noundef %9, i64 noundef %12, i64 noundef 16) #24
  tail call void @fa_free(ptr noundef nonnull %0, i64 noundef 24, i64 noundef 8) #24
  ret ptr %4

13:                                               ; preds = %3, %13
  %14 = phi i64 [ %19, %13 ], [ 0, %3 ]
  %15 = tail call ptr @fa_segmented_push_slot(ptr noundef %4, i64 noundef %1, i32 noundef %2) #25
  %16 = load ptr, ptr %0, align 8, !tbaa !34
  %17 = mul nsw i64 %14, %1
  %18 = getelementptr inbounds i8, ptr %16, i64 %17
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %15, ptr align 1 %18, i64 %1, i1 false)
  %19 = add nuw nsw i64 %14, 1
  %20 = load i64, ptr %5, align 8, !tbaa !36
  %21 = icmp slt i64 %19, %20
  br i1 %21, label %13, label %8, !llvm.loop !112
}

; Function Attrs: nounwind optsize
define ptr @fa_heap_alloc(i64 noundef %0) local_unnamed_addr #0 {
  %2 = icmp ugt i64 %0, 8192
  br i1 %2, label %3, label %14

3:                                                ; preds = %1
  %4 = add i64 %0, 65551
  %5 = lshr i64 %4, 16
  %6 = icmp ult i64 %4, 65536
  br i1 %6, label %69, label %7

7:                                                ; preds = %3
  %8 = tail call fastcc ptr @take_pages(i64 noundef %5) #25
  %9 = icmp eq ptr %8, null
  br i1 %9, label %69, label %10

10:                                               ; preds = %7
  store i32 65535, ptr %8, align 4, !tbaa !113
  %11 = trunc i64 %5 to i32
  %12 = getelementptr inbounds %struct.PageHeader, ptr %8, i64 0, i32 1
  store i32 %11, ptr %12, align 4, !tbaa !115
  %13 = getelementptr inbounds i8, ptr %8, i64 16
  br label %69

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
  br i1 %38, label %39, label %29, !llvm.loop !116

39:                                               ; preds = %35
  %40 = shl i64 %26, 1
  br label %24

41:                                               ; preds = %29, %16
  %42 = phi i64 [ %20, %16 ], [ %33, %29 ]
  %43 = phi i32 [ %23, %16 ], [ %31, %29 ]
  %44 = zext i32 %43 to i64
  %45 = getelementptr inbounds [40 x ptr], ptr @free_blocks, i64 0, i64 %44
  %46 = load ptr, ptr %45, align 8, !tbaa !6
  %47 = icmp eq ptr %46, null
  br i1 %47, label %50, label %48

48:                                               ; preds = %41
  %49 = load ptr, ptr %46, align 8, !tbaa !117
  store ptr %49, ptr %45, align 8, !tbaa !6
  br label %69

50:                                               ; preds = %41
  %51 = getelementptr inbounds [40 x ptr], ptr @class_next, i64 0, i64 %44
  %52 = load ptr, ptr %51, align 8, !tbaa !6
  %53 = getelementptr inbounds i8, ptr %52, i64 %42
  %54 = getelementptr inbounds [40 x ptr], ptr @class_end, i64 0, i64 %44
  %55 = load ptr, ptr %54, align 8, !tbaa !6
  %56 = icmp ugt ptr %53, %55
  %57 = icmp eq ptr %52, null
  %58 = or i1 %57, %56
  br i1 %58, label %59, label %66

59:                                               ; preds = %50
  %60 = tail call fastcc ptr @take_pages(i64 noundef 1) #25
  %61 = icmp eq ptr %60, null
  br i1 %61, label %69, label %62

62:                                               ; preds = %59
  store i32 %43, ptr %60, align 4, !tbaa !113
  %63 = getelementptr inbounds %struct.PageHeader, ptr %60, i64 0, i32 1
  store i32 1, ptr %63, align 4, !tbaa !115
  %64 = getelementptr inbounds i8, ptr %60, i64 16
  %65 = getelementptr inbounds i8, ptr %60, i64 65536
  store ptr %65, ptr %54, align 8, !tbaa !6
  br label %66

66:                                               ; preds = %62, %50
  %67 = phi ptr [ %64, %62 ], [ %52, %50 ]
  %68 = getelementptr inbounds i8, ptr %67, i64 %42
  store ptr %68, ptr %51, align 8, !tbaa !6
  br label %69

69:                                               ; preds = %48, %66, %59, %3, %7, %10
  %70 = phi ptr [ null, %3 ], [ %13, %10 ], [ null, %7 ], [ %46, %48 ], [ %67, %66 ], [ null, %59 ]
  ret ptr %70
}

; Function Attrs: nounwind optsize
define internal fastcc ptr @take_pages(i64 noundef %0) unnamed_addr #0 {
  br label %2

2:                                                ; preds = %6, %1
  %3 = phi ptr [ @free_spans, %1 ], [ %4, %6 ]
  %4 = load ptr, ptr %3, align 8, !tbaa !6
  %5 = icmp eq ptr %4, null
  br i1 %5, label %22, label %6

6:                                                ; preds = %2
  %7 = getelementptr inbounds %struct.FreeSpan, ptr %4, i64 0, i32 1
  %8 = load i64, ptr %7, align 8, !tbaa !119
  %9 = icmp ult i64 %8, %0
  br i1 %9, label %2, label %10

10:                                               ; preds = %6
  %11 = icmp eq i64 %8, %0
  br i1 %11, label %12, label %14

12:                                               ; preds = %10
  %13 = load ptr, ptr %4, align 8, !tbaa !121
  br label %20

14:                                               ; preds = %10
  %15 = shl nuw i64 %0, 16
  %16 = getelementptr inbounds i8, ptr %4, i64 %15
  %17 = load ptr, ptr %4, align 8, !tbaa !121
  store ptr %17, ptr %16, align 8, !tbaa !121
  %18 = sub i64 %8, %0
  %19 = getelementptr inbounds %struct.FreeSpan, ptr %16, i64 0, i32 1
  store i64 %18, ptr %19, align 8, !tbaa !119
  br label %20

20:                                               ; preds = %12, %14
  %21 = phi ptr [ %16, %14 ], [ %13, %12 ]
  store ptr %21, ptr %3, align 8, !tbaa !6
  br label %40

22:                                               ; preds = %2
  %23 = tail call i64 @llvm.umax.i64(i64 %0, i64 16)
  %24 = tail call ptr @fa_pages(i64 noundef %23) #24
  %25 = icmp eq ptr %24, null
  br i1 %25, label %40, label %26

26:                                               ; preds = %22
  %27 = icmp ult i64 %0, 16
  br i1 %27, label %28, label %40

28:                                               ; preds = %26
  %29 = shl nuw nsw i64 %0, 16
  %30 = getelementptr inbounds i8, ptr %24, i64 %29
  %31 = sub nuw nsw i64 %23, %0
  %32 = getelementptr inbounds %struct.FreeSpan, ptr %30, i64 0, i32 1
  store i64 %31, ptr %32, align 8, !tbaa !119
  store ptr null, ptr %30, align 8, !tbaa !121
  br label %33

33:                                               ; preds = %33, %28
  %34 = phi ptr [ @free_spans, %28 ], [ %35, %33 ]
  %35 = load ptr, ptr %34, align 8, !tbaa !6
  %36 = icmp ne ptr %35, null
  %37 = icmp ult ptr %35, %30
  %38 = and i1 %36, %37
  br i1 %38, label %33, label %39, !llvm.loop !122

39:                                               ; preds = %33
  store ptr %35, ptr %30, align 8, !tbaa !121
  store ptr %30, ptr %34, align 8, !tbaa !6
  br label %40

40:                                               ; preds = %20, %22, %39, %26
  %41 = phi ptr [ null, %22 ], [ %24, %39 ], [ %24, %26 ], [ %4, %20 ]
  ret ptr %41
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define void @fa_heap_free(ptr noundef %0) local_unnamed_addr #11 {
  %2 = ptrtoint ptr %0 to i64
  %3 = and i64 %2, -65536
  %4 = inttoptr i64 %3 to ptr
  %5 = load i32, ptr %4, align 65536, !tbaa !113
  %6 = icmp eq i32 %5, 65535
  br i1 %6, label %7, label %43

7:                                                ; preds = %1
  %8 = getelementptr inbounds %struct.PageHeader, ptr %4, i64 0, i32 1
  %9 = load i32, ptr %8, align 4, !tbaa !115
  %10 = zext i32 %9 to i64
  %11 = getelementptr inbounds %struct.FreeSpan, ptr %4, i64 0, i32 1
  store i64 %10, ptr %11, align 8, !tbaa !119
  br label %12

12:                                               ; preds = %12, %7
  %13 = phi ptr [ null, %7 ], [ %15, %12 ]
  %14 = phi ptr [ @free_spans, %7 ], [ %15, %12 ]
  %15 = load ptr, ptr %14, align 8, !tbaa !6
  %16 = icmp ne ptr %15, null
  %17 = icmp ult ptr %15, %4
  %18 = and i1 %16, %17
  br i1 %18, label %12, label %19, !llvm.loop !123

19:                                               ; preds = %12
  store ptr %15, ptr %4, align 65536, !tbaa !121
  %20 = shl nuw nsw i64 %10, 16
  %21 = getelementptr inbounds i8, ptr %4, i64 %20
  %22 = icmp eq ptr %21, %15
  %23 = select i1 %16, i1 %22, i1 false
  br i1 %23, label %24, label %29

24:                                               ; preds = %19
  %25 = getelementptr inbounds %struct.FreeSpan, ptr %15, i64 0, i32 1
  %26 = load i64, ptr %25, align 8, !tbaa !119
  %27 = add i64 %26, %10
  store i64 %27, ptr %11, align 8, !tbaa !119
  %28 = load ptr, ptr %15, align 8, !tbaa !121
  store ptr %28, ptr %4, align 65536, !tbaa !121
  br label %29

29:                                               ; preds = %24, %19
  %30 = phi ptr [ %28, %24 ], [ %15, %19 ]
  %31 = phi i64 [ %27, %24 ], [ %10, %19 ]
  %32 = icmp eq ptr %13, null
  br i1 %32, label %42, label %33

33:                                               ; preds = %29
  %34 = getelementptr inbounds %struct.FreeSpan, ptr %13, i64 0, i32 1
  %35 = load i64, ptr %34, align 8, !tbaa !119
  %36 = shl i64 %35, 16
  %37 = getelementptr inbounds i8, ptr %13, i64 %36
  %38 = icmp eq ptr %37, %4
  br i1 %38, label %39, label %41

39:                                               ; preds = %33
  %40 = add i64 %35, %31
  store i64 %40, ptr %34, align 8, !tbaa !119
  store ptr %30, ptr %13, align 8, !tbaa !121
  br label %47

41:                                               ; preds = %33
  store ptr %4, ptr %13, align 8, !tbaa !121
  br label %47

42:                                               ; preds = %29
  store ptr %4, ptr @free_spans, align 8, !tbaa !6
  br label %47

43:                                               ; preds = %1
  %44 = zext i32 %5 to i64
  %45 = getelementptr inbounds [40 x ptr], ptr @free_blocks, i64 0, i64 %44
  %46 = load ptr, ptr %45, align 8, !tbaa !6
  store ptr %46, ptr %0, align 8, !tbaa !117
  store ptr %0, ptr %45, align 8, !tbaa !6
  br label %47

47:                                               ; preds = %42, %41, %39, %43
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_leak_check() local_unnamed_addr #0 {
  %1 = load i64, ptr @output_length, align 8, !tbaa !2
  %2 = icmp eq i64 %1, 0
  br i1 %2, label %4, label %3

3:                                                ; preds = %0
  tail call void @fa_write(i32 noundef 1, ptr noundef nonnull @output, i64 noundef %1) #24
  store i64 0, ptr @output_length, align 8, !tbaa !2
  br label %4

4:                                                ; preds = %0, %3
  %5 = load ptr, ptr @builder, align 8, !tbaa !6
  %6 = icmp eq ptr %5, null
  br i1 %6, label %9, label %7

7:                                                ; preds = %4
  %8 = load i64, ptr @builder_capacity, align 8, !tbaa !2
  tail call void @fa_free(ptr noundef nonnull %5, i64 noundef %8, i64 noundef 1) #24
  store ptr null, ptr @builder, align 8, !tbaa !6
  store i64 0, ptr @builder_capacity, align 8, !tbaa !2
  br label %9

9:                                                ; preds = %7, %4
  %10 = load ptr, ptr @argument_items, align 8, !tbaa !6
  %11 = icmp eq ptr %10, null
  br i1 %11, label %16, label %12

12:                                               ; preds = %9
  %13 = load i64, ptr @argument_count, align 8, !tbaa !2
  %14 = mul i64 %13, 24
  %15 = add i64 %14, 24
  tail call void @fa_free(ptr noundef nonnull %10, i64 noundef %15, i64 noundef 8) #24
  store ptr null, ptr @argument_items, align 8, !tbaa !6
  br label %16

16:                                               ; preds = %12, %9
  %17 = load i64, ptr @fa_live_blocks, align 8, !tbaa !2
  %18 = icmp eq i64 %17, 0
  br i1 %18, label %21, label %19

19:                                               ; preds = %16
  store i64 0, ptr @builder_length, align 8, !tbaa !2
  tail call fastcc void @append_c(ptr noundef nonnull @.str.13) #25
  %20 = load i64, ptr @fa_live_blocks, align 8, !tbaa !2
  tail call void @fa_b_u64(i64 noundef %20) #25
  tail call fastcc void @append_c(ptr noundef nonnull @.str.14) #25
  tail call void @fa_b_line(i32 noundef 2) #25
  tail call void @fa_exit(i32 noundef 102) #26
  unreachable

21:                                               ; preds = %16
  ret void
}

; Function Attrs: cold noinline noreturn nounwind optsize
define void @fa_overflow(i64 noundef %0, i64 noundef %1, i32 noundef %2, ptr noundef %3) local_unnamed_addr #15 {
  store i64 0, ptr @builder_length, align 8, !tbaa !2
  tail call fastcc void @reserve(i64 noundef 18) #25
  %5 = load ptr, ptr @builder, align 8, !tbaa !6
  %6 = load i64, ptr @builder_length, align 8, !tbaa !2
  %7 = getelementptr inbounds i8, ptr %5, i64 %6
  tail call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(18) %7, ptr noundef nonnull align 1 dereferenceable(18) @.str.15, i64 18, i1 false)
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
  tail call fastcc void @append_u128(i64 noundef %20, i64 noundef %22, i32 noundef %17) #25
  br label %24

23:                                               ; preds = %4
  tail call fastcc void @append_u128(i64 noundef %0, i64 noundef %1, i32 noundef 0) #25
  br label %24

24:                                               ; preds = %23, %10
  tail call fastcc void @append_c(ptr noundef nonnull @.str.16) #25
  tail call fastcc void @append_c(ptr noundef %3) #25
  br label %25

25:                                               ; preds = %25, %24
  %26 = phi i64 [ 0, %24 ], [ %30, %25 ]
  %27 = getelementptr inbounds i8, ptr %3, i64 %26
  %28 = load i8, ptr %27, align 1, !tbaa !10
  %29 = icmp eq i8 %28, 0
  %30 = add i64 %26, 1
  br i1 %29, label %31, label %25, !llvm.loop !22

31:                                               ; preds = %25
  %32 = getelementptr inbounds i8, ptr %3, i64 %26
  %33 = getelementptr inbounds i8, ptr %32, i64 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %33) #28
  unreachable
}

; Function Attrs: noreturn nounwind optsize
define internal fastcc void @stop_with_builder(ptr noundef %0) unnamed_addr #2 {
  tail call void @fa_flush() #25
  %2 = load ptr, ptr @builder, align 8, !tbaa !6
  %3 = load i64, ptr @builder_length, align 8, !tbaa !2
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.36, i64 noundef 6) #24
  tail call void @fa_write(i32 noundef 2, ptr noundef %2, i64 noundef %3) #24
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.37, i64 noundef 6) #24
  br label %4

4:                                                ; preds = %4, %1
  %5 = phi i64 [ 0, %1 ], [ %9, %4 ]
  %6 = getelementptr inbounds i8, ptr %0, i64 %5
  %7 = load i8, ptr %6, align 1, !tbaa !10
  %8 = icmp eq i8 %7, 0
  %9 = add i64 %5, 1
  br i1 %8, label %10, label %4, !llvm.loop !22

10:                                               ; preds = %4
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull %0, i64 noundef %5) #24
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.21, i64 noundef 1) #24
  tail call void @fa_exit(i32 noundef 101) #26
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define void @fa_overflow_unknown(ptr noundef %0) local_unnamed_addr #15 {
  store i64 0, ptr @builder_length, align 8, !tbaa !2
  tail call fastcc void @append_c(ptr noundef nonnull @.str.17) #25
  tail call fastcc void @append_c(ptr noundef %0) #25
  br label %2

2:                                                ; preds = %2, %1
  %3 = phi i64 [ 0, %1 ], [ %7, %2 ]
  %4 = getelementptr inbounds i8, ptr %0, i64 %3
  %5 = load i8, ptr %4, align 1, !tbaa !10
  %6 = icmp eq i8 %5, 0
  %7 = add i64 %3, 1
  br i1 %6, label %8, label %2, !llvm.loop !22

8:                                                ; preds = %2
  %9 = getelementptr inbounds i8, ptr %0, i64 %3
  %10 = getelementptr inbounds i8, ptr %9, i64 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %10) #28
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define void @fa_float_overflow(double noundef %0, ptr noundef %1) local_unnamed_addr #15 {
  store i64 0, ptr @builder_length, align 8, !tbaa !2
  tail call fastcc void @append_c(ptr noundef nonnull @.str.15) #25
  tail call void @fa_b_f64(double noundef %0) #25
  tail call fastcc void @append_c(ptr noundef nonnull @.str.16) #25
  tail call fastcc void @append_c(ptr noundef %1) #25
  br label %3

3:                                                ; preds = %3, %2
  %4 = phi i64 [ 0, %2 ], [ %8, %3 ]
  %5 = getelementptr inbounds i8, ptr %1, i64 %4
  %6 = load i8, ptr %5, align 1, !tbaa !10
  %7 = icmp eq i8 %6, 0
  %8 = add i64 %4, 1
  br i1 %7, label %9, label %3, !llvm.loop !22

9:                                                ; preds = %3
  %10 = getelementptr inbounds i8, ptr %1, i64 %4
  %11 = getelementptr inbounds i8, ptr %10, i64 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %11) #28
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define void @fa_assert_failed(ptr noundef %0) local_unnamed_addr #15 {
  store i64 0, ptr @builder_length, align 8, !tbaa !2
  tail call fastcc void @append_c(ptr noundef nonnull @.str.18) #25
  tail call fastcc void @append_c(ptr noundef %0) #25
  br label %2

2:                                                ; preds = %2, %1
  %3 = phi i64 [ 0, %1 ], [ %7, %2 ]
  %4 = getelementptr inbounds i8, ptr %0, i64 %3
  %5 = load i8, ptr %4, align 1, !tbaa !10
  %6 = icmp eq i8 %5, 0
  %7 = add i64 %3, 1
  br i1 %6, label %8, label %2, !llvm.loop !22

8:                                                ; preds = %2
  %9 = getelementptr inbounds i8, ptr %0, i64 %3
  %10 = getelementptr inbounds i8, ptr %9, i64 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %10) #28
  unreachable
}

; Function Attrs: nounwind optsize
define void @fa_guard_stack() local_unnamed_addr #0 {
  %1 = alloca %struct.AlternateStack, align 8
  %2 = alloca %struct.SignalAction, align 8
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %1) #27
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %1, ptr noundef nonnull align 8 dereferenceable(24) @__const.fa_guard_stack.stack, i64 24, i1 false)
  %3 = call i32 @sigaltstack(ptr noundef nonnull %1, ptr noundef null) #24
  call void @llvm.lifetime.start.p0(i64 152, ptr nonnull %2) #27
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(152) %2, ptr noundef nonnull align 8 dereferenceable(152) @__const.fa_guard_stack.action, i64 152, i1 false)
  %4 = call i32 @sigaction(i32 noundef 11, ptr noundef nonnull %2, ptr noundef null) #24
  call void @llvm.lifetime.end.p0(i64 152, ptr nonnull %2) #27
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %1) #27
  ret void
}

; Function Attrs: optsize
declare i32 @sigaltstack(ptr noundef, ptr noundef) local_unnamed_addr #1

; Function Attrs: noreturn nounwind optsize
define internal void @on_fault(i32 %0) #2 {
  tail call void @fa_flush() #25
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @on_fault.message, i64 noundef 21) #24
  tail call void @_exit(i32 noundef 101) #26
  unreachable
}

; Function Attrs: optsize
declare i32 @sigaction(i32 noundef, ptr noundef, ptr noundef) local_unnamed_addr #1

; Function Attrs: cold noinline noreturn nounwind optsize
define void @fa_assert_failed_operands(ptr noundef %0) local_unnamed_addr #15 {
  %2 = alloca %struct.FaString, align 8
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %2) #27
  call void @fa_b_take(ptr noundef nonnull %2) #25
  tail call fastcc void @append_c(ptr noundef nonnull @.str.18) #25
  tail call fastcc void @append_c(ptr noundef %0) #25
  tail call fastcc void @append_c(ptr noundef nonnull @.str.19) #25
  %3 = load ptr, ptr %2, align 8, !tbaa !34
  %4 = getelementptr inbounds %struct.FaString, ptr %2, i64 0, i32 1
  %5 = load i64, ptr %4, align 8, !tbaa !36
  tail call void @fa_b_bytes(ptr noundef %3, i64 noundef %5) #25
  br label %6

6:                                                ; preds = %6, %1
  %7 = phi i64 [ 0, %1 ], [ %11, %6 ]
  %8 = getelementptr inbounds i8, ptr %0, i64 %7
  %9 = load i8, ptr %8, align 1, !tbaa !10
  %10 = icmp eq i8 %9, 0
  %11 = add i64 %7, 1
  br i1 %10, label %12, label %6, !llvm.loop !22

12:                                               ; preds = %6
  %13 = getelementptr inbounds i8, ptr %0, i64 %7
  %14 = getelementptr inbounds i8, ptr %13, i64 1
  tail call fastcc void @stop_with_builder(ptr noundef nonnull %14) #28
  unreachable
}

; Function Attrs: cold noinline noreturn nounwind optsize
define void @fa_uncaught(ptr noundef %0) local_unnamed_addr #15 {
  store i64 0, ptr @builder_length, align 8, !tbaa !2
  tail call void @fa_format_union(ptr noundef %0, i32 noundef 1) #24
  tail call void @fa_flush() #25
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.20, i64 noundef 7) #24
  %2 = load ptr, ptr @builder, align 8, !tbaa !6
  %3 = load i64, ptr @builder_length, align 8, !tbaa !2
  tail call void @fa_write(i32 noundef 2, ptr noundef %2, i64 noundef %3) #24
  tail call void @fa_write(i32 noundef 2, ptr noundef nonnull @.str.21, i64 noundef 1) #24
  tail call void @fa_exit(i32 noundef 1) #26
  unreachable
}

; Function Attrs: optsize
declare void @fa_format_union(ptr noundef, i32 noundef) local_unnamed_addr #1

; Function Attrs: cold mustprogress nofree noinline norecurse nosync nounwind optsize willreturn memory(none)
define noundef nonnull ptr @fa_error_division_by_zero() local_unnamed_addr #16 {
  ret ptr @division_by_zero
}

; Function Attrs: cold noinline nounwind optsize
define noundef ptr @fa_error_conversion(i64 noundef %0, i64 noundef %1, i32 noundef %2, ptr nocapture noundef readonly %3) local_unnamed_addr #10 {
  store i64 0, ptr @builder_length, align 8, !tbaa !2
  %5 = icmp eq i32 %2, 0
  br i1 %5, label %19, label %6

6:                                                ; preds = %4
  %7 = zext i64 %1 to i128
  %8 = shl nuw i128 %7, 64
  %9 = zext i64 %0 to i128
  %10 = or disjoint i128 %8, %9
  %11 = icmp slt i64 %1, 0
  %12 = lshr i64 %1, 63
  %13 = trunc i64 %12 to i32
  %14 = sub i128 0, %10
  %15 = select i1 %11, i128 %14, i128 %10
  %16 = trunc i128 %15 to i64
  %17 = lshr i128 %15, 64
  %18 = trunc i128 %17 to i64
  tail call fastcc void @append_u128(i64 noundef %16, i64 noundef %18, i32 noundef %13) #25
  br label %20

19:                                               ; preds = %4
  tail call fastcc void @append_u128(i64 noundef %0, i64 noundef %1, i32 noundef 0) #25
  br label %20

20:                                               ; preds = %19, %6
  tail call fastcc void @reserve(i64 noundef 16) #25
  %21 = load ptr, ptr @builder, align 8, !tbaa !6
  %22 = load i64, ptr @builder_length, align 8, !tbaa !2
  %23 = getelementptr inbounds i8, ptr %21, i64 %22
  tail call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(16) %23, ptr noundef nonnull align 1 dereferenceable(16) @.str.16, i64 16, i1 false)
  %24 = add i64 %22, 16
  store i64 %24, ptr @builder_length, align 8, !tbaa !2
  br label %25

25:                                               ; preds = %25, %20
  %26 = phi i64 [ 0, %20 ], [ %30, %25 ]
  %27 = getelementptr inbounds i8, ptr %3, i64 %26
  %28 = load i8, ptr %27, align 1, !tbaa !10
  %29 = icmp eq i8 %28, 0
  %30 = add i64 %26, 1
  br i1 %29, label %31, label %25, !llvm.loop !22

31:                                               ; preds = %25
  tail call fastcc void @reserve(i64 noundef %26) #25
  %32 = load ptr, ptr @builder, align 8, !tbaa !6
  %33 = load i64, ptr @builder_length, align 8, !tbaa !2
  %34 = getelementptr inbounds i8, ptr %32, i64 %33
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %34, ptr nonnull align 1 %3, i64 %26, i1 false)
  %35 = add i64 %33, %26
  store i64 %35, ptr @builder_length, align 8, !tbaa !2
  %36 = tail call ptr @fa_alloc(i64 noundef 32, i64 noundef 8) #24
  store i64 4, ptr %36, align 8, !tbaa !124
  %37 = getelementptr inbounds %struct.MessageError, ptr %36, i64 0, i32 1
  tail call void @fa_b_take(ptr noundef nonnull %37) #25
  ret ptr %36
}

; Function Attrs: cold noinline nounwind optsize
define noundef ptr @fa_error_negative_exponent(i64 noundef %0) local_unnamed_addr #10 {
  store i64 0, ptr @builder_length, align 8, !tbaa !2
  tail call fastcc void @reserve(i64 noundef 19) #25
  %2 = load ptr, ptr @builder, align 8, !tbaa !6
  %3 = load i64, ptr @builder_length, align 8, !tbaa !2
  %4 = getelementptr inbounds i8, ptr %2, i64 %3
  tail call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(19) %4, ptr noundef nonnull align 1 dereferenceable(19) @.str.22, i64 19, i1 false)
  %5 = add i64 %3, 19
  store i64 %5, ptr @builder_length, align 8, !tbaa !2
  %6 = tail call i64 @llvm.abs.i64(i64 %0, i1 false)
  %7 = lshr i64 %0, 63
  %8 = trunc i64 %7 to i32
  tail call fastcc void @append_u64(i64 noundef %6, i32 noundef %8) #25
  %9 = tail call ptr @fa_alloc(i64 noundef 32, i64 noundef 8) #24
  store i64 4, ptr %9, align 8, !tbaa !124
  %10 = getelementptr inbounds %struct.MessageError, ptr %9, i64 0, i32 1
  tail call void @fa_b_take(ptr noundef nonnull %10) #25
  ret ptr %9
}

; Function Attrs: cold noinline nounwind optsize
define noundef ptr @fa_error_float(double noundef %0, i32 noundef %1) local_unnamed_addr #10 {
  store i64 0, ptr @builder_length, align 8, !tbaa !2
  tail call fastcc void @append_double(double noundef %0) #25
  %3 = icmp eq i32 %1, 0
  %4 = select i1 %3, ptr @.str.24, ptr @.str.23
  br label %5

5:                                                ; preds = %5, %2
  %6 = phi i64 [ 0, %2 ], [ %10, %5 ]
  %7 = getelementptr inbounds i8, ptr %4, i64 %6
  %8 = load i8, ptr %7, align 1, !tbaa !10
  %9 = icmp eq i8 %8, 0
  %10 = add i64 %6, 1
  br i1 %9, label %11, label %5, !llvm.loop !22

11:                                               ; preds = %5
  tail call fastcc void @reserve(i64 noundef %6) #25
  %12 = load ptr, ptr @builder, align 8, !tbaa !6
  %13 = load i64, ptr @builder_length, align 8, !tbaa !2
  %14 = getelementptr inbounds i8, ptr %12, i64 %13
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %14, ptr nonnull align 1 %4, i64 %6, i1 false)
  %15 = add i64 %13, %6
  store i64 %15, ptr @builder_length, align 8, !tbaa !2
  %16 = tail call ptr @fa_alloc(i64 noundef 32, i64 noundef 8) #24
  store i64 4, ptr %16, align 8, !tbaa !124
  %17 = getelementptr inbounds %struct.MessageError, ptr %16, i64 0, i32 1
  tail call void @fa_b_take(ptr noundef nonnull %17) #25
  ret ptr %16
}

; Function Attrs: cold noinline nounwind optsize
define noundef ptr @fa_error_shift(i64 noundef %0, i64 noundef %1) local_unnamed_addr #10 {
  store i64 0, ptr @builder_length, align 8, !tbaa !2
  tail call fastcc void @reserve(i64 noundef 13) #25
  %3 = load ptr, ptr @builder, align 8, !tbaa !6
  %4 = load i64, ptr @builder_length, align 8, !tbaa !2
  %5 = getelementptr inbounds i8, ptr %3, i64 %4
  tail call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(13) %5, ptr noundef nonnull align 1 dereferenceable(13) @.str.25, i64 13, i1 false)
  %6 = add i64 %4, 13
  store i64 %6, ptr @builder_length, align 8, !tbaa !2
  %7 = tail call i64 @llvm.abs.i64(i64 %0, i1 false)
  %8 = lshr i64 %0, 63
  %9 = trunc i64 %8 to i32
  tail call fastcc void @append_u64(i64 noundef %7, i32 noundef %9) #25
  tail call fastcc void @reserve(i64 noundef 15) #25
  %10 = load ptr, ptr @builder, align 8, !tbaa !6
  %11 = load i64, ptr @builder_length, align 8, !tbaa !2
  %12 = getelementptr inbounds i8, ptr %10, i64 %11
  tail call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(15) %12, ptr noundef nonnull align 1 dereferenceable(15) @.str.26, i64 15, i1 false)
  %13 = add i64 %11, 15
  store i64 %13, ptr @builder_length, align 8, !tbaa !2
  %14 = tail call i64 @llvm.abs.i64(i64 %1, i1 false)
  %15 = lshr i64 %1, 63
  %16 = trunc i64 %15 to i32
  tail call fastcc void @append_u64(i64 noundef %14, i32 noundef %16) #25
  %17 = tail call ptr @fa_alloc(i64 noundef 32, i64 noundef 8) #24
  store i64 4, ptr %17, align 8, !tbaa !124
  %18 = getelementptr inbounds %struct.MessageError, ptr %17, i64 0, i32 1
  tail call void @fa_b_take(ptr noundef nonnull %18) #25
  ret ptr %17
}

; Function Attrs: cold noinline nounwind optsize
define noundef ptr @fa_error_checked(i64 noundef %0, i64 noundef %1, ptr nocapture noundef readonly %2) local_unnamed_addr #10 {
  store i64 0, ptr @builder_length, align 8, !tbaa !2
  tail call fastcc void @reserve(i64 noundef 18) #25
  %4 = load ptr, ptr @builder, align 8, !tbaa !6
  %5 = load i64, ptr @builder_length, align 8, !tbaa !2
  %6 = getelementptr inbounds i8, ptr %4, i64 %5
  tail call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(18) %6, ptr noundef nonnull align 1 dereferenceable(18) @.str.15, i64 18, i1 false)
  %7 = add i64 %5, 18
  store i64 %7, ptr @builder_length, align 8, !tbaa !2
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
  tail call fastcc void @append_u128(i64 noundef %17, i64 noundef %19, i32 noundef %14) #25
  tail call fastcc void @reserve(i64 noundef 16) #25
  %20 = load ptr, ptr @builder, align 8, !tbaa !6
  %21 = load i64, ptr @builder_length, align 8, !tbaa !2
  %22 = getelementptr inbounds i8, ptr %20, i64 %21
  tail call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(16) %22, ptr noundef nonnull align 1 dereferenceable(16) @.str.16, i64 16, i1 false)
  %23 = add i64 %21, 16
  store i64 %23, ptr @builder_length, align 8, !tbaa !2
  br label %24

24:                                               ; preds = %24, %3
  %25 = phi i64 [ 0, %3 ], [ %29, %24 ]
  %26 = getelementptr inbounds i8, ptr %2, i64 %25
  %27 = load i8, ptr %26, align 1, !tbaa !10
  %28 = icmp eq i8 %27, 0
  %29 = add i64 %25, 1
  br i1 %28, label %30, label %24, !llvm.loop !22

30:                                               ; preds = %24
  tail call fastcc void @reserve(i64 noundef %25) #25
  %31 = load ptr, ptr @builder, align 8, !tbaa !6
  %32 = load i64, ptr @builder_length, align 8, !tbaa !2
  %33 = getelementptr inbounds i8, ptr %31, i64 %32
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %33, ptr nonnull align 1 %2, i64 %25, i1 false)
  %34 = add i64 %32, %25
  store i64 %34, ptr @builder_length, align 8, !tbaa !2
  %35 = tail call ptr @fa_alloc(i64 noundef 32, i64 noundef 8) #24
  store i64 4, ptr %35, align 8, !tbaa !124
  %36 = getelementptr inbounds %struct.MessageError, ptr %35, i64 0, i32 1
  tail call void @fa_b_take(ptr noundef nonnull %36) #25
  ret ptr %35
}

; Function Attrs: nounwind optsize
define noundef ptr @fa_parse_integer(ptr nocapture noundef readonly %0, i64 noundef %1, ptr nocapture noundef writeonly %2) local_unnamed_addr #0 {
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %13, label %5

5:                                                ; preds = %3
  %6 = load i8, ptr %0, align 1, !tbaa !10
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
  br i1 %12, label %22, label %32

13:                                               ; preds = %3
  store i64 0, ptr @builder_length, align 8, !tbaa !2
  br label %33

14:                                               ; preds = %22
  %15 = icmp eq i32 %29, 0
  br i1 %15, label %16, label %17

16:                                               ; preds = %14
  store i64 0, ptr @builder_length, align 8, !tbaa !2
  br label %38

17:                                               ; preds = %14
  br i1 %12, label %18, label %85

18:                                               ; preds = %17
  %19 = getelementptr inbounds i8, ptr %0, i64 %11
  %20 = load i8, ptr %19, align 1, !tbaa !10
  %21 = zext i8 %20 to i64
  br label %74

22:                                               ; preds = %9, %22
  %23 = phi i64 [ %30, %22 ], [ %11, %9 ]
  %24 = phi i32 [ %29, %22 ], [ 1, %9 ]
  %25 = getelementptr inbounds i8, ptr %0, i64 %23
  %26 = load i8, ptr %25, align 1, !tbaa !10
  %27 = add i8 %26, -48
  %28 = icmp ult i8 %27, 10
  %29 = select i1 %28, i32 %24, i32 0
  %30 = add nuw i64 %23, 1
  %31 = icmp eq i64 %30, %1
  br i1 %31, label %14, label %22, !llvm.loop !126

32:                                               ; preds = %9
  store i64 0, ptr @builder_length, align 8, !tbaa !2
  br i1 %4, label %33, label %38

33:                                               ; preds = %13, %32
  tail call fastcc void @reserve(i64 noundef 17) #25
  %34 = load ptr, ptr @builder, align 8, !tbaa !6
  %35 = load i64, ptr @builder_length, align 8, !tbaa !2
  %36 = getelementptr inbounds i8, ptr %34, i64 %35
  tail call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(17) %36, ptr noundef nonnull align 1 dereferenceable(17) @.str.27, i64 17, i1 false)
  %37 = add i64 %35, 17
  br label %64

38:                                               ; preds = %16, %32
  tail call fastcc void @reserve(i64 noundef 1) #25
  %39 = load ptr, ptr @builder, align 8, !tbaa !6
  %40 = load i64, ptr @builder_length, align 8, !tbaa !2
  %41 = getelementptr inbounds i8, ptr %39, i64 %40
  store i8 34, ptr %41, align 1, !tbaa !10
  %42 = add i64 %40, 1
  store i64 %42, ptr @builder_length, align 8, !tbaa !2
  br label %43

43:                                               ; preds = %38, %52
  %44 = phi i64 [ 0, %38 ], [ %57, %52 ]
  %45 = getelementptr inbounds i8, ptr %0, i64 %44
  %46 = load i8, ptr %45, align 1, !tbaa !10
  switch i8 %46, label %52 [
    i8 92, label %47
    i8 34, label %47
  ]

47:                                               ; preds = %43, %43
  tail call fastcc void @reserve(i64 noundef 1) #25
  %48 = load ptr, ptr @builder, align 8, !tbaa !6
  %49 = load i64, ptr @builder_length, align 8, !tbaa !2
  %50 = getelementptr inbounds i8, ptr %48, i64 %49
  store i8 92, ptr %50, align 1, !tbaa !10
  %51 = add i64 %49, 1
  store i64 %51, ptr @builder_length, align 8, !tbaa !2
  br label %52

52:                                               ; preds = %43, %47
  tail call fastcc void @reserve(i64 noundef 1) #25
  %53 = load ptr, ptr @builder, align 8, !tbaa !6
  %54 = load i64, ptr @builder_length, align 8, !tbaa !2
  %55 = getelementptr inbounds i8, ptr %53, i64 %54
  store i8 %46, ptr %55, align 1, !tbaa !10
  %56 = add i64 %54, 1
  store i64 %56, ptr @builder_length, align 8, !tbaa !2
  %57 = add nuw i64 %44, 1
  %58 = icmp eq i64 %57, %1
  br i1 %58, label %59, label %43, !llvm.loop !127

59:                                               ; preds = %52
  tail call fastcc void @reserve(i64 noundef 19) #25
  %60 = load ptr, ptr @builder, align 8, !tbaa !6
  %61 = load i64, ptr @builder_length, align 8, !tbaa !2
  %62 = getelementptr inbounds i8, ptr %60, i64 %61
  tail call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(19) %62, ptr noundef nonnull align 1 dereferenceable(19) @.str.28, i64 19, i1 false)
  %63 = add i64 %61, 19
  br label %64

64:                                               ; preds = %59, %33
  %65 = phi i64 [ %63, %59 ], [ %37, %33 ]
  store i64 %65, ptr @builder_length, align 8, !tbaa !2
  %66 = tail call fastcc ptr @parse_error(ptr noundef %0, i64 noundef %1) #25
  br label %104

67:                                               ; preds = %74
  %68 = getelementptr inbounds i8, ptr %0, i64 %81
  %69 = load i8, ptr %68, align 1, !tbaa !10
  %70 = zext i8 %69 to i64
  %71 = sub nsw i64 47, %70
  %72 = udiv i64 %71, 10
  %73 = icmp ugt i64 %80, %72
  br i1 %73, label %83, label %74, !llvm.loop !128

74:                                               ; preds = %18, %67
  %75 = phi i64 [ %21, %18 ], [ %70, %67 ]
  %76 = phi i64 [ 0, %18 ], [ %80, %67 ]
  %77 = phi i64 [ %11, %18 ], [ %81, %67 ]
  %78 = mul nuw i64 %76, 10
  %79 = add i64 %78, -48
  %80 = add i64 %79, %75
  %81 = add i64 %77, 1
  %82 = icmp eq i64 %81, %1
  br i1 %82, label %83, label %67, !llvm.loop !128

83:                                               ; preds = %67, %74
  %84 = icmp ult i64 %81, %1
  br label %85

85:                                               ; preds = %83, %17
  %86 = phi i64 [ 0, %17 ], [ %80, %83 ]
  %87 = phi i1 [ false, %17 ], [ %84, %83 ]
  %88 = select i1 %10, i64 9223372036854775807, i64 -9223372036854775808
  %89 = icmp ugt i64 %86, %88
  %90 = select i1 %87, i1 true, i1 %89
  br i1 %90, label %91, label %101

91:                                               ; preds = %85
  store i64 0, ptr @builder_length, align 8, !tbaa !2
  tail call fastcc void @reserve(i64 noundef %1) #25
  %92 = load ptr, ptr @builder, align 8, !tbaa !6
  %93 = load i64, ptr @builder_length, align 8, !tbaa !2
  %94 = getelementptr inbounds i8, ptr %92, i64 %93
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %94, ptr nonnull align 1 %0, i64 %1, i1 false)
  %95 = add i64 %93, %1
  store i64 %95, ptr @builder_length, align 8, !tbaa !2
  tail call fastcc void @reserve(i64 noundef 23) #25
  %96 = load ptr, ptr @builder, align 8, !tbaa !6
  %97 = load i64, ptr @builder_length, align 8, !tbaa !2
  %98 = getelementptr inbounds i8, ptr %96, i64 %97
  tail call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(23) %98, ptr noundef nonnull align 1 dereferenceable(23) @.str.23, i64 23, i1 false)
  %99 = add i64 %97, 23
  store i64 %99, ptr @builder_length, align 8, !tbaa !2
  %100 = tail call fastcc ptr @parse_error(ptr noundef nonnull %0, i64 noundef %1) #25
  br label %104

101:                                              ; preds = %85
  %102 = sub i64 0, %86
  %103 = select i1 %10, i64 %86, i64 %102
  store i64 %103, ptr %2, align 8, !tbaa !2
  br label %104

104:                                              ; preds = %91, %101, %64
  %105 = phi ptr [ %66, %64 ], [ %100, %91 ], [ null, %101 ]
  ret ptr %105
}

; Function Attrs: nounwind optsize
define internal fastcc noundef ptr @parse_error(ptr nocapture noundef readonly %0, i64 noundef %1) unnamed_addr #0 {
  %3 = tail call ptr @fa_alloc(i64 noundef 56, i64 noundef 8) #24
  store i64 5, ptr %3, align 8, !tbaa !129
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %7, label %5

5:                                                ; preds = %2
  %6 = tail call ptr @fa_alloc(i64 noundef %1, i64 noundef 1) #24, !noalias !131
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %6, ptr align 1 %0, i64 %1, i1 false), !noalias !131
  br label %7

7:                                                ; preds = %2, %5
  %8 = phi ptr [ %6, %5 ], [ @.str.29, %2 ]
  %9 = getelementptr inbounds %struct.ParseError, ptr %3, i64 0, i32 1
  store ptr %8, ptr %9, align 8, !tbaa.struct !14
  %10 = getelementptr inbounds %struct.ParseError, ptr %3, i64 0, i32 1, i32 1
  store i64 %1, ptr %10, align 8, !tbaa.struct !15
  %11 = getelementptr inbounds %struct.ParseError, ptr %3, i64 0, i32 1, i32 2
  store i64 %1, ptr %11, align 8, !tbaa.struct !16
  %12 = getelementptr inbounds %struct.ParseError, ptr %3, i64 0, i32 2
  tail call void @fa_b_take(ptr noundef nonnull %12) #25
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
declare { i64, i1 } @llvm.smul.with.overflow.i64(i64, i64) #17

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(read, inaccessiblemem: none)
define ptr @fa_table_entry(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #18 {
  %4 = load ptr, ptr %0, align 8, !tbaa !134
  %5 = icmp ult i64 %1, 4
  br i1 %5, label %12, label %6

6:                                                ; preds = %3
  %7 = lshr i64 %1, 2
  %8 = tail call i64 @llvm.ctlz.i64(i64 %7, i1 true), !range !106
  %9 = sub nuw nsw i64 64, %8
  %10 = sub nuw nsw i64 65, %8
  %11 = shl nsw i64 -1, %10
  br label %12

12:                                               ; preds = %3, %6
  %13 = phi i64 [ %11, %6 ], [ 0, %3 ]
  %14 = phi i64 [ %9, %6 ], [ 0, %3 ]
  %15 = getelementptr inbounds %struct.FaSegmented, ptr %4, i64 0, i32 2, i64 %14
  %16 = load ptr, ptr %15, align 8, !tbaa !6
  %17 = add i64 %13, %1
  %18 = mul nsw i64 %17, %2
  %19 = getelementptr inbounds i8, ptr %16, i64 %18
  ret ptr %19
}

; Function Attrs: nounwind optsize
define ptr @fa_table_new(i64 noundef %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = tail call ptr @fa_alloc(i64 noundef 56, i64 noundef 8) #24
  %4 = tail call i64 @llvm.smax.i64(i64 %0, i64 4)
  %5 = mul nsw i64 %4, %1
  %6 = tail call ptr @fa_alloc(i64 noundef %5, i64 noundef 16) #24
  store ptr %6, ptr %3, align 8, !tbaa !134
  %7 = getelementptr inbounds %struct.FaTable, ptr %3, i64 0, i32 1
  store i64 0, ptr %7, align 8, !tbaa !136
  %8 = getelementptr inbounds %struct.FaTable, ptr %3, i64 0, i32 2
  store i64 0, ptr %8, align 8, !tbaa !137
  %9 = getelementptr inbounds %struct.FaTable, ptr %3, i64 0, i32 3
  store i64 %4, ptr %9, align 8, !tbaa !138
  %10 = getelementptr inbounds %struct.FaTable, ptr %3, i64 0, i32 4
  store ptr null, ptr %10, align 8, !tbaa !139
  %11 = getelementptr inbounds %struct.FaTable, ptr %3, i64 0, i32 6
  store i64 0, ptr %11, align 8, !tbaa !140
  %12 = shl nuw nsw i64 %4, 1
  br label %13

13:                                               ; preds = %13, %2
  %14 = phi i64 [ 8, %2 ], [ %16, %13 ]
  %15 = icmp slt i64 %14, %12
  %16 = shl nsw i64 %14, 1
  br i1 %15, label %13, label %17, !llvm.loop !141

17:                                               ; preds = %13
  tail call fastcc void @table_index(ptr noundef nonnull %3, i64 noundef %14, i64 noundef %1) #25
  ret ptr %3
}

; Function Attrs: nounwind optsize
define internal fastcc void @table_index(ptr nocapture noundef %0, i64 noundef %1, i64 noundef %2) unnamed_addr #0 {
  %4 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 4
  %5 = load ptr, ptr %4, align 8, !tbaa !139
  %6 = icmp eq ptr %5, null
  br i1 %6, label %12, label %7

7:                                                ; preds = %3
  %8 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 5
  %9 = load i64, ptr %8, align 8, !tbaa !142
  %10 = shl i64 %9, 3
  %11 = add i64 %10, 8
  tail call void @fa_free(ptr noundef nonnull %5, i64 noundef %11, i64 noundef 8) #24
  br label %12

12:                                               ; preds = %7, %3
  %13 = shl nsw i64 %1, 3
  %14 = tail call ptr @fa_alloc(i64 noundef %13, i64 noundef 8) #24
  store ptr %14, ptr %4, align 8, !tbaa !139
  tail call void @llvm.memset.p0.i64(ptr align 8 %14, i8 0, i64 %13, i1 false)
  %15 = add nsw i64 %1, -1
  %16 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 5
  store i64 %15, ptr %16, align 8, !tbaa !142
  %17 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 2
  %18 = load i64, ptr %17, align 8, !tbaa !137
  %19 = icmp sgt i64 %18, 0
  br i1 %19, label %20, label %23

20:                                               ; preds = %12
  %21 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 6
  %22 = load ptr, ptr %0, align 8, !tbaa !134
  br label %24

23:                                               ; preds = %68, %12
  ret void

24:                                               ; preds = %20, %68
  %25 = phi i64 [ %18, %20 ], [ %70, %68 ]
  %26 = phi i64 [ 0, %20 ], [ %69, %68 ]
  %27 = load i64, ptr %21, align 8, !tbaa !140
  %28 = icmp eq i64 %27, 0
  br i1 %28, label %43, label %29

29:                                               ; preds = %24
  %30 = icmp ult i64 %26, 4
  br i1 %30, label %37, label %31

31:                                               ; preds = %29
  %32 = lshr i64 %26, 2
  %33 = tail call i64 @llvm.ctlz.i64(i64 %32, i1 true), !range !106
  %34 = sub nuw nsw i64 64, %33
  %35 = sub nuw nsw i64 65, %33
  %36 = shl nsw i64 -1, %35
  br label %37

37:                                               ; preds = %31, %29
  %38 = phi i64 [ %36, %31 ], [ 0, %29 ]
  %39 = phi i64 [ %34, %31 ], [ 0, %29 ]
  %40 = getelementptr inbounds %struct.FaSegmented, ptr %22, i64 0, i32 2, i64 %39
  %41 = load ptr, ptr %40, align 8, !tbaa !6
  %42 = add i64 %38, %26
  br label %43

43:                                               ; preds = %24, %37
  %44 = phi i64 [ %42, %37 ], [ %26, %24 ]
  %45 = phi ptr [ %41, %37 ], [ %22, %24 ]
  %46 = mul nsw i64 %44, %2
  %47 = getelementptr inbounds i8, ptr %45, i64 %46
  %48 = getelementptr inbounds i8, ptr %47, i64 8
  %49 = load i64, ptr %48, align 8, !tbaa !2
  %50 = icmp eq i64 %49, 0
  br i1 %50, label %51, label %53

51:                                               ; preds = %43
  %52 = add nuw nsw i64 %26, 1
  br label %68

53:                                               ; preds = %43
  %54 = load i64, ptr %47, align 8, !tbaa !2
  %55 = load i64, ptr %16, align 8, !tbaa !142
  %56 = load ptr, ptr %4, align 8, !tbaa !139
  br label %57

57:                                               ; preds = %57, %53
  %58 = phi i64 [ %54, %53 ], [ %63, %57 ]
  %59 = and i64 %58, %55
  %60 = getelementptr inbounds i64, ptr %56, i64 %59
  %61 = load i64, ptr %60, align 8, !tbaa !2
  %62 = icmp eq i64 %61, 0
  %63 = add nsw i64 %59, 1
  br i1 %62, label %64, label %57, !llvm.loop !143

64:                                               ; preds = %57
  %65 = getelementptr inbounds i64, ptr %56, i64 %59
  %66 = add nuw nsw i64 %26, 1
  store i64 %66, ptr %65, align 8, !tbaa !2
  %67 = load i64, ptr %17, align 8, !tbaa !137
  br label %68

68:                                               ; preds = %51, %64
  %69 = phi i64 [ %52, %51 ], [ %66, %64 ]
  %70 = phi i64 [ %25, %51 ], [ %67, %64 ]
  %71 = icmp slt i64 %69, %70
  br i1 %71, label %24, label %23, !llvm.loop !144
}

; Function Attrs: nounwind optsize
define ptr @fa_table_new_growing(i64 noundef %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = tail call ptr @fa_alloc(i64 noundef 56, i64 noundef 8) #24
  %4 = tail call i64 @llvm.smax.i64(i64 %0, i64 4)
  %5 = tail call noundef ptr @fa_alloc(i64 noundef 400, i64 noundef 8) #24
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(400) %5, i8 0, i64 400, i1 false)
  store ptr %5, ptr %3, align 8, !tbaa !134
  %6 = getelementptr inbounds %struct.FaTable, ptr %3, i64 0, i32 1
  store i64 0, ptr %6, align 8, !tbaa !136
  %7 = getelementptr inbounds %struct.FaTable, ptr %3, i64 0, i32 2
  store i64 0, ptr %7, align 8, !tbaa !137
  %8 = getelementptr inbounds %struct.FaTable, ptr %3, i64 0, i32 3
  store i64 %4, ptr %8, align 8, !tbaa !138
  %9 = getelementptr inbounds %struct.FaTable, ptr %3, i64 0, i32 4
  store ptr null, ptr %9, align 8, !tbaa !139
  %10 = getelementptr inbounds %struct.FaTable, ptr %3, i64 0, i32 6
  store i64 1, ptr %10, align 8, !tbaa !140
  %11 = shl nuw nsw i64 %4, 1
  br label %12

12:                                               ; preds = %12, %2
  %13 = phi i64 [ 8, %2 ], [ %15, %12 ]
  %14 = icmp slt i64 %13, %11
  %15 = shl nsw i64 %13, 1
  br i1 %14, label %12, label %16, !llvm.loop !141

16:                                               ; preds = %12
  tail call fastcc void @table_index(ptr noundef nonnull %3, i64 noundef %13, i64 noundef %1) #25
  ret ptr %3
}

; Function Attrs: alwaysinline nounwind optsize
define i64 @fa_table_find(ptr nocapture noundef readonly %0, i64 noundef %1, ptr noundef %2, ptr nocapture noundef readonly %3, i64 noundef %4) local_unnamed_addr #19 {
  %6 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 5
  %7 = load i64, ptr %6, align 8, !tbaa !142
  %8 = and i64 %7, %1
  %9 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 4
  %10 = load ptr, ptr %9, align 8, !tbaa !139
  %11 = getelementptr inbounds i64, ptr %10, i64 %8
  %12 = load i64, ptr %11, align 8, !tbaa !2
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
  %22 = load ptr, ptr %0, align 8, !tbaa !134
  %23 = mul nsw i64 %21, %4
  %24 = getelementptr inbounds i8, ptr %22, i64 %23
  %25 = load i64, ptr %24, align 8, !tbaa !2
  %26 = icmp eq i64 %25, %1
  br i1 %26, label %27, label %34

27:                                               ; preds = %20
  %28 = getelementptr inbounds i8, ptr %24, i64 16
  %29 = tail call i32 %3(ptr noundef nonnull %28, ptr noundef %2) #24
  %30 = icmp eq i32 %29, 0
  br i1 %30, label %31, label %42

31:                                               ; preds = %27
  %32 = load i64, ptr %6, align 8, !tbaa !142
  %33 = load ptr, ptr %9, align 8, !tbaa !139
  br label %34

34:                                               ; preds = %31, %20, %14
  %35 = phi ptr [ %33, %31 ], [ %15, %20 ], [ %15, %14 ]
  %36 = phi i64 [ %32, %31 ], [ %16, %20 ], [ %16, %14 ]
  %37 = add nsw i64 %18, 1
  %38 = and i64 %36, %37
  %39 = getelementptr inbounds i64, ptr %35, i64 %38
  %40 = load i64, ptr %39, align 8, !tbaa !2
  %41 = icmp eq i64 %40, 0
  br i1 %41, label %42, label %14

42:                                               ; preds = %27, %34, %5
  %43 = phi i64 [ -1, %5 ], [ -1, %34 ], [ %21, %27 ]
  ret i64 %43
}

; Function Attrs: alwaysinline nounwind optsize
define i64 @fa_table_find_growing(ptr nocapture noundef readonly %0, i64 noundef %1, ptr noundef %2, ptr nocapture noundef readonly %3, i64 noundef %4) local_unnamed_addr #19 {
  %6 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 5
  %7 = load i64, ptr %6, align 8, !tbaa !142
  %8 = and i64 %7, %1
  %9 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 4
  %10 = load ptr, ptr %9, align 8, !tbaa !139
  %11 = getelementptr inbounds i64, ptr %10, i64 %8
  %12 = load i64, ptr %11, align 8, !tbaa !2
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
  %22 = load ptr, ptr %0, align 8, !tbaa !134
  %23 = icmp ult i64 %17, 5
  br i1 %23, label %30, label %24

24:                                               ; preds = %20
  %25 = lshr i64 %21, 2
  %26 = tail call i64 @llvm.ctlz.i64(i64 %25, i1 true), !range !106
  %27 = sub nuw nsw i64 64, %26
  %28 = sub nuw nsw i64 65, %26
  %29 = shl nsw i64 -1, %28
  br label %30

30:                                               ; preds = %24, %20
  %31 = phi i64 [ %29, %24 ], [ 0, %20 ]
  %32 = phi i64 [ %27, %24 ], [ 0, %20 ]
  %33 = getelementptr inbounds %struct.FaSegmented, ptr %22, i64 0, i32 2, i64 %32
  %34 = load ptr, ptr %33, align 8, !tbaa !6
  %35 = add nsw i64 %31, %21
  %36 = mul nsw i64 %35, %4
  %37 = getelementptr inbounds i8, ptr %34, i64 %36
  %38 = load i64, ptr %37, align 8, !tbaa !2
  %39 = icmp eq i64 %38, %1
  br i1 %39, label %40, label %47

40:                                               ; preds = %30
  %41 = getelementptr inbounds i8, ptr %37, i64 16
  %42 = tail call i32 %3(ptr noundef nonnull %41, ptr noundef %2) #24
  %43 = icmp eq i32 %42, 0
  br i1 %43, label %44, label %55

44:                                               ; preds = %40
  %45 = load i64, ptr %6, align 8, !tbaa !142
  %46 = load ptr, ptr %9, align 8, !tbaa !139
  br label %47

47:                                               ; preds = %44, %30, %14
  %48 = phi ptr [ %46, %44 ], [ %15, %30 ], [ %15, %14 ]
  %49 = phi i64 [ %45, %44 ], [ %16, %30 ], [ %16, %14 ]
  %50 = add nsw i64 %18, 1
  %51 = and i64 %49, %50
  %52 = getelementptr inbounds i64, ptr %48, i64 %51
  %53 = load i64, ptr %52, align 8, !tbaa !2
  %54 = icmp eq i64 %53, 0
  br i1 %54, label %55, label %14

55:                                               ; preds = %40, %47, %5
  %56 = phi i64 [ -1, %5 ], [ -1, %47 ], [ %21, %40 ]
  ret i64 %56
}

; Function Attrs: nounwind optsize
define noundef ptr @fa_table_append(ptr nocapture noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #0 {
  %4 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 2
  %5 = load i64, ptr %4, align 8, !tbaa !137
  %6 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 3
  %7 = load i64, ptr %6, align 8, !tbaa !138
  %8 = icmp slt i64 %5, %7
  br i1 %8, label %66, label %9

9:                                                ; preds = %3
  %10 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 6
  %11 = load i64, ptr %10, align 8, !tbaa !140
  %12 = icmp eq i64 %11, 0
  br i1 %12, label %21, label %13

13:                                               ; preds = %9
  %14 = shl nsw i64 %7, 1
  store i64 %14, ptr %6, align 8, !tbaa !138
  %15 = shl nsw i64 %7, 2
  br label %16

16:                                               ; preds = %16, %13
  %17 = phi i64 [ 8, %13 ], [ %19, %16 ]
  %18 = icmp slt i64 %17, %15
  %19 = shl nsw i64 %17, 1
  br i1 %18, label %16, label %20, !llvm.loop !141

20:                                               ; preds = %16
  tail call fastcc void @table_index(ptr noundef %0, i64 noundef %17, i64 noundef %2) #25
  br label %66

21:                                               ; preds = %9
  %22 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 1
  %23 = load i64, ptr %22, align 8, !tbaa !136
  %24 = shl nsw i64 %23, 1
  %25 = icmp sgt i64 %24, %5
  %26 = zext i1 %25 to i64
  %27 = shl nsw i64 %7, %26
  %28 = mul nsw i64 %27, %2
  %29 = tail call ptr @fa_alloc(i64 noundef %28, i64 noundef 16) #24
  %30 = load i64, ptr %4, align 8, !tbaa !137
  %31 = icmp sgt i64 %30, 0
  br i1 %31, label %41, label %32

32:                                               ; preds = %56, %21
  %33 = phi i64 [ 0, %21 ], [ %58, %56 ]
  %34 = load ptr, ptr %0, align 8, !tbaa !134
  %35 = load i64, ptr %6, align 8, !tbaa !138
  %36 = mul nsw i64 %35, %2
  tail call void @fa_free(ptr noundef %34, i64 noundef %36, i64 noundef 16) #24
  store ptr %29, ptr %0, align 8, !tbaa !134
  store i64 %33, ptr %4, align 8, !tbaa !137
  store i64 %27, ptr %6, align 8, !tbaa !138
  %37 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 5
  %38 = load i64, ptr %37, align 8, !tbaa !142
  %39 = add nsw i64 %38, 1
  %40 = shl nsw i64 %27, 1
  br label %61

41:                                               ; preds = %21, %56
  %42 = phi i64 [ %57, %56 ], [ %30, %21 ]
  %43 = phi i64 [ %59, %56 ], [ 0, %21 ]
  %44 = phi i64 [ %58, %56 ], [ 0, %21 ]
  %45 = load ptr, ptr %0, align 8, !tbaa !134
  %46 = mul nsw i64 %43, %2
  %47 = getelementptr inbounds i8, ptr %45, i64 %46
  %48 = getelementptr inbounds i8, ptr %47, i64 8
  %49 = load i64, ptr %48, align 8, !tbaa !2
  %50 = icmp eq i64 %49, 0
  br i1 %50, label %56, label %51

51:                                               ; preds = %41
  %52 = mul nsw i64 %44, %2
  %53 = getelementptr inbounds i8, ptr %29, i64 %52
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %53, ptr nonnull align 1 %47, i64 %2, i1 false)
  %54 = add nsw i64 %44, 1
  %55 = load i64, ptr %4, align 8, !tbaa !137
  br label %56

56:                                               ; preds = %51, %41
  %57 = phi i64 [ %55, %51 ], [ %42, %41 ]
  %58 = phi i64 [ %54, %51 ], [ %44, %41 ]
  %59 = add nuw nsw i64 %43, 1
  %60 = icmp slt i64 %59, %57
  br i1 %60, label %41, label %32, !llvm.loop !145

61:                                               ; preds = %61, %32
  %62 = phi i64 [ %39, %32 ], [ %64, %61 ]
  %63 = icmp slt i64 %62, %40
  %64 = shl nsw i64 %62, 1
  br i1 %63, label %61, label %65, !llvm.loop !146

65:                                               ; preds = %61
  tail call fastcc void @table_index(ptr noundef nonnull %0, i64 noundef %62, i64 noundef %2) #25
  br label %66

66:                                               ; preds = %3, %20, %65
  %67 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 6
  %68 = load i64, ptr %67, align 8, !tbaa !140
  %69 = icmp eq i64 %68, 0
  %70 = load ptr, ptr %0, align 8, !tbaa !134
  br i1 %69, label %73, label %71

71:                                               ; preds = %66
  %72 = tail call ptr @fa_segmented_push_slot(ptr noundef %70, i64 noundef %2, i32 noundef 2) #25
  br label %77

73:                                               ; preds = %66
  %74 = load i64, ptr %4, align 8, !tbaa !137
  %75 = mul nsw i64 %74, %2
  %76 = getelementptr inbounds i8, ptr %70, i64 %75
  br label %77

77:                                               ; preds = %73, %71
  %78 = phi ptr [ %72, %71 ], [ %76, %73 ]
  store i64 %1, ptr %78, align 8, !tbaa !2
  %79 = getelementptr inbounds i8, ptr %78, i64 8
  store i64 1, ptr %79, align 8, !tbaa !2
  %80 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 5
  %81 = load i64, ptr %80, align 8, !tbaa !142
  %82 = and i64 %81, %1
  %83 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 4
  %84 = load ptr, ptr %83, align 8, !tbaa !139
  %85 = getelementptr inbounds i64, ptr %84, i64 %82
  %86 = load i64, ptr %85, align 8, !tbaa !2
  %87 = icmp sgt i64 %86, 0
  br i1 %87, label %88, label %95

88:                                               ; preds = %77, %88
  %89 = phi i64 [ %91, %88 ], [ %82, %77 ]
  %90 = add nsw i64 %89, 1
  %91 = and i64 %90, %81
  %92 = getelementptr inbounds i64, ptr %84, i64 %91
  %93 = load i64, ptr %92, align 8, !tbaa !2
  %94 = icmp sgt i64 %93, 0
  br i1 %94, label %88, label %95, !llvm.loop !147

95:                                               ; preds = %88, %77
  %96 = phi i64 [ %82, %77 ], [ %91, %88 ]
  %97 = getelementptr inbounds i64, ptr %84, i64 %96
  %98 = load i64, ptr %4, align 8, !tbaa !137
  %99 = add nsw i64 %98, 1
  store i64 %99, ptr %97, align 8, !tbaa !2
  %100 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 1
  %101 = load <2 x i64>, ptr %100, align 8, !tbaa !2
  %102 = add nsw <2 x i64> %101, <i64 1, i64 1>
  store <2 x i64> %102, ptr %100, align 8, !tbaa !2
  ret ptr %78
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none)
define void @fa_table_kill(ptr nocapture noundef %0, i64 noundef %1, i64 noundef %2) local_unnamed_addr #11 {
  %4 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 6
  %5 = load i64, ptr %4, align 8, !tbaa !140
  %6 = icmp eq i64 %5, 0
  %7 = load ptr, ptr %0, align 8, !tbaa !134
  br i1 %6, label %24, label %8

8:                                                ; preds = %3
  %9 = icmp ult i64 %1, 4
  br i1 %9, label %16, label %10

10:                                               ; preds = %8
  %11 = lshr i64 %1, 2
  %12 = tail call i64 @llvm.ctlz.i64(i64 %11, i1 true), !range !106
  %13 = sub nuw nsw i64 64, %12
  %14 = sub nuw nsw i64 65, %12
  %15 = shl nsw i64 -1, %14
  br label %16

16:                                               ; preds = %10, %8
  %17 = phi i64 [ %15, %10 ], [ 0, %8 ]
  %18 = phi i64 [ %13, %10 ], [ 0, %8 ]
  %19 = getelementptr inbounds %struct.FaSegmented, ptr %7, i64 0, i32 2, i64 %18
  %20 = load ptr, ptr %19, align 8, !tbaa !6
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
  %29 = load i64, ptr %28, align 8, !tbaa !2
  %30 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 5
  %31 = load i64, ptr %30, align 8, !tbaa !142
  %32 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 4
  %33 = load ptr, ptr %32, align 8, !tbaa !139
  %34 = add nsw i64 %1, 1
  br label %35

35:                                               ; preds = %35, %27
  %36 = phi i64 [ %29, %27 ], [ %41, %35 ]
  %37 = and i64 %36, %31
  %38 = getelementptr inbounds i64, ptr %33, i64 %37
  %39 = load i64, ptr %38, align 8, !tbaa !2
  %40 = icmp eq i64 %39, %34
  %41 = add nsw i64 %37, 1
  br i1 %40, label %42, label %35, !llvm.loop !148

42:                                               ; preds = %35
  %43 = getelementptr inbounds i64, ptr %33, i64 %37
  store i64 -1, ptr %43, align 8, !tbaa !2
  %44 = getelementptr inbounds i8, ptr %28, i64 8
  store i64 0, ptr %44, align 8, !tbaa !2
  %45 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 1
  %46 = load i64, ptr %45, align 8, !tbaa !136
  %47 = add nsw i64 %46, -1
  store i64 %47, ptr %45, align 8, !tbaa !136
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, argmem: readwrite, inaccessiblemem: none)
define void @fa_table_clear(ptr nocapture noundef %0) local_unnamed_addr #20 {
  %2 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 1
  store i64 0, ptr %2, align 8, !tbaa !136
  %3 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 2
  store i64 0, ptr %3, align 8, !tbaa !137
  %4 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 6
  %5 = load i64, ptr %4, align 8, !tbaa !140
  %6 = icmp eq i64 %5, 0
  br i1 %6, label %9, label %7

7:                                                ; preds = %1
  %8 = load ptr, ptr %0, align 8, !tbaa !134
  store i64 0, ptr %8, align 8, !tbaa !107
  br label %9

9:                                                ; preds = %7, %1
  %10 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 4
  %11 = load ptr, ptr %10, align 8, !tbaa !139
  %12 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 5
  %13 = load i64, ptr %12, align 8, !tbaa !142
  %14 = shl i64 %13, 3
  %15 = add i64 %14, 8
  tail call void @llvm.memset.p0.i64(ptr align 8 %11, i8 0, i64 %15, i1 false)
  ret void
}

; Function Attrs: nounwind optsize
define void @fa_table_free(ptr noundef %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 6
  %4 = load i64, ptr %3, align 8, !tbaa !140
  %5 = icmp eq i64 %4, 0
  %6 = load ptr, ptr %0, align 8, !tbaa !134
  br i1 %5, label %8, label %7

7:                                                ; preds = %2
  tail call void @fa_segmented_free(ptr noundef %6, i64 noundef %1, i32 noundef 2) #25
  br label %12

8:                                                ; preds = %2
  %9 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 3
  %10 = load i64, ptr %9, align 8, !tbaa !138
  %11 = mul nsw i64 %10, %1
  tail call void @fa_free(ptr noundef %6, i64 noundef %11, i64 noundef 16) #24
  br label %12

12:                                               ; preds = %8, %7
  %13 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 4
  %14 = load ptr, ptr %13, align 8, !tbaa !139
  %15 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 5
  %16 = load i64, ptr %15, align 8, !tbaa !142
  %17 = shl i64 %16, 3
  %18 = add i64 %17, 8
  tail call void @fa_free(ptr noundef %14, i64 noundef %18, i64 noundef 8) #24
  tail call void @fa_free(ptr noundef nonnull %0, i64 noundef 56, i64 noundef 8) #24
  ret void
}

; Function Attrs: nounwind optsize
define ptr @fa_table_copy(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #0 {
  %3 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 6
  %4 = load i64, ptr %3, align 8, !tbaa !140
  %5 = icmp eq i64 %4, 0
  %6 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 1
  %7 = load i64, ptr %6, align 8, !tbaa !136
  %8 = tail call i64 @llvm.smax.i64(i64 %7, i64 4)
  br i1 %5, label %11, label %9

9:                                                ; preds = %2
  %10 = tail call ptr @fa_table_new_growing(i64 noundef %8, i64 noundef %1) #25
  br label %13

11:                                               ; preds = %2
  %12 = tail call ptr @fa_table_new(i64 noundef %8, i64 noundef %1) #25
  br label %13

13:                                               ; preds = %11, %9
  %14 = phi ptr [ %10, %9 ], [ %12, %11 ]
  %15 = getelementptr inbounds %struct.FaTable, ptr %0, i64 0, i32 2
  %16 = load i64, ptr %15, align 8, !tbaa !137
  %17 = icmp sgt i64 %16, 0
  br i1 %17, label %25, label %18

18:                                               ; preds = %67, %13
  %19 = phi i64 [ 0, %13 ], [ %69, %67 ]
  %20 = getelementptr inbounds %struct.FaTable, ptr %14, i64 0, i32 2
  store i64 %19, ptr %20, align 8, !tbaa !137
  %21 = getelementptr inbounds %struct.FaTable, ptr %14, i64 0, i32 1
  store i64 %19, ptr %21, align 8, !tbaa !136
  %22 = getelementptr inbounds %struct.FaTable, ptr %14, i64 0, i32 5
  %23 = load i64, ptr %22, align 8, !tbaa !142
  %24 = add nsw i64 %23, 1
  tail call fastcc void @table_index(ptr noundef %14, i64 noundef %24, i64 noundef %1) #25
  ret ptr %14

25:                                               ; preds = %13, %67
  %26 = phi i64 [ %68, %67 ], [ %16, %13 ]
  %27 = phi i64 [ %69, %67 ], [ 0, %13 ]
  %28 = phi i64 [ %70, %67 ], [ 0, %13 ]
  %29 = load ptr, ptr %0, align 8, !tbaa !134
  br i1 %5, label %38, label %30

30:                                               ; preds = %25
  %31 = icmp ult i64 %28, 4
  br i1 %31, label %44, label %32

32:                                               ; preds = %30
  %33 = lshr i64 %28, 2
  %34 = tail call i64 @llvm.ctlz.i64(i64 %33, i1 true), !range !106
  %35 = sub nuw nsw i64 64, %34
  %36 = sub nuw nsw i64 65, %34
  %37 = shl nsw i64 -1, %36
  br label %44

38:                                               ; preds = %25
  %39 = mul nsw i64 %28, %1
  %40 = getelementptr inbounds i8, ptr %29, i64 %39
  %41 = getelementptr inbounds i8, ptr %40, i64 8
  %42 = load i64, ptr %41, align 8, !tbaa !2
  %43 = icmp eq i64 %42, 0
  br i1 %43, label %67, label %58

44:                                               ; preds = %30, %32
  %45 = phi i64 [ %37, %32 ], [ 0, %30 ]
  %46 = phi i64 [ %35, %32 ], [ 0, %30 ]
  %47 = getelementptr inbounds %struct.FaSegmented, ptr %29, i64 0, i32 2, i64 %46
  %48 = load ptr, ptr %47, align 8, !tbaa !6
  %49 = add i64 %45, %28
  %50 = mul nsw i64 %49, %1
  %51 = getelementptr inbounds i8, ptr %48, i64 %50
  %52 = getelementptr inbounds i8, ptr %51, i64 8
  %53 = load i64, ptr %52, align 8, !tbaa !2
  %54 = icmp eq i64 %53, 0
  br i1 %54, label %67, label %55

55:                                               ; preds = %44
  %56 = load ptr, ptr %14, align 8, !tbaa !134
  %57 = tail call ptr @fa_segmented_push_slot(ptr noundef %56, i64 noundef %1, i32 noundef 2) #25
  br label %62

58:                                               ; preds = %38
  %59 = load ptr, ptr %14, align 8, !tbaa !134
  %60 = mul nsw i64 %27, %1
  %61 = getelementptr inbounds i8, ptr %59, i64 %60
  br label %62

62:                                               ; preds = %58, %55
  %63 = phi ptr [ %51, %55 ], [ %40, %58 ]
  %64 = phi ptr [ %57, %55 ], [ %61, %58 ]
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %64, ptr nonnull align 1 %63, i64 %1, i1 false)
  %65 = add nsw i64 %27, 1
  %66 = load i64, ptr %15, align 8, !tbaa !137
  br label %67

67:                                               ; preds = %44, %62, %38
  %68 = phi i64 [ %66, %62 ], [ %26, %38 ], [ %26, %44 ]
  %69 = phi i64 [ %65, %62 ], [ %27, %38 ], [ %27, %44 ]
  %70 = add nuw nsw i64 %28, 1
  %71 = icmp slt i64 %70, %68
  br i1 %71, label %25, label %18, !llvm.loop !149
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: read)
define i64 @fa_hash_bytes(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr #7 {
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
  %15 = load i8, ptr %14, align 1, !tbaa !10
  %16 = zext i8 %15 to i64
  %17 = xor i64 %13, %16
  %18 = mul i64 %17, 1099511628211
  %19 = add nuw i64 %12, 1
  %20 = icmp eq i64 %19, %1
  br i1 %20, label %4, label %11, !llvm.loop !150
}

; Function Attrs: cold noinline nounwind optsize
define noundef ptr @fa_error_key_not_found() local_unnamed_addr #10 {
  %1 = tail call ptr @fa_alloc(i64 noundef 32, i64 noundef 8) #24
  store i64 2, ptr %1, align 8, !tbaa !124
  %2 = getelementptr inbounds %struct.MessageError, ptr %1, i64 0, i32 1
  tail call void @fa_b_take(ptr noundef nonnull %2) #25
  ret ptr %1
}

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
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %5) #27
  %6 = call fastcc { i64, i64 } @divide_u128(i64 noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3, ptr noundef nonnull %5) #25
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %5) #27
  ret { i64, i64 } %6
}

; Function Attrs: nofree norecurse nosync nounwind optsize memory(argmem: write)
define internal fastcc { i64, i64 } @divide_u128(i64 noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3, ptr nocapture noundef writeonly %4) unnamed_addr #21 {
  br label %13

6:                                                ; preds = %45
  %7 = zext i64 %46 to i128
  %8 = shl nuw i128 %7, 64
  %9 = zext i64 %47 to i128
  %10 = or disjoint i128 %8, %9
  store i128 %10, ptr %4, align 16, !tbaa !151
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
  br i1 %51, label %6, label %13, !llvm.loop !153
}

; Function Attrs: nounwind optsize
define weak { i64, i64 } @__umodti3(i64 noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3) local_unnamed_addr #0 {
  %5 = alloca i128, align 16
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %5) #27
  %6 = call fastcc { i64, i64 } @divide_u128(i64 noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3, ptr noundef nonnull %5) #25
  %7 = load i128, ptr %5, align 16, !tbaa !151
  %8 = trunc i128 %7 to i64
  %9 = lshr i128 %7, 64
  %10 = trunc i128 %9 to i64
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %5) #27
  %11 = insertvalue { i64, i64 } poison, i64 %8, 0
  %12 = insertvalue { i64, i64 } %11, i64 %10, 1
  ret { i64, i64 } %12
}

; Function Attrs: nounwind optsize
define weak { i64, i64 } @__divti3(i64 noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3) local_unnamed_addr #0 {
  %5 = alloca i128, align 16
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %5) #27
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
  %22 = call fastcc { i64, i64 } @divide_u128(i64 noundef %11, i64 noundef %13, i64 noundef %19, i64 noundef %21, ptr noundef nonnull %5) #25
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
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %5) #27
  %36 = insertvalue { i64, i64 } poison, i64 %33, 0
  %37 = insertvalue { i64, i64 } %36, i64 %35, 1
  ret { i64, i64 } %37
}

; Function Attrs: nounwind optsize
define weak { i64, i64 } @__modti3(i64 noundef %0, i64 noundef %1, i64 noundef %2, i64 noundef %3) local_unnamed_addr #0 {
  %5 = alloca i128, align 16
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %5) #27
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
  %22 = call fastcc { i64, i64 } @divide_u128(i64 noundef %11, i64 noundef %13, i64 noundef %19, i64 noundef %21, ptr noundef nonnull %5) #25
  %23 = load i128, ptr %5, align 16
  %24 = sub i128 0, %23
  %25 = icmp slt i64 %1, 0
  %26 = select i1 %25, i128 %24, i128 %23
  %27 = trunc i128 %26 to i64
  %28 = lshr i128 %26, 64
  %29 = trunc i128 %28 to i64
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %5) #27
  %30 = insertvalue { i64, i64 } poison, i64 %27, 0
  %31 = insertvalue { i64, i64 } %30, i64 %29, 1
  ret { i64, i64 } %31
}

; Function Attrs: optsize
declare i32 @snprintf(ptr noundef, i64 noundef, ptr noundef, ...) local_unnamed_addr #1

; Function Attrs: optsize
declare double @strtod(ptr noundef, ptr noundef) local_unnamed_addr #1

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
  %59 = load i32, ptr %58, align 8, !tbaa !113
  %60 = icmp ugt i32 %59, %0
  br i1 %60, label %52, label %61, !llvm.loop !154

61:                                               ; preds = %55
  %62 = getelementptr inbounds %struct.Range, ptr @EXTEND, i64 %57, i32 1
  %63 = load i32, ptr %62, align 4, !tbaa !115
  %64 = icmp ult i32 %63, %0
  %65 = add nuw i64 %57, 1
  br i1 %64, label %49, label %83, !llvm.loop !154

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
  %76 = load i32, ptr %75, align 8, !tbaa !113
  %77 = icmp ugt i32 %76, %0
  br i1 %77, label %69, label %78, !llvm.loop !154

78:                                               ; preds = %72
  %79 = getelementptr inbounds %struct.Range, ptr @PICTOGRAPHIC, i64 %74, i32 1
  %80 = load i32, ptr %79, align 4, !tbaa !115
  %81 = icmp ult i32 %80, %0
  %82 = add nuw i64 %74, 1
  br i1 %81, label %66, label %83, !llvm.loop !154

83:                                               ; preds = %61, %78, %69, %35, %29, %23, %20, %18, %3, %5, %1, %44, %13, %2
  %84 = phi i32 [ 2, %2 ], [ %17, %13 ], [ %48, %44 ], [ 1, %1 ], [ 3, %5 ], [ 3, %3 ], [ 5, %18 ], [ 6, %20 ], [ 8, %23 ], [ 9, %29 ], [ 10, %35 ], [ 0, %69 ], [ 7, %78 ], [ 4, %61 ]
  ret i32 %84
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.ctlz.i64(i64, i1 immarg) #17

; Function Attrs: optsize
declare ptr @fa_pages(i64 noundef) local_unnamed_addr #1

; Function Attrs: noreturn optsize
declare void @_exit(i32 noundef) local_unnamed_addr #3

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.abs.i64(i64, i1 immarg) #23

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.umin.i64(i64, i64) #23

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.umax.i64(i64, i64) #23

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.smin.i64(i64, i64) #23

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.smax.i64(i64, i64) #23

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.fshl.i64(i64, i64, i64) #23

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i128 @llvm.abs.i128(i128, i1 immarg) #23

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.smin.i32(i32, i32) #23

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.abs.i32(i32, i1 immarg) #23

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.usub.sat.i64(i64, i64) #23

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.vector.reduce.or.v4i32(<4 x i32>) #23

attributes #0 = { nounwind optsize "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #1 = { optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #2 = { noreturn nounwind optsize "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #3 = { noreturn optsize "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #4 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, argmem: none, inaccessiblemem: none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #5 = { mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite) }
attributes #6 = { mustprogress nocallback nofree nounwind willreturn memory(argmem: readwrite) }
attributes #7 = { nofree norecurse nosync nounwind optsize memory(argmem: read) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #8 = { nofree norecurse nosync nounwind optsize memory(argmem: readwrite) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #9 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #10 = { cold noinline nounwind optsize "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #11 = { nofree norecurse nosync nounwind optsize memory(readwrite, inaccessiblemem: none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #12 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(readwrite, inaccessiblemem: none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #13 = { alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(argmem: read) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #14 = { mustprogress nocallback nofree nounwind willreturn memory(argmem: write) }
attributes #15 = { cold noinline noreturn nounwind optsize "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #16 = { cold mustprogress nofree noinline norecurse nosync nounwind optsize willreturn memory(none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #17 = { mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #18 = { alwaysinline mustprogress nofree norecurse nosync nounwind optsize willreturn memory(read, inaccessiblemem: none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #19 = { alwaysinline nounwind optsize "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #20 = { mustprogress nofree norecurse nosync nounwind optsize willreturn memory(write, argmem: readwrite, inaccessiblemem: none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #21 = { nofree norecurse nosync nounwind optsize memory(argmem: write) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #22 = { nofree norecurse nosync nounwind optsize memory(none) "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #23 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #24 = { nounwind optsize }
attributes #25 = { optsize }
attributes #26 = { noreturn nounwind optsize }
attributes #27 = { nounwind }
attributes #28 = { noreturn optsize }
attributes #29 = { cold noreturn optsize }

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
!11 = !{!12}
!12 = distinct !{!12, !13, !"owned_copy: argument 0"}
!13 = distinct !{!13, !"owned_copy"}
!14 = !{i64 0, i64 8, !6, i64 8, i64 8, !2, i64 16, i64 8, !2}
!15 = !{i64 0, i64 8, !2, i64 8, i64 8, !2}
!16 = !{i64 0, i64 8, !2}
!17 = distinct !{!17, !9}
!18 = !{!19, !19, i64 0}
!19 = !{!"int", !4, i64 0}
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
!34 = !{!35, !7, i64 0}
!35 = !{!"", !7, i64 0, !3, i64 8, !3, i64 16}
!36 = !{!35, !3, i64 8}
!37 = distinct !{!37, !9}
!38 = !{!35, !3, i64 16}
!39 = distinct !{!39, !9}
!40 = distinct !{!40, !9}
!41 = !{i32 0, i32 13}
!42 = distinct !{!42, !9}
!43 = distinct !{!43, !9}
!44 = distinct !{!44, !9}
!45 = distinct !{!45, !9}
!46 = distinct !{!46, !9}
!47 = !{!48, !7, i64 0}
!48 = !{!"", !7, i64 0, !3, i64 8, !3, i64 16, !7, i64 24, !3, i64 32, !19, i64 40, !19, i64 44}
!49 = !{!48, !3, i64 8}
!50 = !{!48, !3, i64 16}
!51 = !{!48, !7, i64 24}
!52 = !{!48, !3, i64 32}
!53 = !{!48, !19, i64 40}
!54 = !{!48, !19, i64 44}
!55 = !{ptr @replace_one, ptr @split_one}
!56 = distinct !{!56, !9}
!57 = !{!58, !60}
!58 = distinct !{!58, !59, !"owned_copy: argument 0"}
!59 = distinct !{!59, !"owned_copy"}
!60 = distinct !{!60, !61, !"slice: argument 0"}
!61 = distinct !{!61, !"slice"}
!62 = distinct !{!62, !9}
!63 = !{!64, !7, i64 0}
!64 = !{!"", !7, i64 0, !3, i64 8, !3, i64 16, !7, i64 24}
!65 = !{!64, !3, i64 8}
!66 = !{!64, !3, i64 16}
!67 = !{!64, !7, i64 24}
!68 = !{!69, !71}
!69 = distinct !{!69, !70, !"owned_copy: argument 0"}
!70 = distinct !{!70, !"owned_copy"}
!71 = distinct !{!71, !72, !"slice: argument 0"}
!72 = distinct !{!72, !"slice"}
!73 = !{!74, !76}
!74 = distinct !{!74, !75, !"owned_copy: argument 0"}
!75 = distinct !{!75, !"owned_copy"}
!76 = distinct !{!76, !77, !"slice: argument 0"}
!77 = distinct !{!77, !"slice"}
!78 = distinct !{!78, !9}
!79 = distinct !{!79, !9}
!80 = distinct !{!80, !9}
!81 = !{!82, !84}
!82 = distinct !{!82, !83, !"owned_copy: argument 0"}
!83 = distinct !{!83, !"owned_copy"}
!84 = distinct !{!84, !85, !"slice: argument 0"}
!85 = distinct !{!85, !"slice"}
!86 = distinct !{!86, !9}
!87 = !{i32 0, i32 2}
!88 = distinct !{!88, !9}
!89 = !{!90}
!90 = distinct !{!90, !91, !"owned_copy: argument 0"}
!91 = distinct !{!91, !"owned_copy"}
!92 = distinct !{!92, !9}
!93 = distinct !{!93, !9}
!94 = distinct !{!94, !9}
!95 = distinct !{!95, !9}
!96 = distinct !{!96, !9}
!97 = !{!98, !3, i64 0}
!98 = !{!"", !3, i64 0, !3, i64 8, !3, i64 16}
!99 = !{!98, !3, i64 8}
!100 = !{!98, !3, i64 16}
!101 = distinct !{!101, !9}
!102 = distinct !{!102, !9}
!103 = distinct !{!103, !9}
!104 = distinct !{!104, !9}
!105 = distinct !{!105, !9}
!106 = !{i64 0, i64 65}
!107 = !{!108, !3, i64 0}
!108 = !{!"", !3, i64 0, !3, i64 8, !4, i64 16}
!109 = !{!108, !3, i64 8}
!110 = distinct !{!110, !9}
!111 = distinct !{!111, !9}
!112 = distinct !{!112, !9}
!113 = !{!114, !19, i64 0}
!114 = !{!"", !19, i64 0, !19, i64 4}
!115 = !{!114, !19, i64 4}
!116 = distinct !{!116, !9}
!117 = !{!118, !7, i64 0}
!118 = !{!"FreeBlock", !7, i64 0}
!119 = !{!120, !3, i64 8}
!120 = !{!"FreeSpan", !7, i64 0, !3, i64 8}
!121 = !{!120, !7, i64 0}
!122 = distinct !{!122, !9}
!123 = distinct !{!123, !9}
!124 = !{!125, !3, i64 0}
!125 = !{!"", !3, i64 0, !35, i64 8}
!126 = distinct !{!126, !9}
!127 = distinct !{!127, !9}
!128 = distinct !{!128, !9}
!129 = !{!130, !3, i64 0}
!130 = !{!"", !3, i64 0, !35, i64 8, !35, i64 32}
!131 = !{!132}
!132 = distinct !{!132, !133, !"owned_copy: argument 0"}
!133 = distinct !{!133, !"owned_copy"}
!134 = !{!135, !7, i64 0}
!135 = !{!"", !7, i64 0, !3, i64 8, !3, i64 16, !3, i64 24, !7, i64 32, !3, i64 40, !3, i64 48}
!136 = !{!135, !3, i64 8}
!137 = !{!135, !3, i64 16}
!138 = !{!135, !3, i64 24}
!139 = !{!135, !7, i64 32}
!140 = !{!135, !3, i64 48}
!141 = distinct !{!141, !9}
!142 = !{!135, !3, i64 40}
!143 = distinct !{!143, !9}
!144 = distinct !{!144, !9}
!145 = distinct !{!145, !9}
!146 = distinct !{!146, !9}
!147 = distinct !{!147, !9}
!148 = distinct !{!148, !9}
!149 = distinct !{!149, !9}
!150 = distinct !{!150, !9}
!151 = !{!152, !152, i64 0}
!152 = !{!"__int128", !4, i64 0}
!153 = distinct !{!153, !9}
!154 = distinct !{!154, !9}
