import React from 'react';
import DashboardIcon from '@material-ui/icons/Dashboard';
import { navigation } from '../../navigation'

export const secondaryMenuContent = {
    header: "Dashboard",
    menus: [
      {
        icon: <DashboardIcon />,
        path: navigation.channelNew,
        text: "Create new channel"
      }
    ]
  }
  