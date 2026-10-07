// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App.tsx";

// The WebView's own menu is a browser's (Back, Refresh, Print). Only text fields and selected
// text keep it, for cut, copy and paste.
document.addEventListener("contextmenu", (event) => {
  const editable = event.target instanceof Element && event.target.closest("input, textarea, [contenteditable]");
  if (!editable && !window.getSelection()?.toString()) event.preventDefault();
});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
