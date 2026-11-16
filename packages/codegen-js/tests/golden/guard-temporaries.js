// helpers: FaStop, faStop, faOverflow, faCheckInteger, faUtf8Encoder, faIsBytes, faSharedText, faUtf8Decoder, faSharedParse, faSharedHandle, faNeedSharedMemory, FaGuard, FaMutex, faUnlock, faCancelSignal, faSched, faLater, faMacrotask, faReady, faAlive, faRemove, faWaitShared, faLock, faWriteError, faExit, faStopMessage, faIsStackOverflow, faIsError, faReport, FaTask, faWhenDone, faCancel, faEnd, faFatal, faSettle, faRunTask, faRunAsync, faWaiting, faLockNow

function read(_guard) {
	return 1;
}

async function main($task) {
	let $t1, $t2, $t5, $t6, $t8, $t9, $t11;
	let mutex = new FaMutex(0);
	try {
		const $t0 = mutex;
		const $t3 = ($t2 = ($t1 = faLockNow($t0)) !== faWaiting ? $t1 : await faLock($task, $t0, "guard-temporaries.fa:4:2"));
		$t3.value = faCheckInteger($t3.value + 1, "guard-temporaries.fa:4:1");
	} finally { faUnlock($t2); }
	let value;
	try {
		const $t4 = mutex;
		value = read(($t6 = ($t5 = faLockNow($t4)) !== faWaiting ? $t5 : await faLock($task, $t4, "guard-temporaries.fa:5:27")));
	} finally { faUnlock($t6); }
	try {
		const $t7 = mutex;
		console.log(`${value} ${($t9 = ($t8 = faLockNow($t7)) !== faWaiting ? $t8 : await faLock($task, $t7, "guard-temporaries.fa:6:21")).value}`);
	} finally { faUnlock($t9); }
	faUnlock(mutex.tryLock());
	const $t10 = mutex;
	faUnlock(($t11 = faLockNow($t10)) !== faWaiting ? $t11 : await faLock($task, $t10, "guard-temporaries.fa:8:1"));
}

faRunAsync(main);
