import json

from gretchen_flow.config import Config


def test_defaults():
    cfg = Config()
    assert cfg.engine == "faster-whisper"
    assert cfg.model == "large-v3-turbo"
    assert cfg.hotkey_mode == "toggle"
    assert cfg.sample_rate == 16_000


def test_save_and_load_roundtrip(tmp_path):
    path = tmp_path / "config.json"
    cfg = Config(model="small", language="de", hotkey_mode="hold")
    cfg.save(path)
    loaded = Config.load(path)
    assert loaded == cfg


def test_load_missing_file_returns_defaults(tmp_path):
    assert Config.load(tmp_path / "nope.json") == Config()


def test_load_preserves_unknown_keys(tmp_path):
    path = tmp_path / "config.json"
    path.write_text(json.dumps({"model": "small", "future_option": 42}))
    cfg = Config.load(path)
    assert cfg.model == "small"
    assert cfg.extra == {"future_option": 42}
    cfg.save(path)
    assert json.loads(path.read_text())["future_option"] == 42
