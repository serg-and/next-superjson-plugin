import { withSuperJSONPage as _withSuperJSONPage } from "superjson-next/tools";
import { withSuperJSONProps as _withSuperJSONProps } from "superjson-next/tools";
export const getServerSideProps = _withSuperJSONProps(async function() {}, [
    "smth"
]);
function Page() {
    return <></>;
}
export default _withSuperJSONPage(Page);
