define internal fastcc ptr @fa.load.async(i64 %0, i32 %1) {
allocas:
  %local = alloca i64, align 8
  %owns_parameters = alloca i1, align 1
  %parameter = alloca i64, align 8
  %promise = alloca { { ptr, ptr, ptr, i32, i32 }, { i64, ptr } }, align 16
  br label %start

start:                                            ; preds = %allocas
  %2 = call token @llvm.coro.id(i32 16, ptr %promise, ptr null, ptr null)
  %3 = call i1 @llvm.coro.alloc(token %2)
  br i1 %3, label %coro_allocate, label %coro_begin

coro_allocate:                                    ; preds = %start
  %4 = call i64 @llvm.coro.size.i64()
  %5 = call ptr @fa_alloc(i64 %4, i64 16)
  br label %coro_begin

coro_begin:                                       ; preds = %coro_allocate, %start
  %memory = phi ptr [ null, %start ], [ %5, %coro_allocate ]
  %6 = call ptr @llvm.coro.begin(token %2, ptr %memory)
  %7 = getelementptr inbounds { { ptr, ptr, ptr, i32, i32 }, { i64, ptr } }, ptr %promise, i32 0, i32 0
  store { ptr, ptr, ptr, i32, i32 } { ptr null, ptr null, ptr null, i32 4, i32 0 }, ptr %7, align 8
  store i64 %0, ptr %parameter, align 8
  %8 = and i32 %1, 2
  %9 = icmp ne i32 %8, 0
  store i1 %9, ptr %owns_parameters, align 1
  %10 = and i32 %1, 1
  %11 = icmp ne i32 %10, 0
  br i1 %11, label %started, label %first_turn

coro_cleanup:                                     ; preds = %coro_destroyed_after_end, %cancelled2, %cancelled
  %12 = call ptr @llvm.coro.free(token %2, ptr %6)
  %13 = icmp ne ptr %12, null
  br i1 %13, label %coro_free, label %coro_end

coro_free:                                        ; preds = %coro_cleanup
  %14 = call i64 @llvm.coro.size.i64()
  call void @fa_free(ptr %12, i64 %14, i64 16)
  br label %coro_end

coro_end:                                         ; preds = %coro_finish, %await, %started, %coro_free, %coro_cleanup
  %15 = call i1 @llvm.coro.end(ptr %6, i1 false, token none)
  ret ptr %6

coro_finish:                                      ; preds = %check_passed
  call void @fa_task_finish(ptr %6)
  %16 = call i8 @llvm.coro.suspend(token none, i1 true)
  switch i8 %16, label %coro_end [
    i8 0, label %coro_resumed_after_end
    i8 1, label %coro_destroyed_after_end
  ]

started:                                          ; preds = %coro_begin
  %17 = call i8 @llvm.coro.suspend(token none, i1 false)
  switch i8 %17, label %coro_end [
    i8 0, label %resumed
    i8 1, label %cancelled
  ]

first_turn:                                       ; preds = %resumed, %coro_begin
  %18 = load i64, ptr %parameter, align 8
  %task = call fastcc ptr @fa.fetch.async(i64 %18, i32 0)
  %resume = load ptr, ptr %task, align 8
  %done = icmp eq ptr %resume, null
  br i1 %done, label %awaited, label %await

resumed:                                          ; preds = %started
  br label %first_turn

cancelled:                                        ; preds = %started
  br label %coro_cleanup

await:                                            ; preds = %first_turn
  call void @fa_task_await(ptr %task, ptr %6)
  %19 = call i8 @llvm.coro.suspend(token none, i1 false)
  switch i8 %19, label %coro_end [
    i8 0, label %resumed1
    i8 1, label %cancelled2
  ]

awaited:                                          ; preds = %resumed1, %first_turn
  %promise3 = getelementptr inbounds i8, ptr %task, i64 16
  %20 = getelementptr inbounds { { ptr, ptr, ptr, i32, i32 }, { i64, ptr } }, ptr %promise3, i32 0, i32 1
  %21 = getelementptr inbounds { i64, ptr }, ptr %20, i32 0, i32 0
  %result = load i64, ptr %21, align 8
  %22 = getelementptr inbounds { ptr, ptr, ptr, i32, i32 }, ptr %promise3, i32 0, i32 4
  %23 = load i32, ptr %22, align 4
  %24 = and i32 %23, -3
  store i32 %24, ptr %22, align 4
  %25 = getelementptr inbounds ptr, ptr %task, i64 1
  %26 = load ptr, ptr %25, align 8
  call fastcc void %26(ptr %task)
  store i64 %result, ptr %local, align 8
  %27 = load i64, ptr %local, align 8
  %28 = call { i64, i1 } @llvm.smul.with.overflow.i64(i64 %27, i64 2)
  %29 = extractvalue { i64, i1 } %28, 0
  %30 = extractvalue { i64, i1 } %28, 1
  br i1 %30, label %check_failed, label %check_passed

resumed1:                                         ; preds = %await
  br label %awaited

cancelled2:                                       ; preds = %await
  call void @fa_task_drop(ptr %task)
  br label %coro_cleanup

check_failed:                                     ; preds = %awaited
  %31 = sext i64 %27 to i128
  %32 = mul i128 %31, 2
  %33 = trunc i128 %32 to i64
  %34 = lshr i128 %32, 64
  %35 = trunc i128 %34 to i64
  call void @fa_overflow(i64 %33, i64 %35, i32 1, ptr @fa.constant.1)
  unreachable

check_passed:                                     ; preds = %awaited
  %36 = insertvalue { i64, ptr } undef, i64 %29, 0
  %37 = insertvalue { i64, ptr } %36, ptr null, 1
  %38 = getelementptr inbounds { { ptr, ptr, ptr, i32, i32 }, { i64, ptr } }, ptr %promise, i32 0, i32 1
  store { i64, ptr } %37, ptr %38, align 8
  br label %coro_finish

coro_resumed_after_end:                           ; preds = %coro_finish
  unreachable

coro_destroyed_after_end:                         ; preds = %coro_finish
  br label %coro_cleanup
}

