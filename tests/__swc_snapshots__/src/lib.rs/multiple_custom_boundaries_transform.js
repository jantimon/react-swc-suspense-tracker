import { BoundaryTrackerSWC } from "react-swc-suspense-tracker/context";
import { ErrorBoundary } from "my-package-name";
import { LoadingBoundary } from "another-package";
function App() {
    return <div>
      <BoundaryTrackerSWC fallback={<ErrorFallback/>} boundaryId="my/file.tsx:0" boundary={ErrorBoundary}>
        <Component1/>
      </BoundaryTrackerSWC>
      <BoundaryTrackerSWC fallback={<div>Loading...</div>} boundaryId="my/file.tsx:0" boundary={LoadingBoundary}>
        <Component2/>
      </BoundaryTrackerSWC>
    </div>;
}
