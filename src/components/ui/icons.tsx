import type { ReactNode, SVGProps } from "react";

function Icon({ children, ...props }: SVGProps<SVGSVGElement> & { children: ReactNode }) {
  return (
    <svg
      width="14"
      height="14"
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.4"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      {...props}
    >
      {children}
    </svg>
  );
}

type Props = SVGProps<SVGSVGElement>;

export const BranchIcon = (props: Props) => (
  <Icon {...props}>
    <circle cx="4.5" cy="3.5" r="1.6" />
    <circle cx="4.5" cy="12.5" r="1.6" />
    <circle cx="11.5" cy="5.5" r="1.6" />
    <path d="M4.5 5.1v5.8M11.5 7.1c0 2.4-7 1.4-7 3.8" />
  </Icon>
);

export const TagIcon = (props: Props) => (
  <Icon {...props}>
    <path d="M2.5 2.5h5l6 6-5 5-6-6z" />
    <circle cx="5.3" cy="5.3" r="0.9" />
  </Icon>
);

export const RemoteIcon = (props: Props) => (
  <Icon {...props}>
    <path d="M4.5 12.5a3 3 0 0 1-.4-5.97 4 4 0 0 1 7.7.90 2.6 2.6 0 0 1-.3 5.07z" />
  </Icon>
);

export const StashIcon = (props: Props) => (
  <Icon {...props}>
    <path d="M2.5 4.5h11v3h-11zM3.5 7.5v5h9v-5M6.5 10h3" />
  </Icon>
);

export const FolderIcon = (props: Props) => (
  <Icon {...props}>
    <path d="M2 4.5a1 1 0 0 1 1-1h3l1.5 1.5H13a1 1 0 0 1 1 1v5.5a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1z" />
  </Icon>
);

export const ChevronIcon = ({ open, ...props }: Props & { open: boolean }) => (
  <Icon {...props} style={{ transform: open ? "rotate(90deg)" : undefined, transition: "transform 120ms" }}>
    <path d="M6 3.5 10.5 8 6 12.5" />
  </Icon>
);

export const SearchIcon = (props: Props) => (
  <Icon {...props}>
    <circle cx="7" cy="7" r="4.2" />
    <path d="m10.2 10.2 3.3 3.3" />
  </Icon>
);

export const SidebarIcon = (props: Props) => (
  <Icon {...props}>
    <rect x="2" y="3" width="12" height="10" rx="1.5" />
    <path d="M6 3v10" />
  </Icon>
);

export const SparkIcon = (props: Props) => (
  <Icon {...props}>
    <path d="M8 2.5v3M8 10.5v3M2.5 8h3M10.5 8h3M4.3 4.3l1.7 1.7M10 10l1.7 1.7M11.7 4.3 10 6M6 10l-1.7 1.7" />
  </Icon>
);
