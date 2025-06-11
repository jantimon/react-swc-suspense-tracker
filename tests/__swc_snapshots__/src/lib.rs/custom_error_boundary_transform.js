import { BoundaryTrackerSWC } from "react-swc-suspense-tracker/context";
import { ErrorBoundary } from "my-package-name";
function App() {
    return <BoundaryTrackerSWC fallback={<ErrorFallback/>} boundaryId="my/file.tsx:0" boundary={ErrorBoundary}>
      <MyComponent/>
    </BoundaryTrackerSWC>;
}
