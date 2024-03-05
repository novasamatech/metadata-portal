import React from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter as Router, Route, Routes } from "react-router-dom";
import App from "./components/App";
import "./index.css";

createRoot(document.getElementById("root") as HTMLElement).render(
  <Router>
    <React.StrictMode>
      <Routes>
        <Route path="/" element={<App />} />
      </Routes>
    </React.StrictMode>
  </Router>
);
