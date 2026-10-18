import re

PALETTE: list[str] = [
    "#e30613",
    "#0a7cff",
    "#7c5cff",
    "#00b86b",
    "#ff8a00",
    "#e5008c",
    "#00a3a8",
    "#5c6bc0",
    "#c26a00",
    "#d1112b",
    "#2e7d32",
    "#00838f",
]


def accent_for(seed: str) -> str:
    total = 0
    for char in seed:
        total = (total * 31 + ord(char)) % 100_000
    return PALETTE[total % len(PALETTE)]


def make_initials(name: str) -> str:
    words = re.findall(r"[A-Za-z0-9\u00c0-\u00ff]+", name.upper())
    if not words:
        return "TV"
    if len(words) == 1:
        return words[0][:8]
    initials = "".join(word[0] for word in words[:4])
    return initials[:8] or "TV"
