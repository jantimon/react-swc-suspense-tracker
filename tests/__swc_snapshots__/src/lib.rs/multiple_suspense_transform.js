import { BoundaryTrackerSWC } from "react-swc-suspense-tracker/context";
import { Suspense } from "react";
function App() {
    return <div>
      <BoundaryTrackerSWC fallback={<Loading/>} boundaryId="my/file.tsx:0" boundary={Suspense}>
        <Component1/>
      </BoundaryTrackerSWC>
      <BoundaryTrackerSWC fallback={<div>Loading...</div>} boundaryId="my/file.tsx:0" boundary={Suspense}>
        <Component2/>
      </BoundaryTrackerSWC>
    </div>;
}
