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


@pytest.mark.parametrize(
    "model",
    ["../../etc/passwd", "a/../../b", "", "bad\nname", "x;rm -rf", "a" * 257],
)
def test_invalid_model_rejected(model):
    with pytest.raises(ValueError, match="invalid model identifier"):
        create_engine("faster-whisper", model)


@pytest.mark.parametrize(
    "model",
    ["large-v3-turbo", "small", "Systran/faster-whisper-large-v3", "base.en"],
)
def test_valid_model_accepted(model):
    assert create_engine("faster-whisper", model).name == "faster-whisper"
