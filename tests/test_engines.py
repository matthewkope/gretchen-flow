import pytest

from gretchen_flow.engines import create_engine


def test_create_faster_whisper():
    engine = create_engine("faster-whisper", "small")
    assert engine.name == "faster-whisper"


def test_create_mlx_whisper():
    engine = create_engine("mlx-whisper", "large-v3-turbo")
    assert engine.name == "mlx-whisper"


def test_unknown_engine_raises():
    with pytest.raises(ValueError, match="Unknown engine"):
        create_engine("bogus", "small")
