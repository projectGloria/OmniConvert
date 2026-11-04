import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import "./styles/index.css";

const root = document.getElementById("root");
if (!root) throw new Error("#root missing from index.html");

// StrictMode double-invokes effects in dev — every Tauri listener below is written
// to survive mount→unmount→mount without leaking a second subscription.
createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
