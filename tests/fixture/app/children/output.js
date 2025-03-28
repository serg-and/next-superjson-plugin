import { serialize } from "superjson-next/tools";
import SuperJSONComponent from "superjson-next/client";
import ClientComponent from "./ClientComponent";
export default function Page() {
    const rest = {};
    const date = new Date();
    return <SuperJSONComponent props={serialize({
        date: date,
        ...rest
    })} component={ClientComponent}>
      <p>children</p>
    </SuperJSONComponent>;
}
