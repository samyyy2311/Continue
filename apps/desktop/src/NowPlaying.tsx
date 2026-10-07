// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useState } from "react";
import { Music, Pause, Play, SkipBack, SkipForward, Volume1, Volume2 } from "lucide-react";
import { errorMessage, mediaCommand, onNowPlaying } from "./api.ts";
import type { MediaCommand, NowPlaying as Playing } from "./types.ts";

export function NowPlaying({ peer, onError }: { peer: string; onError: (message: string) => void }) {
  const [playing, setPlaying] = useState<Playing | null>(null);

  useEffect(() => {
    const unlisten = onNowPlaying((now) => {
      if (now.peerId === peer) setPlaying(now.title ? now : null);
    });
    return () => void unlisten.then((stop) => stop());
  }, [peer]);

  if (!playing) return null;
  const send = (command: MediaCommand) =>
    void mediaCommand(peer, command).catch((error) => onError(errorMessage(error)));

  return (
    <section className="section">
      <h2 className="label">Playing on your phone</h2>
      <div className="now-playing">
        {playing.art ? (
          <img className="list-thumb" src={playing.art} alt="" />
        ) : (
          <span className="list-leading">
            <Music size={18} />
          </span>
        )}
        <div className="list-text">
          <span className="list-title">{playing.title}</span>
          <span className="list-sub">{[playing.artist, playing.app].filter(Boolean).join(" · ")}</span>
        </div>
        <button type="button" className="icon-btn" aria-label="Previous" onClick={() => send("PREVIOUS")}>
          <SkipBack size={18} />
        </button>
        <button
          type="button"
          className="icon-btn"
          aria-label={playing.playing ? "Pause" : "Play"}
          onClick={() => send("PLAY_PAUSE")}
        >
          {playing.playing ? <Pause size={18} /> : <Play size={18} />}
        </button>
        <button type="button" className="icon-btn" aria-label="Next" onClick={() => send("NEXT")}>
          <SkipForward size={18} />
        </button>
        <button type="button" className="icon-btn" aria-label="Volume down" onClick={() => send("VOLUME_DOWN")}>
          <Volume1 size={18} />
        </button>
        <button type="button" className="icon-btn" aria-label="Volume up" onClick={() => send("VOLUME_UP")}>
          <Volume2 size={18} />
        </button>
      </div>
    </section>
  );
}
