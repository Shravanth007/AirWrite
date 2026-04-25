import { BrowserRouter, Route, Routes } from "react-router-dom";
import { Overlay } from "./components/Overlay";
import { SettingsPanel } from "./components/SettingsPanel";

export default function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<Overlay />} />
        <Route path="/settings" element={<SettingsPanel />} />
      </Routes>
    </BrowserRouter>
  );
}
