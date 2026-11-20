import json
from datetime import datetime

import pytest

from app.tools.registry import Registry, device_registry


@pytest.fixture
def registry():
    return device_registry()


@pytest.mark.parametrize(("expression", "result"), [
    ("12 * 18", 216), ("(125 / 5) + 17", 42), ("sqrt(144)", 12), ("2 ** 10", 1024),
    ("-7 % 3", 2), ("157 * 243", 38151), ("-(1.5 + 3)", -4.5),
])
def test_arithmetic(registry, expression, result):
    assert registry.run("calculator", {"expression": expression}).result == {"value": result}


@pytest.mark.parametrize("arguments", [
    {}, {"expression": 4}, {"expression": "12", "extra": True}, {"expression": "1/0"},
    {"expression": "2**10000"}, {"expression": "sqrt(-1)"}, {"expression": "type(1)"},
    {"expression": "'abc'"}, {"expression": "9" * 193}, {"expression": "1e309"},
    {"expression": "[1, 2]"}, {"expression": "2+"}, "[]", "{invalid",
])
def test_calculator_rejects_invalid_inputs(registry, arguments):
    assert not registry.run("calculator", arguments).ok


def test_device_time(registry):
    result = registry.run("get_current_datetime", "{}")
    stamp = datetime.fromisoformat(result.result["iso_datetime"])
    assert result.ok and stamp.tzinfo and abs((stamp - datetime.now().astimezone()).total_seconds()) < 3


def test_system_information_is_replaceable():
    class FixtureDevice:
        def read(self):
            return {"operating_system": "TestOS"}

    assert device_registry(FixtureDevice()).run("get_system_information", {}).result == {"operating_system": "TestOS"}
    assert set(device_registry().run("get_system_information", {}).result) == {
        "operating_system", "architecture", "release", "application_version"}


def test_registry_validates_names_and_arguments(registry):
    assert not registry.run("unknown", {}).ok
    assert not registry.run("get_current_datetime", {"timezone": "override"}).ok
    definitions = registry.definitions()
    assert len(definitions) == 3 and json.dumps(definitions)
    assert all(t["function"]["parameters"]["additionalProperties"] is False for t in definitions)
    with pytest.raises(ValueError):
        Registry([registry.tools["calculator"]] * 2)
