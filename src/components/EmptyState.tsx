import type { ReactNode } from "react";
import { Icon, type IconName } from "./Icon";

export function EmptyState({ title, description, icon = "download", action }: { title: string; description: string; icon?: IconName; action?: ReactNode }) {
  return <div className="empty-state">
    <div className="empty-state-icon"><Icon name={icon} size={30} /></div>
    <h3>{title}</h3><p>{description}</p>{action}
  </div>;
}
