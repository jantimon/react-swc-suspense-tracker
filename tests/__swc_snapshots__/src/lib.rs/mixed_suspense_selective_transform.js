import "react";
import { SuspenseTrackerSWC } from "react-swc-suspense-tracker/context";
// User's own Suspense component
function Suspense(props) {
    return <div className="my-suspense">{props.children}</div>;
}
function App() {
    return <div>
      <ReactSuspense fallback={<Loading/>}>
        <Component1/>
      </ReactSuspense>
      <SuspenseTrackerSWC fallback={<div>Should not transform</div>} id="my/file.tsx:4">
        <Component2/>
      </SuspenseTrackerSWC>
    </div>;
}
