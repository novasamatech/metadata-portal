import { useEffect, useState } from "react";
import { Chains, Portals } from "../scheme";
import { About } from "./About";
import { Banner } from "./Banner";
import { FAQ } from "./FAQ";
import { Hr } from "./Hr";
import { Links } from "./Links";
import { Network } from "./Network";
import { NetworkAndPortalSelectMobile } from "./NetworkAndPortalSelectMobile";
import { NetworkSelect } from "./NetworkSelect";
import { PortalSelect } from "./PortalSelect";

export default function App() {
  const [chains, setChains] = useState({} as Chains);
  const [portals, setPortals] = useState({} as Portals);
  const [currentChain, setCurrentChain] = useState<string>("");
  const spec = chains[currentChain];

  useEffect(() => {
    fetch("data.json")
      .then((res) => res.json())
      .catch(() => {
        console.error(
          "Unable to fetch data file. Run `make collector` to generate it",
        );
      })
      .then((res) => {
        const polkadot = "polkadot";
        const kusama = "kusama";
        const westend = "westend";
        const rococo = "rococo";
        const polkadotParachains: string[] = [];
        const kusamaParachains: string[] = [];
        const westendParachain: string[] = [];
        const rococoParachains: string[] = [];
        const solochains: string[] = [];
        const testSolochains: string[] = [];

        Object.keys(res).forEach((key) => {
          const chainSpec = res[key];
          if (chainSpec.relayChain === polkadot) {
            polkadotParachains.push(key);
          } else if (chainSpec.relayChain === kusama) {
            kusamaParachains.push(key);
          } else if (chainSpec.relayChain === westend) {
            westendParachain.push(key);
          } else if (chainSpec.relayChain === rococo) {
            rococoParachains.push(key);
          } else if (chainSpec.testnet && !chainSpec.relayChain) {
            testSolochains.push(key);
          } else if (!chainSpec.testnet && !chainSpec.relayChain) {
            solochains.push(key);
          }
        });

        polkadotParachains.sort((a, b) =>
          res[a].title.localeCompare(res[b].title),
        );
        kusamaParachains.sort((a, b) =>
          res[a].title.localeCompare(res[b].title),
        );
        westendParachain.sort((a, b) =>
          res[a].title.localeCompare(res[b].title),
        );
        rococoParachains.sort((a, b) =>
          res[a].title.localeCompare(res[b].title),
        );
        solochains.sort((a, b) => res[a].title.localeCompare(res[b].title));
        testSolochains.sort((a, b) => res[a].title.localeCompare(res[b].title));

        const sortedChains: Chains = {};
        sortedChains[polkadot] = res[polkadot];
        polkadotParachains.forEach((key) => (sortedChains[key] = res[key]));
        sortedChains[kusama] = res[kusama];
        kusamaParachains.forEach((key) => (sortedChains[key] = res[key]));
        solochains.forEach((key) => (sortedChains[key] = res[key]));
        sortedChains[westend] = res[westend];
        westendParachain.forEach((key) => (sortedChains[key] = res[key]));
        sortedChains[rococo] = res[rococo];
        rococoParachains.forEach((key) => (sortedChains[key] = res[key]));
        testSolochains.forEach((key) => (sortedChains[key] = res[key]));
        setChains(sortedChains);
      });
  }, []);

  useEffect(() => {
    fetch("portals.json")
      .then((res) => res.json())
      .catch(() => {
        console.error("Unable to fetch portals file");
      })
      .then(setPortals);
  }, []);

  useEffect(() => {
    if (Object.keys(chains).length === 0 || currentChain) return;
    const polkadot = "polkadot";
    const kusama = "kusama";
    const westend = "westend";
    const rococo = "rococo";
    const polkadotParachains: string[] = [];
    const kusamaParachains: string[] = [];
    const westendParachain: string[] = [];
    const rococoParachains: string[] = [];
    const solochains: string[] = [];
    const testSolochains: string[] = [];

    Object.keys(chains).forEach((key) => {
      const chainSpec = chains[key];
      if (chainSpec.relayChain === polkadot) {
        polkadotParachains.push(key);
      } else if (chainSpec.relayChain === kusama) {
        kusamaParachains.push(key);
      } else if (chainSpec.relayChain === westend) {
        westendParachain.push(key);
      } else if (chainSpec.relayChain === rococo) {
        rococoParachains.push(key);
      } else if (chainSpec.testnet && !chainSpec.relayChain) {
        testSolochains.push(key);
      } else if (!chainSpec.testnet && !chainSpec.relayChain) {
        solochains.push(key);
      }
    });

    polkadotParachains.sort((a, b) =>
      chains[a].title.localeCompare(chains[b].title),
    );
    kusamaParachains.sort((a, b) =>
      chains[a].title.localeCompare(chains[b].title),
    );
    westendParachain.sort((a, b) =>
      chains[a].title.localeCompare(chains[b].title),
    );
    rococoParachains.sort((a, b) =>
      chains[a].title.localeCompare(chains[b].title),
    );
    solochains.sort((a, b) => chains[a].title.localeCompare(chains[b].title));
    testSolochains.sort((a, b) =>
      chains[a].title.localeCompare(chains[b].title),
    );

    const sortedChains: Chains = {};
    sortedChains[polkadot] = chains[polkadot];
    polkadotParachains.forEach((key) => (sortedChains[key] = chains[key]));
    sortedChains[kusama] = chains[kusama];
    kusamaParachains.forEach((key) => (sortedChains[key] = chains[key]));
    solochains.forEach((key) => (sortedChains[key] = chains[key]));
    sortedChains[westend] = chains[westend];
    westendParachain.forEach((key) => (sortedChains[key] = chains[key]));
    sortedChains[rococo] = chains[rococo];
    rococoParachains.forEach((key) => (sortedChains[key] = chains[key]));
    testSolochains.forEach((key) => (sortedChains[key] = chains[key]));
    setChains(sortedChains);

    const locationChain = location.hash.replace("#/", "");
    const network =
      (Object.keys(chains).includes(locationChain) && locationChain) ||
      Object.keys(chains).find(
        (key) => chains[key].genesisHash == locationChain,
      ) ||
      Object.keys(chains)[0];
    setCurrentChain(network);
  }, [chains]);

  useEffect(() => {
    if (currentChain) location.assign("#/" + currentChain);
  }, [currentChain]);

  if (!spec) return null;

  return (
    <div>
      <Banner />
      <div className="flex flex-col xl:flex-row">
        <div className="flex flex-col xl:top-0 w-full p-2 md:px-4 xl:p-4 xl:pr-2 xl:pt-12 xl:w-full xl:max-w-[360px] xl:min-h-screen">
          <div className="xl:hidden mb-2">
            <About />
          </div>
          <div className="xl:hidden">
            <NetworkAndPortalSelectMobile
              chains={chains}
              portals={portals}
              currentChain={currentChain}
              onSelect={setCurrentChain}
            />
          </div>
          <div className="hidden xl:block mb-10 empty:hidden">
            <PortalSelect portals={portals} />
          </div>
          <div className="hidden xl:block mb-6">
            <About />
          </div>
          <div className="hidden xl:flex xl:overflow-y-auto h-0 grow">
            <NetworkSelect
              chains={chains}
              currentChain={currentChain}
              onSelect={setCurrentChain}
            />
          </div>
        </div>
        <div className="w-full p-2 pt-0 pb-8 md:pb-24 md:p-4 xl:pl-2 xl:pt-12 space-y-4">
          <Network spec={spec} />
          <FAQ />
          <div className="py-4 xl:hidden">
            <Hr />
          </div>
          <div className="xl:hidden">
            <Links />
          </div>
        </div>
      </div>
    </div>
  );
}
