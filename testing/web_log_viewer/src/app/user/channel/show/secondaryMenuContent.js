import React from 'react';
import DashboardIcon from '@material-ui/icons/Dashboard';
import { navigation } from '../../../navigation'

export const secondaryMenuContent = {
    header: "Header",
    menus: [
        {
            icon: <DashboardIcon />,
            path: navigation.dashboard,
            text: "Dashboard"
        }
    ]
}