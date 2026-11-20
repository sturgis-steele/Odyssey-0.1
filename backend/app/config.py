from pathlib import Path
from typing import Literal
from urllib.parse import urlsplit

from pydantic import Field, field_validator, model_validator
from pydantic_settings import BaseSettings, SettingsConfigDict

ROOT = Path(__file__).resolve().parents[2]
LOCAL_HOSTS = {"127.0.0.1", "localhost", "::1"}
DEFAULT_PROMPT = (
    "You are Odyssey, a concise offline assistant on this device. Answer accurately and admit uncertainty. "
    "Always call calculator for arithmetic, get_current_datetime for the current time or date, and "
    "get_system_information for device details. Use actual tool results. Never invent results. "
    "Internet, reference libraries, personal memory, maps and calendar are unavailable."
)


class Settings(BaseSettings):
    model_config = SettingsConfigDict(env_file=ROOT / "backend/.env", env_file_encoding="utf-8", extra="ignore")
    app_name: str = Field(default="Odyssey", min_length=1, max_length=64)
    app_env: Literal["development", "test", "production"] = "development"
    backend_host: str = "127.0.0.1"
    backend_port: int = Field(default=8000, ge=1, le=65535)
    llama_base_url: str = "http://127.0.0.1:8080/v1"
    llama_model: str = Field(default="odyssey-local", min_length=1, max_length=200)
    database_path: Path = Path("data/odyssey.db")
    max_tool_iterations: int = Field(default=3, ge=1, le=8)
    max_output_tokens: int = Field(default=512, ge=32, le=8192)
    context_tokens: int = Field(default=4096, ge=1024, le=131072)
    history_turns: int = Field(default=8, ge=0, le=50)
    model_timeout_seconds: float = Field(default=120, ge=1, le=600)
    temperature: float = Field(default=0.1, ge=0, le=2)
    enable_thinking: bool = False
    system_prompt: str = Field(default=DEFAULT_PROMPT, min_length=1, max_length=4096)

    @field_validator("backend_host")
    @classmethod
    def loopback_host(cls, value: str) -> str:
        if value not in LOCAL_HOSTS:
            raise ValueError("Use a loopback address for the backend.")
        return value

    @field_validator("llama_base_url")
    @classmethod
    def local_inference(cls, value: str) -> str:
        url = urlsplit(value)
        if (url.scheme != "http" or url.hostname not in LOCAL_HOSTS or url.username is not None
                or url.password is not None or url.query or url.fragment or url.path.rstrip("/") != "/v1"):
            raise ValueError("Use the local llama-server HTTP /v1 endpoint.")
        _ = url.port  # Validate the port before the first request.
        return value.rstrip("/")

    @field_validator("database_path")
    @classmethod
    def resolve_database(cls, value: Path) -> Path:
        return (ROOT / value).resolve()

    @model_validator(mode="after")
    def generation_budget(self):
        if self.max_output_tokens + 128 >= self.context_tokens:
            raise ValueError("Reserve prompt space within CONTEXT_TOKENS.")
        return self
