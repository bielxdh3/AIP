export type ProfileCanonicalField = "ageCategory" | "species" | "pronouns";

export type LocalizedCanonicalOption = {
  value: string;
  primary: string;
  secondary?: string;
};

export const profileCanonicalOptions: Record<
  ProfileCanonicalField,
  readonly LocalizedCanonicalOption[]
> = {
  ageCategory: [{ value: "adult", primary: "Adulto", secondary: "Adult" }],
  species: [{ value: "agent", primary: "Agente", secondary: "Agent" }],
  pronouns: [
    { value: "they/them", primary: "Elu / delu", secondary: "they/them" },
  ],
};

export function localizedCanonicalValue(
  field: ProfileCanonicalField,
  value: string,
): LocalizedCanonicalOption {
  return (
    profileCanonicalOptions[field].find((option) => option.value === value) ?? {
      value,
      primary: value,
    }
  );
}
