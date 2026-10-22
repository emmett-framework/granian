import asyncio
import heapq
import itertools

import pytest

from granian._futures import _fair_steps


class FakeLane:
    """Mirrors the Rust loop-lane protocol, with a static priority per request."""

    def __init__(self):
        self.busy = False
        self.waiting = []
        self.seq = itertools.count()
        self.running = 0
        self.max_running = 0


class FakeAcct:
    def __init__(self, lane, prio=0):
        self.lane = lane
        self.prio = prio

    def lane_wait(self, fut):
        heapq.heappush(self.lane.waiting, (self.prio, next(self.lane.seq), fut))
        if self.lane.busy:
            return False
        self.lane.busy = True
        return True

    def lane_release(self):
        if self.lane.waiting:
            return True
        self.lane.busy = False
        return False

    def lane_dispatch(self):
        while self.lane.waiting:
            _, _, fut = heapq.heappop(self.lane.waiting)
            if fut.done():
                continue
            fut.set_result(None)
            return
        self.lane.busy = False


async def drive(coro, acct):
    """What `future_watcher` does: await the driver from a regular coroutine."""
    return await _fair_steps(coro, acct)


def _step(lane, log, name):
    lane.running += 1
    lane.max_running = max(lane.max_running, lane.running)
    log.append(name)
    lane.running -= 1


@pytest.mark.asyncio
async def test_fair_steps_passes_results_and_errors_through():
    lane = FakeLane()

    async def ok():
        await asyncio.sleep(0)
        await asyncio.sleep(0.001)
        return 42

    async def boom():
        await asyncio.sleep(0)
        raise ValueError('boom')

    assert await _fair_steps(ok(), FakeAcct(lane)) == 42
    with pytest.raises(ValueError, match='boom'):
        await _fair_steps(boom(), FakeAcct(lane))
    assert not lane.busy
    assert not lane.waiting


@pytest.mark.asyncio
async def test_fair_steps_dispatch_follows_the_scheduler_not_fifo():
    lane = FakeLane()
    log = []

    async def app(name, steps):
        for idx in range(steps):
            _step(lane, log, f'{name}{idx}')
            await asyncio.sleep(0)

    # the hogs are ready first, but the light request has the higher priority
    tasks = [asyncio.create_task(drive(app(f'hog{n}-', 3), FakeAcct(lane, prio=1))) for n in range(3)]
    tasks.append(asyncio.create_task(drive(app('light', 3), FakeAcct(lane, prio=0))))
    await asyncio.gather(*tasks)

    assert log[:3] == ['light0', 'light1', 'light2']
    assert sorted(log[3:]) == sorted(f'hog{n}-{idx}' for n in range(3) for idx in range(3))
    assert lane.max_running == 1
    assert not lane.busy


@pytest.mark.asyncio
async def test_fair_steps_cancellation_while_waiting_for_a_turn():
    lane = FakeLane()
    seen = []
    gate = asyncio.Event()

    async def holder():
        await gate.wait()

    async def victim():
        try:
            await asyncio.sleep(10)
        except asyncio.CancelledError:
            seen.append('cancelled')
            raise

    holding = asyncio.create_task(drive(holder(), FakeAcct(lane)))
    waiting = asyncio.create_task(drive(victim(), FakeAcct(lane)))
    await asyncio.sleep(0.01)
    waiting.cancel()
    with pytest.raises(asyncio.CancelledError):
        await waiting
    gate.set()
    await holding

    assert seen == ['cancelled']
    assert not lane.busy
    assert not lane.waiting

    # the lane is still usable afterwards
    async def after():
        await asyncio.sleep(0)
        return 'ok'

    assert await _fair_steps(after(), FakeAcct(lane)) == 'ok'


@pytest.mark.asyncio
async def test_fair_steps_closing_a_suspended_driver_frees_the_lane():
    lane = FakeLane()

    async def forever():
        await asyncio.Event().wait()

    task = asyncio.create_task(drive(forever(), FakeAcct(lane)))
    await asyncio.sleep(0.01)
    task.cancel()
    with pytest.raises(asyncio.CancelledError):
        await task

    assert not lane.busy
    assert not lane.waiting
