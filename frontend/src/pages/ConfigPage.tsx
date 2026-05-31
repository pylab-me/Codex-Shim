import {ConfigEditor} from "@/components/ConfigEditor";
import {GenerationParamsPanel} from "@/components/GenerationParamsPanel";

export function ConfigPage() {
  return (
    <div className="space-y-5">
      <section>
        <ConfigEditor/>
      </section>
      <section>
        <GenerationParamsPanel/>
      </section>
    </div>
  );
}