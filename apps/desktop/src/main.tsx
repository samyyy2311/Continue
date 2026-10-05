// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App.tsx";
import { NotificationOverlay } from "./NotificationOverlay.tsx";
import { PermissionDialog } from "./PermissionDialog.tsx";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
    <PermissionDialog />
    <NotificationOverlay />
  </React.StrictMode>,
);
