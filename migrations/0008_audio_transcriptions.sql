CREATE TABLE zain.audio_transcriptions (
    id              TEXT PRIMARY KEY,
    transcription   TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
