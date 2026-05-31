import React from "react";
import {createRoot} from "react-dom/client";
import {BrowserRouter, Route, Routes} from "react-router-dom";
import {ThemeProvider} from "./lib/theme";
import {ErrorBoundary} from "./components/ErrorBoundary";
import {App} from "./App";
import {DashboardPage} from "./pages/DashboardPage";
import {TrcPage} from "./pages/TrcPage";
import {ProvidersPage} from "./pages/ProvidersPage";
import {ConfigPage} from "./pages/ConfigPage";
import "./index.css";

createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <ThemeProvider>
      <ErrorBoundary>
        <BrowserRouter>
          <Routes>
            <Route path="/" element={<App/>}>
              <Route index element={<DashboardPage/>}/>
              <Route path="providers" element={<ProvidersPage/>}/>
              <Route path="trc" element={<TrcPage/>}/>
              <Route path="config" element={<ConfigPage/>}/>
            </Route>
          </Routes>
        </BrowserRouter>
      </ErrorBoundary>
    </ThemeProvider>
  </React.StrictMode>,
);