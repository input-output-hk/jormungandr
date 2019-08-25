import React from 'react';
import ChannelErrorPage from './ChannelErrorPage';
import ChannelConfirmationPage from './ChannelConfirmationPage';
import ChannelConfirmationLoadingPage from './loadingConfirmation/ChannelConfirmationLoadingPage';

class Confirmation extends React.Component {

    render() {
        console.log(this)
        if (this.props.isLoading) {
            return <ChannelConfirmationLoadingPage channelData={this.channelData} isLoading={true} />
        }
        if (this.props.error) {
            return <ChannelErrorPage channelData={this.channelData} isLoading={false} />
        }
        return <ChannelConfirmationPage channelData={this.channelData} isLoading={false} />
    }

}

export default Confirmation;