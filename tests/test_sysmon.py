import os

from granian._granian import ProcInfoCollector


def test_procinfo_collector():
    collector = ProcInfoCollector()
    mem = collector.memory()
    assert mem[os.getpid()]
