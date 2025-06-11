import { BoundaryTrackerSWC } from "react-swc-suspense-tracker/context";
import { useEffect, Suspense } from "react";
function App() {
    return <BoundaryTrackerSWC fallback={<Loading/>} boundaryId="my/file.tsx:0" boundary={Suspense}>
      <MyComponent/>
    </BoundaryTrackerSWC>;
}
