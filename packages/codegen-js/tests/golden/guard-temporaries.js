// helpers: FaStop, faStop, faOverflow, faCheckInteger, faUtf8Encoder, faIsBytes, faSharedText, faUtf8Decoder, faSharedParse, faSharedHandle, faNeedSharedMemory, FaGuard, FaMutex, faUnlock, faCancelSignal, faSched, faLater, faMacrotask, faReady, faAlive, faRemove, faWaitShared, faLock, faWriteError, faExit, faStopMessage, faIsStackOverflow, faIsError, faReport, FaTask, faWhenDone, faCancel, faEnd, faFatal, faSettle, faRunTask, faRunAsync, faWaiting, faLockNow

const drop$none = () => {};

function read($$Value$drop, _guard) {
	return 1;
}

async function main($task) {
	let $t1, $t2, $t5, $t6, $t7, $t8, $t11, $t12, $t13, $t14, $t17;
	let mutex = new FaMutex(0);
	try {
		const $t0 = mutex;
		const $t3 = ($t2 = ($t1 = faLockNow($t0)) !== faWaiting ? $t1 : await faLock($task, $t0, "guard-temporaries.fa:4:2"));
		$t3.value = faCheckInteger($t3.value + 1, "guard-temporaries.fa:4:1");
	} finally { faUnlock($t2); }
	let value;
	try {
		$t8 = false;
		try {
			const $t4 = mutex;
			$t7 = read(drop$none, ($t6 = ($t5 = faLockNow($t4)) !== faWaiting ? $t5 : await faLock($task, $t4, "guard-temporaries.fa:5:27")));
		} catch ($t9) {
			$t8 = true; faUnlock($t6); throw $t9;
		}
		value = $t7;
	} finally { if (!$t8) { faUnlock($t6); } }
	try {
		$t14 = false;
		try {
			const $t10 = mutex;
			$t13 = console.log(`${value} ${($t12 = ($t11 = faLockNow($t10)) !== faWaiting ? $t11 : await faLock($task, $t10, "guard-temporaries.fa:6:21")).value}`);
		} catch ($t15) {
			$t14 = true; faUnlock($t12); throw $t15;
		}
	} finally { if (!$t14) { faUnlock($t12); } }
	faUnlock(mutex.tryLock());
	const $t16 = mutex;
	faUnlock(($t17 = faLockNow($t16)) !== faWaiting ? $t17 : await faLock($task, $t16, "guard-temporaries.fa:8:1"));
}

faRunAsync(main);
