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
          "Unable to fetch data file. Run `make collector` to generate it"
        );
      })
      .then((res) => {
        const sortedChainKeys = Object.keys(res).sort((a, b) => {
          if (
            (res[a].testnet && res[b].testnet) ||
            (!res[a].testnet && !res[b].testnet)
          ) {
            if (a == "polkadot") {
              return -100;
            }
            if (b == "polkadot") {
              return 1;
            }
            if (a == "kusama") {
              return -50;
            }
            if (b == "kusama") {
              return 1;
            }
            if (a == "westend") {
              return -10;
            }
            if (b == "westend") {
              return 1;
            }
            return res[a].title.localeCompare(res[b].title);
          } else if (res[a].testnet) {
            return 1;
          } else {
            return -1;
          }
        });
        const sortedChains: Chains = {};
        sortedChainKeys.forEach((k) => (sortedChains[k] = res[k]));
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
    const sortedChainKeys = Object.keys(chains).sort((a, b) => {
      if (
        (chains[a].testnet && chains[b].testnet) ||
        (!chains[a].testnet && !chains[b].testnet)
      ) {
        if (a == "polkadot") {
          return -100;
        }
        if (b == "polkadot") {
          return 1;
        }
        if (a == "kusama") {
          return -50;
        }
        if (b == "kusama") {
          return 1;
        }
        if (a == "westend") {
          return -10;
        }
        if (b == "westend") {
          return 1;
        }
        return chains[a].title.localeCompare(chains[b].title);
      } else if (chains[a].testnet) {
        return 1;
      } else {
        return -1;
      }
    });

    const sortedChains: Chains = {};
    sortedChainKeys.forEach((k) => (sortedChains[k] = chains[k]));
    setChains(sortedChains);

    const locationChain = location.hash.replace("#/", "");
    const network =
      (Object.keys(chains).includes(locationChain) && locationChain) ||
      Object.keys(chains).find(
        (key) => chains[key].genesisHash == locationChain
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
