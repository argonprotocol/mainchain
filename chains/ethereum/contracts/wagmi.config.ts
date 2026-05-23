import { defineConfig, type Plugin } from '@wagmi/cli';
import { hardhat } from '@wagmi/cli/plugins';
import { encodeEventTopics } from 'viem';

type AbiParameter = {
  name: string;
  type: string;
  internalType?: string;
  components?: AbiParameter[];
};

function toPascalCase(value: string) {
  return value
    .replace(/(^|[^A-Za-z0-9]+)([A-Za-z0-9])/g, (_match, _prefix, char: string) =>
      char.toUpperCase(),
    )
    .replace(/[^A-Za-z0-9]/g, '');
}

function formatPropertyName(name: string) {
  return /^[A-Za-z_$][A-Za-z0-9_$]*$/.test(name) ? name : JSON.stringify(name);
}

function qualifyContractName(contractName: string, memberName: string) {
  if (contractName.endsWith('Gateway') && memberName.startsWith('Gateway')) {
    return `${contractName}${memberName.slice('Gateway'.length)}`;
  }

  return `${contractName}${memberName}`;
}

function getStructTypeName(
  contractName: string,
  internalType: string | undefined,
  fallbackName: string,
) {
  const cleanedInternalType = internalType?.replace(/^struct\s+/, '').replace(/\[[^\]]*\]/g, '');
  const structName = cleanedInternalType?.split('.').at(-1);

  return qualifyContractName(contractName, structName ?? toPascalCase(fallbackName));
}

function abiParameterTypeToTs(
  parameter: AbiParameter,
  contractName: string,
  generatedTypes: Map<string, string>,
): string {
  const arrayDimensions = parameter.type.match(/\[[^\]]*\]/g)?.length ?? 0;
  const baseType = parameter.type.replace(/\[[^\]]*\]/g, '');

  let tsType: string;
  switch (baseType) {
    case 'address':
    case 'bytes':
      tsType = 'Hex';
      break;
    case 'bool':
      tsType = 'boolean';
      break;
    case 'string':
      tsType = 'string';
      break;
    case 'tuple':
      tsType = tupleTypeToTs(
        parameter.components ?? [],
        contractName,
        generatedTypes,
        parameter.internalType,
        parameter.name || 'Tuple',
      );
      break;
    default:
      if (/^bytes\d+$/.test(baseType)) {
        tsType = 'Hex';
      } else if (/^u?int\d*$/.test(baseType)) {
        tsType = 'bigint';
      } else {
        tsType = 'unknown';
      }
      break;
  }

  for (let i = 0; i < arrayDimensions; i += 1) {
    tsType = `readonly ${tsType}[]`;
  }

  return tsType;
}

function tupleTypeToTs(
  components: AbiParameter[],
  contractName: string,
  generatedTypes: Map<string, string>,
  internalType: string | undefined,
  fallbackName: string,
): string {
  if (!internalType) {
    return `{\n${renderTypeFields(components, contractName, generatedTypes)}\n}`;
  }

  const typeName = getStructTypeName(contractName, internalType, fallbackName);
  if (!generatedTypes.has(typeName)) {
    generatedTypes.set(
      typeName,
      `export type ${typeName} = {\n${renderTypeFields(components, contractName, generatedTypes)}\n};`,
    );
  }

  return typeName;
}

function renderTypeFields(
  parameters: AbiParameter[],
  contractName: string,
  generatedTypes: Map<string, string>,
) {
  return parameters
    .map((parameter, index) => {
      const fieldName = formatPropertyName(parameter.name || `field${index}`);
      const fieldType = abiParameterTypeToTs(parameter, contractName, generatedTypes);
      return `  ${fieldName}: ${fieldType};`;
    })
    .join('\n');
}

const eventMetadata: Plugin = {
  name: 'EventMetadataConstants',
  run({ contracts }) {
    const content: string[] = [];
    const generatedTypes = new Map<string, string>();

    for (const contract of contracts) {
      const contractEvents: string[] = [];
      const seenEventNames = new Set<string>();
      const contractEventsName = `${toPascalCase(contract.name)}Events`;

      for (const item of contract.abi) {
        if (item.type !== 'event' || seenEventNames.has(item.name)) {
          continue;
        }

        seenEventNames.add(item.name);
        const eventTopic = encodeEventTopics({
          abi: [item],
          eventName: item.name,
        })[0];
        const eventTypeName = `${toPascalCase(contract.name)}${item.name}`;

        generatedTypes.set(
          eventTypeName,
          `export type ${eventTypeName} = {\n${renderTypeFields(item.inputs as AbiParameter[], contract.name, generatedTypes)}\n};`,
        );

        contractEvents.push(
          `  ${formatPropertyName(item.name)}: {\n    name: '${item.name}',\n    topic: '${eventTopic?.toLowerCase() ?? ''}',\n  },`,
        );
      }

      if (contractEvents.length > 0) {
        content.push(
          `export const ${contractEventsName} = {\n${contractEvents.join('\n')}\n} as const;`,
        );
      }
    }

    const typeContent = generatedTypes.size
      ? `import type { Hex } from 'viem';\n\n${[...generatedTypes.values()].join('\n\n')}\n\n`
      : '';

    return { content: `${typeContent}${content.join('\n\n')}` };
  },
};

export default defineConfig({
  out: 'generated.ts',
  plugins: [
    hardhat({
      project: '.',
    }),
    eventMetadata,
  ],
});
