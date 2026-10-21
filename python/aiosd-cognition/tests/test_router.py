import asyncio
import unicodedata

from fakes import DownGateway, FakeBroker, FakeCore, FakeGateway, NoGateway, denied, reply, tool_call

from aiosd_cognition.clients import ToolResult
from aiosd_cognition.router import Router, load_grammar, match_l0


def run(router: Router, text: str):
    return asyncio.run(router.ask(text))


def test_status_goes_l0_without_gateway() -> None:
    broker = FakeBroker()
    out = run(Router("s", broker, NoGateway()), "trạng thái máy")
    assert out.route == "L0" and out.error is None
    assert broker.calls == [("sys.info", {}, "s:0:0")]
    assert out.answer == "sys.info ok"


def test_find_file_uppercase_and_decomposed() -> None:
    broker = FakeBroker()
    text = unicodedata.normalize("NFD", "  Tìm FILE Hợp   Đồng ")
    out = run(Router("s", broker, NoGateway()), text)
    assert out.route == "L0"
    assert broker.calls == [("fs.search", {"pattern": "*hợp đồng*"}, "s:0:0")]


def test_l0_still_works_when_gateway_down() -> None:
    out = run(Router("s", FakeBroker(), DownGateway()), "tình trạng hệ thống")
    assert out.route == "L0" and out.error is None


def test_grammar_rejects_near_misses() -> None:
    rules = load_grammar()
    assert match_l0("trạng thái máy tính", rules) is None
    assert match_l0("tìm file", rules) is None
    assert match_l0("tìm file kế hoạch rồi ghi một ghi chú tên là tom-tat", rules) is None
    assert match_l0("tìm file kế hoạch, ghi chú lại", rules) is None


def test_note_goes_l2_through_confirmation() -> None:
    key = "s:1:0"

    async def slow_ok(_args: dict, _key: str) -> ToolResult:
        await asyncio.sleep(0.05)  # the broker holds the call until /v1/confirm
        return ToolResult('{"path": "notes/mai.md"}', False, "personal", "observed")

    broker = FakeBroker({"fs.write_note": slow_ok})
    args = {"name": "mai", "content": "mai họp 9h"}
    gw = FakeGateway([reply(calls=[tool_call("fs__write_note", args)]), reply("Đã ghi chú.")])
    core = FakeCore([{"request_id": "r1", "task_step": key}])
    events: list[dict] = []
    router = Router("s", broker, gw, core=core, on_step=events.append, pending_poll=0.01)
    out = run(router, "ghi chú: mai họp 9h")
    assert out.route == "L2" and out.error is None and out.answer == "Đã ghi chú."
    assert broker.calls == [("fs.write_note", args, key)]
    waits = [e for e in events if e["event"] == "waiting_confirm"]
    assert waits == [{"event": "waiting_confirm", "request_id": "r1", "key": key}]
    assert out.steps[0]["status"] == "ok"
    model, messages, tools = gw.requests[1]
    assert model == "s2"
    assert {t["function"]["name"] for t in tools} == {"sys__info", "fs__search", "fs__write_note"}
    assert messages[-1] == {"role": "tool", "tool_call_id": "c1", "content": '{"path": "notes/mai.md"}'}


def test_stops_after_max_rounds() -> None:
    broker = FakeBroker()
    gw = FakeGateway([reply(calls=[tool_call("sys__info", {"n": n})]) for n in range(20)])
    out = run(Router("s", broker, gw), "lặp mãi")
    assert out.error and out.error.startswith("max_rounds")
    assert len(gw.requests) == 8 and len(broker.calls) == 8
    assert [c[2] for c in broker.calls] == [f"s:{r}:0" for r in range(1, 9)]


def test_broker_error_goes_back_to_model_and_is_not_retried_twice() -> None:
    broker = FakeBroker({"sys.info": denied})
    gw = FakeGateway([reply(calls=[tool_call("sys__info", {})])] * 4 + [reply("Không được phép.")])
    out = run(Router("s", broker, gw), "xem máy giúp tôi")
    assert out.answer == "Không được phép."
    assert len(broker.calls) == 2  # first try + one retry
    tool_msgs = [m for m in gw.requests[-1][1] if m["role"] == "tool"]
    assert tool_msgs[0]["content"] == "error: denied: policy forbids it"
    assert tool_msgs[2]["content"].startswith("error: not_retried")


def test_external_results_are_prefixed() -> None:
    broker = FakeBroker({"fs.search": lambda a, k: ToolResult("a.txt", False, "personal", "external")})
    gw = FakeGateway([reply(calls=[tool_call("fs__search", {"pattern": "*"})]), reply("xong")])
    run(Router("s", broker, gw), "liệt kê tệp")
    assert gw.requests[1][1][-1]["content"] == "[external] a.txt"


def test_unknown_tool_and_bad_json_are_reported_to_model() -> None:
    bad = {"id": "c2", "type": "function", "function": {"name": "fs__search", "arguments": "{"}}
    gw = FakeGateway([reply(calls=[tool_call("rm__rf", {}), bad]), reply("ok")])
    broker = FakeBroker()
    run(Router("s", broker, gw), "làm gì đó")
    contents = [m["content"] for m in gw.requests[1][1] if m["role"] == "tool"]
    assert contents[0].startswith("error: unknown_tool")
    assert contents[1].startswith("error: invalid_params")
    assert broker.calls == []


def test_token_budget() -> None:
    gw = FakeGateway([reply(calls=[tool_call("sys__info", {"x": 1})], tokens=15_000)])
    out = run(Router("s", FakeBroker(), gw), "tốn token")
    assert out.error and out.error.startswith("token_budget_exhausted")
    assert len(gw.requests) == 2


def test_l2_reports_gateway_down() -> None:
    out = run(Router("s", FakeBroker(), DownGateway()), "kể chuyện")
    assert out.route == "L2" and out.error == "gateway: unavailable: ConnectError: connection refused"
