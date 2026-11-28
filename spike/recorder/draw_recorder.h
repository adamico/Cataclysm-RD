#pragma once
// Fields spike (issue #14) — recorder API.
//
// Implemented in rng_recording.cpp, the recording copy of BN's src/rng.cpp.
// The recorder program links that file in place of BN's rng.cpp.o, so every
// helper-level draw made by untouched BN code passes through here.
//
// Case file format: see spike/recorder/CASE_FORMAT.md.

#include <string>

namespace draw_recorder
{

// Open a case file and write its header line. `header_json` is a JSON object
// body (without braces) appended to the header, e.g. "\"case\":\"smoke\"".
// Recording starts immediately; the engine is NOT reseeded here.
void begin( const std::string &path, const std::string &header_json );

// Write a marker line: {"k":"mark","label":...,<extra_json>}.
// Use it to segment a case (turn boundaries, rule boundaries, ...).
void mark( const std::string &label, const std::string &extra_json = "" );

// Pause/resume recording without closing the file (draws while paused are
// counted in the footer as "unrecorded").
void pause();
void resume();

// Write the footer line and close the file.
void end();

bool active();

} // namespace draw_recorder
