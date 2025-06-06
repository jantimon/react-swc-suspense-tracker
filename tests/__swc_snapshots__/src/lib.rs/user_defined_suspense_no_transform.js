import { useEffect } from "react";
// User's own Suspense component - should NOT be transformed
function Suspense(props) {
    return <div className="my-suspense">{props.children}</div>;
}
function App() {
    return <Suspense fallback={<Loading/>}>
      <MyComponent/>
    </Suspense>;
}
