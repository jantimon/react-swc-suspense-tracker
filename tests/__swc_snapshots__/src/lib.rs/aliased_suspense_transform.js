import { BoundaryTrackerSWC } from "react-swc-suspense-tracker/context";
import { Suspense as MySuspense } from "react";
function App() {
    return <BoundaryTrackerSWC fallback={<Loading/>} boundaryId="my/file.tsx:0" boundary={MySuspense}>
      <MyComponent/>
    </BoundaryTrackerSWC>;
}
