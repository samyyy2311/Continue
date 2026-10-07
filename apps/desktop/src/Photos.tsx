// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useEffect, useState } from "react";
import { RefreshCw } from "lucide-react";
import { errorMessage, getPhoto, listPhotos, onPhotoTaken } from "./api.ts";
import { byDay } from "./format.ts";
import type { Photo } from "./types.ts";

export function Photos({ peer, onError }: { peer: string; onError: (message: string) => void }) {
  // Undefined while asking; null when the phone hasn't given access.
  const [photos, setPhotos] = useState<Photo[] | null>();
  const [problem, setProblem] = useState<string | null>(null);

  const load = useCallback(() => {
    setPhotos(undefined);
    setProblem(null);
    listPhotos(peer).then(setPhotos, (error) => setProblem(errorMessage(error)));
  }, [peer]);
  useEffect(load, [load]);

  useEffect(() => {
    const unlisten = onPhotoTaken((from, photo) => {
      if (from !== peer) return;
      setPhotos((shown) => shown && [photo, ...shown.filter((p) => p.id !== photo.id)]);
    });
    return () => void unlisten.then((stop) => stop());
  }, [peer]);

  let note: string | null = null;
  if (problem) note = problem;
  else if (photos === undefined) note = "Asking your phone…";
  else if (photos === null) note = "Turn on Show photos on your computer in Continue's settings on your phone.";
  else if (photos.length === 0) note = "No photos on your phone yet.";

  return (
    <section className="section">
      <div className="section-head">
        <h2 className="label">Photos</h2>
        <button type="button" className="icon-btn" title="Refresh" aria-label="Refresh photos" onClick={load}>
          <RefreshCw size={16} />
        </button>
      </div>
      {note ? (
        <p className="supporting">{note}</p>
      ) : (
        byDay(photos ?? [], (photo) => photo.takenAt).map(([day, taken]) => (
          <div key={day} className="photo-day">
            <h3 className="photo-day-label">{day}</h3>
            <div className="photos">
              {taken.map((photo) => (
                <button
                  key={photo.id}
                  type="button"
                  className="photo"
                  title={`Copy ${photo.name} here`}
                  onClick={() => getPhoto(peer, photo.id).catch((error) => onError(errorMessage(error)))}
                >
                  <img src={photo.thumbnail} alt={photo.name} loading="lazy" />
                </button>
              ))}
            </div>
          </div>
        ))
      )}
    </section>
  );
}
