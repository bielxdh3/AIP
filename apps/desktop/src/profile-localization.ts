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
  ageCategory: [
    { value: "adult", primary: "Adulto", secondary: "Adult" },
    { value: "adolescent", primary: "Adolescente", secondary: "Adolescent" },
    { value: "child", primary: "Criança", secondary: "Child" },
  ],
  species: [
    { value: "agent", primary: "Agente", secondary: "Agent" },
    { value: "human", primary: "Humano", secondary: "Human" },
    { value: "android", primary: "Androide", secondary: "Android" },
  ],
  pronouns: [
    { value: "they/them", primary: "Elu / delu", secondary: "they/them" },
    { value: "ela/dela", primary: "Ela / dela", secondary: "she/her" },
    { value: "ele/dele", primary: "Ele / dele", secondary: "he/him" },
    { value: "custom", primary: "Personalizado" },
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
