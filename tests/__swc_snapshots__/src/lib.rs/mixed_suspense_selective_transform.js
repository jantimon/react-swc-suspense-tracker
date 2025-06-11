import { BoundaryTrackerSWC } from "react-swc-suspense-tracker/context";
import { Suspense as ReactSuspense } from "react";
// User's own Suspense component
function Suspense(props) {
    return <div className="my-suspense">{props.children}</div>;
}
function App() {
    return <div>
      <BoundaryTrackerSWC fallback={<Loading/>} boundaryId="my/file.tsx:0" boundary={ReactSuspense}>
        <Component1/>
      </BoundaryTrackerSWC>
      <Suspense fallback={<div>Should not transform</div>}>
        <Component2/>
      </Suspense>
    </div>;
}
