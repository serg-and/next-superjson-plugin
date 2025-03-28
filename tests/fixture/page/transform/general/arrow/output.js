import { withSuperJSONPage as _withSuperJSONPage } from "superjson-next/tools";
import { withSuperJSONProps as _withSuperJSONProps } from "superjson-next/tools";
export const getServerSideProps = _withSuperJSONProps(async ()=>{}, [
    "smth"
]);
export default _withSuperJSONPage(()=>{
    return <></>;
});