define internal fastcc ptr @fa.fetch.async(i64 %0, i32 %1) {
allocas:
  %owns_parameters = alloca i1, align 1
  %parameter = alloca i64, align 8
  %promise = alloca { { ptr, ptr, ptr, i32, i32 }, { i64, ptr } }, align 16
  br label %start

start:                                            ; preds = %allocas
  %2 = call token @llvm.coro.id(i32 16, ptr %promise, ptr null, ptr null)
  %3 = call i1 @llvm.coro.alloc(token %2)
  br i1 %3, label %coro_allocate, label %coro_begin

coro_allocate:                                    ; preds = %start
  %4 = call i64 @llvm.coro.size.i64()
  %5 = call ptr @fa_alloc(i64 %4, i64 16)
  br label %coro_begin

coro_begin:                                       ; preds = %coro_allocate, %start
  %memory = phi ptr [ null, %start ], [ %5, %coro_allocate ]
  %6 = call ptr @llvm.coro.begin(token %2, ptr %memory)
  %7 = getelementptr inbounds { { ptr, ptr, ptr, i32, i32 }, { i64, ptr } }, ptr %promise, i32 0, i32 0
  store { ptr, ptr, ptr, i32, i32 } { ptr null, ptr null, ptr null, i32 4, i32 0 }, ptr %7, align 8
  store i64 %0, ptr %parameter, align 8
  %8 = and i32 %1, 2
  %9 = icmp ne i32 %8, 0
  store i1 %9, ptr %owns_parameters, align 1
  %10 = and i32 %1, 1
  %11 = icmp ne i32 %10, 0
  br i1 %11, label %started, label %first_turn

coro_cleanup:                                     ; preds = %coro_destroyed_after_end, %cancelled2, %cancelled
  %12 = call ptr @llvm.coro.free(token %2, ptr %6)
  %13 = icmp ne ptr %12, null
  br i1 %13, label %coro_free, label %coro_end

coro_free:                                        ; preds = %coro_cleanup
  %14 = call i64 @llvm.coro.size.i64()
  call void @fa_free(ptr %12, i64 %14, i64 16)
  br label %coro_end

coro_end:                                         ; preds = %coro_finish, %first_turn, %started, %coro_free, %coro_cleanup
  %15 = call i1 @llvm.coro.end(ptr %6, i1 false, token none)
  ret ptr %6

coro_finish:                                      ; preds = %check_passed
  call void @fa_task_finish(ptr %6)
  %16 = call i8 @llvm.coro.suspend(token none, i1 true)
  switch i8 %16, label %coro_end [
    i8 0, label %coro_resumed_after_end
    i8 1, label %coro_destroyed_after_end
  ]

started:                                          ; preds = %coro_begin
  %17 = call i8 @llvm.coro.suspend(token none, i1 false)
  switch i8 %17, label %coro_end [
    i8 0, label %resumed
    i8 1, label %cancelled
  ]

first_turn:                                       ; preds = %resumed, %coro_begin
  call void @fa_task_yield(ptr %6)
  %18 = call i8 @llvm.coro.suspend(token none, i1 false)
  switch i8 %18, label %coro_end [
    i8 0, label %resumed1
    i8 1, label %cancelled2
  ]

resumed:                                          ; preds = %started
  br label %first_turn

cancelled:                                        ; preds = %started
  br label %coro_cleanup

resumed1:                                         ; preds = %first_turn
  %19 = load i64, ptr %parameter, align 8
  %20 = call { i64, i1 } @llvm.sadd.with.overflow.i64(i64 %19, i64 1)
  %21 = extractvalue { i64, i1 } %20, 0
  %22 = extractvalue { i64, i1 } %20, 1
  br i1 %22, label %check_failed, label %check_passed

cancelled2:                                       ; preds = %first_turn
  br label %coro_cleanup

check_failed:                                     ; preds = %resumed1
  %23 = sext i64 %19 to i128
  %24 = add i128 %23, 1
  %25 = trunc i128 %24 to i64
  %26 = lshr i128 %24, 64
  %27 = trunc i128 %26 to i64
  call void @fa_overflow(i64 %25, i64 %27, i32 1, ptr @fa.constant)
  unreachable

check_passed:                                     ; preds = %resumed1
  %28 = insertvalue { i64, ptr } undef, i64 %21, 0
  %29 = insertvalue { i64, ptr } %28, ptr null, 1
  %30 = getelementptr inbounds { { ptr, ptr, ptr, i32, i32 }, { i64, ptr } }, ptr %promise, i32 0, i32 1
  store { i64, ptr } %29, ptr %30, align 8
  br label %coro_finish

coro_resumed_after_end:                           ; preds = %coro_finish
  unreachable

coro_destroyed_after_end:                         ; preds = %coro_finish
  br label %coro_cleanup
}

