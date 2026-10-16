import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { installLogBridge } from "./logBridge";
import "./styles.css";

installLogBridge();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
