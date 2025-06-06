import "react";
import { SuspenseTrackerSWC } from "react-swc-suspense-tracker/context";
function App() {
    return <div>
      <SuspenseTrackerSWC fallback={<Loading/>} id="my/file.tsx:1">
        <Component1/>
      </SuspenseTrackerSWC>
      <SuspenseTrackerSWC fallback={<div>Loading...</div>} id="my/file.tsx:3">
        <Component2/>
      </SuspenseTrackerSWC>
    </div>;
}
