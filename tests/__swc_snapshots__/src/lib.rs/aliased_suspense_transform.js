import "react";
function App() {
    return <MySuspense fallback={<Loading/>}>
      <MyComponent/>
    </MySuspense>;
}
