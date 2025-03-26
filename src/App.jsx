import { useEffect } from "react";
import "./App.css";
import init, { test_bigfile } from "./wasm/pkg/hello_world";

function App() {
  const handleFileChange = async (event) => {
    const file = event.target.files[0];
    if (file) {
      await test_bigfile(file);
    }
  };

  useEffect(() => {
    init().then(() => {});
  }, []);

  return (
    <>
      <input type="file" onChange={handleFileChange} />
    </>
  );
}

export default App;
