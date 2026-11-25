import json
import logging
import platform
from dataclasses import dataclass
from datetime import datetime
from typing import Callable, Literal, Protocol

from pydantic import ValidationError

from .. import VERSION
from ..contracts import Input, ToolInfo, ToolResult
from .calculator import Calculation, calculate

log = logging.getLogger("odyssey")


class NoArguments(Input):
    pass


class DeviceInformation(Protocol):
    def read(self) -> dict: ...


class LocalDevice:
    def read(self) -> dict:
        return {"operating_system": platform.system(), "release": platform.release(),
                "architecture": platform.machine(), "application_version": VERSION}


def local_time(_: NoArguments) -> dict:
    instant = datetime.now().astimezone()
    return {"iso_datetime": instant.isoformat(timespec="seconds"), "timezone": instant.tzname(),
            "date": instant.date().isoformat(), "time": instant.strftime("%H:%M:%S"),
            "utc_offset": instant.strftime("%z")}


@dataclass(frozen=True)
class Tool:
    name: str
    description: str
    schema: type[Input]
    execute: Callable
    permission: Literal["read-only"] = "read-only"

    def info(self) -> ToolInfo:
        return ToolInfo(name=self.name, description=self.description, permission=self.permission,
                        parameters=self.schema.model_json_schema())


class Registry:
    def __init__(self, definitions: list[Tool]):
        self.tools = {tool.name: tool for tool in definitions}
        if len(self.tools) != len(definitions):
            raise ValueError("Tool names must be unique.")

    def definitions(self) -> list[dict]:
        return [{"type": "function", "function": tool.info().model_dump(exclude={"permission"})}
                for tool in self.tools.values()]

    def run(self, name: str, arguments: str | dict) -> ToolResult:
        tool = self.tools.get(name)
        if tool is None:
            log.info("tool_rejected", extra={"tool": "unknown"})
            return ToolResult(ok=False, result={"error": "This tool is unavailable."})
        try:
            if isinstance(arguments, str):
                if len(arguments) > 2048:
                    raise ValueError("Tool arguments are too long.")
                arguments = json.loads(arguments)
            validated = tool.schema.model_validate(arguments)
            result = ToolResult(ok=True, result=tool.execute(validated))
        except (ValidationError, json.JSONDecodeError, RecursionError):
            result = ToolResult(ok=False, result={"error": "Arguments must match the tool's JSON schema."})
        except ValueError as exc:
            # Only the bounded calculator raises user-facing domain errors.
            result = ToolResult(ok=False, result={"error": str(exc) if tool.name == "calculator" else "Invalid arguments."})
        except Exception:
            result = ToolResult(ok=False, result={"error": "The device could not complete this tool."})
        log.info("tool_complete", extra={"tool": name, "ok": result.ok})
        return result


def device_registry(device: DeviceInformation | None = None) -> Registry:
    source = device or LocalDevice()
    return Registry([
        Tool("calculator", "Calculate an arithmetic or scientific expression: arithmetic, powers, roots, trigonometry, logarithms, exp, abs, floor and ceil. Use for all math.", Calculation, calculate),
        Tool("get_current_datetime", "Read this device's current local date, time and UTC offset.", NoArguments, local_time),
        Tool("get_system_information", "Read the device operating system, architecture and Odyssey version.",
             NoArguments, lambda _: source.read()),
    ])
