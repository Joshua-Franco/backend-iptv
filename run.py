import uvicorn

from config.logging_setup import setup_logging
from config.settings import settings

if __name__ == "__main__":
    setup_logging()
    uvicorn.run(
        "main:app",
        host=settings.HOST,
        port=settings.PORT,
        reload=settings.RELOAD,
        log_config=None,
    )
