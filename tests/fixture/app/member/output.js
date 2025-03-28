import { serialize } from "superjson-next/tools";
import SuperJSONComponent from "superjson-next/client";
import ServerComponent from "./ServerComponent";
import Client from "./Client";

export default function Page() {
  const rest = {};
  const date = new Date();

  return <>
      <ServerComponent date={date} />
      <SuperJSONComponent
        props={serialize({
          date: date,
          ...rest,
        })}
        component={Client.Component}
      />
    </>;
}
