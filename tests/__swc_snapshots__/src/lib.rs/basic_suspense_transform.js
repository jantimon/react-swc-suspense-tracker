import { useEffect } from "react";
import { SuspenseTrackerSWC } from "react-swc-suspense-tracker/context";
function App() {
    return <SuspenseTrackerSWC fallback={<Loading/>} id="my/file.tsx:1">
      <MyComponent/>
    </SuspenseTrackerSWC>;
}
