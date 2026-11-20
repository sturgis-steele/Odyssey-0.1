"""Run one local backend process with the same validated settings as the API."""
import json
import logging

import uvicorn

from app.config import Settings


class EventLog(logging.Formatter):
    def format(self, record):
        data = {"level": record.levelname.lower(), "event": record.getMessage()}
        for field in ("tool", "ok", "kind"):
            if hasattr(record, field):
                data[field] = getattr(record, field)
        return json.dumps(data)


if __name__ == "__main__":
    settings = Settings()
    handler = logging.StreamHandler()
    handler.setFormatter(EventLog())
    logger = logging.getLogger("odyssey")
    logger.addHandler(handler)
    logger.setLevel(logging.INFO)
    uvicorn.run("app.main:app", host=settings.backend_host, port=settings.backend_port,
                workers=1, access_log=False, server_header=False)
