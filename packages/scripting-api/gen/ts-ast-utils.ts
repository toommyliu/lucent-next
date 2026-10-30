import ts from "typescript";

export type TypeReferenceInfo = {
  readonly name: string;
  readonly unqualifiedName: string;
  readonly args: readonly ts.TypeNode[];
};

export const getPropertyName = (name: ts.PropertyName): string | null => {
  if (
    ts.isIdentifier(name) ||
    ts.isStringLiteral(name) ||
    ts.isNumericLiteral(name)
  ) {
    return name.text;
  }
  return null;
};

const getEntityNameText = (name: ts.EntityName): string =>
  ts.isIdentifier(name)
    ? name.text
    : `${getEntityNameText(name.left)}.${name.right.text}`;

const getUnqualifiedEntityName = (name: ts.EntityName): string =>
  ts.isIdentifier(name) ? name.text : name.right.text;

export const parseTypeReference = (
  node: ts.TypeNode | undefined,
): TypeReferenceInfo | null => {
  if (!node || !ts.isTypeReferenceNode(node)) {
    return null;
  }

  return {
    name: getEntityNameText(node.typeName),
    unqualifiedName: getUnqualifiedEntityName(node.typeName),
    args: Array.from(node.typeArguments ?? []),
  };
};
